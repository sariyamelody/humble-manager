use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

use crate::{
    config::Config,
    db::Db,
    export::csv::export_csv,
    sync::coordinator,
    tui::{
        app_event::{AppEvent, Cmd},
        event::{event_reader, tick_timer},
        render::render,
        state::{Mode, UiState},
        terminal,
        update::update,
    },
};

pub async fn run() -> Result<()> {
    // Setup logging to file (not stdout — would corrupt TUI)
    let data_dir = Config::data_dir()?;
    let log_file = tracing_appender::rolling::never(&data_dir, "humble-manager.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    info!("humble-manager starting");

    let config = Config::load()?;
    let db = Db::open(&Config::db_path()?)?;

    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(512);
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(64);

    // Spawn background tasks
    tokio::spawn(event_reader(event_tx.clone()));
    tokio::spawn(tick_timer(event_tx.clone(), config.ui.tick_rate_ms));
    tokio::spawn(coordinator::run(
        config.clone(),
        db.clone(),
        cmd_rx,
        event_tx.clone(),
    ));

    // Build initial UI state
    let mut state = UiState::new(&config.ui.default_sort, config.ui.show_redeemed);

    // If no cookie configured, start in auth mode
    if config.needs_auth() {
        state.mode = Mode::Auth;
    }

    terminal::install_panic_hook();
    let mut terminal = terminal::init()?;

    let result = run_loop(&mut terminal, &mut state, &mut event_rx, &cmd_tx, &config, &db).await;

    terminal::restore();
    result
}

async fn run_loop(
    terminal: &mut terminal::Tui,
    state: &mut UiState,
    event_rx: &mut mpsc::Receiver<AppEvent>,
    cmd_tx: &mpsc::Sender<Cmd>,
    config: &Config,
    db: &Db,
) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, state))?;

        let event = match event_rx.recv().await {
            Some(e) => e,
            None => break,
        };

        // Handle auth submission specially (need to save config)
        if let AppEvent::Input(crossterm::event::Event::Key(key)) = &event {
            if state.mode == Mode::Auth && key.code == crossterm::event::KeyCode::Enter {
                let cookie = state.auth_input.trim().to_string();
                if !cookie.is_empty() {
                    let mut new_config = config.clone();
                    new_config.auth.session_cookie = cookie;
                    new_config.save()?;
                    state.mode = Mode::Normal;
                    let _ = cmd_tx.send(Cmd::StartFullSync).await;
                    continue;
                }
            }
        }

        let maybe_cmd = update(state, event);

        if let Some(cmd) = maybe_cmd {
            match cmd {
                Cmd::Quit => break,
                Cmd::ExportCsv(path) => {
                    let keys = state.visible.clone();
                    if let Err(e) = export_csv(&path, &keys) {
                        state.last_error = Some(format!("Export failed: {}", e));
                        state.mode = Mode::Error;
                    }
                }
                other => {
                    let _ = cmd_tx.send(other).await;
                }
            }
        }

        // If an error was set and we're not already in error mode, switch to it
        if state.last_error.is_some() && state.mode == Mode::Normal {
            state.mode = Mode::Error;
        }
    }

    let _ = cmd_tx.send(Cmd::Quit).await;
    Ok(())
}
