use anyhow::Result;
use rusqlite::Connection;

const SCHEMA: &str = include_str!("schema.sql");

pub fn run(conn: &Connection) -> Result<()> {
    // Read current schema version (0 if table doesn't exist yet)
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version == 0 {
        conn.execute_batch(SCHEMA)?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (1)", [])?;
    }

    if version < 2 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS game_metadata (
                steam_app_id      INTEGER PRIMARY KEY,
                steam_tags        TEXT NOT NULL DEFAULT '[]',
                steam_genres      TEXT NOT NULL DEFAULT '[]',
                metacritic_score  INTEGER,
                igdb_id           INTEGER,
                igdb_genres       TEXT NOT NULL DEFAULT '[]',
                igdb_rating       REAL,
                enriched_at       INTEGER NOT NULL
            );",
        )?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (2)", [])?;
    }

    if version < 3 {
        // Add Steam Deck compatibility rating to game_metadata
        conn.execute_batch(
            "ALTER TABLE game_metadata ADD COLUMN steam_deck_compat INTEGER;",
        )?;
        // Normalized tag/genre table replaces JSON blobs in game_metadata.
        // source is one of: 'steam_genre', 'steam_tag', 'igdb_genre'
        // vote_rank is NULL for genres, ordinal position (1 = most-voted) for steam_tags.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS game_tags (
                steam_app_id  INTEGER NOT NULL,
                tag           TEXT NOT NULL,
                source        TEXT NOT NULL,
                vote_rank     INTEGER,
                PRIMARY KEY (steam_app_id, tag, source)
            );
            CREATE INDEX IF NOT EXISTS idx_game_tags_tag ON game_tags(tag);
            CREATE INDEX IF NOT EXISTS idx_game_tags_app ON game_tags(steam_app_id);",
        )?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (3)", [])?;
    }

    if version < 4 {
        conn.execute_batch(
            "ALTER TABLE game_metadata ADD COLUMN steam_user_rating REAL;",
        )?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (4)", [])?;
    }

    Ok(())
}
