use anyhow::Result;
use std::path::Path;

use crate::tui::state::ListItem;

pub fn export_csv(path: &Path, items: &[ListItem]) -> Result<()> {
    // Expand ~ in path
    let expanded = expand_tilde(path);

    // Ensure parent directory exists
    if let Some(parent) = expanded.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut writer = csv::Writer::from_path(&expanded)?;

    // Write header
    writer.write_record(&[
        "Name", "Platform", "Status", "Bundle", "Purchase Date",
        "Key Type", "Has Key", "Expiry", "Genres", "Steam App ID",
    ])?;

    for item in items {
        let (name, platform, status, bundle, purchase_date, key_type, has_key, expiry, genres, steam_app_id) = match item {
            ListItem::Key(k) => (
                k.human_name.clone(),
                k.platform.display_name().to_string(),
                k.redeem_status.as_str().to_string(),
                k.bundle_human_name.clone(),
                k.purchase_date.format("%Y-%m-%d").to_string(),
                k.key_type.clone(),
                if k.redeemed_key_val.is_some() { "yes" } else { "no" }.to_string(),
                k.expiry_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                k.igdb_genres.join("; "),
                k.steam_app_id.map(|id| id.to_string()).unwrap_or_default(),
            ),
            ListItem::Choice(p) => (
                p.human_name.clone(),
                p.platform.display_name().to_string(),
                "unclaimed".to_string(),
                format!("Choice: {}", p.choice_month),
                String::new(),
                "Steam".to_string(),
                "no".to_string(),
                p.claim_deadline
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                p.genres.join("; "),
                p.steam_app_id.map(|id| id.to_string()).unwrap_or_default(),
            ),
        };

        writer.write_record(&[
            &name, &platform, &status, &bundle, &purchase_date,
            &key_type, &has_key, &expiry, &genres, &steam_app_id,
        ])?;
    }

    writer.flush()?;
    Ok(())
}

fn expand_tilde(path: &Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with("~/") {
        if let Some(home) = dirs_home() {
            return std::path::PathBuf::from(home).join(&s[2..]);
        }
    }
    path.to_path_buf()
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}
