use super::{
    gib,
    panels::at,
    rate, uptime,
    widgets::{spark, text},
};
use crate::{app::App, logo::Logo};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) fn canvas(
    frame: &mut Frame,
    app: &App,
    _logo: &Logo,
    area: Rect,
    readouts: bool,
) -> Option<Rect> {
    if area.height < 3 {
        super::details::compact(frame, app, area);
        return None;
    }
    let p = app.config.theme.palette();
    let chart_height = if readouts && area.height >= 20 {
        4
    } else if readouts {
        2
    } else {
        0
    };
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(chart_height)]).split(area);
    let space = rows[0];
    let region = if readouts && space.width >= 88 && space.height >= 14 {
        let columns = Layout::horizontal([
            Constraint::Length(21),
            Constraint::Min(28),
            Constraint::Length(21),
        ])
        .split(space);
        left(frame, app, columns[0]);
        right(frame, app, columns[2]);
        columns[1]
    } else {
        space
    };
    let caption = if app.demo {
        "DEMO EFFECTS · STATS ARE LIVE".to_string()
    } else {
        format!(
            "{} · {}",
            app.config.effect.to_uppercase(),
            app.config.glyphs.label().to_uppercase()
        )
    };
    frame.render_widget(
        Paragraph::new(caption)
            .alignment(Alignment::Center)
            .style(Style::default().fg(if app.demo { p.amber } else { p.muted })),
        at(region, 0, 1),
    );
    let footer = if region.height >= 21 { 2 } else { 1 };
    let info = if readouts {
        format!("UPTIME {}", uptime(app.snapshot.uptime))
    } else {
        format!("{} · {} cores", app.host.cpu_name, app.host.cores)
    };
    frame.render_widget(
        Paragraph::new(info)
            .alignment(Alignment::Center)
            .style(Style::default().fg(p.muted)),
        at(region, region.height.saturating_sub(footer), 1),
    );
    if footer > 1 && !readouts {
        frame.render_widget(
            Paragraph::new(format!("UPTIME {}", uptime(app.snapshot.uptime)))
                .alignment(Alignment::Center)
                .style(Style::default().fg(p.muted)),
            at(region, region.height - 1, 1),
        );
    }
    if chart_height > 0 {
        charts(frame, app, rows[1]);
    }
    Some(Rect::new(
        region.x,
        region.y + 1,
        region.width,
        region.height.saturating_sub(footer + 1),
    ))
}

fn readout(
    frame: &mut Frame,
    area: Rect,
    offset: u16,
    title: &str,
    value: String,
    detail: String,
    color: ratatui::style::Color,
) {
    let lines = vec![
        Line::from(Span::styled(title, Style::default().fg(color))),
        Line::from(value),
        Line::from(Span::styled(detail, Style::default().fg(color))),
    ];
    frame.render_widget(Paragraph::new(lines), at(area, offset, 3));
}
fn left(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let s = &app.snapshot;
    let flags = app.config.panels;
    if flags.cpu {
        readout(
            frame,
            area,
            1,
            "CPU / BREATH",
            percent(s.cpu),
            format!("{} logical cores", app.host.cores),
            p.cpu,
        );
    }
    if flags.memory {
        readout(
            frame,
            area,
            6,
            "MEMORY / FILL",
            format!("{:.1}%", s.memory_percent()),
            format!("{} / {} GiB", gib(s.used_memory), gib(s.total_memory)),
            p.ram,
        );
        readout(
            frame,
            area,
            11,
            "SWAP",
            format!("{} GiB", gib(s.telemetry.swap_used)),
            format!("{} GiB total", gib(s.telemetry.swap_total)),
            p.pink,
        );
    }
}
fn right(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let t = &app.snapshot.telemetry;
    let flags = app.config.panels;
    if flags.network {
        readout(
            frame,
            area,
            1,
            "↓ DOWNLOAD / FLOW",
            rate(t.network_down),
            "non-loopback total".into(),
            p.cyan,
        );
        readout(
            frame,
            area,
            6,
            "↑ UPLOAD",
            rate(t.network_up),
            "measured / second".into(),
            p.pink,
        );
    }
    if flags.power {
        readout(
            frame,
            area,
            11,
            "POWER",
            t.battery.as_ref().map_or_else(
                || "unavailable".into(),
                |b| {
                    format!(
                        "{:.0}% {}",
                        b.percent,
                        if b.charging {
                            "charging"
                        } else if b.on_ac {
                            "AC"
                        } else {
                            "battery"
                        }
                    )
                },
            ),
            String::new(),
            p.ram,
        );
    }
    if flags.storage {
        readout(
            frame,
            area,
            15,
            "STORAGE",
            t.root_disk.as_ref().map_or_else(
                || "unavailable".into(),
                |d| format!("{} GiB free", gib(d.total.saturating_sub(d.used))),
            ),
            "root filesystem".into(),
            p.amber,
        );
    }
}
fn charts(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let flags = app.config.panels;
    let readings: Vec<(&str, &[u64], u64, ratatui::style::Color)> = [
        flags
            .cpu
            .then_some(("CPU", app.history.cpu.as_slice(), 100, p.cpu)),
        flags
            .memory
            .then_some(("MEMORY", app.history.ram.as_slice(), 100, p.ram)),
        flags.network.then_some((
            "NETWORK",
            app.history.down.as_slice(),
            app.history.down.iter().copied().max().unwrap_or(1).max(1),
            p.cyan,
        )),
    ]
    .into_iter()
    .flatten()
    .collect();
    if readings.is_empty() {
        return;
    }
    let columns = Layout::horizontal(vec![
        Constraint::Ratio(1, readings.len() as u32);
        readings.len()
    ])
    .split(area);
    for (index, (label, data, maximum, color)) in readings.iter().enumerate() {
        text(
            frame,
            at(columns[index], 0, 1),
            format!("{label}  /  30s"),
            *color,
        );
        spark(
            frame,
            at(columns[index], 1, area.height.saturating_sub(1)),
            data,
            *maximum,
            *color,
        );
    }
}
fn percent(value: Option<f32>) -> String {
    value.map_or_else(|| "sampling".into(), |value| format!("{value:.1}%"))
}
