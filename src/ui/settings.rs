use crate::{app::App, config::LayoutMode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph},
};

pub(super) fn customize(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" CUSTOMIZE / LIVE PREVIEW ")
        .border_style(Style::default().fg(p.line))
        .style(Style::default().bg(p.panel).fg(p.fg));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(inner);
    let c = &app.config;
    let on = |value| if value { "on" } else { "off" }.to_string();
    let entries = [
        ("Theme", c.theme.label().into()),
        ("Logo effect", c.effect.clone()),
        ("Glyphs", c.glyphs.label().into()),
        ("Motion speed", format!("{:.2}×", c.speed)),
        ("Intensity", format!("{:.0}%", c.intensity * 100.0)),
        ("Frame rate", format!("{} fps", c.fps)),
        ("Motion", on(!c.reduced_motion)),
        (
            "View",
            if c.layout == LayoutMode::Focus {
                "canvas"
            } else {
                "dashboard"
            }
            .into(),
        ),
        ("CPU panel", on(c.panels.cpu)),
        ("Memory panel", on(c.panels.memory)),
        ("Network panel", on(c.panels.network)),
        ("Storage panel", on(c.panels.storage)),
        ("Power panel", on(c.panels.power)),
        ("Demo effects", on(app.demo)),
    ];
    let items: Vec<ListItem> = entries
        .into_iter()
        .map(|(label, value)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{label:<17}"), Style::default().fg(p.muted)),
                Span::styled(value, Style::default().fg(p.fg)),
            ]))
        })
        .collect();
    let list = List::new(items).highlight_symbol("› ").highlight_style(
        Style::default()
            .bg(p.line)
            .fg(p.cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(
        list,
        rows[0],
        &mut ListState::default().with_selected(Some(app.selected)),
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("↑↓ select   ←→ change"),
            Line::from("s save TOML   Esc close"),
            Line::from("Changes preview immediately"),
        ])
        .style(Style::default().fg(p.muted)),
        rows[1],
    );
}

pub(super) fn help(frame: &mut Frame, app: &App, area: Rect) {
    let p = app.config.theme.palette();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" OSBEAT / CONTROLS ")
        .border_style(Style::default().fg(p.cyan))
        .style(Style::default().bg(p.panel).fg(p.fg));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = [
        "The logo IS the monitor.",
        "CPU → breathing speed · RAM → liquid fill · Network → shimmer",
        "",
        "Tab    Living canvas / detailed dashboard",
        "c      Open customization with live preview",
        "t / e  Cycle theme / logo effect",
        "g      Solid / ASCII / dot-matrix glyphs",
        "a      Pause or resume motion",
        "m      Demo logo reactions (stats stay real)",
        "s      Save current preferences to TOML",
        "d      Show osbeat's own CPU and RAM",
        "?      Toggle this help",
        "Esc    Close a panel",
        "q      Quit and restore the terminal",
        "",
        "Demo is always labeled; no temperatures are invented.",
    ];
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>()),
        inner,
    );
}
