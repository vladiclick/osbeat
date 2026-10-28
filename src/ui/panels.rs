use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, LineGauge, Paragraph, Sparkline},
};

use super::{BG, CPU, FG, LINE, MUTED, RAM, gib, uptime};
use crate::app::App;

pub(super) fn stats(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 12 {
        compact(frame, app, area);
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Min(0),
    ])
    .split(area);
    let cpu_value = app
        .snapshot
        .cpu
        .map_or_else(|| "warming up".into(), |value| format!("{value:.1}%"));
    metric(
        frame,
        rows[0],
        "CPU",
        &cpu_value,
        &app.history.cpu,
        app.snapshot.cpu.unwrap_or(0.0),
        CPU,
    );
    let memory_value = format!(
        "{} / {} GiB",
        gib(app.snapshot.used_memory),
        gib(app.snapshot.total_memory)
    );
    metric(
        frame,
        rows[1],
        "MEMORY",
        &memory_value,
        &app.history.ram,
        app.snapshot.memory_percent(),
        RAM,
    );
    system(frame, app, rows[2]);
}

fn metric(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    history: &[u64],
    percent: f32,
    color: Color,
) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    let cols = Layout::horizontal([Constraint::Length(8), Constraint::Min(0)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(MUTED)),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(value)
            .alignment(Alignment::Right)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        cols[1],
    );
    // Pad on the left so the newest point always sits at the right edge.
    let width = rows[2].width as usize;
    let data: Vec<u64> = std::iter::repeat_n(0, width.saturating_sub(history.len()))
        .chain(
            history
                .iter()
                .skip(history.len().saturating_sub(width))
                .copied(),
        )
        .collect();
    frame.render_widget(
        Sparkline::default()
            .data(&data)
            .max(100)
            .style(Style::default().fg(color)),
        rows[2],
    );
    frame.render_widget(
        LineGauge::default()
            .ratio(f64::from(percent.clamp(0.0, 100.0)) / 100.0)
            .label("")
            .filled_symbol("━")
            .unfilled_symbol("━")
            .filled_style(Style::default().fg(color))
            .unfilled_style(Style::default().fg(LINE)),
        rows[4],
    );
}

fn system(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = format!("{} · {} cores", app.host.cpu_name, app.host.cores);
    let entries = [
        ("UPTIME", uptime(app.snapshot.uptime)),
        ("SYSTEM", app.host.os_name.clone()),
        ("CHIP", cpu),
        ("KERNEL", app.host.kernel.clone()),
        ("ARCH", app.host.arch.clone()),
    ];
    let lines: Vec<Line> = entries
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label:<8}"), Style::default().fg(MUTED)),
                Span::styled(value.as_str(), Style::default().fg(FG)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn compact(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = app
        .snapshot
        .cpu
        .map_or_else(|| "warming up".into(), |cpu| format!("{cpu:.1}%"));
    let lines = vec![
        Line::from(vec![
            Span::styled("CPU     ", Style::default().fg(CPU)),
            Span::raw(cpu),
        ]),
        Line::from(vec![
            Span::styled("MEMORY  ", Style::default().fg(RAM)),
            Span::raw(format!(
                "{} / {} GiB",
                gib(app.snapshot.used_memory),
                gib(app.snapshot.total_memory)
            )),
        ]),
        Line::from(format!("UPTIME  {}", uptime(app.snapshot.uptime))),
        Line::from(app.host.os_name.as_str()),
        Line::from(app.host.cpu_name.as_str()),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

pub(super) fn diagnostics(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = app
        .snapshot
        .own_cpu
        .map_or_else(|| "warming up".into(), |cpu| format!("{cpu:.2}%"));
    let color = if app.snapshot.own_cpu.is_some_and(|cpu| cpu >= 2.0) {
        Color::Rgb(239, 159, 118)
    } else {
        RAM
    };
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(LINE))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("SELF CPU {cpu}"), Style::default().fg(color)),
                Span::raw(format!(
                    "    RSS {:.1} MiB",
                    app.snapshot.own_memory as f64 / (1_u64 << 20) as f64
                )),
            ]),
            Line::from(Span::styled(
                "Budget <2% of one core · d to close",
                Style::default().fg(MUTED),
            )),
        ])
        .style(Style::default().fg(FG)),
        inner,
    );
}
