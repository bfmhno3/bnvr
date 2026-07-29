use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, ListState, Paragraph,
};

use super::app::{AppState, FocusedArea};

pub fn render(frame: &mut Frame, state: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(15),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_upper_section(frame, main_chunks[0], state);
    render_log_section(frame, main_chunks[1], state);
    render_status_bar(frame, main_chunks[2], state);
}

fn render_upper_section(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    render_status_panel(frame, chunks[0], state);
    render_traffic_chart(frame, chunks[1], state);
    render_nodes_panel(frame, chunks[2], state);
}

fn render_status_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let daemon = match &state.daemon_status {
        Some(daemon) => format!(
            "Daemon: running (pid {}, uptime {}s)",
            daemon.pid, daemon.uptime_secs
        ),
        None if state.daemon_connected => "Daemon: running".to_string(),
        None => "Daemon: not running".to_string(),
    };
    let profile = state
        .profile_info
        .as_ref()
        .and_then(|profile| profile.active.as_deref())
        .unwrap_or("--");
    let profile_count = state
        .profile_info
        .as_ref()
        .map(|profile| profile.list.len())
        .unwrap_or(0);
    let kernel = match &state.kernel_status {
        Some(kernel) if kernel.running => format!(
            "Kernel: {} (running pid {})",
            kernel.version.as_deref().unwrap_or("unknown"),
            kernel
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "--".to_string())
        ),
        Some(_) => "Kernel: stopped".to_string(),
        None => "Kernel: --".to_string(),
    };
    let plugin = state
        .plugin_info
        .as_ref()
        .and_then(|plugin| plugin.active.as_deref())
        .unwrap_or("--");
    let plugin_count = state
        .plugin_info
        .as_ref()
        .map(|plugin| plugin.list.len())
        .unwrap_or(0);
    let connections = match &state.connection_stats {
        Some(stats) => format!(
            "Connections: {} (up {} down {})",
            stats.total,
            format_bytes(stats.upload_bytes),
            format_bytes(stats.download_bytes)
        ),
        None => "Connections: -- (data unavailable)".to_string(),
    };

    let rows = vec![
        ListItem::new(daemon),
        ListItem::new(format!("Profile: {profile} ({profile_count} total)")),
        ListItem::new(kernel),
        ListItem::new(format!("Plugin: {plugin} ({plugin_count} total)")),
        ListItem::new(connections),
    ];
    let block = Block::default().title("Status").borders(Borders::ALL);
    frame.render_widget(List::new(rows).block(block), area);
}

fn render_traffic_chart(frame: &mut Frame, area: Rect, state: &AppState) {
    let upload: Vec<(f64, f64)> = state
        .traffic_samples
        .iter()
        .enumerate()
        .map(|(i, sample)| (i as f64, sample.upload_bps as f64))
        .collect();
    let download: Vec<(f64, f64)> = state
        .traffic_samples
        .iter()
        .enumerate()
        .map(|(i, sample)| (i as f64, sample.download_bps as f64))
        .collect();
    let max_rate = state
        .traffic_samples
        .iter()
        .map(|sample| sample.upload_bps.max(sample.download_bps))
        .max()
        .unwrap_or(1)
        .max(1);
    let title = if state.traffic_samples.is_empty() {
        "Network Traffic (unavailable)"
    } else {
        "Network Traffic (60s)"
    };
    let datasets = vec![
        Dataset::default()
            .name("up")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&upload),
        Dataset::default()
            .name("down")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue))
            .data(&download),
    ];
    let chart = Chart::new(datasets)
        .block(Block::default().title(title).borders(Borders::ALL))
        .x_axis(Axis::default().bounds([0.0, 59.0]))
        .y_axis(Axis::default().bounds([0.0, max_rate as f64]));
    frame.render_widget(chart, area);
}

fn render_nodes_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let border_style = focus_style(state.focused == FocusedArea::NodeList);
    let block = Block::default()
        .title("Nodes (n: test speed)")
        .borders(Borders::ALL)
        .border_style(border_style);
    let items = if state.nodes.is_empty() {
        vec![ListItem::new("No nodes available")]
    } else {
        state
            .nodes
            .iter()
            .map(|node| {
                let current = state.current_node.as_deref() == Some(&node.name);
                let mark = if current { "[*]" } else { "[ ]" };
                let delay = node
                    .delay
                    .map(|delay| format!("{delay}ms"))
                    .unwrap_or_else(|| "---".to_string());
                ListItem::new(format!(
                    "{mark} {} [{}] ({delay})",
                    node.name, node.proxy_type
                ))
            })
            .collect()
    };
    let mut list_state = ListState::default();
    if !state.nodes.is_empty() {
        list_state.select(Some(state.selected_node_index));
    }
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_log_section(frame: &mut Frame, area: Rect, state: &AppState) {
    let border_style = focus_style(state.focused == FocusedArea::LogView);
    let height = usize::from(area.height.saturating_sub(2)).max(1);
    let max_start = state.log_lines.len().saturating_sub(height);
    let start = if state.log_auto_scroll {
        max_start
    } else {
        state.log_scroll_offset.min(max_start)
    };
    let end = (start + height).min(state.log_lines.len());
    let width = usize::from(area.width.saturating_sub(2));
    let mut lines: Vec<Line<'_>> = state.log_lines[start..end]
        .iter()
        .map(|line| Line::from(truncate_line(line, width)))
        .collect();
    if lines.is_empty() {
        lines.push(Line::from("No log file found"));
    }
    let more = end < state.log_lines.len();
    if more && !lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "down more",
            Style::default().fg(Color::Yellow),
        )));
    }
    let block = Block::default()
        .title("Logs (Space: toggle auto-scroll)")
        .borders(Borders::ALL)
        .border_style(border_style);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let focus = match state.focused {
        FocusedArea::NodeList => "nodes",
        FocusedArea::LogView => "logs",
    };
    let hints = Paragraph::new(format!(
        " q: quit | Tab: focus | focus: {focus} | nodes: j/k Enter n | logs: j/k u/d Space G "
    ))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hints, area);
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if line.chars().count() <= width {
        return line.to_string();
    }
    let keep = width.saturating_sub(3);
    let mut truncated: String = line.chars().take(keep).collect();
    truncated.push_str("...");
    truncated
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}
