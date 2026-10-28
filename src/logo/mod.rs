use std::io::{self, ErrorKind};

use ratatui::style::Color;
use serde::Deserialize;

use crate::model::Os;

#[derive(Clone, Debug)]
pub struct Glyph {
    pub character: char,
    pub color: Color,
}

#[derive(Debug)]
pub struct Logo {
    pub name: String,
    pub rows: Vec<Vec<Glyph>>,
    pub width: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Asset {
    name: String,
    colors: Vec<String>,
    art: String,
}

impl Logo {
    pub fn for_os(os: Os) -> io::Result<Self> {
        let data = match os {
            Os::Mac => include_str!("../../assets/logos/macos.toml"),
            _ => include_str!("../../assets/logos/unknown.toml"),
        };
        Self::parse(data)
    }

    pub fn parse(data: &str) -> io::Result<Self> {
        let asset: Asset = toml::from_str(data).map_err(invalid)?;
        if asset.art.is_empty() || asset.colors.is_empty() || !asset.art.is_ascii() {
            return Err(invalid("logo needs ASCII art and a nonempty palette"));
        }
        let palette: Vec<Color> = asset
            .colors
            .iter()
            .map(|c| parse_color(c))
            .collect::<io::Result<_>>()?;
        let mut color = palette[0];
        let rows = asset
            .art
            .lines()
            .map(|line| parse_row(line, &palette, &mut color))
            .collect::<io::Result<Vec<_>>>()?;
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        if width == 0 {
            return Err(invalid("logo is empty"));
        }
        Ok(Self {
            name: asset.name,
            rows,
            width,
        })
    }
}

fn parse_color(value: &str) -> io::Result<Color> {
    let hex = value
        .strip_prefix('#')
        .filter(|s| s.len() == 6)
        .ok_or_else(|| invalid("expected #rrggbb"))?;
    let rgb = u32::from_str_radix(hex, 16).map_err(invalid)?;
    Ok(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))
}

fn parse_row(line: &str, palette: &[Color], color: &mut Color) -> io::Result<Vec<Glyph>> {
    let mut chars = line.chars();
    let mut row = Vec::with_capacity(line.len());
    while let Some(character) = chars.next() {
        if character == '$' {
            let id = chars
                .next()
                .and_then(|c| c.to_digit(10))
                .and_then(|n| n.checked_sub(1));
            *color = id
                .and_then(|n| palette.get(n as usize))
                .copied()
                .ok_or_else(|| invalid("invalid color marker"))?;
        } else if character.is_control() {
            return Err(invalid("control characters are not allowed in logos"));
        } else {
            row.push(Glyph {
                character,
                color: *color,
            });
        }
    }
    Ok(row)
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, error.to_string())
}
