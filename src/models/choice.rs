use chrono::{DateTime, NaiveDate, Utc};
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
    /// Which Choice month this belongs to (e.g. "april_2025_choice")
    pub choice_month: String,
}

impl ChoicePick {
    /// Returns the first of the month this pick belongs to, derived from
    /// `choice_month` (e.g. "april_2025_choice" → 2025-04-01 00:00 UTC).
    pub fn month_date(&self) -> Option<DateTime<Utc>> {
        let slug = self.choice_month
            .strip_suffix("_choice")
            .unwrap_or(&self.choice_month);
        let mut parts = slug.splitn(2, '_');
        let month_name = parts.next()?;
        let year: i32 = parts.next()?.parse().ok()?;
        let month_num = match month_name {
            "january"   => 1,  "february" => 2,  "march"    => 3,
            "april"     => 4,  "may"       => 5,  "june"     => 6,
            "july"      => 7,  "august"    => 8,  "september"=> 9,
            "october"   => 10, "november"  => 11, "december" => 12,
            _ => return None,
        };
        NaiveDate::from_ymd_opt(year, month_num, 1)
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
    }
}
