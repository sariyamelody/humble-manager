# humble-manager

A terminal UI for browsing, filtering, and managing your Humble Bundle game keys and Choice picks.

![Rust](https://img.shields.io/badge/rust-2021-orange)

## Features

- Browse all your Humble Bundle keys in a fast, keyboard-driven TUI
- Filter by status (Unredeemed / Redeemed / All), source (Keys / Choice / All), and sort order
- Live fuzzy search across game names
- Humble Choice picks shown alongside keys, with claim deadlines
- Press `o` to open the Humble download/claim page for any item in your browser
- Press `y` to yank a revealed key value to clipboard
- Export the current filtered view to CSV
- SQLite cache — loads instantly on startup, syncs on demand
- Sync suggested (never forced) when cache is more than a day old

## Installation

Requires Rust (stable). Build from source:

```bash
cargo build --release
cp target/release/humble-manager ~/.local/bin/
```

## Setup

You need a `_simpleauth_sess` session cookie from a logged-in Humble Bundle browser session:

1. Log in to [humblebundle.com](https://www.humblebundle.com) in your browser
2. Open DevTools → Application (Chrome) or Storage (Firefox) → Cookies
3. Copy the value of `_simpleauth_sess`
4. Run `humble-manager` — it will prompt you to paste it on first launch

The cookie is stored in `~/Library/Application Support/humble-manager/config.toml` on macOS.

## Usage

```
humble-manager
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | Move selection |
| `g` / `G` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `/` | Search (live fuzzy filter) |
| `f` | Cycle status filter (All → Unredeemed → Redeemed) |
| `s` | Cycle sort order |
| `c` | Cycle source (All → Keys → Choice) |
| `o` | Open Humble download / claim page in browser |
| `O` | Open platform store page (Steam, GOG, Epic, etc.) |
| `y` | Yank revealed key value to clipboard |
| `r` | Start a full sync |
| `e` | Export current view to CSV |
| `q` / `Ctrl+c` | Quit |

### Sync

humble-manager never syncs automatically. On startup it loads from the local SQLite cache. If the cache is more than 24 hours old you'll see a prompt — press `r` to sync or any other key to dismiss and use the cached data.

## Data

All data is cached locally in SQLite. On macOS:

| File | Path |
|------|------|
| Config | `~/Library/Application Support/humble-manager/config.toml` |
| Database | `~/Library/Application Support/humble-manager/humble-manager.db` |
| Log | `~/Library/Application Support/humble-manager/humble-manager.log` |

## License

MIT — see [LICENSE](LICENSE)
