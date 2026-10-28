use super::{
    gib,
    panels::at,
    rate, uptime,
    widgets::{gauge, spark, text},
};
use crate::{app::App, logo::Logo};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
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
    let region = if readouts && space.width >= 88 && space.height >= 16 {
        let columns = Layout::horizontal([
            Constraint::Length(22),
            Constraint::Length(2),
            Constraint::Min(28),
            Constraint::Length(2),
            Constraint::Length(22),
        ])
        .split(space);
        rails(frame, app, columns[0], columns[4]);
        columns[2]
    } else if readouts {
        let summary_height = if space.height >= 8 { 3 } else { 2 };
        compact(frame, app, at(space, 0, summary_height));
        Rect::new(
            space.x,
            space.y + summary_height,
            space.width,
            space.height.saturating_sub(summary_height),
        )
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

struct Reading<'a> {
    title: &'a str,
    value: String,
    detail: String,
    color: Color,
}

fn readout(frame: &mut Frame, app: &App, area: Rect, reading: Reading<'_>) {
    let p = app.config.theme.palette();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                reading.title,
                Style::default().fg(reading.color),
            )),
            Line::from(Span::styled(
                reading.value,
                Style::default().fg(p.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(reading.detail, Style::default().fg(p.muted))),
        ]),
        at(area, 0, 3),
    );
}

fn rails(frame: &mut Frame, app: &App, left: Rect, right: Rect) {
    // Both rails share a single vertical rhythm, independent of enabled panels.
    let top = left.height.saturating_sub(16) / 2;
    let left_rows = [top, top + 6, top + 12].map(|y| at(left, y, 4));
    let right_rows = [top, top + 6, top + 12].map(|y| at(right, y, 4));
    left_readings(frame, app, left_rows);
    right_readings(frame, app, right_rows);
}

fn left_readings(frame: &mut Frame, app: &App, rows: [Rect; 3]) {
    let p = app.config.theme.palette();
    let s = &app.snapshot;
    let t = &s.telemetry;
    if app.config.panels.cpu {
        readout(
            frame,
            app,
            rows[0],
            Reading {
                title: "CPU",
                value: percent(s.cpu),
                detail: format!("{} cores · load {:.1}", app.host.cores, t.load_average[0]),
                color: p.cpu,
            },
        );
        gauge(
            frame,
            at(rows[0], 3, 1),
            s.cpu.unwrap_or(0.0) as u64,
            100,
            p.cpu,
            p.line,
        );
    }
    if app.config.panels.memory {
        readout(
            frame,
            app,
            rows[1],
            Reading {
                title: "MEMORY",
                value: format!("{:.1}%", s.memory_percent()),
                detail: format!("{} / {} GiB", gib(s.used_memory), gib(s.total_memory)),
                color: p.ram,
            },
        );
        gauge(
            frame,
            at(rows[1], 3, 1),
            s.used_memory,
            s.total_memory,
            p.ram,
            p.line,
        );
        readout(
            frame,
            app,
            rows[2],
            Reading {
                title: "SWAP",
                value: format!("{} GiB used", gib(t.swap_used)),
                detail: format!("{} GiB allocated", gib(t.swap_total)),
                color: p.pink,
            },
        );
        gauge(
            frame,
            at(rows[2], 3, 1),
            t.swap_used,
            t.swap_total,
            p.pink,
            p.line,
        );
    }
}

fn right_readings(frame: &mut Frame, app: &App, rows: [Rect; 3]) {
    let p = app.config.theme.palette();
    let t = &app.snapshot.telemetry;
    if app.config.panels.network {
        readout(
            frame,
            app,
            rows[0],
            Reading {
                title: "NETWORK",
                value: format!("↓ {}", rate(t.network_down)),
                detail: format!("↑ {}", rate(t.network_up)),
                color: p.cyan,
            },
        );
        text(frame, at(rows[0], 3, 1), interfaces(app), p.muted);
    }
    if app.config.panels.power {
        let (value, detail) = t.battery.as_ref().map_or_else(
            || ("Unavailable".into(), "No battery reading".into()),
            |b| {
                (
                    format!(
                        "{:.0}% · {}",
                        b.percent,
                        if b.charging {
                            "Charging"
                        } else if b.on_ac {
                            "AC"
                        } else {
                            "Battery"
                        }
                    ),
                    if b.on_ac {
                        "External power"
                    } else {
                        "Battery power"
                    }
                    .into(),
                )
            },
        );
        readout(
            frame,
            app,
            rows[1],
            Reading {
                title: "POWER",
                value,
                detail,
                color: p.ram,
            },
        );
        if let Some(battery) = &t.battery {
            gauge(
                frame,
                at(rows[1], 3, 1),
                battery.percent as u64,
                100,
                p.ram,
                p.line,
            );
        }
    }
    if app.config.panels.storage {
        let (value, detail) = t.root_disk.as_ref().map_or_else(
            || ("Unavailable".into(), "No filesystem reading".into()),
            |disk| {
                (
                    format!("{} GiB free", gib(disk.total.saturating_sub(disk.used))),
                    format!("{} / {} GiB", gib(disk.used), gib(disk.total)),
                )
            },
        );
        readout(
            frame,
            app,
            rows[2],
            Reading {
                title: "STORAGE",
                value,
                detail,
                color: p.amber,
            },
        );
        if let Some(disk) = &t.root_disk {
            gauge(
                frame,
                at(rows[2], 3, 1),
                disk.used,
                disk.total,
                p.amber,
                p.line,
            );
        }
    }
}

fn compact(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let readings: Vec<Reading<'_>> = [
        app.config.panels.cpu.then(|| Reading {
            title: "CPU",
            value: percent(app.snapshot.cpu),
            detail: format!("{} cores", app.host.cores),
            color: p.cpu,
        }),
        app.config.panels.memory.then(|| Reading {
            title: "MEMORY",
            value: format!("{:.1}%", app.snapshot.memory_percent()),
            detail: format!(
                "{} / {} GiB",
                gib(app.snapshot.used_memory),
                gib(app.snapshot.total_memory)
            ),
            color: p.ram,
        }),
        app.config.panels.network.then(|| Reading {
            title: "NETWORK",
            value: format!("↓ {}", rate(app.snapshot.telemetry.network_down)),
            detail: format!("↑ {}", rate(app.snapshot.telemetry.network_up)),
            color: p.cyan,
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    let columns = separated(area, readings.len());
    for (reading, column) in readings.into_iter().zip(columns) {
        readout(frame, app, column, reading);
    }
}

fn interfaces(app: &App) -> String {
    let interfaces = &app.snapshot.telemetry.network_interfaces;
    match interfaces.as_slice() {
        [] => "No active interface".into(),
        [interface] => interface.clone(),
        [first, rest @ ..] => format!("{first} +{} interfaces", rest.len()),
    }
}

fn charts(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let flags = app.config.panels;
    let readings: Vec<(&str, String, &[u64], u64, Color)> = [
        flags.cpu.then(|| {
            (
                "CPU",
                percent(app.snapshot.cpu),
                app.history.cpu.as_slice(),
                100,
                p.cpu,
            )
        }),
        flags.memory.then(|| {
            (
                "MEMORY",
                format!("{:.1}%", app.snapshot.memory_percent()),
                app.history.ram.as_slice(),
                100,
                p.ram,
            )
        }),
        flags.network.then(|| {
            (
                "NETWORK",
                format!("↓ {}", rate(app.snapshot.telemetry.network_down)),
                app.history.down.as_slice(),
                app.history.down.iter().copied().max().unwrap_or(1).max(1),
                p.cyan,
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    let columns = separated(area, readings.len());
    for ((label, value, data, maximum, color), column) in readings.iter().zip(columns) {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{label}  "), Style::default().fg(*color)),
                Span::styled(
                    value,
                    Style::default().fg(p.fg).add_modifier(Modifier::BOLD),
                ),
            ])),
            at(column, 0, 1),
        );
        spark(
            frame,
            at(column, 1, area.height.saturating_sub(1)),
            data,
            *maximum,
            *color,
        );
    }
}

fn separated(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let constraints: Vec<Constraint> = (0..count)
        .flat_map(|index| {
            let gap = (index + 1 < count).then_some(Constraint::Length(2));
            std::iter::once(Constraint::Fill(1)).chain(gap)
        })
        .collect();
    Layout::horizontal(constraints)
        .split(area)
        .iter()
        .step_by(2)
        .copied()
        .collect()
}

fn percent(value: Option<f32>) -> String {
    value.map_or_else(|| "Sampling".into(), |value| format!("{value:.1}%"))
}
