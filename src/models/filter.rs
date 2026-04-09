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
    ExpiryAsc,
    PlatformAsc,
}

impl SortOrder {
    pub fn next(&self) -> Self {
        match self {
            SortOrder::PurchaseDateDesc => SortOrder::PurchaseDateAsc,
            SortOrder::PurchaseDateAsc => SortOrder::NameAsc,
            SortOrder::NameAsc => SortOrder::NameDesc,
            SortOrder::NameDesc => SortOrder::ExpiryAsc,
            SortOrder::ExpiryAsc => SortOrder::PlatformAsc,
            SortOrder::PlatformAsc => SortOrder::PurchaseDateDesc,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            SortOrder::PurchaseDateDesc => "Purchase↓",
            SortOrder::PurchaseDateAsc => "Purchase↑",
            SortOrder::NameAsc => "Name A-Z",
            SortOrder::NameDesc => "Name Z-A",
            SortOrder::ExpiryAsc => "Expiry↑",
            SortOrder::PlatformAsc => "Platform",
        }
    }
}
