use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    bundle::{Bundle, BundleType},
    key::{GameKey, Platform, RedeemStatus},
};

use super::client::HumbleClient;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OrderRef {
    gamekey: String,
}

#[derive(Debug, Deserialize)]
struct OrderDetail {
    gamekey: String,
    created: Option<String>,
    product: OrderProduct,
    #[serde(default)]
    tpkd_dict: Option<TpkdDict>,
}

#[derive(Debug, Deserialize)]
struct OrderProduct {
    machine_name: String,
    human_name: String,
    #[serde(default)]
    category: String,
    /// Present on Choice subscription orders — the URL slug for the membership page
    /// e.g. "april-2025" → /membership/april-2025
    #[serde(default)]
    choice_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TpkdDict {
    #[serde(default)]
    all_tpks: Vec<TpkEntry>,
}

#[derive(Debug, Deserialize)]
struct TpkEntry {
    machine_name: String,
    human_name: String,
    key_type: Option<String>,
    key_type_human_name: Option<String>,
    /// Present and non-null when the key has been revealed by the user
    redeemed_key_val: Option<String>,
    /// If present the key has expired
    #[serde(default)]
    is_expired: bool,
    steam_app_id: Option<serde_json::Value>,
    #[serde(default)]
    num_days_until_expired: i32,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Fetch the list of all order gamekeys for the authenticated user.
pub async fn fetch_order_refs(client: &HumbleClient) -> Result<Vec<String>> {
    let refs: Vec<OrderRef> = client
        .client()
        .get("https://www.humblebundle.com/api/v1/user/order")
        .send()
        .await
        .context("fetching order list")?
        .error_for_status()
        .context("order list HTTP error")?
        .json()
        .await
        .context("parsing order list JSON")?;

    Ok(refs.into_iter().map(|r| r.gamekey).collect())
}

/// Fetch a single order and convert it into a Bundle + Vec<GameKey>.
/// Also returns `Some(choice_url)` when the order is a Choice subscription month
/// (e.g. `Some("april-2025")`), so the caller can fetch the membership page.
pub async fn fetch_order(
    client: &HumbleClient,
    gamekey: &str,
) -> Result<(Bundle, Vec<GameKey>, Option<String>)> {
    let url = format!(
        "https://www.humblebundle.com/api/v1/order/{}?all_tpkds=true",
        gamekey
    );

    let detail: OrderDetail = client
        .client()
        .get(&url)
        .send()
        .await
        .with_context(|| format!("fetching order {}", gamekey))?
        .error_for_status()
        .with_context(|| format!("order {} HTTP error", gamekey))?
        .json()
        .await
        .with_context(|| format!("parsing order {} JSON", gamekey))?;

    let purchase_date = detail
        .created
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_default();

    let choice_url = detail.product.choice_url.clone();

    let bundle = Bundle {
        machine_name: detail.gamekey.clone(),
        human_name: detail.product.human_name.clone(),
        product_machine_name: detail.product.machine_name.clone(),
        purchased_at: purchase_date,
        bundle_type: BundleType::from_str(&detail.product.category),
        cached_at: Utc::now(),
    };

    let keys = match detail.tpkd_dict {
        None => vec![],
        Some(tpkd) => tpkd
            .all_tpks
            .into_iter()
            .map(|t| tpk_to_game_key(t, &bundle))
            .collect(),
    };

    Ok((bundle, keys, choice_url))
}

fn tpk_to_game_key(t: TpkEntry, bundle: &Bundle) -> GameKey {
    let is_revealed = t.redeemed_key_val.is_some();

    let redeem_status = if t.is_expired {
        RedeemStatus::Expired
    } else if is_revealed {
        // Having revealed the key doesn't mean it's redeemed — keep as Unredeemed
        // (we can't know from the API whether it was actually redeemed on the platform).
        // Use Redeemed only if there's no key val and the key type is direct-redeem.
        RedeemStatus::Unredeemed
    } else {
        RedeemStatus::Unredeemed
    };

    // steam_app_id can be a JSON number or null
    let steam_app_id = t.steam_app_id.as_ref().and_then(|v| v.as_u64()).map(|n| n as u32);

    let platform = Platform::from_str(
        t.key_type.as_deref().unwrap_or(""),
    );

    GameKey {
        id: Uuid::new_v4().to_string(),
        tpkd_machine_name: t.machine_name,
        human_name: t.human_name,
        platform,
        key_type: t.key_type_human_name.unwrap_or_default(),
        redeemed_key_val: t.redeemed_key_val,
        is_revealed,
        redeem_status,
        bundle_machine_name: bundle.machine_name.clone(),
        bundle_human_name: bundle.human_name.clone(),
        purchase_date: bundle.purchased_at,
        expiry_date: None, // num_days_until_expired=-1 means no expiry; we don't have an absolute date here
        steam_app_id,
        igdb_genres: vec![],
        is_owned_on_steam: None,
    }
}
