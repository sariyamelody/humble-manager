use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyModifiers};

use crate::tui::{
    app_event::{AppEvent, Cmd},
    state::{Mode, UiState},
};
use crate::models::filter::SourceFilter;

/// Process one event, mutate state, and optionally return a Cmd to the coordinator.
pub fn update(state: &mut UiState, event: AppEvent) -> Option<Cmd> {
    match event {
        AppEvent::Tick => {
            // Just trigger a re-render for countdown refresh; no state change needed
            None
        }

        AppEvent::CacheLoaded { keys, picks } => {
            state.all_keys = keys;
            state.all_picks = picks;
            state.apply_filters();
            None
        }

        AppEvent::OrderRefsLoaded(refs) => {
            state.sync_progress = Some((0, refs.len() as u32));
            None
        }

        AppEvent::OrderLoaded { keys } => {
            // Merge new keys into all_keys (replace existing by tpkd_machine_name)
            for key in keys {
                if let Some(existing) = state.all_keys.iter_mut()
                    .find(|k| k.tpkd_machine_name == key.tpkd_machine_name)
                {
                    *existing = key;
                } else {
                    state.all_keys.push(key);
                }
            }
            state.apply_filters();
            None
        }

        AppEvent::ChoicePicksLoaded { month, picks } => {
            // Merge into all_picks, replacing existing entries for the same month
            // (so re-syncs stay fresh) but keeping other months intact.
            state.all_picks.retain(|p| p.choice_month != month);
            state.all_picks.extend(picks);
            state.apply_filters();
            None
        }

        AppEvent::SyncProgress { done, total, label } => {
            state.sync_progress = Some((done, total));
            state.sync_label = label;
            None
        }

        AppEvent::SyncError(msg) => {
            state.last_error = Some(msg);
            None
        }

        AppEvent::SyncStateLoaded(last_synced) => {
            let stale = match last_synced {
                None => Some("never synced".to_string()),
                Some(t) => {
                    let age_secs = (Utc::now() - t).num_seconds().max(0) as u64;
                    if age_secs > 24 * 3600 {
                        let days = age_secs / 86400;
                        Some(if days == 1 {
                            "1 day ago".to_string()
                        } else {
                            format!("{} days ago", days)
                        })
                    } else {
                        None
                    }
                }
            };
            if let Some(msg) = stale {
                state.sync_prompt_msg = msg;
                state.mode = Mode::SyncPrompt;
            }
            None
        }

        AppEvent::Input(event) => handle_input(state, event),
    }
}

fn handle_input(state: &mut UiState, event: Event) -> Option<Cmd> {
    let Event::Key(key) = event else { return None; };

    match &state.mode {
        Mode::Auth => handle_auth_input(state, key),
        Mode::Search => handle_search_input(state, key),
        Mode::ExportPrompt => handle_export_input(state, key),
        Mode::Error => {
            // Any key dismisses the error
            state.last_error = None;
            state.mode = Mode::Normal;
            None
        }
        Mode::SyncPrompt => {
            state.mode = Mode::Normal;
            if key.code == KeyCode::Char('r') {
                return Some(Cmd::StartFullSync);
            }
            None
        }
        Mode::Normal => handle_normal_input(state, key),
    }
}

fn handle_normal_input(state: &mut UiState, key: crossterm::event::KeyEvent) -> Option<Cmd> {
    match (key.modifiers, key.code) {
        // Navigation
        (KeyModifiers::NONE, KeyCode::Char('j')) |
        (KeyModifiers::NONE, KeyCode::Down) => { state.move_down(); None }

        (KeyModifiers::NONE, KeyCode::Char('k')) |
        (KeyModifiers::NONE, KeyCode::Up) => { state.move_up(); None }

        (KeyModifiers::NONE, KeyCode::Char('g')) => { state.jump_top(); None }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => { state.jump_bottom(); None }

        (KeyModifiers::CONTROL, KeyCode::Char('d')) |
        (KeyModifiers::NONE, KeyCode::PageDown) => { state.page_down(15); None }

        (KeyModifiers::CONTROL, KeyCode::Char('u')) |
        (KeyModifiers::NONE, KeyCode::PageUp) => { state.page_up(15); None }

        // Mode switches
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            state.mode = Mode::Search;
            None
        }

        // Cycle status filter: All → Unredeemed → Redeemed → All
        (KeyModifiers::NONE, KeyCode::Char('f')) => {
            state.filter.redeem_status = match &state.filter.redeem_status {
                None => Some(crate::models::key::RedeemStatus::Unredeemed),
                Some(crate::models::key::RedeemStatus::Unredeemed) => Some(crate::models::key::RedeemStatus::Redeemed),
                Some(_) => None,
            };
            state.apply_filters();
            None
        }

        // Actions on selected item
        // O — open platform store page (Steam/GOG/Epic/etc.)
        (KeyModifiers::SHIFT, KeyCode::Char('O')) => {
            if let Some(crate::tui::state::ListItem::Key(k)) = state.selected_item() {
                if let Some(url) = k.platform.store_url(&k.human_name, k.steam_app_id) {
                    let _ = open::that(url);
                }
            }
            None
        }

        (KeyModifiers::NONE, KeyCode::Char('o')) => {
            // Open store / claim page in browser
            if let Some(item) = state.selected_item() {
                let url = match item {
                    crate::tui::state::ListItem::Key(k) => {
                        // Humble download page is where keys are revealed/redeemed
                        Some(format!(
                            "https://www.humblebundle.com/downloads?key={}",
                            k.bundle_machine_name
                        ))
                    }
                    crate::tui::state::ListItem::Choice(p) => {
                        // Derive URL slug from choice_month:
                        // "april_2025_choice" → strip "_choice" → replace "_" with "-" → "april-2025"
                        let slug = p.choice_month
                            .strip_suffix("_choice")
                            .unwrap_or(&p.choice_month)
                            .replace('_', "-");
                        Some(format!("https://www.humblebundle.com/membership/{}", slug))
                    }
                };
                if let Some(url) = url {
                    let _ = open::that(url);
                }
            }
            None
        }

        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            // Yank key value to clipboard
            if let Some(crate::tui::state::ListItem::Key(k)) = state.selected_item() {
                if let Some(val) = &k.redeemed_key_val {
                    let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(val.clone()));
                }
            }
            None
        }

        // Sort cycle
        (KeyModifiers::NONE, KeyCode::Char('s')) => {
            state.filter.sort = state.filter.sort.next();
            state.apply_filters();
            None
        }

        // Toggle Choice picks
        (KeyModifiers::NONE, KeyCode::Char('c')) => {
            state.filter.source = match state.filter.source {
                SourceFilter::All => SourceFilter::Choice,
                SourceFilter::Choice => SourceFilter::Keys,
                SourceFilter::Keys => SourceFilter::All,
            };
            state.apply_filters();
            None
        }

        // Refresh / sync
        (KeyModifiers::NONE, KeyCode::Char('r')) => {
            Some(Cmd::StartFullSync)
        }

        // Export
        (KeyModifiers::NONE, KeyCode::Char('e')) => {
            state.export_input = String::new();
            state.mode = Mode::ExportPrompt;
            None
        }

        // Quit
        (KeyModifiers::NONE, KeyCode::Char('q')) |
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            Some(Cmd::Quit)
        }

        _ => None,
    }
}

fn handle_search_input(state: &mut UiState, key: crossterm::event::KeyEvent) -> Option<Cmd> {
    match key.code {
        KeyCode::Esc => {
            state.filter.search_query.clear();
            state.search_input.clear();
            state.mode = Mode::Normal;
            state.apply_filters();
        }
        KeyCode::Enter => {
            state.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            state.search_input.pop();
            state.filter.search_query = state.search_input.clone();
            state.apply_filters();
        }
        KeyCode::Char(c) => {
            state.search_input.push(c);
            state.filter.search_query = state.search_input.clone();
            state.apply_filters();
        }
        _ => {}
    }
    None
}


fn handle_auth_input(state: &mut UiState, key: crossterm::event::KeyEvent) -> Option<Cmd> {
    match key.code {
        KeyCode::Enter => {
            // Auth cookie accepted — stored via app.rs
            state.mode = Mode::Normal;
        }
        KeyCode::Backspace => { state.auth_input.pop(); }
        KeyCode::Char(c) => { state.auth_input.push(c); }
        KeyCode::Esc => {
            // Can't escape auth if it's required
        }
        _ => {}
    }
    None
}

fn handle_export_input(state: &mut UiState, key: crossterm::event::KeyEvent) -> Option<Cmd> {
    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let path = std::path::PathBuf::from(state.export_input.trim());
            state.mode = Mode::Normal;
            return Some(Cmd::ExportCsv(path));
        }
        KeyCode::Backspace => { state.export_input.pop(); }
        KeyCode::Char(c) => { state.export_input.push(c); }
        _ => {}
    }
    None
}

