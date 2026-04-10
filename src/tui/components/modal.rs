use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::tui::state::UiState;

/// Auth modal: prompts the user to paste their session cookie.
pub struct AuthModal<'a> {
    pub state: &'a UiState,
}

impl<'a> Widget for AuthModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(70, 12, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(" Humble Bundle Authentication ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let instructions = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Paste your _simpleauth_sess cookie value below.",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "To get it: open humblebundle.com → F12 → Application → Cookies",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Cookie: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if self.state.auth_input.is_empty() {
                        "_".to_string()
                    } else {
                        // Show a masked version
                        format!("{}…", &self.state.auth_input[..self.state.auth_input.len().min(20)])
                    },
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to confirm.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        Paragraph::new(instructions)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

/// Export path modal.
pub struct ExportModal<'a> {
    pub state: &'a UiState,
}

impl<'a> Widget for ExportModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 7, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(" Export to CSV ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("Export path:", Style::default().fg(Color::Gray))),
            Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{}_", self.state.export_input),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled("Enter=export  Esc=cancel", Style::default().fg(Color::DarkGray))),
        ];

        Paragraph::new(lines).render(inner, buf);
    }
}

/// Error modal.
pub struct ErrorModal<'a> {
    pub message: &'a str,
}

impl<'a> Widget for ErrorModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(60, 7, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Red).fg(Color::White));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines = vec![
            Line::from(""),
            Line::from(Span::raw(self.message.to_string())),
            Line::from(""),
            Line::from(Span::styled("Press any key to dismiss.", Style::default().fg(Color::DarkGray))),
        ];

        Paragraph::new(lines).wrap(Wrap { trim: true }).render(inner, buf);
    }
}

/// Stale-cache sync suggestion modal.
pub struct SyncPromptModal<'a> {
    pub last_synced_msg: &'a str,
}

impl<'a> Widget for SyncPromptModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(55, 7, area);
        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(" Sync Suggested ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Last sync: ", Style::default().fg(Color::Gray)),
                Span::styled(self.last_synced_msg.to_string(), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(Span::styled("r=sync now  any other key=dismiss", Style::default().fg(Color::DarkGray))),
        ];

        Paragraph::new(lines).render(inner, buf);
    }
}

/// Genre/tag picker modal. Scrollable, searchable, multi-select.
pub struct GenrePickerModal<'a> {
    pub state: &'a mut UiState,
}

impl<'a> Widget for GenrePickerModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let modal_area = centered_rect(62, 26, area);
        Clear.render(modal_area, buf);

        let picker = match &mut self.state.genre_picker {
            Some(p) => p,
            None => return,
        };

        let block = Block::default()
            .title(" Filter by Genre / Tag ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        // Layout: search bar (3 rows) / list / footer hint (1 row)
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        // Search input
        let search_block = Block::default().borders(Borders::BOTTOM);
        let search_inner = search_block.inner(layout[0]);
        search_block.render(layout[0], buf);
        Paragraph::new(Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}_", picker.search),
                Style::default().fg(Color::White),
            ),
        ])).render(search_inner, buf);

        // Tag list
        let active_style = Style::default()
            .fg(Color::Black).bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let items: Vec<ListItem> = picker.filtered_indices.iter().enumerate().map(|(pos, &idx)| {
            let (name, count, is_genre) = &picker.all_items[idx];
            let is_current = pos == picker.cursor;
            let checked = picker.pending_filter.contains(name);

            // Checkbox: green when selected, dim otherwise
            let (checkbox, checkbox_style) = if checked {
                ("[x]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                ("[ ]", Style::default().fg(Color::DarkGray))
            };

            // Prefix + name style differs by kind
            let (prefix, name_style) = if *is_genre {
                ("◆ ", Style::default().fg(Color::Cyan))
            } else {
                ("# ", Style::default().fg(Color::Yellow))
            };

            // Count always visible; invert on highlighted row for readability
            let count_style = if is_current {
                Style::default().fg(Color::Black)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", checkbox), checkbox_style),
                Span::styled(prefix, if is_current { Style::default().fg(Color::Black) } else { name_style }),
                Span::styled(name.clone(), if is_current { Style::default().fg(Color::Black).add_modifier(Modifier::BOLD) } else { name_style }),
                Span::styled(format!("  ({})", count), count_style),
            ]))
        }).collect();

        let list = List::new(items)
            .highlight_style(active_style)
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, layout[1], buf, &mut picker.list_state);

        // Footer
        let active_count = picker.pending_filter.len();
        let footer_text = if active_count > 0 {
            format!(" Space:toggle  Enter:apply ({} active)  Esc:cancel  Ctrl+C:clear all ", active_count)
        } else {
            " Space:toggle  Enter:apply  Esc:cancel ".to_string()
        };
        Paragraph::new(Span::styled(footer_text, Style::default().fg(Color::DarkGray)))
            .render(layout[2], buf);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
