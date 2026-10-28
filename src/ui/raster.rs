//! Turn data assets into cached terminal cells. No OS-specific geometry lives here.
use crate::{config::GlyphMode, logo::Logo};
use ratatui::{layout::Rect, style::Color};

#[derive(Clone, Copy)]
pub(super) struct Pixel {
    pub x: u16,
    pub y: u16,
    pub source_x: usize,
    pub source_y: usize,
    pub field_width: usize,
    pub field_height: usize,
    pub character: char,
    pub color: Color,
    pub coverage: Option<[u8; 2]>,
    pub background: Color,
}

pub(super) fn raster(logo: &Logo, area: Rect, mode: GlyphMode) -> Vec<Pixel> {
    if area.is_empty() {
        return Vec::new();
    }
    if logo.mask.is_some() && mode != GlyphMode::Ascii {
        return detailed(logo, area, mode);
    }
    ascii(logo, area, mode)
}

fn detailed(logo: &Logo, area: Rect, mode: GlyphMode) -> Vec<Pixel> {
    let mask = logo.mask.as_ref().expect("detailed raster needs a mask");
    // Terminal cells are about twice as tall as they are wide. Two vertical
    // subpixels per cell restore square-pixel proportions without image protocols.
    let aspect = mask.aspect() * 2.0;
    let height = (area.height as f32)
        .min(area.width as f32 / aspect)
        .min((1000.0 / aspect).sqrt())
        .floor()
        .max(1.0) as u16;
    let width = (height as f32 * aspect).floor().max(1.0) as u16;
    let width = width.min(area.width);
    let height = height.min(area.height);
    let left = area.x + (area.width - width) / 2;
    let top = area.y + (area.height - height) / 2;
    let fallback = logo
        .rows
        .iter()
        .flatten()
        .find(|g| g.character != ' ')
        .map_or(Color::White, |g| g.color);
    let mut pixels = Vec::new();
    for y in 0..height {
        let row = &logo.rows[y as usize * logo.rows.len() / height as usize];
        let row_fallback = row
            .iter()
            .find(|g| g.character != ' ')
            .map_or(fallback, |g| g.color);
        for x in 0..width {
            let (character, coverage) = match mode {
                GlyphMode::Dots => {
                    let bits = [
                        (0, 0, 0),
                        (0, 1, 1),
                        (0, 2, 2),
                        (1, 0, 3),
                        (1, 1, 4),
                        (1, 2, 5),
                        (0, 3, 6),
                        (1, 3, 7),
                    ]
                    .into_iter()
                    .fold(0, |bits, (dx, dy, bit)| {
                        let a = mask.coverage(
                            x as usize * 2 + dx,
                            y as usize * 4 + dy,
                            width as usize * 2,
                            height as usize * 4,
                        );
                        bits | if a >= 96 { 1 << bit } else { 0 }
                    });
                    if bits == 0 {
                        continue;
                    }
                    (char::from_u32(0x2800 + bits).unwrap(), None)
                }
                _ => {
                    let upper = mask.coverage(
                        x as usize,
                        y as usize * 2,
                        width as usize,
                        height as usize * 2,
                    );
                    let lower = mask.coverage(
                        x as usize,
                        y as usize * 2 + 1,
                        width as usize,
                        height as usize * 2,
                    );
                    if upper == 0 && lower == 0 {
                        continue;
                    }
                    (
                        if upper == 255 && lower == 255 {
                            '█'
                        } else {
                            '▀'
                        },
                        Some([upper, lower]),
                    )
                }
            };
            let color = row
                .get(x as usize * logo.width / width as usize)
                .filter(|g| g.character != ' ')
                .map_or(row_fallback, |g| g.color);
            pixels.push(Pixel {
                x: left + x,
                y: top + y,
                source_x: x as usize,
                source_y: y as usize,
                field_width: width as usize,
                field_height: height as usize,
                character,
                color,
                coverage,
                background: Color::Reset,
            });
        }
    }
    pixels
}

fn ascii(logo: &Logo, area: Rect, mode: GlyphMode) -> Vec<Pixel> {
    let factor = (area.width as f32 / logo.width.max(1) as f32)
        .min(area.height as f32 / logo.rows.len().max(1) as f32);
    let visible = logo
        .rows
        .iter()
        .flatten()
        .filter(|g| g.character != ' ')
        .count()
        .max(1);
    let factor = factor.min((700.0 / visible as f32).sqrt());
    let width = (logo.width as f32 * factor).floor().max(1.0) as u16;
    let height = (logo.rows.len() as f32 * factor).floor().max(1.0) as u16;
    let left = area.x + area.width.saturating_sub(width) / 2;
    let top = area.y + area.height.saturating_sub(height) / 2;
    let mut pixels = Vec::new();
    for y in 0..height.min(area.height) {
        let source_y = (y as usize * logo.rows.len() / height as usize).min(logo.rows.len() - 1);
        for x in 0..width.min(area.width) {
            let source_x = (x as usize * logo.width / width as usize).min(logo.width - 1);
            let Some(glyph) = logo.rows[source_y]
                .get(source_x)
                .filter(|g| g.character != ' ')
            else {
                continue;
            };
            pixels.push(Pixel {
                x: left + x,
                y: top + y,
                source_x,
                source_y,
                field_width: logo.width,
                field_height: logo.rows.len(),
                character: match mode {
                    GlyphMode::Solid => '█',
                    GlyphMode::Ascii => glyph.character,
                    GlyphMode::Dots => '⣿',
                },
                color: glyph.color,
                coverage: None,
                background: Color::Reset,
            });
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_transparent_holes_survive_both_rendering_modes() {
        let logo = Logo::parse(
            "name='ring'\ncolors=['#ffffff']\nart='X'\nmask='''ffff\nf00f\nf00f\nffff'''\n",
        )
        .unwrap();
        for mode in [GlyphMode::Solid, GlyphMode::Dots] {
            let pixels = raster(&logo, Rect::new(3, 5, 40, 20), mode);
            assert!(
                !pixels.iter().any(|p| p.x == 23 && p.y == 15),
                "hole must remain transparent"
            );
            assert!(
                pixels
                    .iter()
                    .all(|p| p.x >= 3 && p.x < 43 && p.y >= 5 && p.y < 25)
            );
            assert!(pixels.iter().any(|p| p.x == 3));
            assert!(pixels.iter().any(|p| p.y == 5));
        }
    }

    #[test]
    fn detailed_geometry_stays_bounded_in_huge_or_narrow_viewports() {
        let logo = Logo::for_os(crate::model::Os::Mac).unwrap();
        for mode in [GlyphMode::Solid, GlyphMode::Dots, GlyphMode::Ascii] {
            for area in [
                Rect::new(0, 0, 0, 0),
                Rect::new(4, 6, 1, 1),
                Rect::new(3, 5, 2, 50),
                Rect::new(0, 0, 1000, 1000),
            ] {
                let pixels = raster(&logo, area, mode);
                assert!(pixels.len() <= 1000);
                assert!(pixels.iter().all(|p| area.contains((p.x, p.y).into())));
            }
        }
    }
}
