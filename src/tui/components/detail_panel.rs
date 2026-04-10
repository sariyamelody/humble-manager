use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::models::key::Platform;
use crate::tui::state::{ListItem, UiState};

pub struct DetailPanel<'a> {
    pub state: &'a UiState,
}

impl<'a> Widget for DetailPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Detail ");

        let lines = match self.state.selected_item() {
            None => vec![Line::from(Span::styled(
                "No item selected",
                Style::default().fg(Color::DarkGray),
            ))],
            Some(ListItem::Key(k)) => {
                let mut lines = vec![
                    field("Bundle", &k.bundle_human_name),
                    field("Purchased", &k.purchase_date.format("%Y-%m-%d").to_string()),
                    field("Platform", k.platform.display_name()),
                    field("Status", k.redeem_status.as_str()),
                    field("Type", &k.key_type),
                ];

                // Key value
                if let Some(val) = &k.redeemed_key_val {
                    lines.push(Line::from(vec![
                        label("Key"),
                        Span::styled(val.clone(), Style::default().fg(Color::Green)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        label("Key"),
                        Span::styled("[not revealed]", Style::default().fg(Color::DarkGray)),
                    ]));
                }

                // Expiry
                if let Some(exp) = k.expiry_date {
                    let now = Utc::now();
                    let expires_line = if exp < now {
                        Line::from(vec![
                            label("Expires"),
                            Span::styled("EXPIRED", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ])
                    } else {
                        let delta = exp - now;
                        Line::from(vec![
                            label("Expires"),
                            Span::styled(format_duration(delta), Style::default().fg(Color::Yellow)),
                        ])
                    };
                    lines.push(expires_line);
                }

                // Genres
                if !k.igdb_genres.is_empty() {
                    lines.push(field("Genres", &k.igdb_genres.join(", ")));
                }

                // Steam ownership
                match k.is_owned_on_steam {
                    Some(true) => lines.push(Line::from(vec![
                        label("Steam"),
                        Span::styled("Already owned", Style::default().fg(Color::DarkGray)),
                    ])),
                    Some(false) => lines.push(field("Steam", "Not owned")),
                    None => {}
                }

                if let Some(app_id) = k.steam_app_id {
                    lines.push(field("App ID", &app_id.to_string()));
                }

                lines.push(Line::from(vec![
                    Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                    Span::styled("o", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(" to open Humble page", Style::default().fg(Color::DarkGray)),
                ]));

                if let Some(store_hint) = platform_store_hint(&k.platform, k.steam_app_id) {
                    lines.push(Line::from(vec![
                        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled("O", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(store_hint, Style::default().fg(Color::DarkGray)),
                    ]));
                }

                lines
            }

            Some(ListItem::Choice(p)) => {
                let month_str = p.month_date()
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| p.choice_month.clone());
                let mut lines = vec![
                    field("Month", &p.choice_month),
                    field("Available", &month_str),
                    field("Platform", p.platform.display_name()),
                    Line::from(vec![
                        label("Status"),
                        Span::styled("[UNCLAIMED]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    ]),
                ];

                // Claim deadline
                if let Some(deadline) = p.claim_deadline {
                    let now = Utc::now();
                    if p.is_expired || deadline < now {
                        lines.push(Line::from(vec![
                            label("Claim by"),
                            Span::styled("EXPIRED", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ]));
                    } else {
                        let delta = deadline - now;
                        lines.push(Line::from(vec![
                            label("Claim by"),
                            Span::styled(format_duration(delta), Style::default().fg(Color::Yellow)),
                        ]));
                    }
                }

                // Genres (come directly from Humble)
                if !p.genres.is_empty() {
                    lines.push(field("Genres", &p.genres.join(", ")));
                }

                match p.is_owned_on_steam {
                    Some(true) => lines.push(Line::from(vec![
                        label("Steam"),
                        Span::styled("Already owned", Style::default().fg(Color::DarkGray)),
                    ])),
                    Some(false) => lines.push(field("Steam", "Not owned")),
                    None => {}
                }

                lines.push(Line::from(vec![
                    Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                    Span::styled("o", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(" to open claim page", Style::default().fg(Color::DarkGray)),
                ]));

                lines
            }
        };

        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        para.render(area, buf);
    }
}

fn label(name: &str) -> Span<'static> {
    Span::styled(
        format!("{:<10} ", name),
        Style::default().fg(Color::DarkGray),
    )
}

fn field<'a>(name: &str, value: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<10} ", name), Style::default().fg(Color::DarkGray)),
        Span::raw(value.to_string()),
    ])
}

/// Returns a hint string like " to open Steam page" or " to search Steam for this game",
/// or None if the platform has no store URL.
fn platform_store_hint(platform: &Platform, steam_app_id: Option<u32>) -> Option<String> {
    // Verify the platform has a store URL at all
    if platform.store_url("x", steam_app_id).is_none() {
        return None;
    }
    Some(match platform {
        Platform::Steam => {
            if steam_app_id.is_some() {
                " to open Steam page".to_string()
            } else {
                " to search Steam for this game".to_string()
            }
        }
        other => format!(" to open {} store page", other.display_name()),
    })
}

fn format_duration(delta: chrono::TimeDelta) -> String {
    let total_secs = delta.num_seconds();
    if total_secs <= 0 {
        return "expired".to_string();
    }
    let days = delta.num_days();
    let hours = delta.num_hours() % 24;
    let mins = delta.num_minutes() % 60;
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}
