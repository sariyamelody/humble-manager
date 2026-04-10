use std::collections::{HashMap, HashSet};
use ratatui::widgets::{ListState, TableState};

use crate::models::{
    choice::ChoicePick,
    filter::{FilterState, SortOrder, SourceFilter},
    key::{GameKey, RedeemStatus},
    metadata::GameMetadata,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Navigating the key list
    Normal,
    /// Typing a search query
    Search,
    /// Auth modal: paste session cookie
    Auth,
    /// Export path input
    ExportPrompt,
    /// Non-fatal error displayed
    Error,
    /// Suggestion to sync (shown when cache is stale or never synced)
    SyncPrompt,
    /// Genre/tag picker modal
    GenrePicker,
    /// Sort order picker modal
    SortPicker,
}

/// Sub-mode within the genre picker: navigating the list vs typing a search query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerSubMode {
    Navigate,
    Search,
}

/// Sort order for the genre/tag picker list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerSort {
    /// Most common first (default)
    CountDesc,
    /// Alphabetical A-Z
    NameAsc,
}

impl PickerSort {
    pub fn next(&self) -> Self {
        match self {
            PickerSort::CountDesc => PickerSort::NameAsc,
            PickerSort::NameAsc => PickerSort::CountDesc,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            PickerSort::CountDesc => "Count↓",
            PickerSort::NameAsc => "Name A-Z",
        }
    }
}

/// Type filter for the genre/tag picker: show all, genres only, or tags only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerTypeFilter {
    All,
    GenresOnly,
    TagsOnly,
}

impl PickerTypeFilter {
    pub fn next(&self) -> Self {
        match self {
            PickerTypeFilter::All => PickerTypeFilter::GenresOnly,
            PickerTypeFilter::GenresOnly => PickerTypeFilter::TagsOnly,
            PickerTypeFilter::TagsOnly => PickerTypeFilter::All,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            PickerTypeFilter::All => "All",
            PickerTypeFilter::GenresOnly => "Genres",
            PickerTypeFilter::TagsOnly => "Tags",
        }
    }
}

/// State for the genre/tag picker modal.
pub struct GenrePickerState {
    /// All (name, library_count, is_genre) tuples stored in canonical order (by count desc, then name).
    /// is_genre = true when the name appears as a steam_genre or igdb_genre for any library item.
    pub all_items: Vec<(String, u32, bool)>,
    /// Indices into all_items that pass the current search + type filter, in current sort order.
    pub filtered_indices: Vec<usize>,
    /// Cursor position within filtered_indices
    pub cursor: usize,
    /// Live search query typed in the modal
    pub search: String,
    /// Tags selected in this picker session (applied on Enter, discarded on Esc)
    pub pending_filter: HashSet<String>,
    pub list_state: ListState,
    pub sub_mode: PickerSubMode,
    pub sort: PickerSort,
    pub type_filter: PickerTypeFilter,
}

impl GenrePickerState {
    pub fn new(
        metadata_map: &HashMap<u32, crate::models::metadata::GameMetadata>,
        current_filter: &HashSet<String>,
    ) -> Self {
        // Count occurrences and track which names are genres vs user tags.
        // A name is a genre if it appears as steam_genre or igdb_genre for any item.
        let mut counts: HashMap<String, u32> = HashMap::new();
        let mut genre_names: HashSet<String> = HashSet::new();
        for meta in metadata_map.values() {
            for tag in &meta.steam_tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
            for g in meta.steam_genres.iter().chain(meta.igdb_genres.iter()) {
                *counts.entry(g.clone()).or_insert(0) += 1;
                genre_names.insert(g.clone());
            }
        }
        let mut all_items: Vec<(String, u32, bool)> = counts
            .into_iter()
            .map(|(name, count)| {
                let is_genre = genre_names.contains(&name);
                (name, count, is_genre)
            })
            .collect();
        all_items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let len = all_items.len();
        let mut list_state = ListState::default();
        if len > 0 { list_state.select(Some(0)); }

        let mut state = Self {
            filtered_indices: vec![],
            cursor: 0,
            search: String::new(),
            pending_filter: current_filter.clone(),
            all_items,
            list_state,
            sub_mode: PickerSubMode::Navigate,
            sort: PickerSort::CountDesc,
            type_filter: PickerTypeFilter::All,
        };
        state.apply_view();
        state
    }

    /// Rebuild `filtered_indices` applying search query, type filter, and sort order.
    pub fn apply_view(&mut self) {
        let q = self.search.to_lowercase();
        let mut indices: Vec<usize> = self.all_items.iter().enumerate()
            .filter(|(_, (_name, _, is_genre))| {
                // Type filter
                match self.type_filter {
                    PickerTypeFilter::All => true,
                    PickerTypeFilter::GenresOnly => *is_genre,
                    PickerTypeFilter::TagsOnly => !is_genre,
                }
            })
            .filter(|(_, (name, _, _))| q.is_empty() || name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        match self.sort {
            PickerSort::CountDesc => {
                // all_items is already sorted by count desc; preserve that order
                indices.sort_unstable();
                // Re-sort by count desc (stable relative to all_items order)
                indices.sort_by(|&a, &b| self.all_items[b].1.cmp(&self.all_items[a].1)
                    .then(self.all_items[a].0.cmp(&self.all_items[b].0)));
            }
            PickerSort::NameAsc => {
                indices.sort_by(|&a, &b| self.all_items[a].0.cmp(&self.all_items[b].0));
            }
        }

        self.filtered_indices = indices;
        self.cursor = 0;
        let sel = if self.filtered_indices.is_empty() { None } else { Some(0) };
        self.list_state.select(sel);
    }

    pub fn move_down(&mut self) {
        if self.filtered_indices.is_empty() { return; }
        self.cursor = (self.cursor + 1).min(self.filtered_indices.len() - 1);
        self.list_state.select(Some(self.cursor));
    }

    pub fn move_up(&mut self) {
        if self.filtered_indices.is_empty() { return; }
        self.cursor = self.cursor.saturating_sub(1);
        self.list_state.select(Some(self.cursor));
    }

    pub fn jump_top(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.cursor = 0;
            self.list_state.select(Some(0));
        }
    }

    pub fn jump_bottom(&mut self) {
        let len = self.filtered_indices.len();
        if len > 0 {
            self.cursor = len - 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn toggle_current(&mut self) {
        if let Some(&item_idx) = self.filtered_indices.get(self.cursor) {
            let tag = self.all_items[item_idx].0.clone();  // .0 = name regardless of tuple size
            if self.pending_filter.contains(&tag) {
                self.pending_filter.remove(&tag);
            } else {
                self.pending_filter.insert(tag);
            }
        }
    }
}

/// A unified row that can be either a regular key or a Choice pick
#[derive(Debug, Clone)]
pub enum ListItem {
    Key(GameKey),
    Choice(ChoicePick),
}

impl ListItem {
    pub fn human_name(&self) -> &str {
        match self {
            ListItem::Key(k) => &k.human_name,
            ListItem::Choice(p) => &p.human_name,
        }
    }

    pub fn platform_label(&self) -> &str {
        match self {
            ListItem::Key(k) => k.platform.short_label(),
            ListItem::Choice(p) => p.platform.short_label(),
        }
    }

    pub fn status_label(&self) -> &str {
        match self {
            ListItem::Key(k) => match k.redeem_status {
                RedeemStatus::Redeemed => "●",
                RedeemStatus::Unredeemed => "○",
                RedeemStatus::Expired => "✗",
                RedeemStatus::Unknown => "?",
            },
            ListItem::Choice(_) => "⊕", // unclaimed
        }
    }

    pub fn bundle_name(&self) -> &str {
        match self {
            ListItem::Key(k) => &k.bundle_human_name,
            ListItem::Choice(p) => &p.choice_month,
        }
    }

    pub fn is_choice(&self) -> bool {
        matches!(self, ListItem::Choice(_))
    }

    /// A stable unique identifier for this item, used to preserve cursor position across sorts.
    fn stable_id(&self) -> &str {
        match self {
            ListItem::Key(k) => &k.tpkd_machine_name,
            ListItem::Choice(p) => &p.machine_name,
        }
    }
}

pub struct UiState {
    pub mode: Mode,
    pub filter: FilterState,
    /// All keys loaded (from cache or sync)
    pub all_keys: Vec<GameKey>,
    /// All Choice picks loaded
    pub all_picks: Vec<ChoicePick>,
    /// Filtered + sorted view shown in the table
    pub visible: Vec<ListItem>,
    pub table_state: TableState,
    /// Sync progress (done/total)
    pub sync_progress: Option<(u32, u32)>,
    pub sync_label: String,
    /// Metadata enrichment progress (done/total), cleared when complete
    pub metadata_progress: Option<(u32, u32)>,
    /// Game metadata keyed by steam_app_id
    pub metadata_map: HashMap<u32, GameMetadata>,
    /// State for the genre/tag picker modal (populated when Mode::GenrePicker is entered)
    pub genre_picker: Option<GenrePickerState>,
    /// Cursor position in the sort picker modal (index into SortOrder::all())
    pub sort_picker_cursor: usize,
    pub last_error: Option<String>,
    /// Accumulator for search input
    pub search_input: String,
    /// Auth input (session cookie)
    pub auth_input: String,
    /// Export path input
    pub export_input: String,
    /// Message shown in SyncPrompt modal (e.g. "3 days ago" or "never")
    pub sync_prompt_msg: String,
    matcher: SkimMatcherV2,
}


impl UiState {
    pub fn new(default_sort: &str, show_redeemed: bool) -> Self {
        let sort = match default_sort {
            "purchase_date_asc" => SortOrder::PurchaseDateAsc,
            "name_asc" => SortOrder::NameAsc,
            "name_desc" => SortOrder::NameDesc,
            "expiry_asc" => SortOrder::ExpiryAsc,
            _ => SortOrder::PurchaseDateDesc,
        };
        let filter = FilterState {
            show_expired: !show_redeemed,
            sort,
            ..Default::default()
        };
        Self {
            mode: Mode::Normal,
            filter,
            all_keys: vec![],
            all_picks: vec![],
            visible: vec![],
            table_state: TableState::default(),
            sync_progress: None,
            sync_label: String::new(),
            metadata_progress: None,
            metadata_map: HashMap::new(),
            genre_picker: None,
            sort_picker_cursor: 0,
            last_error: None,
            search_input: String::new(),
            auth_input: String::new(),
            export_input: String::new(),
            sync_prompt_msg: String::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Rebuild the `visible` list from all_keys + all_picks applying current filters.
    pub fn apply_filters(&mut self) {
        let selected_id: Option<String> = self
            .table_state
            .selected()
            .and_then(|i| self.visible.get(i))
            .map(|item| item.stable_id().to_owned());
        let query = self.filter.search_query.trim().to_lowercase();

        let mut items: Vec<ListItem> = vec![];

        // Regular keys
        if matches!(self.filter.source, SourceFilter::All | SourceFilter::Keys) {
            for key in &self.all_keys {
                if !self.filter.show_expired
                    && matches!(key.redeem_status, RedeemStatus::Expired)
                {
                    continue;
                }
                if let Some(status) = &self.filter.redeem_status {
                    if &key.redeem_status != status {
                        continue;
                    }
                }
                if !self.filter.platforms.is_empty()
                    && !self.filter.platforms.contains(&key.platform)
                {
                    continue;
                }
                if !query.is_empty() {
                    if self.matcher.fuzzy_match(&key.human_name.to_lowercase(), &query).is_none() {
                        continue;
                    }
                }
                if !self.filter.genre_filter.is_empty() {
                    match key.steam_app_id.and_then(|id| self.metadata_map.get(&id)) {
                        Some(meta) => {
                            let hit = meta.steam_tags.iter()
                                .chain(meta.steam_genres.iter())
                                .chain(meta.igdb_genres.iter())
                                .any(|t| self.filter.genre_filter.contains(t));
                            if !hit { continue; }
                        }
                        None => continue,
                    }
                }
                items.push(ListItem::Key(key.clone()));
            }
        }

        // Choice picks — skip any that have already been claimed (same machine_name exists as a GameKey)
        let claimed: std::collections::HashSet<&str> = self.all_keys.iter()
            .map(|k| k.tpkd_machine_name.as_str())
            .collect();

        if matches!(self.filter.source, SourceFilter::All | SourceFilter::Choice) {
            for pick in &self.all_picks {
                if claimed.contains(pick.machine_name.as_str()) {
                    continue;
                }
                // Choice picks are unclaimed by definition — hide them when filtering to Redeemed only
                if matches!(&self.filter.redeem_status, Some(s) if matches!(s, RedeemStatus::Redeemed)) {
                    continue;
                }
                if !self.filter.show_expired && pick.is_expired {
                    continue;
                }
                if !self.filter.platforms.is_empty()
                    && !self.filter.platforms.contains(&pick.platform)
                {
                    continue;
                }
                if !query.is_empty() {
                    if self.matcher.fuzzy_match(&pick.human_name.to_lowercase(), &query).is_none() {
                        continue;
                    }
                }
                if !self.filter.genre_filter.is_empty() {
                    match pick.steam_app_id.and_then(|id| self.metadata_map.get(&id)) {
                        Some(meta) => {
                            let hit = meta.steam_tags.iter()
                                .chain(meta.steam_genres.iter())
                                .chain(meta.igdb_genres.iter())
                                .any(|t| self.filter.genre_filter.contains(t));
                            if !hit { continue; }
                        }
                        None => continue,
                    }
                }
                items.push(ListItem::Choice(pick.clone()));
            }
        }

        // Sort
        match self.filter.sort {
            SortOrder::PurchaseDateDesc => {
                items.sort_by(|a, b| item_date(b).cmp(&item_date(a)));
            }
            SortOrder::PurchaseDateAsc => {
                items.sort_by(|a, b| item_date(a).cmp(&item_date(b)));
            }
            SortOrder::NameAsc => items.sort_by(|a, b| a.human_name().cmp(b.human_name())),
            SortOrder::NameDesc => items.sort_by(|a, b| b.human_name().cmp(a.human_name())),
            SortOrder::BundleAsc => items.sort_by(|a, b| a.bundle_name().cmp(b.bundle_name()).then(a.human_name().cmp(b.human_name()))),
            SortOrder::BundleDesc => items.sort_by(|a, b| b.bundle_name().cmp(a.bundle_name()).then(a.human_name().cmp(b.human_name()))),
            SortOrder::PlatformAsc => items.sort_by(|a, b| a.platform_label().cmp(b.platform_label())),
            SortOrder::ExpiryAsc => {
                items.sort_by(|a, b| {
                    let a_exp = if let ListItem::Key(k) = a { k.expiry_date } else if let ListItem::Choice(p) = a { p.claim_deadline } else { None };
                    let b_exp = if let ListItem::Key(k) = b { k.expiry_date } else if let ListItem::Choice(p) = b { p.claim_deadline } else { None };
                    match (a_exp, b_exp) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            SortOrder::MetacriticDesc => {
                items.sort_by(|a, b| {
                    let score = |item: &ListItem| -> Option<u32> {
                        item_steam_app_id(item)
                            .and_then(|id| self.metadata_map.get(&id))
                            .and_then(|m| m.metacritic_score)
                    };
                    match (score(a), score(b)) {
                        (Some(a), Some(b)) => b.cmp(&a),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            SortOrder::UserRatingDesc => {
                items.sort_by(|a, b| {
                    let rating = |item: &ListItem| -> Option<f32> {
                        item_steam_app_id(item)
                            .and_then(|id| self.metadata_map.get(&id))
                            .and_then(|m| m.steam_user_rating)
                    };
                    match (rating(a), rating(b)) {
                        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
        }

        self.visible = items;

        let len = self.visible.len();
        if len == 0 {
            self.table_state.select(None);
        } else {
            // Try to keep the cursor on the same game after a sort/filter change.
            let new_pos = selected_id
                .as_deref()
                .and_then(|id| self.visible.iter().position(|item| item.stable_id() == id));
            match new_pos {
                Some(pos) => self.table_state.select(Some(pos)),
                None => {
                    let current = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some(current.min(len - 1)));
                }
            }
        }
    }

    pub fn selected_item(&self) -> Option<&ListItem> {
        self.table_state.selected().and_then(|i| self.visible.get(i))
    }

    pub fn move_down(&mut self) {
        let len = self.visible.len();
        if len == 0 { return; }
        let next = self.table_state.selected().map_or(0, |i| (i + 1).min(len - 1));
        self.table_state.select(Some(next));
    }

    pub fn move_up(&mut self) {
        let len = self.visible.len();
        if len == 0 { return; }
        let next = self.table_state.selected().map_or(0, |i| i.saturating_sub(1));
        self.table_state.select(Some(next));
    }

    pub fn jump_top(&mut self) {
        if !self.visible.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn jump_bottom(&mut self) {
        let len = self.visible.len();
        if len > 0 {
            self.table_state.select(Some(len - 1));
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        let len = self.visible.len();
        if len == 0 { return; }
        let next = self.table_state.selected().map_or(0, |i| (i + page_size).min(len - 1));
        self.table_state.select(Some(next));
    }

    pub fn page_up(&mut self, page_size: usize) {
        if self.visible.is_empty() { return; }
        let next = self.table_state.selected().map_or(0, |i| i.saturating_sub(page_size));
        self.table_state.select(Some(next));
    }
}

fn item_steam_app_id(item: &ListItem) -> Option<u32> {
    match item {
        ListItem::Key(k) => k.steam_app_id,
        ListItem::Choice(p) => p.steam_app_id,
    }
}

fn item_date(item: &ListItem) -> chrono::DateTime<chrono::Utc> {
    match item {
        ListItem::Key(k) => k.purchase_date,
        ListItem::Choice(p) => p.month_date().unwrap_or_default(),
    }
}
