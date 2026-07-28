#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Nodes,
    Traffic,
    Logs,
}

impl FocusedPanel {
    pub fn next(self) -> Self {
        match self {
            Self::Nodes => Self::Traffic,
            Self::Traffic => Self::Logs,
            Self::Logs => Self::Nodes,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Nodes => Self::Logs,
            Self::Traffic => Self::Nodes,
            Self::Logs => Self::Traffic,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Nodes => "Nodes",
            Self::Traffic => "Traffic",
            Self::Logs => "Logs",
        }
    }
}

pub struct AppState {
    pub focused: FocusedPanel,
    pub should_quit: bool,
    pub daemon_connected: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            focused: FocusedPanel::Nodes,
            should_quit: false,
            daemon_connected: false,
        }
    }

    pub fn focus_next(&mut self) {
        self.focused = self.focused.next();
    }

    pub fn focus_prev(&mut self) {
        self.focused = self.focused.prev();
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
