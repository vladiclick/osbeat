use super::{BG, FG, LINE, MUTED, PANEL, RAM};
use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, LineGauge, Paragraph, Sparkline},
};

pub(super) fn panel(frame: &mut Frame, area: Rect, title: &str, color: Color) -> Rect {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(LINE))
        .style(Style::default().bg(PANEL));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

pub(super) fn text(frame: &mut Frame, area: Rect, value: impl Into<String>, color: Color) {
    frame.render_widget(
        Paragraph::new(value.into()).style(Style::default().fg(color)),
        area,
    );
}

pub(super) fn spark(frame: &mut Frame, area: Rect, history: &[u64], maximum: u64, color: Color) {
    let width = area.width as usize;
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
            .max(maximum.max(1))
            .style(Style::default().fg(color)),
        area,
    );
}

pub(super) fn gauge(frame: &mut Frame, area: Rect, used: u64, total: u64, color: Color) {
    let ratio = if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64).clamp(0.0, 1.0)
    };
    frame.render_widget(
        LineGauge::default()
            .ratio(ratio)
            .label("")
            .filled_symbol("━")
            .unfilled_symbol("━")
            .filled_style(Style::default().fg(color))
            .unfilled_style(Style::default().fg(LINE)),
        area,
    );
}

pub(super) fn diagnostics(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = app
        .snapshot
        .own_cpu
        .map_or_else(|| "sampling".into(), |cpu| format!("{cpu:.2}%"));
    let color = if app.snapshot.own_cpu.is_some_and(|cpu| cpu >= 2.0) {
        super::AMBER
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
