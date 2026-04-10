use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub machine_name: String,
    pub human_name: String,
    pub product_machine_name: String,
    pub purchased_at: DateTime<Utc>,
    pub bundle_type: BundleType,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleType {
    Classic,
    Monthly,
    Choice,
    Book,
    Software,
    Unknown,
}

impl BundleType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "classic" => BundleType::Classic,
            "monthly" | "humble_monthly" => BundleType::Monthly,
            "choice" | "humble_choice" => BundleType::Choice,
            "book" | "books" => BundleType::Book,
            "software" => BundleType::Software,
            _ => BundleType::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            BundleType::Classic => "classic",
            BundleType::Monthly => "monthly",
            BundleType::Choice => "choice",
            BundleType::Book => "book",
            BundleType::Software => "software",
            BundleType::Unknown => "unknown",
        }
    }
}
