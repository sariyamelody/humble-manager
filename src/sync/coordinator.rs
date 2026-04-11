use std::collections::HashSet;
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, warn};

use crate::{
    api::{
        client::HumbleClient,
        humble::{fetch_order, fetch_order_refs},
        humble_choice::fetch_choice_picks_for_url,
        igdb::{fetch_igdb_by_steam_id, fetch_igdb_token},
        steam::fetch_steam_metadata,
    },
    config::Config,
    db::Db,
    models::metadata::GameMetadata,
    tui::app_event::{AppEvent, Cmd},
};

/// Runs until the event_tx is dropped or Cmd::Quit is received.
pub async fn run(
    config: Config,
    db: Db,
    mut cmd_rx: mpsc::Receiver<Cmd>,
    event_tx: mpsc::Sender<AppEvent>,
) {
    load_cache(&db, &event_tx).await;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Cmd::StartFullSync => {
                let config = config.clone();
                let db = db.clone();
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    run_full_sync(config, db, tx).await;
                });
            }
            Cmd::StartMetadataSync => {
                let config = config.clone();
                let db = db.clone();
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    run_metadata_sync(config, db, tx).await;
                });
            }
            Cmd::ExportCsv(_) | Cmd::SaveColumns(_) => {
                // Handled directly in app.rs
            }
            Cmd::Quit => break,
        }
    }
}

async fn load_cache(db: &Db, tx: &mpsc::Sender<AppEvent>) {
    match tokio::join!(db.load_all_keys(), db.load_all_choice_picks()) {
        (Ok(keys), Ok(picks)) => {
            let _ = tx.send(AppEvent::CacheLoaded { keys, picks }).await;
        }
        (Err(e), _) | (_, Err(e)) => {
            warn!("Failed to load cache: {}", e);
        }
    }

    match db.load_all_game_metadata().await {
        Ok(meta) => { let _ = tx.send(AppEvent::AllMetadataLoaded(meta)).await; }
        Err(e) => warn!("Failed to load game metadata: {}", e),
    }

    match db.load_sync_state("full_sync".to_string()).await {
        Ok(last_synced) => {
            let _ = tx.send(AppEvent::SyncStateLoaded(last_synced)).await;
        }
        Err(e) => {
            warn!("Failed to load sync state: {}", e);
            let _ = tx.send(AppEvent::SyncStateLoaded(None)).await;
        }
    }
}

async fn run_metadata_sync(config: Config, db: Db, tx: mpsc::Sender<AppEvent>) {
    // Collect all unique steam_app_ids across keys and picks
    let (keys, picks) = match tokio::join!(db.load_all_keys(), db.load_all_choice_picks()) {
        (Ok(k), Ok(p)) => (k, p),
        _ => {
            let _ = tx.send(AppEvent::SyncError("Metadata sync: failed to load items".into())).await;
            return;
        }
    };

    let mut app_ids: Vec<u32> = keys.iter()
        .filter_map(|k| k.steam_app_id)
        .chain(picks.iter().filter_map(|p| p.steam_app_id))
        .collect::<HashSet<u32>>()
        .into_iter()
        .collect();
    app_ids.sort();

    if app_ids.is_empty() {
        let _ = tx.send(AppEvent::MetadataSyncComplete).await;
        return;
    }

    let total = app_ids.len() as u32;
    let _ = tx.send(AppEvent::MetadataProgress { done: 0, total }).await;

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .gzip(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AppEvent::SyncError(format!("Metadata sync: HTTP client error: {}", e))).await;
            return;
        }
    };

    // Fetch IGDB token once if credentials are configured
    let igdb_token = if !config.igdb.client_id.is_empty() && !config.igdb.client_secret.is_empty() {
        match fetch_igdb_token(&client, &config.igdb.client_id, &config.igdb.client_secret).await {
            Ok(t) => {
                info!("IGDB token fetched successfully");
                Some(t)
            }
            Err(e) => {
                warn!("IGDB token fetch failed (enrichment will use Steam only): {}", e);
                None
            }
        }
    } else {
        None
    };

    for (i, &app_id) in app_ids.iter().enumerate() {
        let steam = fetch_steam_metadata(&client, app_id).await.unwrap_or_else(|e| {
            warn!("Steam metadata failed for app {}: {}", app_id, e);
            crate::api::steam::SteamMetadata { genres: vec![], metacritic_score: None, tags: vec![], deck_compat: None, user_rating: None }
        });

        let (igdb_id, igdb_genres, igdb_rating) = if let Some(ref token) = igdb_token {
            match fetch_igdb_by_steam_id(&client, &config.igdb.client_id, token, app_id).await {
                Ok(Some(m)) => (Some(m.igdb_id), m.genres, m.rating),
                Ok(None) => (None, vec![], None),
                Err(e) => {
                    warn!("IGDB lookup failed for app {}: {}", app_id, e);
                    (None, vec![], None)
                }
            }
        } else {
            (None, vec![], None)
        };

        let meta = GameMetadata {
            steam_app_id: app_id,
            steam_tags: steam.tags,
            steam_genres: steam.genres,
            metacritic_score: steam.metacritic_score,
            steam_user_rating: steam.user_rating,
            igdb_id,
            igdb_genres,
            igdb_rating,
            steam_deck_compat: steam.deck_compat,
            enriched_at: chrono::Utc::now(),
        };

        if let Err(e) = db.upsert_game_metadata(meta.clone()).await {
            warn!("Failed to save metadata for app {}: {}", app_id, e);
        }

        let _ = tx.send(AppEvent::MetadataEnriched(meta)).await;
        let done = (i + 1) as u32;
        let _ = tx.send(AppEvent::MetadataProgress { done, total }).await;

        // Steam rate limit: ~200 req/5 min. 600ms gives ~100/min, comfortable headroom.
        // When IGDB is also active we've made two requests; the sleep covers both.
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    info!("Metadata sync complete ({} apps)", app_ids.len());
    let _ = db.update_sync_state("metadata_sync".to_string(), "ok".to_string(), None).await;
    let _ = tx.send(AppEvent::MetadataSyncComplete).await;
}

async fn run_full_sync(config: Config, db: Db, tx: mpsc::Sender<AppEvent>) {
    let session = config.auth.session_cookie.clone();

    let client = match HumbleClient::new(&session) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AppEvent::SyncError(format!("Failed to create HTTP client: {}", e))).await;
            return;
        }
    };

    // Fetch the current month's Choice picks immediately (fast path)
    fetch_and_store_choice(&client, "home", &db, &tx).await;

    // Fetch all order refs
    let order_refs = match fetch_order_refs(&client).await {
        Ok(refs) => refs,
        Err(e) => {
            let _ = tx.send(AppEvent::SyncError(format!("Failed to fetch orders: {}", e))).await;
            return;
        }
    };

    let total = order_refs.len() as u32;
    let _ = tx.send(AppEvent::OrderRefsLoaded(order_refs.clone())).await;

    // Fetch all orders concurrently (bounded by semaphore)
    let sem = Arc::new(Semaphore::new(config.sync.max_concurrent_requests));
    let completed = Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();

    for gamekey in order_refs.into_iter() {
        let sem = sem.clone();
        let completed = completed.clone();
        let session = session.clone();
        let tx = tx.clone();
        let db = db.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let client = match HumbleClient::new(&session) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(AppEvent::SyncError(format!("client error: {}", e))).await;
                    return;
                }
            };

            match fetch_order(&client, &gamekey).await {
                Ok((bundle, keys, choice_url)) => {
                    if let Err(e) = db.upsert_bundle(bundle.clone()).await {
                        warn!("Failed to upsert bundle {}: {}", bundle.machine_name, e);
                    }
                    for key in &keys {
                        if let Err(e) = db.upsert_game_key(key.clone()).await {
                            warn!("Failed to upsert key {}: {}", key.tpkd_machine_name, e);
                        }
                    }
                    let _ = tx.send(AppEvent::OrderLoaded { keys }).await;

                    if let Some(slug) = choice_url {
                        fetch_and_store_choice(&client, &slug, &db, &tx).await;
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch order {}: {}", gamekey, e);
                }
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = tx.send(AppEvent::SyncProgress {
                done,
                total,
                label: gamekey.clone(),
            }).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    info!("Full sync complete");
    let _ = db.update_sync_state("full_sync".to_string(), "ok".to_string(), None).await;
}

async fn fetch_and_store_choice(
    client: &HumbleClient,
    url_slug: &str,
    db: &Db,
    tx: &mpsc::Sender<AppEvent>,
) {
    match fetch_choice_picks_for_url(client, url_slug).await {
        Ok((month, picks)) => {
            info!("Fetched {} Choice picks for {} ({})", picks.len(), month, url_slug);
            for pick in &picks {
                if let Err(e) = db.upsert_choice_pick(pick.clone()).await {
                    warn!("Failed to upsert Choice pick {}: {}", pick.machine_name, e);
                }
            }
            let _ = tx.send(AppEvent::ChoicePicksLoaded { month, picks }).await;
        }
        Err(e) => {
            warn!("Choice picks fetch failed for {}: {}", url_slug, e);
        }
    }
}
