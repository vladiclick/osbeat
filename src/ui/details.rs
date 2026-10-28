use super::panels::{at, cores};
use super::widgets::{gauge, panel, spark, text};
use super::{gib, rate, uptime};
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) fn network(frame: &mut Frame, app: &App, area: Rect) {
    let palette = app.config.theme.palette();
    let inner = panel(frame, area, "NETWORK", palette.cyan, palette);
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]);
    let telemetry = &app.snapshot.telemetry;
    text(
        frame,
        at(columns[0], 0, 1),
        format!("↓ {}", rate(telemetry.network_down)),
        palette.cyan,
    );
    text(
        frame,
        at(columns[1], 0, 1),
        format!("↑ {}", rate(telemetry.network_up)),
        palette.pink,
    );
    // Share the scale so upload and download heights can be compared directly.
    let width = (inner.width / 2) as usize;
    let maximum = app
        .history
        .down
        .iter()
        .rev()
        .take(width)
        .chain(app.history.up.iter().rev().take(width))
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    spark(
        frame,
        at(columns[0], 1, columns[0].height.saturating_sub(1)),
        &app.history.down,
        maximum,
        palette.cyan,
    );
    spark(
        frame,
        at(columns[1], 1, columns[1].height.saturating_sub(1)),
        &app.history.up,
        maximum,
        palette.pink,
    );
    let interfaces = if telemetry.network_interfaces.is_empty() {
        "No active interface".into()
    } else {
        let shown = telemetry
            .network_interfaces
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        let remaining = telemetry.network_interfaces.len().saturating_sub(3);
        if remaining > 0 {
            format!("IF  {shown}  +{remaining}")
        } else {
            format!("IF  {shown}")
        }
    };
    text(frame, rows[1], interfaces, palette.muted);
}

pub(super) fn peripherals(frame: &mut Frame, app: &App, area: Rect) {
    let enabled = app.config.panels;
    let palette = app.config.theme.palette();
    let count = u32::from(enabled.storage) + u32::from(enabled.power);
    if count == 0 {
        return;
    }
    let columns = Layout::horizontal(vec![Constraint::Ratio(1, count); count as usize]).split(area);
    if enabled.storage {
        let disk = panel(frame, columns[0], "STORAGE", palette.amber, palette);
        match &app.snapshot.telemetry.root_disk {
            Some(value) => {
                text(
                    frame,
                    at(disk, 0, 1),
                    format!("{} / {} GiB", gib(value.used), gib(value.total)),
                    palette.fg,
                );
                text(
                    frame,
                    at(disk, 1, 1),
                    format!(
                        "{} GiB available",
                        gib(value.total.saturating_sub(value.used))
                    ),
                    palette.muted,
                );
                gauge(
                    frame,
                    at(disk, 2, 1),
                    value.used,
                    value.total,
                    palette.amber,
                    palette.line,
                );
                text(frame, at(disk, 3, 1), &value.label, palette.muted);
            }
            None => text(frame, disk, "Not available", palette.muted),
        }
    }
    if enabled.power {
        let power = panel(
            frame,
            columns[usize::from(enabled.storage)],
            "POWER",
            palette.ram,
            palette,
        );
        match &app.snapshot.telemetry.battery {
            Some(value) => {
                text(
                    frame,
                    at(power, 0, 1),
                    format!(
                        "{:.0}%  {}",
                        value.percent,
                        if value.on_ac { "AC" } else { "BAT" }
                    ),
                    palette.fg,
                );
                text(frame, at(power, 1, 1), power_state(value), palette.muted);
                gauge(
                    frame,
                    at(power, 2, 1),
                    value.percent as u64,
                    100,
                    palette.ram,
                    palette.line,
                );
                text(
                    frame,
                    at(power, 3, 1),
                    if value.on_ac {
                        "External power"
                    } else {
                        "Battery power"
                    },
                    palette.muted,
                );
            }
            None => text(frame, power, "Not available", palette.muted),
        }
    }
}

pub(super) fn storage_label(app: &App) -> String {
    app.snapshot.telemetry.root_disk.as_ref().map_or_else(
        || "STORAGE unavailable".into(),
        |disk| format!("STORAGE {} / {} GiB", gib(disk.used), gib(disk.total)),
    )
}

pub(super) fn battery_label(app: &App) -> String {
    app.snapshot.telemetry.battery.as_ref().map_or_else(
        || "POWER unavailable".into(),
        |battery| format!("POWER {:.0}% · {}", battery.percent, power_state(battery)),
    )
}

pub(super) fn compact_peripherals(frame: &mut Frame, app: &App, area: Rect) {
    let palette = app.config.theme.palette();
    if app.config.panels.storage {
        text(frame, at(area, 0, 1), storage_label(app), palette.amber);
    }
    if app.config.panels.power {
        text(
            frame,
            at(area, u16::from(app.config.panels.storage), 1),
            battery_label(app),
            palette.ram,
        );
    }
}

fn power_state(battery: &crate::telemetry::BatteryStatus) -> &'static str {
    if battery.charging {
        "Charging"
    } else if battery.on_ac {
        "Plugged in"
    } else {
        "Discharging"
    }
}

pub(super) fn compact(frame: &mut Frame, app: &App, area: Rect) {
    let enabled = app.config.panels;
    if ![
        enabled.cpu,
        enabled.memory,
        enabled.network,
        enabled.storage,
        enabled.power,
    ]
    .into_iter()
    .any(|shown| shown)
    {
        text(
            frame,
            area,
            "Panels hidden · c customize",
            app.config.theme.palette().muted,
        );
        return;
    }
    if area.height < 12 {
        compact_numbers(frame, app, area);
        return;
    }
    type Renderer = fn(&mut Frame, &App, Rect);
    let groups: Vec<_> = [
        (enabled.cpu, 4, compact_cpu as Renderer),
        (enabled.memory, 4, compact_memory as Renderer),
        (enabled.network, 4, compact_network as Renderer),
        (
            enabled.storage || enabled.power,
            u32::from(enabled.storage) + u32::from(enabled.power),
            compact_peripherals as Renderer,
        ),
    ]
    .into_iter()
    .filter(|(show, _, _)| *show)
    .collect();
    let weight = groups.iter().map(|(_, weight, _)| weight).sum();
    let constraints = groups
        .iter()
        .map(|(_, size, _)| Constraint::Ratio(*size, weight))
        .collect::<Vec<_>>();
    let rows = Layout::vertical(constraints).split(area);
    for ((_, _, renderer), row) in groups.iter().zip(rows.iter()) {
        renderer(frame, app, *row);
    }
}

fn compact_numbers(frame: &mut Frame, app: &App, area: Rect) {
    let s = &app.snapshot;
    let enabled = app.config.panels;
    let palette = app.config.theme.palette();
    let cpu = s
        .cpu
        .map_or_else(|| "sampling".into(), |value| format!("{value:.1}%"));
    let lines = [
        (enabled.cpu, format!("CPU     {cpu}"), palette.cpu),
        (
            enabled.memory,
            format!(
                "MEMORY  {} / {} GiB · SWAP {} GiB",
                gib(s.used_memory),
                gib(s.total_memory),
                gib(s.telemetry.swap_used)
            ),
            palette.ram,
        ),
        (
            enabled.network,
            format!(
                "NET ↓{} ↑{}",
                rate(s.telemetry.network_down),
                rate(s.telemetry.network_up)
            ),
            palette.cyan,
        ),
        (enabled.storage, storage_label(app), palette.amber),
        (enabled.power, battery_label(app), palette.ram),
        (true, format!("UPTIME  {}", uptime(s.uptime)), palette.muted),
    ]
    .into_iter()
    .filter(|(shown, _, _)| *shown)
    .map(|(_, label, color)| Line::from(Span::styled(label, Style::default().fg(color))))
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn compact_cpu(frame: &mut Frame, app: &App, area: Rect) {
    let s = &app.snapshot;
    let palette = app.config.theme.palette();
    let cpu = s
        .cpu
        .map_or_else(|| "sampling".into(), |cpu| format!("{cpu:.1}%"));
    text(
        frame,
        at(area, 0, 1),
        format!("CPU  {cpu} · {} CORES", app.host.cores),
        palette.cpu,
    );
    spark(
        frame,
        at(area, 1, area.height.saturating_sub(2)),
        &app.history.cpu,
        100,
        palette.cpu,
    );
    cores(
        frame,
        at(area, area.height.saturating_sub(1), 1),
        &s.telemetry.per_core,
        palette,
    );
}

fn compact_memory(frame: &mut Frame, app: &App, area: Rect) {
    let s = &app.snapshot;
    let palette = app.config.theme.palette();
    text(
        frame,
        at(area, 0, 1),
        format!(
            "MEMORY {} / {} GiB",
            gib(s.used_memory),
            gib(s.total_memory)
        ),
        palette.ram,
    );
    spark(
        frame,
        at(area, 1, area.height.saturating_sub(2)),
        &app.history.ram,
        100,
        palette.ram,
    );
    text(
        frame,
        at(area, area.height.saturating_sub(1), 1),
        format!(
            "SWAP {} / {} GiB",
            gib(s.telemetry.swap_used),
            gib(s.telemetry.swap_total)
        ),
        palette.pink,
    );
}

fn compact_network(frame: &mut Frame, app: &App, area: Rect) {
    let s = &app.snapshot;
    let palette = app.config.theme.palette();
    text(
        frame,
        at(area, 0, 1),
        format!(
            "NET ↓{} ↑{}",
            rate(s.telemetry.network_down),
            rate(s.telemetry.network_up)
        ),
        palette.cyan,
    );
    let maximum = app.history.down.iter().max().copied().unwrap_or(1).max(1);
    spark(
        frame,
        at(area, 1, area.height.saturating_sub(1)),
        &app.history.down,
        maximum,
        palette.cyan,
    );
}
