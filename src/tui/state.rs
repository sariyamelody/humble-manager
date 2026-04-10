use ratatui::widgets::TableState;

use crate::models::{
    choice::ChoicePick,
    filter::{FilterState, SortOrder, SourceFilter},
    key::{GameKey, Platform, RedeemStatus},
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
    pub last_error: Option<String>,
    /// Accumulator for search input
    pub search_input: String,
    /// Auth input (session cookie)
    pub auth_input: String,
    pub auth_input_visible: bool,
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
            show_expired: false,
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
            last_error: None,
            search_input: String::new(),
            auth_input: String::new(),
            auth_input_visible: false,
            export_input: String::new(),
            sync_prompt_msg: String::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Rebuild the `visible` list from all_keys + all_picks applying current filters.
    pub fn apply_filters(&mut self) {
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
                items.push(ListItem::Choice(pick.clone()));
            }
        }

        // Sort
        match self.filter.sort {
            SortOrder::PurchaseDateDesc => {
                items.sort_by(|a, b| {
                    let a_date = if let ListItem::Key(k) = a { k.purchase_date } else { chrono::DateTime::default() };
                    let b_date = if let ListItem::Key(k) = b { k.purchase_date } else { chrono::DateTime::default() };
                    b_date.cmp(&a_date)
                });
            }
            SortOrder::PurchaseDateAsc => {
                items.sort_by(|a, b| {
                    let a_date = if let ListItem::Key(k) = a { k.purchase_date } else { chrono::DateTime::default() };
                    let b_date = if let ListItem::Key(k) = b { k.purchase_date } else { chrono::DateTime::default() };
                    a_date.cmp(&b_date)
                });
            }
            SortOrder::NameAsc => items.sort_by(|a, b| a.human_name().cmp(b.human_name())),
            SortOrder::NameDesc => items.sort_by(|a, b| b.human_name().cmp(a.human_name())),
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
        }

        self.visible = items;

        // Keep selection in bounds
        let len = self.visible.len();
        if len == 0 {
            self.table_state.select(None);
        } else {
            let current = self.table_state.selected().unwrap_or(0);
            self.table_state.select(Some(current.min(len - 1)));
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
