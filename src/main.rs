mod app;
mod config;
mod db;
mod api;
mod models;
mod sync;
mod tui;
mod export;
mod error;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
