use crossterm::event::Event;
use crate::models::{bundle::Bundle, choice::ChoicePick, key::GameKey};

#[derive(Debug)]
pub enum AppEvent {
    /// 250ms tick for expiry countdown re-render
    Tick,
    /// Keyboard / mouse input from crossterm
    Input(Event),
    /// All order gamekeys fetched from Humble
    OrderRefsLoaded(Vec<String>),
    /// One order's bundle + keys fetched and stored to DB
    OrderLoaded { bundle: Bundle, keys: Vec<GameKey> },
    /// Current Choice picks fetched
    ChoicePicksLoaded { month: String, picks: Vec<ChoicePick> },
    /// Sync progress update (for status bar)
    SyncProgress { done: u32, total: u32, label: String },
    /// Non-fatal sync error
    SyncError(String),
    /// Initial data loaded from local cache (fast path on startup)
    CacheLoaded { keys: Vec<GameKey>, picks: Vec<ChoicePick> },
}

#[derive(Debug)]
pub enum Cmd {
    /// Fetch everything from Humble from scratch
    StartFullSync,
    /// Export currently filtered view to CSV at the given path
    ExportCsv(std::path::PathBuf),
    /// Shut down
    Quit,
}
