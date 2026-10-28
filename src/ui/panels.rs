use super::gib;
use super::widgets::{panel, spark, text};
use crate::app::App;
use crate::config::Palette;
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
        super::details::compact(frame, app, area);
        return;
    }
    let enabled = app.config.panels;
    let groups = [
        enabled.cpu || enabled.memory,
        enabled.network,
        enabled.storage || enabled.power,
    ];
    let count = groups.into_iter().filter(|enabled| *enabled).count();
    if count == 0 {
        text(
            frame,
            area,
            "Panels hidden · c customize",
            app.config.theme.palette().muted,
        );
        return;
    }
    let constraints: Vec<_> = groups
        .into_iter()
        .filter(|enabled| *enabled)
        .map(|_| Constraint::Ratio(1, count as u32))
        .collect();
    let rows = Layout::vertical(constraints).split(area);
    let mut index = 0;
    if groups[0] {
        let panel_count = u32::from(enabled.cpu) + u32::from(enabled.memory);
        let columns = Layout::horizontal(vec![
            Constraint::Ratio(1, panel_count);
            panel_count as usize
        ])
        .split(rows[index]);
        if enabled.cpu {
            cpu(frame, app, columns[0]);
        }
        if enabled.memory {
            memory(frame, app, columns[usize::from(enabled.cpu)]);
        }
        index += 1;
    }
    if enabled.network {
        super::details::network(frame, app, rows[index]);
        index += 1;
    }
    if groups[2] {
        super::details::peripherals(frame, app, rows[index]);
    }
}

fn cpu(frame: &mut Frame, app: &App, area: Rect) {
    let palette = app.config.theme.palette();
    let inner = panel(frame, area, "CPU", palette.cpu, palette);
    let value = app
        .snapshot
        .cpu
        .map_or_else(|| "sampling".into(), |value| format!("{value:.1}%"));
    text(
        frame,
        at(inner, 0, 1),
        format!("{value}  ·  {} CORES", app.host.cores),
        palette.fg,
    );
    spark(frame, at(inner, 1, 2), &app.history.cpu, 100, palette.cpu);
    let load = app.snapshot.telemetry.load_average;
    text(
        frame,
        at(inner, 3, 1),
        format!("LOAD {:.1}  {:.1}  {:.1}", load[0], load[1], load[2]),
        palette.muted,
    );
    cores(
        frame,
        at(inner, 4, inner.height.saturating_sub(4)),
        &app.snapshot.telemetry.per_core,
        palette,
    );
}

pub(super) fn cores(frame: &mut Frame, area: Rect, cores: &[f32], palette: Palette) {
    if cores.is_empty() {
        text(frame, area, "Core counters warming up", palette.muted);
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
                        Style::default().fg(if *value >= 80.0 {
                            palette.amber
                        } else {
                            palette.cpu
                        }),
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
            palette.muted,
        );
    }
}

fn memory(frame: &mut Frame, app: &App, area: Rect) {
    let palette = app.config.theme.palette();
    let inner = panel(frame, area, "MEMORY", palette.ram, palette);
    let s = &app.snapshot;
    text(
        frame,
        at(inner, 0, 1),
        format!("{} / {} GiB", gib(s.used_memory), gib(s.total_memory)),
        palette.fg,
    );
    spark(frame, at(inner, 1, 2), &app.history.ram, 100, palette.ram);
    text(
        frame,
        at(inner, 3, 1),
        format!(
            "{:.1}% used · {} GiB free",
            s.memory_percent(),
            gib(s.total_memory.saturating_sub(s.used_memory))
        ),
        palette.muted,
    );
    text(
        frame,
        at(inner, 4, 1),
        format!(
            "SWAP {} / {} GiB",
            gib(s.telemetry.swap_used),
            gib(s.telemetry.swap_total)
        ),
        palette.pink,
    );
    super::widgets::gauge(
        frame,
        at(inner, 5, 1),
        s.used_memory,
        s.total_memory,
        palette.ram,
        palette.line,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, Panels, ThemeName},
        model::{HostInfo, Os},
    };
    use ratatui::{Terminal, backend::TestBackend};

    fn app(panels: Panels, theme: ThemeName) -> App {
        App {
            config: Config {
                panels,
                theme,
                ..Config::default()
            },
            ..App::new(HostInfo {
                os: Os::Mac,
                os_name: "macOS".into(),
                kernel: "Darwin".into(),
                hostname: "test".into(),
                cpu_name: "Apple".into(),
                cores: 8,
                arch: "arm64".into(),
            })
        }
    }

    fn render(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| stats(frame, app, frame.area()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn contents(buffer: &ratatui::buffer::Buffer) -> String {
        buffer.content.iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn hidden_panels_disappear_and_single_panel_fills_available_area() {
        let disabled = Panels {
            cpu: false,
            memory: false,
            network: false,
            storage: false,
            power: false,
        };
        for (width, height) in [(80, 24), (45, 18), (30, 8)] {
            let empty = render(&app(disabled, ThemeName::Default), width, height);
            assert!(contents(&empty).contains("Panels hidden"));
            let cpu_only = render(
                &app(
                    Panels {
                        cpu: true,
                        ..disabled
                    },
                    ThemeName::Default,
                ),
                width,
                height,
            );
            let source = contents(&cpu_only);
            assert!(source.contains("CPU"));
            assert!(
                !source.contains("MEMORY")
                    && !source.contains("NETWORK")
                    && !source.contains("STORAGE")
                    && !source.contains("POWER")
            );
            if width >= 52 {
                assert_eq!(cpu_only[(width - 1, height - 1)].symbol(), "╯");
            }
        }
    }

    #[test]
    fn dashboard_surface_and_border_follow_selected_theme() {
        let palette = ThemeName::Gruvbox.palette();
        let buffer = render(&app(Panels::default(), ThemeName::Gruvbox), 80, 24);
        assert_eq!(buffer[(0, 0)].fg, palette.line);
        assert_eq!(buffer[(0, 0)].bg, palette.panel);
        assert!(buffer.content.iter().any(|cell| cell.fg == palette.cpu));
        assert!(buffer.content.iter().any(|cell| cell.fg == palette.ram));
    }

    #[test]
    fn every_panel_combination_survives_resize_without_hidden_labels() {
        for mask in 0..32 {
            let enabled = Panels {
                cpu: mask & 1 != 0,
                memory: mask & 2 != 0,
                network: mask & 4 != 0,
                storage: mask & 8 != 0,
                power: mask & 16 != 0,
            };
            for (width, height) in [(80, 24), (45, 18), (30, 8), (1, 1)] {
                let buffer = render(&app(enabled, ThemeName::Phosphor), width, height);
                let source = contents(&buffer);
                for (shown, label) in [
                    (enabled.cpu, "CPU"),
                    (enabled.memory, "MEMORY"),
                    (enabled.network, "NET"),
                    (enabled.storage, "STORAGE"),
                    (enabled.power, "POWER"),
                ] {
                    if !shown {
                        assert!(!source.contains(label), "{label} visible for mask {mask}");
                    }
                }
            }
        }
    }
}
