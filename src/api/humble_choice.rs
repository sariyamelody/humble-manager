use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::models::{choice::ChoicePick, key::Platform};
use super::client::HumbleClient;

// ── Response types ────────────────────────────────────────────────────────────

// The JSON structure is identical for current and past months.
// Current month: script id="webpack-subscriber-hub-data"
// Past months:   script id="webpack-monthly-product-data"

#[derive(Debug, Deserialize)]
struct PageData {
    #[serde(rename = "contentChoiceOptions")]
    content_choice_options: ContentChoiceOptions,
}

#[derive(Debug, Deserialize)]
struct ContentChoiceOptions {
    #[serde(rename = "contentChoiceData")]
    content_choice_data: ContentChoiceData,
    #[serde(rename = "productMachineName", default)]
    product_machine_name: String,
}

#[derive(Debug, Deserialize)]
struct ContentChoiceData {
    #[serde(default)]
    game_data: std::collections::HashMap<String, ChoiceGameEntry>,
}

#[derive(Debug, Deserialize)]
struct ChoiceGameEntry {
    title: String,
    #[serde(default)]
    genres: Vec<String>,
    #[serde(default)]
    delivery_methods: Vec<String>,
    #[serde(default)]
    tpkds: Vec<ChoiceTpkd>,
}

#[derive(Debug, Deserialize)]
struct ChoiceTpkd {
    machine_name: String,
    key_type: Option<String>,
    steam_app_id: Option<u32>,
    #[serde(rename = "expiration_date|datetime")]
    expiration_date: Option<String>,
    #[serde(default)]
    num_days_until_expired: i32,
    #[serde(default)]
    is_expired: bool,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Fetch picks for a specific past Choice month.
/// `url_slug` is the `choice_url` from the order's product, e.g. `"april-2025"`.
pub async fn fetch_choice_picks_for_url(
    client: &HumbleClient,
    url_slug: &str,
) -> Result<(String, Vec<ChoicePick>)> {
    let url = format!("https://www.humblebundle.com/membership/{}", url_slug);

    let html = client
        .client()
        .get(&url)
        .header(reqwest::header::ACCEPT, "text/html")
        .send()
        .await
        .with_context(|| format!("fetching Choice page {}", url))?
        .error_for_status()
        .with_context(|| format!("Choice page {} HTTP error", url))?
        .text()
        .await
        .context("reading Choice page body")?;

    parse_choice_page(&html, url_slug)
}

fn parse_choice_page(html: &str, url_slug: &str) -> Result<(String, Vec<ChoicePick>)> {
    // Try both script tag IDs — current month uses "webpack-subscriber-hub-data",
    // past months use "webpack-monthly-product-data".
    let json_str = extract_webpack_data(html, "webpack-subscriber-hub-data")
        .or_else(|| extract_webpack_data(html, "webpack-monthly-product-data"))
        .with_context(|| format!("could not find Choice data script tag for {}", url_slug))?;

    let data: PageData =
        serde_json::from_str(json_str).context("parsing Choice page JSON")?;

    let opts = &data.content_choice_options;
    // Use productMachineName if present, fall back to the URL slug
    let choice_month = if opts.product_machine_name.is_empty() {
        url_slug.to_string()
    } else {
        opts.product_machine_name.clone()
    };

    let picks = build_picks(&opts.content_choice_data, &choice_month);
    Ok((choice_month, picks))
}

fn build_picks(ccd: &ContentChoiceData, choice_month: &str) -> Vec<ChoicePick> {
    ccd.game_data
        .iter()
        .flat_map(|(_machine_name, entry)| {
            let month = choice_month.to_string();
            entry.tpkds.iter().map(move |tpkd| {
                let platform = tpkd
                    .key_type
                    .as_deref()
                    .map(Platform::from_str)
                    .unwrap_or_else(|| {
                        entry
                            .delivery_methods
                            .first()
                            .map(|s| Platform::from_str(s))
                            .unwrap_or(Platform::Other("unknown".into()))
                    });

                let claim_deadline = tpkd
                    .expiration_date
                    .as_deref()
                    .and_then(parse_humble_datetime);

                ChoicePick {
                    machine_name: tpkd.machine_name.clone(),
                    human_name: entry.title.clone(),
                    platform,
                    steam_app_id: tpkd.steam_app_id,
                    genres: entry.genres.clone(),
                    claim_deadline,
                    num_days_until_expired: Some(tpkd.num_days_until_expired),
                    is_expired: tpkd.is_expired,
                    is_owned_on_steam: None,
                    choice_month: month.clone(),
                }
            })
        })
        .collect()
}

fn extract_webpack_data<'a>(html: &'a str, script_id: &str) -> Option<&'a str> {
    let start_tag = format!(r#"id="{}" type="application/json">"#, script_id);
    let start = html.find(start_tag.as_str())? + start_tag.len();
    let end = html[start..].find("</script>")? + start;
    Some(html[start..end].trim())
}

/// Parse Humble's datetime format: "2027-05-05T17:00:00" (no timezone — assume UTC)
fn parse_humble_datetime(s: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|ndt| ndt.and_utc())
}
