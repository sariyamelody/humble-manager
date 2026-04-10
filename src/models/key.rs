use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameKey {
    pub id: String,
    /// Humble's opaque key ID — unique across all keys
    pub tpkd_machine_name: String,
    pub human_name: String,
    pub platform: Platform,
    pub key_type: String,
    /// None until the user reveals the key
    pub redeemed_key_val: Option<String>,
    pub is_revealed: bool,
    pub redeem_status: RedeemStatus,
    /// FK -> Bundle.machine_name
    pub bundle_machine_name: String,
    /// Denormalized for display without a join
    pub bundle_human_name: String,
    pub purchase_date: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub steam_app_id: Option<u32>,
    /// From IGDB enrichment; stored as JSON in SQLite
    pub igdb_genres: Vec<String>,
    pub is_owned_on_steam: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Steam,
    Gog,
    EpicGames,
    Itch,
    DrmFree,
    HumbleApp,
    Ubisoft,
    BattleNet,
    Other(String),
}

impl Platform {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "steam" => Platform::Steam,
            "gog" | "gog_game" => Platform::Gog,
            "epic" | "epic_games" | "epicgames" => Platform::EpicGames,
            "itch" | "itch.io" => Platform::Itch,
            "download" | "drm_free" | "drmfree" => Platform::DrmFree,
            "humble_app" | "humble" => Platform::HumbleApp,
            "ubisoft" | "uplay" => Platform::Ubisoft,
            "battlenet" | "battle.net" => Platform::BattleNet,
            other => Platform::Other(other.to_string()),
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            Platform::Steam => "STM",
            Platform::Gog => "GOG",
            Platform::EpicGames => "EPC",
            Platform::Itch => "ITCH",
            Platform::DrmFree => "DRM",
            Platform::HumbleApp => "HMB",
            Platform::Ubisoft => "UBI",
            Platform::BattleNet => "BTN",
            Platform::Other(_) => "???",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Platform::Steam => "Steam",
            Platform::Gog => "GOG",
            Platform::EpicGames => "Epic",
            Platform::Itch => "Itch.io",
            Platform::DrmFree => "DRM-free",
            Platform::HumbleApp => "Humble App",
            Platform::Ubisoft => "Ubisoft",
            Platform::BattleNet => "Battle.net",
            Platform::Other(s) => s.as_str(),
        }
    }
}

impl Platform {
    /// Returns a store search/page URL for this platform given a game name,
    /// or None for platforms with no useful public store URL.
    pub fn store_url(&self, name: &str, steam_app_id: Option<u32>) -> Option<String> {
        let q = urlencoding::encode(name);
        match self {
            Platform::Steam => Some(
                steam_app_id
                    .map(|id| format!("https://store.steampowered.com/app/{}", id))
                    .unwrap_or_else(|| format!("https://store.steampowered.com/search/?term={}", q))
            ),
            Platform::Gog => Some(format!("https://www.gog.com/en/games?query={}", q)),
            Platform::EpicGames => Some(format!("https://store.epicgames.com/en-US/browse?q={}", q)),
            Platform::Ubisoft => Some(format!("https://store.ubisoft.com/en-gb/search?searchText={}", q)),
            Platform::Itch => Some(format!("https://itch.io/search?q={}", q)),
            Platform::BattleNet => Some("https://us.battle.net/shop/".to_string()),
            // For DRM-free, Humble App, and unrecognised platforms fall back to a Steam
            // search — most Humble games are on Steam and this is a useful safe default.
            Platform::DrmFree | Platform::HumbleApp | Platform::Other(_) => {
                Some(format!("https://store.steampowered.com/search/?term={}", q))
            }
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RedeemStatus {
    #[default]
    Unredeemed,
    Redeemed,
    Expired,
    Unknown,
}

impl RedeemStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "redeemed" => RedeemStatus::Redeemed,
            "expired" => RedeemStatus::Expired,
            "unredeemed" => RedeemStatus::Unredeemed,
            _ => RedeemStatus::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            RedeemStatus::Unredeemed => "unredeemed",
            RedeemStatus::Redeemed => "redeemed",
            RedeemStatus::Expired => "expired",
            RedeemStatus::Unknown => "unknown",
        }
    }
}
