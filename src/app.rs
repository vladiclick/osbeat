use crate::{
    config::{Config, LayoutMode},
    model::{History, HostInfo, Snapshot},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

pub const SETTING_COUNT: usize = 14;

#[derive(Clone, Debug)]
pub struct App {
    pub host: HostInfo,
    pub snapshot: Snapshot,
    pub history: History,
    pub diagnostics: bool,
    pub config: Config,
    pub customization: bool,
    pub selected: usize,
    pub help: bool,
    pub demo: bool,
    pub notice: Option<String>,
}

impl App {
    pub fn new(host: HostInfo) -> Self {
        Self {
            host,
            snapshot: Snapshot::default(),
            history: History::default(),
            diagnostics: false,
            config: Config::default(),
            customization: false,
            selected: 0,
            help: false,
            demo: false,
            notice: None,
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
    pub fn with_config(&self, config: Config) -> Self {
        Self {
            config,
            ..self.clone()
        }
    }
    pub fn with_notice(&self, notice: impl Into<String>) -> Self {
        Self {
            notice: Some(crate::model::clean(&notice.into())),
            ..self.clone()
        }
    }

    pub fn apply(&self, action: Action) -> Self {
        if self.help && !matches!(action, Action::Close | Action::Help | Action::Quit) {
            return self.clone();
        }
        let config = &self.config;
        match action {
            Action::Diagnostics => self.with_diagnostics(!self.diagnostics),
            Action::Customize => Self {
                customization: !self.customization,
                help: false,
                notice: None,
                ..self.clone()
            },
            Action::Help => Self {
                help: !self.help,
                customization: false,
                ..self.clone()
            },
            Action::Close => Self {
                help: false,
                customization: false,
                notice: None,
                ..self.clone()
            },
            Action::Theme => self.with_config(Config {
                theme: config.theme.next(),
                ..config.clone()
            }),
            Action::Effect => self.with_config(Config {
                effect: next_effect(&config.effect).into(),
                ..config.clone()
            }),
            Action::Glyphs => self.with_config(Config {
                glyphs: config.glyphs.next(),
                ..config.clone()
            }),
            Action::Motion => self.with_config(Config {
                reduced_motion: !config.reduced_motion,
                ..config.clone()
            }),
            Action::View => self.with_config(Config {
                layout: if config.layout == LayoutMode::Focus {
                    LayoutMode::Dashboard
                } else {
                    LayoutMode::Focus
                },
                ..config.clone()
            }),
            Action::Demo => Self {
                demo: !self.demo,
                ..self.clone()
            },
            Action::Select(delta) if self.customization => Self {
                selected: (self.selected as i32 + delta).rem_euclid(SETTING_COUNT as i32) as usize,
                ..self.clone()
            },
            Action::Adjust(delta) if self.customization => self.adjust(delta),
            _ => self.clone(),
        }
    }

    fn adjust(&self, delta: i32) -> Self {
        let c = &self.config;
        let panels = c.panels;
        let config = match self.selected {
            0 => Config {
                theme: cycle(c.theme, delta, 5, |theme| theme.next()),
                ..c.clone()
            },
            1 => Config {
                effect: cycle(c.effect.clone(), delta, 4, |effect| {
                    next_effect(&effect).into()
                }),
                ..c.clone()
            },
            2 => Config {
                glyphs: cycle(c.glyphs, delta, 3, |glyphs| glyphs.next()),
                ..c.clone()
            },
            3 => Config {
                speed: (c.speed + delta as f32 * 0.25).clamp(0.25, 3.0),
                ..c.clone()
            },
            4 => Config {
                intensity: (c.intensity + delta as f32 * 0.05).clamp(0.0, 1.0),
                ..c.clone()
            },
            5 => Config {
                fps: (c.fps as i32 + delta * 5).clamp(10, 60) as u16,
                ..c.clone()
            },
            6 => Config {
                reduced_motion: !c.reduced_motion,
                ..c.clone()
            },
            7 => Config {
                layout: if c.layout == LayoutMode::Focus {
                    LayoutMode::Dashboard
                } else {
                    LayoutMode::Focus
                },
                ..c.clone()
            },
            8 => Config {
                panels: crate::config::Panels {
                    cpu: !panels.cpu,
                    ..panels
                },
                ..c.clone()
            },
            9 => Config {
                panels: crate::config::Panels {
                    memory: !panels.memory,
                    ..panels
                },
                ..c.clone()
            },
            10 => Config {
                panels: crate::config::Panels {
                    network: !panels.network,
                    ..panels
                },
                ..c.clone()
            },
            11 => Config {
                panels: crate::config::Panels {
                    storage: !panels.storage,
                    ..panels
                },
                ..c.clone()
            },
            12 => Config {
                panels: crate::config::Panels {
                    power: !panels.power,
                    ..panels
                },
                ..c.clone()
            },
            _ => {
                return Self {
                    demo: !self.demo,
                    ..self.clone()
                };
            }
        };
        self.with_config(config)
    }
}

fn next_effect(effect: &str) -> &'static str {
    match effect {
        "aurora" => "liquid",
        "liquid" => "spectrum",
        "spectrum" => "original",
        _ => "aurora",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    Diagnostics,
    Redraw,
    None,
    Customize,
    Help,
    Close,
    Theme,
    Effect,
    Glyphs,
    Motion,
    View,
    Demo,
    Save,
    Select(i32),
    Adjust(i32),
}

impl Action {
    pub fn from_event(event: &Event) -> Self {
        match event {
            Event::Resize(_, _) => Self::Redraw,
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q' | 'Q') => Self::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Self::Quit,
                KeyCode::Char('d' | 'D') => Self::Diagnostics,
                KeyCode::Char('c' | 'C') => Self::Customize,
                KeyCode::Char('?') => Self::Help,
                KeyCode::Esc => Self::Close,
                KeyCode::Char('t' | 'T') => Self::Theme,
                KeyCode::Char('e' | 'E') => Self::Effect,
                KeyCode::Char('g' | 'G') => Self::Glyphs,
                KeyCode::Char('a' | 'A') | KeyCode::Char(' ') => Self::Motion,
                KeyCode::Tab => Self::View,
                KeyCode::Char('m' | 'M') => Self::Demo,
                KeyCode::Char('s' | 'S') => Self::Save,
                KeyCode::Up | KeyCode::Char('k') => Self::Select(-1),
                KeyCode::Down | KeyCode::Char('j') => Self::Select(1),
                KeyCode::Left | KeyCode::Char('-') => Self::Adjust(-1),
                KeyCode::Right | KeyCode::Enter | KeyCode::Char('+') => Self::Adjust(1),
                _ => Self::None,
            },
            _ => Self::None,
        }
    }
}

fn cycle<T>(value: T, delta: i32, count: i32, next: impl Fn(T) -> T) -> T {
    (0..delta.rem_euclid(count)).fold(value, |value, _| next(value))
}
