mod details;
mod focus;
pub mod live;
mod panels;
mod raster;
mod settings;
mod widgets;

use crate::{animation::Engine, app::App, config::LayoutMode, logo::Logo};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

/// Deterministic entry point for widgets/tests; the runtime retains Scene and Engine.
pub fn draw(frame: &mut Frame, app: &App, logo: &Logo) {
    live::Scene::default().draw(frame, app, logo, &Engine::default(), true);
}

pub(super) fn base(frame: &mut Frame, app: &App, logo: &Logo) -> Option<Rect> {
    let outer = frame.area();
    let palette = app.config.theme.palette();
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.bg).fg(palette.fg)),
        outer,
    );
    if outer.width < 30 || outer.height < 8 {
        frame.render_widget(Paragraph::new("osbeat\nEnlarge terminal\nq quit"), outer);
        return None;
    }
    let margin = if outer.width >= 76 { 2 } else { 1 };
    let area = Rect::new(
        outer.x + margin,
        outer.y + 1,
        outer.width.saturating_sub(margin * 2),
        outer.height.saturating_sub(2),
    );
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(if area.height >= 20 { 2 } else { 1 }),
    ])
    .split(area);
    header(frame, app, rows[0]);
    let region = if app.customization {
        let width = rows[1].width.min(44);
        if rows[1].width >= 78 {
            let columns = Layout::horizontal([
                Constraint::Min(32),
                Constraint::Length(2),
                Constraint::Length(width),
            ])
            .split(rows[1]);
            settings::customize(frame, app, columns[2]);
            focus::canvas(frame, app, logo, columns[0], false)
        } else {
            settings::customize(frame, app, rows[1]);
            None
        }
    } else if app.config.layout == LayoutMode::Focus {
        focus::canvas(frame, app, logo, rows[1], true)
    } else {
        dashboard(frame, app, logo, rows[1])
    };
    footer(frame, app, rows[2]);
    if app.help {
        settings::help(frame, app, rows[1]);
        return None;
    }
    region
}

fn header(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let columns =
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("◆ ", Style::default().fg(p.cyan)),
                Span::styled(
                    "osbeat",
                    Style::default().fg(p.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" / LIVING SYSTEM", Style::default().fg(p.muted)),
            ]),
            Line::from(Span::styled(
                format!("{} · {}", app.host.os_name, app.host.cpu_name),
                Style::default().fg(p.muted),
            )),
        ]),
        columns[0],
    );
    let status = if app.demo {
        "◈ DEMO LOGO"
    } else if app.config.reduced_motion {
        "Ⅱ PAUSED"
    } else {
        "● LIVE"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("{status}  / {}", app.config.effect.to_uppercase()),
                Style::default().fg(if app.demo { p.amber } else { p.cyan }),
            )),
            Line::from(Span::styled(
                format!("{} · {}", app.host.hostname, app.config.theme.label()),
                Style::default().fg(p.muted),
            )),
        ])
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn dashboard(frame: &mut Frame, app: &App, logo: &Logo, area: Rect) -> Option<Rect> {
    let minimum = logo.width.saturating_add(4).min(u16::MAX as usize) as u16;
    if area.width >= minimum.saturating_add(38) && area.height >= 19 {
        let width = (area.width / 3)
            .max(minimum)
            .min(area.width.saturating_sub(38));
        let columns = Layout::horizontal([
            Constraint::Length(width),
            Constraint::Length(2),
            Constraint::Min(30),
        ])
        .split(area);
        panels::stats(frame, app, columns[2]);
        focus::canvas(frame, app, logo, columns[0], false)
    } else {
        panels::stats(frame, app, area);
        None
    }
}

fn footer(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    if area.height > 1 {
        let line = app
            .notice
            .as_deref()
            .unwrap_or("CPU breathes · RAM fills · Network flows");
        widgets::text(
            frame,
            Rect::new(area.x, area.y, area.width, 1),
            line,
            if app.notice.is_some() {
                p.amber
            } else {
                p.muted
            },
        );
    }
    let keys = if app.customization {
        "↑↓ select  ←→ change  s save  Esc close"
    } else {
        "Tab view  c customize  t theme  e effect  ? keys  q quit"
    };
    widgets::text(
        frame,
        Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        keys,
        p.muted,
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
