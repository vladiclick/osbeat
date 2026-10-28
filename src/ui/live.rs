//! Cache the measured dashboard at collector speed, then repaint only logo pixels.
use crate::{
    animation::{Engine, Style},
    app::App,
    config::GlyphMode,
    logo::Logo,
};
use ratatui::{Frame, Terminal, backend::Backend, buffer::Buffer, layout::Rect, style::Color};

#[derive(Clone, Copy)]
struct Pixel {
    x: u16,
    y: u16,
    source_x: usize,
    source_y: usize,
    character: char,
    color: Color,
}

#[derive(Default)]
pub struct Scene {
    base: Option<Buffer>,
    display: Option<Buffer>,
    area: Rect,
    pixels: Vec<Pixel>,
    updates: Vec<usize>,
}

impl Scene {
    pub fn has_logo(&self) -> bool {
        !self.pixels.is_empty()
    }
    /// The animation backend bypasses full-screen diffing. Before the next
    /// measured frame, make Ratatui's previous buffer match the actual display.
    pub fn sync_before_draw<B: Backend>(&self, terminal: &mut Terminal<B>) {
        if let Some(display) = &self.display
            && terminal.current_buffer_mut().area == display.area
        {
            terminal.current_buffer_mut().clone_from(display);
            terminal.swap_buffers();
        }
    }

    /// Repaint only changed logo cells; untouched charts are never scanned at 30 fps.
    pub fn animate<B: Backend>(
        &mut self,
        backend: &mut B,
        app: &App,
        logo: &Logo,
        engine: &Engine,
    ) -> Result<(), B::Error> {
        let Some(display) = &mut self.display else {
            return Ok(());
        };
        let style = effect(&app.config.effect);
        self.updates.clear();
        for (index, pixel) in self.pixels.iter().enumerate() {
            let color = engine.color(
                pixel.source_x,
                pixel.source_y,
                logo.width,
                logo.rows.len(),
                pixel.color,
                style,
                app.config.intensity,
            );
            let color = if app.config.intensity > 0.0 {
                quantize(color)
            } else {
                color
            };
            let cell = &mut display[(pixel.x, pixel.y)];
            if cell.fg != color {
                cell.set_fg(color);
                self.updates.push(index);
            }
        }
        if self.updates.is_empty() {
            return Ok(());
        }
        backend.draw(self.updates.iter().map(|&index| {
            let p = &self.pixels[index];
            (p.x, p.y, &display[(p.x, p.y)])
        }))?;
        backend.flush()
    }
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        app: &App,
        logo: &Logo,
        engine: &Engine,
        dirty: bool,
    ) {
        if dirty || self.base.is_none() || self.area != frame.area() {
            self.area = frame.area();
            let region = super::base(frame, app, logo);
            self.pixels =
                region.map_or_else(Vec::new, |area| raster(logo, area, app.config.glyphs));
            if app.diagnostics && frame.area().height >= 14 {
                let top = frame.area().bottom().saturating_sub(5);
                self.pixels
                    .retain(|pixel| pixel.y < top || pixel.y >= top + 3);
            }
            self.base = Some(frame.buffer_mut().clone());
        } else if let Some(base) = &self.base {
            // Ratatui clears the next buffer after diffing. Reuse our static scene
            // instead of rebuilding charts, strings, layouts and host info at 30 fps.
            frame.buffer_mut().clone_from(base);
        }
        let style = effect(&app.config.effect);
        for pixel in &self.pixels {
            let color = engine.color(
                pixel.source_x,
                pixel.source_y,
                logo.width,
                logo.rows.len(),
                pixel.color,
                style,
                app.config.intensity,
            );
            // A 4-bit glow palette avoids tiny RGB changes that trigger full ANSI
            // color commands. Wave fronts still advance at the configured frame rate.
            let color = if app.config.intensity > 0.0 {
                quantize(color)
            } else {
                color
            };
            frame.buffer_mut()[(pixel.x, pixel.y)]
                .set_char(pixel.character)
                .set_fg(color);
        }
        if app.diagnostics && frame.area().height >= 14 {
            let area = frame.area();
            super::widgets::diagnostics(
                frame,
                app,
                Rect::new(
                    area.x + 1,
                    area.bottom().saturating_sub(5),
                    area.width.saturating_sub(2),
                    3,
                ),
            );
        }
        self.display = Some(frame.buffer_mut().clone());
    }
}

fn quantize(color: Color) -> Color {
    let channel = |value: u8| (((u16::from(value) + 8) / 16) * 16).min(255) as u8;
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(channel(r), channel(g), channel(b)),
        _ => color,
    }
}

pub(super) fn effect(name: &str) -> Style {
    match name {
        "liquid" => Style::Liquid,
        "spectrum" => Style::Spectrum,
        "original" => Style::Original,
        _ => Style::Aurora,
    }
}

fn raster(logo: &Logo, area: Rect, mode: GlyphMode) -> Vec<Pixel> {
    if area.is_empty() {
        return Vec::new();
    }
    let factor = (area.width as f32 / logo.width.max(1) as f32)
        .min(area.height as f32 / logo.rows.len().max(1) as f32);
    // Bound painted cells so a large terminal doesn't multiply animation cost.
    let visible = logo
        .rows
        .iter()
        .flatten()
        .filter(|glyph| glyph.character != ' ')
        .count()
        .max(1);
    let factor = factor.min((700.0 / visible as f32).sqrt());
    let width = (logo.width as f32 * factor).floor().max(1.0) as u16;
    let height = (logo.rows.len() as f32 * factor).floor().max(1.0) as u16;
    let left = area.x + area.width.saturating_sub(width) / 2;
    let top = area.y + area.height.saturating_sub(height) / 2;
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height.min(area.height) {
        let source_y = (y as usize * logo.rows.len() / height as usize).min(logo.rows.len() - 1);
        for x in 0..width.min(area.width) {
            let source_x = (x as usize * logo.width / width as usize).min(logo.width - 1);
            let Some(glyph) = logo.rows[source_y]
                .get(source_x)
                .filter(|glyph| glyph.character != ' ')
            else {
                continue;
            };
            let character = match mode {
                GlyphMode::Solid => '█',
                GlyphMode::Ascii => glyph.character,
                GlyphMode::Dots => '⣿',
            };
            pixels.push(Pixel {
                x: left + x,
                y: top + y,
                source_x,
                source_y,
                character,
                color: glyph.color,
            });
        }
    }
    pixels
}
