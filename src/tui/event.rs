use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub async fn run_event_loop(tx: mpsc::Sender<AppEvent>) {
    loop {
        match event::poll(Duration::from_millis(200)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if tx.send(AppEvent::Key(key)).await.is_err() => return,
                Ok(_) => {}
                Err(_) => return,
            },
            Ok(false) => {
                if tx.send(AppEvent::Tick).await.is_err() {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

pub fn handle_key(key: KeyEvent, state: &mut crate::tui::app::AppState) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.quit();
        return;
    }

    match key.code {
        KeyCode::Char('q') => state.quit(),
        KeyCode::Char('j') | KeyCode::Down => state.focus_next(),
        KeyCode::Char('k') | KeyCode::Up => state.focus_prev(),
        KeyCode::Tab => state.focus_next(),
        KeyCode::BackTab => state.focus_prev(),
        _ => {}
    }
}
