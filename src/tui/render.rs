use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::{
    components::{
        detail_panel::DetailPanel,
        filter_bar::FilterBar,
        key_table::KeyTable,
        modal::{AuthModal, ColumnPickerModal, ErrorModal, ExportModal, GenrePickerModal, SortPickerModal, SyncPromptModal},
        status_bar::StatusBar,
    },
    state::{Mode, UiState},
};

pub fn render(frame: &mut Frame, state: &mut UiState) {
    let area = frame.area();

    // Outer vertical split: filter bar / main content / status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // filter bar
            Constraint::Min(0),     // main area
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    frame.render_widget(FilterBar { state }, outer[0]);

    // Main area: key table | detail panel
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(outer[1]);

    frame.render_widget(KeyTable { state }, main[0]);
    frame.render_widget(DetailPanel { state }, main[1]);

    frame.render_widget(StatusBar { state }, outer[2]);

    // Overlays (rendered on top)
    match &state.mode {
        Mode::Auth => frame.render_widget(AuthModal { state }, area),
        Mode::ExportPrompt => frame.render_widget(ExportModal { state }, area),
        Mode::Error => {
            if let Some(msg) = &state.last_error {
                let msg = msg.clone();
                frame.render_widget(ErrorModal { message: &msg }, area);
            }
        }
        Mode::SyncPrompt => {
            let msg = state.sync_prompt_msg.clone();
            frame.render_widget(SyncPromptModal { last_synced_msg: &msg }, area);
        }
        Mode::GenrePicker => {
            frame.render_widget(GenrePickerModal { state }, area);
        }
        Mode::SortPicker => {
            frame.render_widget(SortPickerModal { state }, area);
        }
        Mode::ColumnPicker => {
            frame.render_widget(ColumnPickerModal { state }, area);
        }
        _ => {}
    }
}
