//! Cache the measured dashboard at collector speed, then repaint only logo pixels.
use crate::{
    animation::{Engine, Style},
    app::App,
    logo::Logo,
};
use ratatui::{Frame, Terminal, backend::Backend, buffer::Buffer, layout::Rect, style::Color};

use super::raster::{Pixel, raster};

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
        _logo: &Logo,
        engine: &Engine,
    ) -> Result<(), B::Error> {
        let Some(display) = &mut self.display else {
            return Ok(());
        };
        let style = effect(&app.config.effect);
        self.updates.clear();
        for (index, pixel) in self.pixels.iter().enumerate() {
            let (fg, bg) = colors(pixel, engine, style, app.config.intensity);
            let cell = &mut display[(pixel.x, pixel.y)];
            if cell.fg != fg || cell.bg != bg {
                cell.set_fg(fg).set_bg(bg);
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
            for pixel in &mut self.pixels {
                pixel.background = frame.buffer_mut()[(pixel.x, pixel.y)].bg;
            }
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
            let (fg, bg) = colors(pixel, engine, style, app.config.intensity);
            frame.buffer_mut()[(pixel.x, pixel.y)]
                .set_char(pixel.character)
                .set_fg(fg)
                .set_bg(bg);
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

fn colors(pixel: &Pixel, engine: &Engine, style: Style, intensity: f32) -> (Color, Color) {
    let color = engine.color(
        pixel.source_x,
        pixel.source_y,
        pixel.field_width,
        pixel.field_height,
        pixel.color,
        style,
        intensity,
    );
    // Quantization avoids terminal writes for tiny, invisible RGB differences.
    let color = if intensity > 0.0 {
        quantize(color)
    } else {
        color
    };
    match pixel.coverage {
        Some([255, 255]) | None => (color, pixel.background),
        // For ▀, the foreground paints the top half and the background the bottom.
        Some([upper, lower]) => (
            blend(color, pixel.background, upper),
            blend(color, pixel.background, lower),
        ),
    }
}

fn blend(foreground: Color, background: Color, alpha: u8) -> Color {
    let Color::Rgb(r, g, b) = foreground else {
        return foreground;
    };
    let Color::Rgb(br, bg, bb) = background else {
        return foreground;
    };
    let channel = |fg: u8, bg: u8| {
        ((u32::from(fg) * u32::from(alpha) + u32::from(bg) * (255 - u32::from(alpha)) + 127) / 255)
            as u8
    };
    Color::Rgb(channel(r, br), channel(g, bg), channel(b, bb))
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
