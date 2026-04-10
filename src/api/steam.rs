use anyhow::Result;
use reqwest::Client;

use crate::models::metadata::SteamDeckCompat;

pub struct SteamMetadata {
    pub genres: Vec<String>,
    pub metacritic_score: Option<u32>,
    /// Top tags by vote count from SteamSpy
    pub tags: Vec<String>,
    pub deck_compat: Option<SteamDeckCompat>,
    /// User rating as a fraction 0.0–1.0 from SteamSpy positive/negative counts
    pub user_rating: Option<f32>,
}

/// Fetch genres, Metacritic score, and Steam Deck compatibility from Steam,
/// plus popular user tags from SteamSpy. Both calls are best-effort.
pub async fn fetch_steam_metadata(client: &Client, app_id: u32) -> Result<SteamMetadata> {
    let (genres, metacritic_score, deck_compat) =
        fetch_app_details(client, app_id).await.unwrap_or_default();
    let (tags, user_rating) = fetch_steamspy_data(client, app_id).await.unwrap_or_default();
    Ok(SteamMetadata { genres, metacritic_score, tags, deck_compat, user_rating })
}

async fn fetch_app_details(
    client: &Client,
    app_id: u32,
) -> Result<(Vec<String>, Option<u32>, Option<SteamDeckCompat>)> {
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}&filters=genres,metacritic,steam_deck_compatibility",
        app_id
    );
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let entry = &resp[app_id.to_string()];

    if !entry["success"].as_bool().unwrap_or(false) {
        return Ok((vec![], None, None));
    }

    let genres = entry["data"]["genres"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g["description"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let metacritic_score = entry["data"]["metacritic"]["score"]
        .as_u64()
        .map(|s| s as u32);

    // category: 0=Unknown, 1=Unsupported, 2=Playable, 3=Verified
    let deck_compat = entry["data"]["steam_deck_compatibility"]["category"]
        .as_i64()
        .and_then(SteamDeckCompat::from_category);

    Ok((genres, metacritic_score, deck_compat))
}

/// Returns (tags_sorted_by_votes, user_rating_fraction).
async fn fetch_steamspy_data(client: &Client, app_id: u32) -> Result<(Vec<String>, Option<f32>)> {
    let url = format!(
        "https://steamspy.com/api.php?request=appdetails&appid={}",
        app_id
    );
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let mut pairs: Vec<(String, u64)> = resp["tags"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let tags = pairs.into_iter().map(|(k, _)| k).collect();

    let positive = resp["positive"].as_u64().unwrap_or(0);
    let negative = resp["negative"].as_u64().unwrap_or(0);
    let total = positive + negative;
    let user_rating = if total > 0 {
        Some(positive as f32 / total as f32)
    } else {
        None
    };

    Ok((tags, user_rating))
}
