use std::sync::Arc;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use tokio::sync::{Mutex, oneshot};
use chrono::{DateTime, Utc, TimeZone};

use crate::models::{
    bundle::{Bundle, BundleType},
    choice::ChoicePick,
    key::{GameKey, Platform, RedeemStatus},
};

use super::migrations;

/// Messages the DB actor receives over its channel
pub enum DbMsg {
    UpsertBundle(Bundle, oneshot::Sender<Result<()>>),
    UpsertGameKey(GameKey, oneshot::Sender<Result<()>>),
    UpsertChoicePick(ChoicePick, oneshot::Sender<Result<()>>),
    LoadAllKeys(oneshot::Sender<Result<Vec<GameKey>>>),
    LoadAllChoicePicks(oneshot::Sender<Result<Vec<ChoicePick>>>),
    LoadAllBundles(oneshot::Sender<Result<Vec<Bundle>>>),
    KnownGamekeyMachineNames(oneshot::Sender<Result<Vec<String>>>),
    UpdateSyncState { resource: String, status: String, error: Option<String>, sender: oneshot::Sender<Result<()>> },
    LoadSyncState { resource: String, sender: oneshot::Sender<Result<Option<chrono::DateTime<Utc>>>> },
}

/// Wraps the SQLite connection; all access goes through `send()`.
#[derive(Clone)]
pub struct Db {
    tx: tokio::sync::mpsc::Sender<DbMsg>,
}

impl Db {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening database at {}", path.display()))?;
        migrations::run(&conn)?;

        // Tune SQLite for performance
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;
        ")?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<DbMsg>(256);

        // Spawn the DB actor on the tokio blocking pool (rusqlite is !Send across threads
        // but we keep it in a single-threaded actor).
        tokio::task::spawn_blocking(move || {
            while let Some(msg) = rx.blocking_recv() {
                handle_msg(&conn, msg);
            }
        });

        Ok(Self { tx })
    }

    pub async fn upsert_bundle(&self, bundle: Bundle) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::UpsertBundle(bundle, tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn upsert_game_key(&self, key: GameKey) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::UpsertGameKey(key, tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn upsert_choice_pick(&self, pick: ChoicePick) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::UpsertChoicePick(pick, tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn load_all_keys(&self) -> Result<Vec<GameKey>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::LoadAllKeys(tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn load_all_choice_picks(&self) -> Result<Vec<ChoicePick>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::LoadAllChoicePicks(tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn load_all_bundles(&self) -> Result<Vec<Bundle>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::LoadAllBundles(tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn known_gamekey_machine_names(&self) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::KnownGamekeyMachineNames(tx)).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn load_sync_state(&self, resource: String) -> Result<Option<chrono::DateTime<Utc>>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::LoadSyncState { resource, sender: tx }).await.ok();
        rx.await.context("db actor gone")?
    }

    pub async fn update_sync_state(&self, resource: String, status: String, error: Option<String>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(DbMsg::UpdateSyncState { resource, status, error, sender: tx }).await.ok();
        rx.await.context("db actor gone")?
    }
}

fn handle_msg(conn: &Connection, msg: DbMsg) {
    match msg {
        DbMsg::UpsertBundle(b, tx) => { let _ = tx.send(upsert_bundle(conn, &b)); }
        DbMsg::UpsertGameKey(k, tx) => { let _ = tx.send(upsert_game_key(conn, &k)); }
        DbMsg::UpsertChoicePick(p, tx) => { let _ = tx.send(upsert_choice_pick(conn, &p)); }
        DbMsg::LoadAllKeys(tx) => { let _ = tx.send(load_all_keys(conn)); }
        DbMsg::LoadAllChoicePicks(tx) => { let _ = tx.send(load_all_choice_picks(conn)); }
        DbMsg::LoadAllBundles(tx) => { let _ = tx.send(load_all_bundles(conn)); }
        DbMsg::KnownGamekeyMachineNames(tx) => { let _ = tx.send(known_gamekey_machine_names(conn)); }
        DbMsg::UpdateSyncState { resource, status, error, sender } => {
            let _ = sender.send(update_sync_state(conn, &resource, &status, error.as_deref()));
        }
        DbMsg::LoadSyncState { resource, sender } => {
            let _ = sender.send(load_sync_state(conn, &resource));
        }
    }
}

fn upsert_bundle(conn: &Connection, b: &Bundle) -> Result<()> {
    conn.execute(
        "INSERT INTO bundles (machine_name, human_name, product_machine_name, purchased_at, bundle_type, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(machine_name) DO UPDATE SET
           human_name = excluded.human_name,
           cached_at  = excluded.cached_at",
        params![
            b.machine_name,
            b.human_name,
            b.product_machine_name,
            b.purchased_at.timestamp(),
            b.bundle_type.as_str(),
            b.cached_at.timestamp(),
        ],
    )?;
    Ok(())
}

fn upsert_game_key(conn: &Connection, k: &GameKey) -> Result<()> {
    let genres_json = serde_json::to_string(&k.igdb_genres)?;
    conn.execute(
        "INSERT INTO game_keys (
            id, tpkd_machine_name, human_name, platform, key_type,
            redeemed_key_val, is_revealed, redeem_status,
            bundle_machine_name, bundle_human_name, purchase_date,
            expiry_date, steam_app_id, igdb_genres, is_owned_on_steam,
            created_at, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?16)
         ON CONFLICT(tpkd_machine_name) DO UPDATE SET
           human_name          = excluded.human_name,
           redeemed_key_val    = COALESCE(excluded.redeemed_key_val, game_keys.redeemed_key_val),
           is_revealed         = excluded.is_revealed,
           redeem_status       = excluded.redeem_status,
           expiry_date         = excluded.expiry_date,
           is_owned_on_steam   = COALESCE(excluded.is_owned_on_steam, game_keys.is_owned_on_steam),
           igdb_genres         = CASE WHEN excluded.igdb_genres = '[]' THEN game_keys.igdb_genres ELSE excluded.igdb_genres END,
           updated_at          = excluded.updated_at",
        params![
            k.id,
            k.tpkd_machine_name,
            k.human_name,
            serde_json::to_string(&k.platform)?,
            k.key_type,
            k.redeemed_key_val,
            k.is_revealed as i32,
            k.redeem_status.as_str(),
            k.bundle_machine_name,
            k.bundle_human_name,
            k.purchase_date.timestamp(),
            k.expiry_date.map(|d| d.timestamp()),
            k.steam_app_id,
            genres_json,
            k.is_owned_on_steam.map(|b| b as i32),
            Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

fn upsert_choice_pick(conn: &Connection, p: &ChoicePick) -> Result<()> {
    let genres_json = serde_json::to_string(&p.genres)?;
    conn.execute(
        "INSERT INTO choice_picks (
            machine_name, human_name, platform, steam_app_id, genres,
            claim_deadline, num_days_until_expired, is_expired,
            is_owned_on_steam, choice_month, cached_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(machine_name) DO UPDATE SET
           human_name             = excluded.human_name,
           genres                 = excluded.genres,
           claim_deadline         = excluded.claim_deadline,
           num_days_until_expired = excluded.num_days_until_expired,
           is_expired             = excluded.is_expired,
           is_owned_on_steam      = COALESCE(excluded.is_owned_on_steam, choice_picks.is_owned_on_steam),
           cached_at              = excluded.cached_at",
        params![
            p.machine_name,
            p.human_name,
            serde_json::to_string(&p.platform)?,
            p.steam_app_id,
            genres_json,
            p.claim_deadline.map(|d| d.timestamp()),
            p.num_days_until_expired,
            p.is_expired as i32,
            p.is_owned_on_steam.map(|b| b as i32),
            p.choice_month,
            Utc::now().timestamp(),
        ],
    )?;
    Ok(())
}

fn load_all_keys(conn: &Connection) -> Result<Vec<GameKey>> {
    let mut stmt = conn.prepare(
        "SELECT id, tpkd_machine_name, human_name, platform, key_type,
                redeemed_key_val, is_revealed, redeem_status,
                bundle_machine_name, bundle_human_name, purchase_date,
                expiry_date, steam_app_id, igdb_genres, is_owned_on_steam
         FROM game_keys
         ORDER BY purchase_date DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let platform_str: String = row.get(3)?;
        let redeem_status_str: String = row.get(7)?;
        let igdb_genres_str: String = row.get(13)?;
        let purchase_ts: i64 = row.get(10)?;
        let expiry_ts: Option<i64> = row.get(11)?;
        let is_owned: Option<i32> = row.get(14)?;

        Ok((
            row.get::<_, String>(0)?,  // id
            row.get::<_, String>(1)?,  // tpkd_machine_name
            row.get::<_, String>(2)?,  // human_name
            platform_str,
            row.get::<_, String>(4)?,  // key_type
            row.get::<_, Option<String>>(5)?,  // redeemed_key_val
            row.get::<_, i32>(6)?,     // is_revealed
            redeem_status_str,
            row.get::<_, String>(8)?,  // bundle_machine_name
            row.get::<_, String>(9)?,  // bundle_human_name
            purchase_ts,
            expiry_ts,
            row.get::<_, Option<i32>>(12)?, // steam_app_id
            igdb_genres_str,
            is_owned,
        ))
    })?;

    let mut keys = Vec::new();
    for row in rows {
        let (id, tpkd, human_name, platform_str, key_type,
             redeemed_key_val, is_revealed, redeem_status_str,
             bundle_machine_name, bundle_human_name, purchase_ts,
             expiry_ts, steam_app_id, igdb_genres_str, is_owned) = row?;

        let platform: Platform = serde_json::from_str(&platform_str)
            .unwrap_or_else(|_| Platform::from_str(&platform_str));
        let igdb_genres: Vec<String> = serde_json::from_str(&igdb_genres_str).unwrap_or_default();

        keys.push(GameKey {
            id,
            tpkd_machine_name: tpkd,
            human_name,
            platform,
            key_type,
            redeemed_key_val,
            is_revealed: is_revealed != 0,
            redeem_status: RedeemStatus::from_str(&redeem_status_str),
            bundle_machine_name,
            bundle_human_name,
            purchase_date: Utc.timestamp_opt(purchase_ts, 0).single().unwrap_or_default(),
            expiry_date: expiry_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
            steam_app_id: steam_app_id.map(|id| id as u32),
            igdb_genres,
            is_owned_on_steam: is_owned.map(|v| v != 0),
        });
    }
    Ok(keys)
}

fn load_all_choice_picks(conn: &Connection) -> Result<Vec<ChoicePick>> {
    let mut stmt = conn.prepare(
        "SELECT machine_name, human_name, platform, steam_app_id, genres,
                claim_deadline, num_days_until_expired, is_expired,
                is_owned_on_steam, choice_month
         FROM choice_picks
         ORDER BY choice_month DESC, human_name",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<i32>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<i32>>(6)?,
            row.get::<_, i32>(7)?,
            row.get::<_, Option<i32>>(8)?,
            row.get::<_, String>(9)?,
        ))
    })?;

    let mut picks = Vec::new();
    for row in rows {
        let (machine_name, human_name, platform_str, steam_app_id, genres_str,
             claim_ts, days_until_exp, is_expired, is_owned, choice_month) = row?;

        let platform: Platform = serde_json::from_str(&platform_str)
            .unwrap_or_else(|_| Platform::from_str(&platform_str));
        let genres: Vec<String> = serde_json::from_str(&genres_str).unwrap_or_default();

        picks.push(ChoicePick {
            machine_name,
            human_name,
            platform,
            steam_app_id: steam_app_id.map(|id| id as u32),
            genres,
            claim_deadline: claim_ts.and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
            num_days_until_expired: days_until_exp,
            is_expired: is_expired != 0,
            is_owned_on_steam: is_owned.map(|v| v != 0),
            choice_month,
        });
    }
    Ok(picks)
}

fn load_all_bundles(conn: &Connection) -> Result<Vec<Bundle>> {
    let mut stmt = conn.prepare(
        "SELECT machine_name, human_name, product_machine_name, purchased_at, bundle_type, cached_at
         FROM bundles ORDER BY purchased_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;

    let mut bundles = Vec::new();
    for row in rows {
        let (machine_name, human_name, product_machine_name, purchased_ts, bundle_type_str, cached_ts) = row?;
        bundles.push(Bundle {
            machine_name,
            human_name,
            product_machine_name,
            purchased_at: Utc.timestamp_opt(purchased_ts, 0).single().unwrap_or_default(),
            bundle_type: BundleType::from_str(&bundle_type_str),
            cached_at: Utc.timestamp_opt(cached_ts, 0).single().unwrap_or_default(),
        });
    }
    Ok(bundles)
}

fn known_gamekey_machine_names(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT tpkd_machine_name FROM game_keys")?;
    let names: Result<Vec<String>, _> = stmt.query_map([], |row| row.get(0))?.collect();
    Ok(names?)
}

fn load_sync_state(conn: &Connection, resource: &str) -> Result<Option<chrono::DateTime<Utc>>> {
    let mut stmt = conn.prepare(
        "SELECT last_synced_at FROM sync_state WHERE resource = ?1 LIMIT 1"
    )?;
    let ts: Option<i64> = stmt.query_row(params![resource], |row| row.get(0)).ok();
    Ok(ts.and_then(|t| Utc.timestamp_opt(t, 0).single()))
}

fn update_sync_state(conn: &Connection, resource: &str, status: &str, error: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO sync_state (resource, last_synced_at, status, error_message)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(resource) DO UPDATE SET
           last_synced_at = excluded.last_synced_at,
           status         = excluded.status,
           error_message  = excluded.error_message",
        params![resource, Utc::now().timestamp(), status, error],
    )?;
    Ok(())
}
