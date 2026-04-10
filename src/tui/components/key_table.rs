use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, StatefulWidget, Table, Widget},
};

use crate::tui::state::{ListItem, UiState};
use crate::models::key::RedeemStatus;

pub struct KeyTable<'a> {
    pub state: &'a mut UiState,
}

impl<'a> Widget for KeyTable<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let header = Row::new(vec!["#", "Name", "Plat", "St", "Bundle"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self.state.visible
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let num = format!("{}", i + 1);
                let name = item.human_name().to_string();
                let platform = item.platform_label().to_string();
                let status = item.status_label().to_string();
                let bundle = truncate(item.bundle_name(), 28);

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

                Row::new(vec![num, name, platform, status, bundle]).style(style)
            })
            .collect();

        let widths = [
            ratatui::layout::Constraint::Length(4),
            ratatui::layout::Constraint::Min(20),
            ratatui::layout::Constraint::Length(5),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Max(30),
        ];

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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}
