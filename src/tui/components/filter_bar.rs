use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::state::UiState;
use crate::models::filter::SourceFilter;

pub struct FilterBar<'a> {
    pub state: &'a UiState,
}

impl<'a> Widget for FilterBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let active = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
        let inactive = Style::default().fg(Color::DarkGray);
        let label = Style::default().fg(Color::Gray);

        // Source chips
        let source_spans = vec![
            Span::styled("Source: ", label),
            chip("All", self.state.filter.source == SourceFilter::All, active, inactive),
            Span::raw(" "),
            chip("Keys", self.state.filter.source == SourceFilter::Keys, active, inactive),
            Span::raw(" "),
            chip("Choice", self.state.filter.source == SourceFilter::Choice, active, inactive),
            Span::raw("  "),
            Span::styled("Status: ", label),
            chip(
                "All",
                self.state.filter.redeem_status.is_none(),
                active,
                inactive,
            ),
            Span::raw(" "),
            chip(
                "Unredeemed",
                matches!(&self.state.filter.redeem_status, Some(s) if matches!(s, crate::models::key::RedeemStatus::Unredeemed)),
                active,
                inactive,
            ),
            Span::raw(" "),
            chip(
                "Redeemed",
                matches!(&self.state.filter.redeem_status, Some(s) if matches!(s, crate::models::key::RedeemStatus::Redeemed)),
                active,
                inactive,
            ),
            Span::raw("  "),
            Span::styled("Sort: ", label),
            Span::styled(
                format!("[{}]", self.state.filter.sort.label()),
                Style::default().fg(Color::Yellow),
            ),
        ];

        let total = self.state.all_keys.len() + self.state.all_picks.len();
        let shown = self.state.visible.len();
        let count_str = format!("  {}/{} shown", shown, total);

        let lines = vec![
            Line::from(source_spans),
            Line::from(vec![
                Span::styled("Search: ", label),
                Span::styled(
                    if self.state.filter.search_query.is_empty() {
                        "(type / to search)".to_string()
                    } else {
                        format!("\"{}\"", self.state.filter.search_query)
                    },
                    Style::default().fg(Color::White),
                ),
                Span::styled(count_str, Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let block = Block::default().borders(Borders::BOTTOM);
        let inner = block.inner(area);
        block.render(area, buf);

        Paragraph::new(lines).render(inner, buf);
    }
}

fn chip(label: &str, active: bool, active_style: Style, inactive_style: Style) -> Span<'static> {
    let text = format!("[{}]", label);
    if active {
        Span::styled(text, active_style)
    } else {
        Span::styled(text, inactive_style)
    }
}
