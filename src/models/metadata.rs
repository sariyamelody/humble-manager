use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteamDeckCompat {
    Unsupported,
    Playable,
    Verified,
}

impl SteamDeckCompat {
    pub fn from_category(n: i64) -> Option<Self> {
        match n {
            1 => Some(Self::Unsupported),
            2 => Some(Self::Playable),
            3 => Some(Self::Verified),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Self::Unsupported => 1,
            Self::Playable => 2,
            Self::Verified => 3,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Unsupported => "Unsupported",
            Self::Playable => "Playable",
            Self::Verified => "Verified",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameMetadata {
    pub steam_app_id: u32,
    /// User-voted folksonomy tags from SteamSpy, sorted by vote count
    pub steam_tags: Vec<String>,
    /// Valve-assigned genre labels from Steam appdetails
    pub steam_genres: Vec<String>,
    pub metacritic_score: Option<u32>,
    /// Steam user rating as a fraction 0.0–1.0, derived from SteamSpy positive/negative counts
    pub steam_user_rating: Option<f32>,
    pub igdb_id: Option<u64>,
    /// Standardized genre names from IGDB taxonomy
    pub igdb_genres: Vec<String>,
    /// IGDB aggregated critic rating (0–100), present when IGDB has ≥4 reviews
    pub igdb_rating: Option<f64>,
    pub steam_deck_compat: Option<SteamDeckCompat>,
    pub enriched_at: DateTime<Utc>,
}
