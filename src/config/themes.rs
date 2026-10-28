use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Default,
    Catppuccin,
    Tokyonight,
    Gruvbox,
    Phosphor,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub fg: Color,
    pub muted: Color,
    pub line: Color,
    pub cpu: Color,
    pub ram: Color,
    pub cyan: Color,
    pub pink: Color,
    pub amber: Color,
}

impl ThemeName {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Catppuccin,
            Self::Catppuccin => Self::Tokyonight,
            Self::Tokyonight => Self::Gruvbox,
            Self::Gruvbox => Self::Phosphor,
            Self::Phosphor => Self::Default,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Catppuccin => "catppuccin",
            Self::Tokyonight => "tokyonight",
            Self::Gruvbox => "gruvbox",
            Self::Phosphor => "phosphor",
        }
    }

    pub fn palette(self) -> Palette {
        let colors = match self {
            Self::Default => [
                0x080e17, 0x101a29, 0xe8f3ff, 0x8299b3, 0x284055, 0x7bbaff, 0x94e2b9, 0x60e6ed,
                0xe6a1ec, 0xf6ca7c,
            ],
            Self::Catppuccin => [
                0x1e1e2e, 0x242436, 0xcdd6f4, 0x9399b2, 0x45475a, 0x89b4fa, 0xa6e3a1, 0x89dceb,
                0xf5c2e7, 0xf9e2af,
            ],
            Self::Tokyonight => [
                0x16161e, 0x1a1b26, 0xc0caf5, 0x8c9bc4, 0x3b4261, 0x7aa2f7, 0x9ece6a, 0x7dcfff,
                0xbb9af7, 0xe0af68,
            ],
            Self::Gruvbox => [
                0x1d2021, 0x282828, 0xebdbb2, 0xa89984, 0x504945, 0x83a598, 0xb8bb26, 0x8ec07c,
                0xd3869b, 0xfabd2f,
            ],
            Self::Phosphor => [
                0x050c07, 0x09140d, 0xcdffdc, 0x79aa87, 0x234a30, 0x68ff98, 0xb3ff87, 0x63e6c0,
                0x9cf8aa, 0xe3ec8b,
            ],
        };
        let [bg, panel, fg, muted, line, cpu, ram, cyan, pink, amber] = colors.map(rgb);
        Palette {
            bg,
            panel,
            fg,
            muted,
            line,
            cpu,
            ram,
            cyan,
            pink,
            amber,
        }
    }
}

fn rgb(value: u32) -> Color {
    Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}
