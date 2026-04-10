use std::collections::HashSet;
use super::key::{Platform, RedeemStatus};

#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub search_query: String,
    /// Empty = show all platforms
    pub platforms: HashSet<Platform>,
    pub redeem_status: Option<RedeemStatus>,
    pub sort: SortOrder,
    pub show_expired: bool,
    pub source: SourceFilter,
    /// Empty = show all genres/tags. Non-empty = item must have at least one matching tag.
    pub genre_filter: HashSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SourceFilter {
    /// Show both regular keys and Choice picks
    #[default]
    All,
    /// Regular bundle keys only
    Keys,
    /// Humble Choice picks only
    Choice,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    PurchaseDateDesc,
    PurchaseDateAsc,
    NameAsc,
    NameDesc,
    BundleAsc,
    BundleDesc,
    ExpiryAsc,
    PlatformAsc,
    MetacriticDesc,
    UserRatingDesc,
}

impl SortOrder {
    pub fn all() -> &'static [SortOrder] {
        use SortOrder::*;
        &[
            PurchaseDateDesc,
            PurchaseDateAsc,
            NameAsc,
            NameDesc,
            BundleAsc,
            BundleDesc,
            ExpiryAsc,
            PlatformAsc,
            MetacriticDesc,
            UserRatingDesc,
        ]
    }

    pub fn next(&self) -> Self {
        match self {
            SortOrder::PurchaseDateDesc => SortOrder::PurchaseDateAsc,
            SortOrder::PurchaseDateAsc => SortOrder::NameAsc,
            SortOrder::NameAsc => SortOrder::NameDesc,
            SortOrder::NameDesc => SortOrder::BundleAsc,
            SortOrder::BundleAsc => SortOrder::BundleDesc,
            SortOrder::BundleDesc => SortOrder::ExpiryAsc,
            SortOrder::ExpiryAsc => SortOrder::PlatformAsc,
            SortOrder::PlatformAsc => SortOrder::MetacriticDesc,
            SortOrder::MetacriticDesc => SortOrder::UserRatingDesc,
            SortOrder::UserRatingDesc => SortOrder::PurchaseDateDesc,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            SortOrder::PurchaseDateDesc => "Purchase↓",
            SortOrder::PurchaseDateAsc => "Purchase↑",
            SortOrder::NameAsc => "Name A-Z",
            SortOrder::NameDesc => "Name Z-A",
            SortOrder::BundleAsc => "Bundle A-Z",
            SortOrder::BundleDesc => "Bundle Z-A",
            SortOrder::ExpiryAsc => "Expiry↑",
            SortOrder::PlatformAsc => "Platform",
            SortOrder::MetacriticDesc => "Metacritic↓",
            SortOrder::UserRatingDesc => "User%↓",
        }
    }
}
