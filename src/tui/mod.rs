pub mod app;
pub mod event;
pub mod view;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::sync::mpsc;

use app::AppState;
use event::AppEvent;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let result = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::new();

    // Attempt daemon connection
    state.daemon_connected = try_connect_daemon().await;

    let (tx, mut rx) = mpsc::channel(32);
    tokio::spawn(event::run_event_loop(tx));

    loop {
        terminal.draw(|frame| view::render(frame, &state))?;

        if let Some(app_event) = rx.recv().await {
            match app_event {
                AppEvent::Key(key) => event::handle_key(key, &mut state),
                AppEvent::Tick => {}
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

async fn try_connect_daemon() -> bool {
    use interprocess::local_socket::tokio::Stream;
    use interprocess::local_socket::{GenericNamespaced, ToNsName};
    use interprocess::local_socket::traits::tokio::Stream as _;

    let name = "bnvr".to_ns_name::<GenericNamespaced>();
    match name {
        Ok(n) => Stream::connect(n).await.is_ok(),
        Err(_) => false,
    }
}
