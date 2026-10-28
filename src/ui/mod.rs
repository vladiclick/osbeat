mod details;
mod panels;
mod widgets;

use crate::{app::App, logo::Logo};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

pub(super) const BG: Color = Color::Rgb(13, 17, 26);
pub(super) const PANEL: Color = Color::Rgb(19, 25, 37);
pub(super) const FG: Color = Color::Rgb(224, 232, 247);
pub(super) const MUTED: Color = Color::Rgb(144, 159, 186);
pub(super) const LINE: Color = Color::Rgb(47, 61, 84);
pub(super) const CPU: Color = Color::Rgb(140, 174, 250);
pub(super) const RAM: Color = Color::Rgb(168, 217, 151);
pub(super) const CYAN: Color = Color::Rgb(121, 211, 221);
pub(super) const PINK: Color = Color::Rgb(202, 170, 242);
pub(super) const AMBER: Color = Color::Rgb(237, 196, 136);

pub fn draw(frame: &mut Frame, app: &App, logo: &Logo) {
    let outer = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG).fg(FG)),
        outer,
    );
    if outer.width < 30 || outer.height < 8 {
        frame.render_widget(Paragraph::new("osbeat\nEnlarge terminal\nq quit"), outer);
        return;
    }
    let margin = if outer.width >= 76 { 2 } else { 1 };
    let area = Rect::new(
        margin,
        1,
        outer.width.saturating_sub(margin * 2),
        outer.height.saturating_sub(2),
    );
    let roomy = outer.height >= 28;
    let rows = Layout::vertical([
        Constraint::Length(if roomy { 3 } else { 2 }),
        Constraint::Min(1),
        Constraint::Length(if roomy { 2 } else { 1 }),
    ])
    .split(area);
    header(frame, app, rows[0]);
    dashboard(frame, app, logo, rows[1]);
    footer(frame, rows[2]);
    if app.diagnostics && area.height >= 12 {
        widgets::diagnostics(
            frame,
            app,
            Rect::new(area.x, rows[2].y.saturating_sub(3), area.width, 3),
        );
    }
}

fn header(frame: &mut Frame, app: &App, area: Rect) {
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("◆ ", Style::default().fg(CPU)),
                Span::styled(
                    "osbeat",
                    Style::default().fg(FG).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  /  SYSTEM MONITOR", Style::default().fg(MUTED)),
            ]),
            Line::from(Span::styled(
                format!("{} · {}", app.host.cpu_name, app.host.arch),
                Style::default().fg(MUTED),
            )),
        ]),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(&app.host.hostname, Style::default().fg(FG)),
                Span::styled("  ● LIVE", Style::default().fg(RAM)),
            ]),
            Line::from(Span::styled(
                format!(
                    "{} · UPTIME {}",
                    app.host.os_name,
                    uptime(app.snapshot.uptime)
                ),
                Style::default().fg(MUTED),
            )),
        ])
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn dashboard(frame: &mut Frame, app: &App, logo: &Logo, area: Rect) {
    let minimum_logo_width = logo.width.saturating_add(4).min(u16::MAX as usize) as u16;
    if area.width >= minimum_logo_width.saturating_add(38) && area.height >= 19 {
        let width = (area.width / 3)
            .max(minimum_logo_width)
            .min(area.width.saturating_sub(38));
        let columns = Layout::horizontal([
            Constraint::Length(width),
            Constraint::Length(2),
            Constraint::Min(30),
        ])
        .split(area);
        identity(frame, app, logo, columns[0]);
        panels::stats(frame, app, columns[2]);
    } else {
        panels::stats(frame, app, area);
    }
}

fn identity(frame: &mut Frame, app: &App, logo: &Logo, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(Span::styled(
            format!(" {} ", app.host.os_name),
            Style::default().fg(FG),
        )))
        .border_style(Style::default().fg(LINE))
        .style(Style::default().bg(PANEL));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let info_height = if inner.height >= logo.rows.len() as u16 + 4 {
        4
    } else {
        0
    };
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(info_height)]).split(inner);
    draw_logo(frame, logo, rows[0]);
    if info_height > 0 {
        let lines = vec![
            Line::from(Span::styled(
                &app.host.cpu_name,
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("{} logical cores · {}", app.host.cores, app.host.arch),
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                format!("KERNEL {}", app.host.kernel),
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                format!("UPTIME {}", uptime(app.snapshot.uptime)),
                Style::default().fg(CYAN),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rows[1]);
    }
}

fn draw_logo(frame: &mut Frame, logo: &Logo, area: Rect) {
    let x = area.x + area.width.saturating_sub(logo.width as u16) / 2;
    let y = area.y + area.height.saturating_sub(logo.rows.len() as u16) / 2;
    // Preserve the per-cell palette so the animation engine can transform any logo.
    for (row, glyphs) in logo.rows.iter().take(area.height as usize).enumerate() {
        for (column, glyph) in glyphs.iter().take(area.width as usize).enumerate() {
            frame.buffer_mut()[(x + column as u16, y + row as u16)]
                .set_char(glyph.character)
                .set_fg(glyph.color)
                .set_bg(PANEL);
        }
    }
}

fn footer(frame: &mut Frame, area: Rect) {
    let row = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    let columns = Layout::horizontal([Constraint::Min(23), Constraint::Length(26)]).split(row);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("q", Style::default().fg(FG)),
            Span::styled(" quit   ", Style::default().fg(MUTED)),
            Span::styled("d", Style::default().fg(FG)),
            Span::styled(" diagnostics", Style::default().fg(MUTED)),
        ])),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new("30s HISTORY  /  500ms LIVE")
            .alignment(Alignment::Right)
            .style(Style::default().fg(MUTED)),
        columns[1],
    );
}

pub fn gib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1_u64 << 30) as f64)
}

pub fn rate(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "sampling".into();
    };
    let (divisor, unit) = if bytes >= 1 << 30 {
        (1 << 30, "GiB/s")
    } else if bytes >= 1 << 20 {
        (1 << 20, "MiB/s")
    } else if bytes >= 1 << 10 {
        (1 << 10, "KiB/s")
    } else {
        return format!("{bytes} B/s");
    };
    format!("{:.1} {unit}", bytes as f64 / divisor as f64)
}

pub fn uptime(seconds: u64) -> String {
    let (minutes, hours, days) = (seconds / 60 % 60, seconds / 3600 % 24, seconds / 86400);
    if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}
