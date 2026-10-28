use super::panels::at;
use super::widgets::{gauge, panel, spark, text};
use super::{AMBER, CYAN, FG, MUTED, PINK, RAM, gib, rate};
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

pub(super) fn network(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "NETWORK", CYAN);
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]);
    let telemetry = &app.snapshot.telemetry;
    text(
        frame,
        at(columns[0], 0, 1),
        format!("↓ {}", rate(telemetry.network_down)),
        CYAN,
    );
    text(
        frame,
        at(columns[1], 0, 1),
        format!("↑ {}", rate(telemetry.network_up)),
        PINK,
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
        CYAN,
    );
    spark(
        frame,
        at(columns[1], 1, columns[1].height.saturating_sub(1)),
        &app.history.up,
        maximum,
        PINK,
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
    text(frame, rows[1], interfaces, MUTED);
}

pub(super) fn peripherals(frame: &mut Frame, app: &App, area: Rect) {
    let columns =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).split(area);
    let disk = panel(frame, columns[0], "STORAGE", AMBER);
    let power = panel(frame, columns[1], "POWER", RAM);
    match &app.snapshot.telemetry.root_disk {
        Some(value) => {
            text(
                frame,
                at(disk, 0, 1),
                format!("{} / {} GiB", gib(value.used), gib(value.total)),
                FG,
            );
            text(
                frame,
                at(disk, 1, 1),
                format!(
                    "{} GiB available",
                    gib(value.total.saturating_sub(value.used))
                ),
                MUTED,
            );
            gauge(frame, at(disk, 2, 1), value.used, value.total, AMBER);
            text(frame, at(disk, 3, 1), &value.label, MUTED);
        }
        None => text(frame, disk, "Not available", MUTED),
    }
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
                FG,
            );
            text(frame, at(power, 1, 1), power_state(value), MUTED);
            gauge(frame, at(power, 2, 1), value.percent as u64, 100, RAM);
            text(
                frame,
                at(power, 3, 1),
                if value.on_ac {
                    "External power"
                } else {
                    "Battery power"
                },
                MUTED,
            );
        }
        None => text(frame, power, "Not available", MUTED),
    }
}

pub(super) fn compact_peripherals(frame: &mut Frame, app: &App, area: Rect) {
    let telemetry = &app.snapshot.telemetry;
    let disk = telemetry.root_disk.as_ref().map_or_else(
        || "STORAGE unavailable".into(),
        |disk| format!("STORAGE {} / {} GiB", gib(disk.used), gib(disk.total)),
    );
    text(frame, at(area, 0, 1), disk, AMBER);
    let power = telemetry.battery.as_ref().map_or_else(
        || "POWER unavailable".into(),
        |battery| format!("POWER {:.0}% · {}", battery.percent, power_state(battery)),
    );
    text(frame, at(area, 1, 1), power, RAM);
    let load = telemetry.load_average;
    text(
        frame,
        at(area, 2, 1),
        format!("LOAD  {:.1}  {:.1}  {:.1}", load[0], load[1], load[2]),
        MUTED,
    );
    text(
        frame,
        at(area, 3, 1),
        format!("IF    {}", telemetry.network_interfaces.join(" · ")),
        MUTED,
    );
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
