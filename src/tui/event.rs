use crossterm::event::EventStream;
use futures::StreamExt;
use tokio::sync::mpsc;

use super::app_event::AppEvent;

/// Reads crossterm events and sends them as AppEvent::Input.
pub async fn event_reader(tx: mpsc::Sender<AppEvent>) {
    let mut stream = EventStream::new();
    while let Some(Ok(event)) = stream.next().await {
        if tx.send(AppEvent::Input(event)).await.is_err() {
            break;
        }
    }
}

/// Sends AppEvent::Tick at the specified interval.
pub async fn tick_timer(tx: mpsc::Sender<AppEvent>, interval_ms: u64) {
    let mut interval = tokio::time::interval(
        std::time::Duration::from_millis(interval_ms)
    );
    loop {
        interval.tick().await;
        if tx.send(AppEvent::Tick).await.is_err() {
            break;
        }
    }
}
