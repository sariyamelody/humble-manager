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

    // Future migrations go here:
    // if version < 2 { ... }

    Ok(())
}
