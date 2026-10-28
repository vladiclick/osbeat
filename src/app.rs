use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::model::{History, HostInfo, Snapshot};

#[derive(Clone, Debug)]
pub struct App {
    pub host: HostInfo,
    pub snapshot: Snapshot,
    pub history: History,
    pub diagnostics: bool,
}

impl App {
    pub fn new(host: HostInfo) -> Self {
        Self {
            host,
            snapshot: Snapshot::default(),
            history: History::default(),
            diagnostics: false,
        }
    }

    pub fn with_snapshot(&self, snapshot: Snapshot) -> Self {
        Self {
            history: self.history.with_sample(&snapshot),
            snapshot,
            ..self.clone()
        }
    }

    pub fn with_diagnostics(&self, diagnostics: bool) -> Self {
        Self {
            diagnostics,
            ..self.clone()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    Diagnostics,
    Redraw,
    None,
}

impl Action {
    pub fn from_event(event: &Event) -> Self {
        match event {
            Event::Resize(_, _) => Self::Redraw,
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => Self::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Self::Quit,
                KeyCode::Char('d') | KeyCode::Char('D') => Self::Diagnostics,
                _ => Self::None,
            },
            _ => Self::None,
        }
    }
}
