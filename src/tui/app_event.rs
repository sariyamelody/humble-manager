use chrono::{DateTime, Utc};
use crossterm::event::Event;
use crate::models::{choice::ChoicePick, key::GameKey, metadata::GameMetadata};

#[derive(Debug)]
pub enum AppEvent {
    /// 250ms tick for expiry countdown re-render
    Tick,
    /// Keyboard / mouse input from crossterm
    Input(Event),
    /// All order gamekeys fetched from Humble
    OrderRefsLoaded(Vec<String>),
    /// One order's bundle + keys fetched and stored to DB
    OrderLoaded { keys: Vec<GameKey> },
    /// Current Choice picks fetched
    ChoicePicksLoaded { month: String, picks: Vec<ChoicePick> },
    /// Sync progress update (for status bar)
    SyncProgress { done: u32, total: u32, label: String },
    /// Non-fatal sync error
    SyncError(String),
    /// Initial data loaded from local cache (fast path on startup)
    CacheLoaded { keys: Vec<GameKey>, picks: Vec<ChoicePick> },
    /// Last full sync timestamp loaded from DB (None = never synced)
    SyncStateLoaded(Option<DateTime<Utc>>),
    /// All cached game metadata loaded on startup
    AllMetadataLoaded(Vec<GameMetadata>),
    /// One game's metadata was enriched and saved
    MetadataEnriched(GameMetadata),
    /// Progress update during a metadata sync
    MetadataProgress { done: u32, total: u32 },
    /// Metadata sync finished
    MetadataSyncComplete,
}

#[derive(Debug)]
pub enum Cmd {
    /// Fetch everything from Humble from scratch
    StartFullSync,
    /// Enrich all items with Steam/IGDB metadata (separate from Humble sync)
    StartMetadataSync,
    /// Export currently filtered view to CSV at the given path
    ExportCsv(std::path::PathBuf),
    /// Shut down
    Quit,
}
