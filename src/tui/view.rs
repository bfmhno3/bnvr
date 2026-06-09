use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::app::{AppState, FocusedPanel};

pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_panels(frame, chunks[1], state);
    render_status_bar(frame, chunks[2]);
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let conn_status = if state.daemon_connected {
        "connected"
    } else {
        "disconnected"
    };
    let header = Paragraph::new(format!("  bnvr  |  daemon: {conn_status}"))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, area);
}

fn render_panels(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let panels = [
        (FocusedPanel::Nodes, "  No nodes configured  "),
        (FocusedPanel::Traffic, "  No traffic data  "),
        (FocusedPanel::Logs, "  No log output  "),
    ];

    for (i, (panel, placeholder)) in panels.iter().enumerate() {
        let is_focused = state.focused == *panel;
        let border_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(panel.title())
            .borders(Borders::ALL)
            .border_style(border_style);

        let text = Paragraph::new(*placeholder).block(block);
        frame.render_widget(text, chunks[i]);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    let hints = Paragraph::new("  q: quit | j/k: navigate | Tab: next panel  ")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hints, area);
}
