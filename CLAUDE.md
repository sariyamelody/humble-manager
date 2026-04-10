# Humble Manager

Rust TUI for browsing, filtering, and managing Humble Bundle game keys and Choice picks.

## Build & Run

```bash
cargo build --release
./target/release/humble-manager
```

Config lives at `~/Library/Application Support/humble-manager/config.toml` on macOS (via `directories` crate — NOT `~/.config/`). SQLite cache and logs are in the same directory.

## Architecture

Single-crate Rust binary. No workspace.

- **TUI**: Ratatui + crossterm, Helix/Kakoune-style (selection-first, not vim-style)
- **Async**: Tokio runtime. Main thread runs the render loop; background tasks communicate via `mpsc` channels (`AppEvent` inbound, `Cmd` outbound)
- **DB**: rusqlite with a dedicated DB actor task (receives `DbMsg` over a channel) because `rusqlite::Connection` is `!Send`
- **HTTP**: reqwest with session cookie auth

## Humble Bundle API Notes

- **Auth**: Cookie `_simpleauth_sess=<value>`, header `X-Requested-By: hb_android_app`
- **Orders list**: `GET /api/v1/user/order` → `[{"gamekey": "..."}]`
- **Order detail**: `GET /api/v1/order/{gamekey}?all_tpkds=true` — the `?all_tpkds=true` param is **required** or `tpkd_dict` is absent from the response
- **Choice picks** (current month): `GET /membership/home` → parse `<script id="webpack-subscriber-hub-data">`
- **Choice picks** (past months): `GET /membership/{choice_url}` → parse `<script id="webpack-monthly-product-data">` (different script tag ID, same JSON structure)
- **Subscription orders**: identified by `product.category == "subscriptioncontent"`. Have `product.choice_url` (e.g. `"april-2025"`) pointing to the membership page. DB stores machine names with underscores (`"april_2025_choice"`); URL slug uses hyphens.
- **`created` field**: naive datetime, no timezone — `"2016-07-22T22:59:01.787060"`. Parse with `%Y-%m-%dT%H:%M:%S%.f`, assume UTC.
- **Choice expiry dates**: also naive, no fractional seconds — `"2027-05-05T17:00:00"`. Parse with `%Y-%m-%dT%H:%M:%S`.
- **Key reveal endpoint**: Not yet discovered. Post-MVP.

## Event loop

The render loop blocks on the first event then drains all queued events with `try_recv()` before the next draw. This prevents sync floods (345 concurrent `OrderLoaded` events) from causing 345 full redraws and freezing input.

## Session cookie for testing

Stored in `.context/humble_session` (gitignored). Read it with:
```bash
grep -o 'eyJ.*' .context/humble_session | tr -d '\n'
```

## Commit conventions

Commit at feature or fix boundaries. Keep commits atomic and the tree building at all times.
