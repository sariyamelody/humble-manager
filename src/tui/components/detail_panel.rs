use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::models::key::Platform;
use crate::models::metadata::SteamDeckCompat;
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

                // Enriched metadata (Steam/IGDB)
                if let Some(app_id) = k.steam_app_id {
                    if let Some(meta) = self.state.metadata_map.get(&app_id) {
                        if !meta.steam_tags.is_empty() {
                            let tags = meta.steam_tags.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
                            lines.push(field("Tags", &tags));
                        }
                        if !meta.steam_genres.is_empty() {
                            lines.push(field("Genres", &meta.steam_genres.join(", ")));
                        } else if !meta.igdb_genres.is_empty() {
                            lines.push(field("Genres", &meta.igdb_genres.join(", ")));
                        }
                        if let Some(score) = meta.metacritic_score {
                            let (score_str, color) = metacritic_display(score);
                            lines.push(Line::from(vec![
                                label("Metacritic"),
                                Span::styled(score_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(rating) = meta.steam_user_rating {
                            let (rating_str, color) = user_rating_display(rating);
                            lines.push(Line::from(vec![
                                label("User rating"),
                                Span::styled(rating_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(rating) = meta.igdb_rating {
                            let (rating_str, color) = igdb_rating_display(rating);
                            lines.push(Line::from(vec![
                                label("IGDB"),
                                Span::styled(rating_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(compat) = meta.steam_deck_compat {
                            lines.push(Line::from(vec![
                                label("Steam Deck"),
                                Span::styled(compat.label(), deck_compat_style(compat)),
                            ]));
                        }
                    } else if !k.igdb_genres.is_empty() {
                        lines.push(field("Genres", &k.igdb_genres.join(", ")));
                    }
                } else if !k.igdb_genres.is_empty() {
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

                // Enriched metadata (Steam/IGDB), falling back to Humble-provided genres
                if let Some(app_id) = p.steam_app_id {
                    if let Some(meta) = self.state.metadata_map.get(&app_id) {
                        if !meta.steam_tags.is_empty() {
                            let tags = meta.steam_tags.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
                            lines.push(field("Tags", &tags));
                        }
                        let genres = if !meta.steam_genres.is_empty() {
                            &meta.steam_genres
                        } else if !meta.igdb_genres.is_empty() {
                            &meta.igdb_genres
                        } else {
                            &p.genres
                        };
                        if !genres.is_empty() {
                            lines.push(field("Genres", &genres.join(", ")));
                        }
                        if let Some(score) = meta.metacritic_score {
                            let (score_str, color) = metacritic_display(score);
                            lines.push(Line::from(vec![
                                label("Metacritic"),
                                Span::styled(score_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(rating) = meta.steam_user_rating {
                            let (rating_str, color) = user_rating_display(rating);
                            lines.push(Line::from(vec![
                                label("User rating"),
                                Span::styled(rating_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(rating) = meta.igdb_rating {
                            let (rating_str, color) = igdb_rating_display(rating);
                            lines.push(Line::from(vec![
                                label("IGDB"),
                                Span::styled(rating_str, Style::default().fg(color)),
                            ]));
                        }
                        if let Some(compat) = meta.steam_deck_compat {
                            lines.push(Line::from(vec![
                                label("Steam Deck"),
                                Span::styled(compat.label(), deck_compat_style(compat)),
                            ]));
                        }
                    } else if !p.genres.is_empty() {
                        lines.push(field("Genres", &p.genres.join(", ")));
                    }
                } else if !p.genres.is_empty() {
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

                if let Some(store_hint) = platform_store_hint(&p.platform, p.steam_app_id) {
                    lines.push(Line::from(vec![
                        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled("O", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(store_hint, Style::default().fg(Color::DarkGray)),
                    ]));
                }

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

/// Returns a hint string for the O keybinding, e.g. " to open Steam page" or
/// " to search Steam for this game". Returns None only for platforms with no URL.
fn platform_store_hint(platform: &Platform, steam_app_id: Option<u32>) -> Option<String> {
    platform.store_url("x", steam_app_id)?;  // bail if no URL
    Some(match platform {
        Platform::Steam => {
            if steam_app_id.is_some() {
                " to open Steam page".to_string()
            } else {
                " to search Steam for this game".to_string()
            }
        }
        Platform::Gog | Platform::EpicGames | Platform::Ubisoft |
        Platform::Itch | Platform::BattleNet => {
            format!(" to open {} store page", platform.display_name())
        }
        // DrmFree, HumbleApp, Other — fall back to Steam search
        _ => " to search Steam for this game".to_string(),
    })
}

fn deck_compat_style(compat: SteamDeckCompat) -> Style {
    match compat {
        SteamDeckCompat::Verified => Style::default().fg(Color::Green),
        SteamDeckCompat::Playable => Style::default().fg(Color::Yellow),
        SteamDeckCompat::Unsupported => Style::default().fg(Color::Red),
    }
}

fn user_rating_display(rating: f32) -> (String, Color) {
    let pct = (rating * 100.0).round() as u32;
    let color = if pct >= 80 { Color::Green } else if pct >= 60 { Color::Yellow } else { Color::Red };
    (format!("{}%", pct), color)
}

fn metacritic_display(score: u32) -> (String, Color) {
    let color = if score >= 75 { Color::Green } else if score >= 50 { Color::Yellow } else { Color::Red };
    (format!("{}/100", score), color)
}

fn igdb_rating_display(rating: f64) -> (String, Color) {
    let color = if rating >= 75.0 { Color::Green } else if rating >= 50.0 { Color::Yellow } else { Color::Red };
    (format!("{:.0}/100", rating), color)
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
