CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS bundles (
    machine_name         TEXT PRIMARY KEY,
    human_name           TEXT NOT NULL,
    product_machine_name TEXT NOT NULL DEFAULT '',
    purchased_at         INTEGER NOT NULL,
    bundle_type          TEXT NOT NULL DEFAULT 'unknown',
    cached_at            INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_keys (
    id                    TEXT PRIMARY KEY,
    tpkd_machine_name     TEXT NOT NULL UNIQUE,
    human_name            TEXT NOT NULL,
    platform              TEXT NOT NULL,
    key_type              TEXT NOT NULL DEFAULT '',
    redeemed_key_val      TEXT,
    is_revealed           INTEGER NOT NULL DEFAULT 0,
    redeem_status         TEXT NOT NULL DEFAULT 'unredeemed',
    bundle_machine_name   TEXT NOT NULL REFERENCES bundles(machine_name),
    bundle_human_name     TEXT NOT NULL DEFAULT '',
    purchase_date         INTEGER NOT NULL DEFAULT 0,
    expiry_date           INTEGER,
    steam_app_id          INTEGER,
    igdb_genres           TEXT NOT NULL DEFAULT '[]',
    is_owned_on_steam     INTEGER,
    created_at            INTEGER NOT NULL,
    updated_at            INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS choice_picks (
    machine_name             TEXT PRIMARY KEY,
    human_name               TEXT NOT NULL,
    platform                 TEXT NOT NULL,
    steam_app_id             INTEGER,
    genres                   TEXT NOT NULL DEFAULT '[]',
    claim_deadline           INTEGER,
    num_days_until_expired   INTEGER,
    is_expired               INTEGER NOT NULL DEFAULT 0,
    is_owned_on_steam        INTEGER,
    choice_month             TEXT NOT NULL,
    cached_at                INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_state (
    resource       TEXT PRIMARY KEY,
    last_synced_at INTEGER,
    status         TEXT NOT NULL DEFAULT 'never',
    error_message  TEXT
);

CREATE INDEX IF NOT EXISTS idx_game_keys_bundle      ON game_keys(bundle_machine_name);
CREATE INDEX IF NOT EXISTS idx_game_keys_platform    ON game_keys(platform);
CREATE INDEX IF NOT EXISTS idx_game_keys_status      ON game_keys(redeem_status);
CREATE INDEX IF NOT EXISTS idx_game_keys_expiry      ON game_keys(expiry_date);
CREATE INDEX IF NOT EXISTS idx_game_keys_human_name  ON game_keys(human_name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_choice_month          ON choice_picks(choice_month);
