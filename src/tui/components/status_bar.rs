use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::tui::state::{Mode, UiState};

pub struct StatusBar<'a> {
    pub state: &'a UiState,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_span = match self.state.mode {
            Mode::Normal => Span::styled(" NORMAL ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
            Mode::Search => Span::styled(" SEARCH ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Mode::Auth => Span::styled("  AUTH  ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
            Mode::ExportPrompt => Span::styled(" EXPORT ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Mode::Error => Span::styled("  ERROR ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
            Mode::SyncPrompt | Mode::GenrePicker => Span::styled(" NORMAL ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),

        };

        let sync_span = if let Some((done, total)) = self.state.sync_progress {
            if done < total {
                Span::styled(
                    format!(" Syncing {}/{} ", done, total),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::styled(" Sync complete ", Style::default().fg(Color::Green))
            }
        } else {
            Span::styled(" Press r to sync ", Style::default().fg(Color::DarkGray))
        };

        let meta_span = if let Some((done, total)) = self.state.metadata_progress {
            if total == 0 || done < total {
                let label = if total == 0 {
                    " Enriching... ".to_string()
                } else {
                    format!(" Enriching {}/{} ", done, total)
                };
                Span::styled(label, Style::default().fg(Color::Cyan))
            } else {
                Span::styled(" Enriched ".to_string(), Style::default().fg(Color::Green))
            }
        } else {
            Span::raw("")
        };

        let hint = Span::styled(
            " j/k:move  /:search  f:status  s:sort  t:tags  c:source  e:export  r:sync  R:enrich  q:quit",
            Style::default().fg(Color::DarkGray),
        );

        let line = Line::from(vec![mode_span, Span::raw(" "), sync_span, meta_span, hint]);
        Paragraph::new(line).render(area, buf);
    }
}
