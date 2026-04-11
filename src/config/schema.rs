use std::path::PathBuf;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auth: AuthConfig,
    pub sync: SyncConfig,
    pub steam: SteamConfig,
    pub igdb: IgdbConfig,
    pub export: ExportConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// Value of the _simpleauth_sess cookie from browser devtools
    pub session_cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Minutes between auto-refresh; 0 = manual only
    pub auto_refresh_interval: u32,
    pub max_concurrent_requests: usize,
    pub enable_steam_enrichment: bool,
    pub enable_igdb_enrichment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SteamConfig {
    pub api_key: String,
    /// 64-bit SteamID
    pub steam_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IgdbConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub default_export_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub tick_rate_ms: u64,
    pub default_sort: String,
    pub show_redeemed: bool,
    pub show_expired: bool,
    /// Column IDs to display, in order. Valid values: name, platform, status, bundle,
    /// purchase_date, expiry, metacritic, user_rating, steam_deck
    #[serde(default = "UiConfig::default_columns")]
    pub columns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            sync: SyncConfig::default(),
            steam: SteamConfig::default(),
            igdb: IgdbConfig::default(),
            export: ExportConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_refresh_interval: 0,
            max_concurrent_requests: 4,
            enable_steam_enrichment: false,
            enable_igdb_enrichment: false,
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            default_export_dir: "~/Downloads".to_string(),
        }
    }
}

impl UiConfig {
    fn default_columns() -> Vec<String> {
        vec![
            "name".to_string(),
            "platform".to_string(),
            "status".to_string(),
            "bundle".to_string(),
        ]
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 250,
            default_sort: "purchase_date_desc".to_string(),
            show_redeemed: true,
            show_expired: false,
            columns: Self::default_columns(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let config: Config = toml::from_str(&text)
            .with_context(|| format!("parsing config at {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .context("serializing config")?;
        // Atomic write: write to temp file, then rename
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &text)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("", "", "humble-manager")
            .context("could not determine config directory")?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    pub fn data_dir() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("", "", "humble-manager")
            .context("could not determine data directory")?;
        let dir = dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn db_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("cache.db"))
    }

    pub fn needs_auth(&self) -> bool {
        self.auth.session_cookie.trim().is_empty()
    }
}
