use super::widgets::{panel, spark, text};
use super::{CPU, CYAN, FG, MUTED, PINK, RAM, gib, rate, uptime};
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) fn at(area: Rect, offset: u16, height: u16) -> Rect {
    Rect::new(
        area.x,
        area.y.saturating_add(offset).min(area.bottom()),
        area.width,
        height.min(area.height.saturating_sub(offset)),
    )
}

pub(super) fn stats(frame: &mut Frame, app: &App, area: Rect) {
    if area.width < 52 || area.height < 19 {
        compact(frame, app, area);
        return;
    }
    let top = (area.height / 3).clamp(8, 10);
    let rows = Layout::vertical([
        Constraint::Length(top),
        Constraint::Min(5),
        Constraint::Length(6),
    ])
    .split(area);
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]);
    cpu(frame, app, columns[0]);
    memory(frame, app, columns[1]);
    super::details::network(frame, app, rows[1]);
    super::details::peripherals(frame, app, rows[2]);
}

fn cpu(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "CPU", CPU);
    let value = app
        .snapshot
        .cpu
        .map_or_else(|| "sampling".into(), |value| format!("{value:.1}%"));
    text(
        frame,
        at(inner, 0, 1),
        format!("{value}  ·  {} CORES", app.host.cores),
        FG,
    );
    spark(frame, at(inner, 1, 2), &app.history.cpu, 100, CPU);
    let load = app.snapshot.telemetry.load_average;
    text(
        frame,
        at(inner, 3, 1),
        format!("LOAD {:.1}  {:.1}  {:.1}", load[0], load[1], load[2]),
        MUTED,
    );
    cores(
        frame,
        at(inner, 4, inner.height.saturating_sub(4)),
        &app.snapshot.telemetry.per_core,
    );
}

fn cores(frame: &mut Frame, area: Rect, cores: &[f32]) {
    if cores.is_empty() {
        text(frame, area, "Core counters warming up", MUTED);
        return;
    }
    let columns = (area.width / 5).max(1) as usize;
    let capacity = columns * area.height as usize;
    let lines: Vec<Line> = cores
        .chunks(columns)
        .take(area.height as usize)
        .enumerate()
        .map(|(row, values)| {
            let spans: Vec<Span> = values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let position = (value.clamp(0.0, 100.0) / 100.0 * 7.0).round() as usize;
                    let bar = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][position.min(7)];
                    let id = row * columns + index + 1;
                    Span::styled(
                        format!("C{id:02}{bar} "),
                        Style::default().fg(if *value >= 80.0 { super::AMBER } else { CPU }),
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
    if cores.len() > capacity && area.height > 0 && area.width >= 5 {
        text(
            frame,
            Rect::new(area.right() - 5, area.bottom() - 1, 5, 1),
            format!(
                "+{:02}  ",
                cores.len().saturating_sub(capacity).saturating_add(1)
            ),
            MUTED,
        );
    }
}

fn memory(frame: &mut Frame, app: &App, area: Rect) {
    let inner = panel(frame, area, "MEMORY", RAM);
    let s = &app.snapshot;
    text(
        frame,
        at(inner, 0, 1),
        format!("{} / {} GiB", gib(s.used_memory), gib(s.total_memory)),
        FG,
    );
    spark(frame, at(inner, 1, 2), &app.history.ram, 100, RAM);
    text(
        frame,
        at(inner, 3, 1),
        format!(
            "{:.1}% used · {} GiB free",
            s.memory_percent(),
            gib(s.total_memory.saturating_sub(s.used_memory))
        ),
        MUTED,
    );
    text(
        frame,
        at(inner, 4, 1),
        format!(
            "SWAP {} / {} GiB",
            gib(s.telemetry.swap_used),
            gib(s.telemetry.swap_total)
        ),
        PINK,
    );
    super::widgets::gauge(frame, at(inner, 5, 1), s.used_memory, s.total_memory, RAM);
}

fn compact(frame: &mut Frame, app: &App, area: Rect) {
    let s = &app.snapshot;
    if area.height < 12 {
        let lines = vec![
            Line::from(format!(
                "CPU     {}",
                s.cpu
                    .map_or_else(|| "sampling".into(), |cpu| format!("{cpu:.1}%"))
            )),
            Line::from(format!(
                "MEMORY  {} / {} GiB",
                gib(s.used_memory),
                gib(s.total_memory)
            )),
            Line::from(format!(
                "NET ↓{} ↑{}",
                rate(s.telemetry.network_down),
                rate(s.telemetry.network_up)
            )),
            Line::from(format!("SWAP    {} GiB", gib(s.telemetry.swap_used))),
            Line::from(format!("UPTIME  {}", uptime(s.uptime))),
            Line::from(app.host.os_name.as_str()),
        ];
        frame.render_widget(Paragraph::new(lines).style(Style::default().fg(FG)), area);
        return;
    }
    let cpu = s
        .cpu
        .map_or_else(|| "sampling".into(), |cpu| format!("{cpu:.1}%"));
    text(
        frame,
        at(area, 0, 1),
        format!("CPU  {cpu} · {} CORES", app.host.cores),
        CPU,
    );
    spark(frame, at(area, 1, 2), &app.history.cpu, 100, CPU);
    cores(frame, at(area, 3, 1), &s.telemetry.per_core);
    text(
        frame,
        at(area, 4, 1),
        format!(
            "MEMORY {} / {} GiB",
            gib(s.used_memory),
            gib(s.total_memory)
        ),
        RAM,
    );
    spark(frame, at(area, 5, 2), &app.history.ram, 100, RAM);
    text(
        frame,
        at(area, 7, 1),
        format!(
            "SWAP {} / {} GiB",
            gib(s.telemetry.swap_used),
            gib(s.telemetry.swap_total)
        ),
        PINK,
    );
    text(
        frame,
        at(area, 8, 1),
        format!("NETWORK ↓{}", rate(s.telemetry.network_down)),
        CYAN,
    );
    let maximum = app.history.down.iter().max().copied().unwrap_or(1).max(1);
    spark(frame, at(area, 9, 2), &app.history.down, maximum, CYAN);
    text(
        frame,
        at(area, 11, 1),
        format!("UPLOAD  ↑{}", rate(s.telemetry.network_up)),
        PINK,
    );
    super::details::compact_peripherals(frame, app, at(area, 12, area.height.saturating_sub(12)));
}
