pub mod actions;
pub mod app;
pub mod event;
pub mod log_reader;
pub mod view;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use app::AppState;
use event::{AppEvent, AsyncAction};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = TerminalSession::new()?;
    let result = run_app(session.terminal()).await;
    session.restore()?;
    result
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restored: bool,
}

impl TerminalSession {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        let backend = CrosstermBackend::new(stdout);
        match Terminal::new(backend) {
            Ok(terminal) => Ok(Self {
                terminal,
                restored: false,
            }),
            Err(e) => {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
                Err(e.into())
            }
        }
    }

    fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    fn restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::new();
    state.daemon_connected = try_connect_daemon().await;
    let _ = state.refresh_all().await;

    let (tx, mut rx) = mpsc::channel(100);
    tokio::spawn(event::run_event_loop(tx));
    let mut refresh_interval = tokio::time::interval(Duration::from_secs(1));
    let mut log_interval = tokio::time::interval(Duration::from_millis(200));

    loop {
        terminal.draw(|frame| view::render(frame, &state))?;

        tokio::select! {
            Some(app_event) = rx.recv() => match app_event {
                AppEvent::Key(key) => {
                    if let Some(action) = event::handle_key(key, &mut state) {
                        handle_async_action(action, &mut state).await;
                    }
                }
                AppEvent::Tick => {}
            },
            _ = refresh_interval.tick() => {
                let _ = state.refresh_status().await;
                let _ = state.refresh_traffic().await;
            }
            _ = log_interval.tick() => {
                let _ = state.refresh_logs();
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

async fn handle_async_action(action: AsyncAction, state: &mut AppState) {
    match action {
        AsyncAction::SwitchNode(name) => {
            if actions::switch_node(&name).await.is_ok() {
                let _ = state.refresh_nodes().await;
            }
        }
        AsyncAction::TestNode(name) => {
            if let Ok(delay) = actions::test_node_delay(&name).await {
                state.set_node_delay(&name, delay);
            }
        }
    }
}

async fn try_connect_daemon() -> bool {
    use interprocess::local_socket::tokio::Stream;
    use interprocess::local_socket::traits::tokio::Stream as _;
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    let name = "bnvr".to_ns_name::<GenericNamespaced>();
    match name {
        Ok(n) => Stream::connect(n).await.is_ok(),
        Err(_) => false,
    }
}
