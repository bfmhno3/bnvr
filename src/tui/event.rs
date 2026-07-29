use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

use super::app::{AppState, FocusedArea};

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub enum AsyncAction {
    SwitchNode(String),
    TestNode(String),
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

pub fn handle_key(key: KeyEvent, state: &mut AppState) -> Option<AsyncAction> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.quit();
        return None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            state.quit();
            return None;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.toggle_focus();
            return None;
        }
        _ => {}
    }

    match state.focused {
        FocusedArea::NodeList => handle_node_list_keys(key, state),
        FocusedArea::LogView => handle_log_view_keys(key, state),
    }
}

fn handle_node_list_keys(key: KeyEvent, state: &mut AppState) -> Option<AsyncAction> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => state.select_next_node(),
        KeyCode::Char('k') | KeyCode::Up => state.select_prev_node(),
        KeyCode::Enter => return state.selected_node_name().map(AsyncAction::SwitchNode),
        KeyCode::Char('n') => return state.selected_node_name().map(AsyncAction::TestNode),
        KeyCode::Char('p') => {}
        _ => {}
    }
    None
}

fn handle_log_view_keys(key: KeyEvent, state: &mut AppState) -> Option<AsyncAction> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => state.scroll_logs_down(1),
        KeyCode::Char('k') | KeyCode::Up => state.scroll_logs_up(1),
        KeyCode::Char('d') | KeyCode::PageDown => state.scroll_logs_down(10),
        KeyCode::Char('u') | KeyCode::PageUp => state.scroll_logs_up(10),
        KeyCode::Char(' ') => state.toggle_log_auto_scroll(),
        KeyCode::Char('G') => state.jump_to_latest_log(),
        _ => {}
    }
    None
}
