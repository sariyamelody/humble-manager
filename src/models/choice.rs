use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::key::Platform;

/// A game available through the Humble Choice subscription.
/// Unlike GameKey, a ChoicePick has no redeemable key yet — the user must
/// claim it on the Humble website first, after which it appears as a GameKey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoicePick {
    pub machine_name: String,
    pub human_name: String,
    pub platform: Platform,
    pub steam_app_id: Option<u32>,
    /// Genres come directly from Humble Choice data — no IGDB needed
    pub genres: Vec<String>,
    pub claim_deadline: Option<DateTime<Utc>>,
    pub num_days_until_expired: Option<i32>,
    pub is_expired: bool,
    pub is_owned_on_steam: Option<bool>,
    /// Which Choice month this belongs to (e.g. "2025-03")
    pub choice_month: String,
}
