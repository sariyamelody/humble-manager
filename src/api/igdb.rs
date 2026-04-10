use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

pub struct IgdbToken(pub String);

pub struct IgdbMetadata {
    pub igdb_id: u64,
    pub genres: Vec<String>,
    /// Aggregated critic rating (0–100). Only present when IGDB has ≥4 reviews.
    pub rating: Option<f64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct IgdbGame {
    id: u64,
    #[serde(default)]
    genres: Vec<IgdbGenre>,
    aggregated_rating: Option<f64>,
}

#[derive(Deserialize)]
struct IgdbGenre {
    name: String,
}

/// Exchange Twitch client credentials for an IGDB bearer token.
/// Tokens are valid for ~60 days; re-fetch at the start of each metadata sync.
pub async fn fetch_igdb_token(
    client: &Client,
    client_id: &str,
    client_secret: &str,
) -> Result<IgdbToken> {
    #[derive(serde::Serialize)]
    struct Params<'a> {
        client_id: &'a str,
        client_secret: &'a str,
        grant_type: &'static str,
    }

    let resp: TokenResponse = client
        .post("https://id.twitch.tv/oauth2/token")
        .form(&Params { client_id, client_secret, grant_type: "client_credentials" })
        .send()
        .await
        .context("IGDB token request failed")?
        .json()
        .await
        .context("IGDB token parse failed")?;

    Ok(IgdbToken(resp.access_token))
}

/// Look up a game by its Steam app ID via IGDB's external_games index.
/// Returns `None` when no match is found.
pub async fn fetch_igdb_by_steam_id(
    client: &Client,
    client_id: &str,
    token: &IgdbToken,
    steam_app_id: u32,
) -> Result<Option<IgdbMetadata>> {
    // IGDB uses Apicalypse query syntax in the POST body.
    // external_games.category = 1 means Steam.
    let body = format!(
        "fields id,genres.name,aggregated_rating; \
         where external_games.uid = \"{}\" & external_games.category = 1; \
         limit 1;",
        steam_app_id
    );

    let games: Vec<IgdbGame> = client
        .post("https://api.igdb.com/v4/games")
        .header("Client-ID", client_id)
        .header("Authorization", format!("Bearer {}", token.0))
        .header("Content-Type", "text/plain")
        .body(body)
        .send()
        .await
        .context("IGDB games request failed")?
        .json()
        .await
        .context("IGDB games parse failed")?;

    Ok(games.into_iter().next().map(|g| IgdbMetadata {
        igdb_id: g.id,
        genres: g.genres.into_iter().map(|g| g.name).collect(),
        rating: g.aggregated_rating,
    }))
}
