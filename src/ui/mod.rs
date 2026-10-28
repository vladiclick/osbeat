mod panels;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::App, logo::Logo};

pub(super) const BG: Color = Color::Rgb(17, 21, 29);
pub(super) const FG: Color = Color::Rgb(220, 226, 237);
pub(super) const MUTED: Color = Color::Rgb(134, 147, 169);
pub(super) const LINE: Color = Color::Rgb(45, 55, 72);
pub(super) const CPU: Color = Color::Rgb(140, 170, 238);
pub(super) const RAM: Color = Color::Rgb(166, 209, 137);

pub fn draw(frame: &mut Frame, app: &App, logo: &Logo) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG).fg(FG)), area);
    if area.width < 30 || area.height < 8 {
        frame.render_widget(Paragraph::new("osbeat\nEnlarge terminal\nq quit"), area);
        return;
    }
    let margin = if area.width >= 76 { 2 } else { 1 };
    let area = Rect::new(
        margin,
        1,
        area.width.saturating_sub(margin * 2),
        area.height.saturating_sub(2),
    );
    let parts = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    header(frame, app, parts[0]);
    dashboard(frame, app, logo, parts[1]);
    footer(frame, app, parts[2]);
    if app.diagnostics && area.height >= 12 {
        panels::diagnostics(
            frame,
            app,
            Rect::new(area.x, parts[2].y.saturating_sub(3), area.width, 3),
        );
    }
}

fn header(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::horizontal([Constraint::Length(24), Constraint::Min(0)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "osbeat",
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  /  ", Style::default().fg(LINE)),
            Span::styled("● ", Style::default().fg(RAM)),
            Span::styled("LIVE", Style::default().fg(MUTED)),
        ])),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(app.host.hostname.as_str())
            .alignment(Alignment::Right)
            .style(Style::default().fg(MUTED)),
        cols[1],
    );
}

fn dashboard(frame: &mut Frame, app: &App, logo: &Logo, area: Rect) {
    if area.width >= 72 && area.height >= 17 {
        let logo_width = (logo.width as u16 + 6).max(36).min(area.width / 2);
        let cols = Layout::horizontal([
            Constraint::Length(logo_width),
            Constraint::Length(3),
            Constraint::Min(28),
        ])
        .split(area);
        draw_logo(frame, logo, cols[0]);
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(LINE)),
            Rect::new(cols[1].x + 1, cols[1].y, 1, cols[1].height),
        );
        panels::stats(frame, app, cols[2]);
    } else if area.height >= 36 && area.width >= logo.width as u16 {
        let rows = Layout::vertical([
            Constraint::Length(logo.rows.len() as u16 + 2),
            Constraint::Min(12),
        ])
        .split(area);
        draw_logo(frame, logo, rows[0]);
        panels::stats(frame, app, rows[1]);
    } else {
        panels::stats(frame, app, area);
    }
}

fn draw_logo(frame: &mut Frame, logo: &Logo, area: Rect) {
    let x = area.x + area.width.saturating_sub(logo.width as u16) / 2;
    let y = area.y + area.height.saturating_sub(logo.rows.len() as u16) / 2;
    // Each character is a cell with its own base color. The future animation
    // engine can transform those colors without knowing which OS this is.
    for (row, glyphs) in logo.rows.iter().take(area.height as usize).enumerate() {
        for (column, glyph) in glyphs.iter().take(area.width as usize).enumerate() {
            frame.buffer_mut()[(x + column as u16, y + row as u16)]
                .set_char(glyph.character)
                .set_fg(glyph.color)
                .set_bg(BG);
        }
    }
}

fn footer(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::horizontal([Constraint::Min(22), Constraint::Length(14)]).split(area);
    let label = if app.diagnostics {
        "d hide diagnostics"
    } else {
        "d diagnostics"
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("q", Style::default().fg(FG)),
            Span::styled(format!(" quit   {label}"), Style::default().fg(MUTED)),
        ])),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new("500 ms / live")
            .alignment(Alignment::Right)
            .style(Style::default().fg(MUTED)),
        cols[1],
    );
}

pub fn gib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1_u64 << 30) as f64)
}

pub fn uptime(seconds: u64) -> String {
    let minutes = seconds / 60 % 60;
    let hours = seconds / 3600 % 24;
    let days = seconds / 86400;
    if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}
