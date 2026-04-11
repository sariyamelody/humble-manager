use std::collections::HashMap;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, StatefulWidget, Table, Widget},
};

use crate::tui::state::{ColumnId, ListItem, UiState};
use crate::models::key::RedeemStatus;
use crate::models::metadata::{GameMetadata, SteamDeckCompat};

pub struct KeyTable<'a> {
    pub state: &'a mut UiState,
}

impl<'a> Widget for KeyTable<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.state.active_columns.clone();

        // Header: always starts with "#"
        let mut header_cells: Vec<String> = vec!["#".to_string()];
        header_cells.extend(cols.iter().map(|&c| col_header(c).to_string()));
        let header = Row::new(header_cells)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self.state.visible
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let mut cells = vec![format!("{}", i + 1)];
                for &col in &cols {
                    cells.push(render_cell(col, item, &self.state.metadata_map));
                }

                let style = if item.is_choice() {
                    Style::default().fg(Color::Cyan)
                } else {
                    match item {
                        ListItem::Key(k) if k.redeem_status == RedeemStatus::Expired => {
                            Style::default().fg(Color::DarkGray)
                        }
                        ListItem::Key(k) if k.is_revealed => {
                            Style::default().fg(Color::Green)
                        }
                        _ => Style::default(),
                    }
                };

                Row::new(cells).style(style)
            })
            .collect();

        // Constraints: index col always fixed, then one per active column
        let mut widths = vec![Constraint::Length(4)];
        widths.extend(cols.iter().map(|&c| col_constraint(c)));

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Keys "),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(table, area, buf, &mut self.state.table_state);
    }
}

fn col_header(col: ColumnId) -> &'static str {
    match col {
        ColumnId::Name => "Name",
        ColumnId::Platform => "Plat",
        ColumnId::Status => "St",
        ColumnId::Bundle => "Bundle",
        ColumnId::PurchaseDate => "Date",
        ColumnId::Expiry => "Expiry",
        ColumnId::Metacritic => "MC",
        ColumnId::UserRating => "Rating",
        ColumnId::SteamDeck => "Deck",
    }
}

fn col_constraint(col: ColumnId) -> Constraint {
    match col {
        ColumnId::Name => Constraint::Min(20),
        ColumnId::Platform => Constraint::Length(5),
        ColumnId::Status => Constraint::Length(3),
        ColumnId::Bundle => Constraint::Max(30),
        ColumnId::PurchaseDate => Constraint::Length(11),
        ColumnId::Expiry => Constraint::Length(11),
        ColumnId::Metacritic => Constraint::Length(4),
        ColumnId::UserRating => Constraint::Length(6),
        ColumnId::SteamDeck => Constraint::Length(5),
    }
}

fn render_cell(col: ColumnId, item: &ListItem, metadata_map: &HashMap<u32, GameMetadata>) -> String {
    match col {
        ColumnId::Name => item.human_name().to_string(),
        ColumnId::Platform => item.platform_label().to_string(),
        ColumnId::Status => item.status_label().to_string(),
        ColumnId::Bundle => truncate(item.bundle_name(), 28),
        ColumnId::PurchaseDate => match item {
            ListItem::Key(k) => k.purchase_date.format("%Y-%m-%d").to_string(),
            ListItem::Choice(p) => p.month_date()
                .map(|d| d.format("%b %Y").to_string())
                .unwrap_or_default(),
        },
        ColumnId::Expiry => {
            let exp = match item {
                ListItem::Key(k) => k.expiry_date,
                ListItem::Choice(p) => p.claim_deadline,
            };
            exp.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
        }
        ColumnId::Metacritic => item
            .steam_app_id()
            .and_then(|id| metadata_map.get(&id))
            .and_then(|m| m.metacritic_score)
            .map(|s| s.to_string())
            .unwrap_or_default(),
        ColumnId::UserRating => item
            .steam_app_id()
            .and_then(|id| metadata_map.get(&id))
            .and_then(|m| m.steam_user_rating)
            .map(|r| format!("{:.0}%", r * 100.0))
            .unwrap_or_default(),
        ColumnId::SteamDeck => item
            .steam_app_id()
            .and_then(|id| metadata_map.get(&id))
            .and_then(|m| m.steam_deck_compat)
            .map(|c| match c {
                SteamDeckCompat::Verified => "✓",
                SteamDeckCompat::Playable => "~",
                SteamDeckCompat::Unsupported => "✗",
            }.to_string())
            .unwrap_or_default(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}
