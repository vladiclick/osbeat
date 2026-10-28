use osbeat::{
    animation::Engine,
    app::App,
    config::{Config, GlyphMode},
    logo::Logo,
    model::{HostInfo, Os},
    ui::live::Scene,
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::time::Duration;

fn app(mode: GlyphMode) -> App {
    App::new(HostInfo {
        os: Os::Mac,
        os_name: "macOS".into(),
        kernel: "26".into(),
        hostname: "preview".into(),
        cpu_name: "Apple Silicon".into(),
        cores: 10,
        arch: "arm64".into(),
    })
    .with_config(Config {
        glyphs: mode,
        ..Default::default()
    })
}

fn render(app: &App, logo: &Logo, engine: &Engine) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
    terminal
        .draw(|f| Scene::default().draw(f, app, logo, engine, true))
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn any_logo_can_supply_a_detailed_coverage_mask() {
    let logo = Logo::parse(
        "name='ring'\ncolors=['#ffffff']\nart='X'\nmask='''ffff\nf00f\nf00f\nffff'''\n",
    )
    .unwrap();
    let image = render(&app(GlyphMode::Solid), &logo, &Engine::default());
    assert!(image.content().iter().any(|c| c.symbol() == "▀"));
}

#[test]
fn detailed_masks_preserve_horizontal_palette_changes() {
    let logo = Logo::parse("name='split'\ncolors=['#ff0000','#00ff00']\nart='XX$2XX'\nmask='''ffff\nffff\nffff\nffff'''\n").unwrap();
    let mut app = app(GlyphMode::Solid);
    app.config.intensity = 0.0;
    let image = render(&app, &logo, &Engine::default());
    for color in [
        ratatui::style::Color::Rgb(255, 0, 0),
        ratatui::style::Color::Rgb(0, 255, 0),
    ] {
        assert!(
            image
                .content()
                .iter()
                .any(|c| c.symbol() == "█" && c.fg == color),
            "missing source palette color {color:?}"
        );
    }
}

#[test]
fn coverage_masks_reject_empty_ragged_nonhex_and_oversized_data() {
    for mask in [
        "".into(),
        "00\n00".into(),
        "ff\nf".into(),
        "f?".into(),
        "f".repeat(513),
        vec!["f"; 257].join("\n"),
    ] {
        let data = format!("name='bad'\ncolors=['#ffffff']\nart='X'\nmask='''{mask}'''\n");
        assert!(Logo::parse(&data).is_err());
    }
}

#[test]
fn apple_uses_subcell_edges_and_distinct_braille_outlines() {
    let logo = Logo::for_os(Os::Mac).unwrap();
    let solid = render(&app(GlyphMode::Solid), &logo, &Engine::default());
    assert!(
        solid.content().iter().any(|c| c.symbol() == "▀"),
        "curves must retain half-cell detail"
    );
    let dots = render(&app(GlyphMode::Dots), &logo, &Engine::default());
    let partial = dots
        .content()
        .iter()
        .filter(|c| {
            c.symbol()
                .chars()
                .any(|ch| ('\u{2801}'..='\u{28fe}').contains(&ch))
        })
        .count();
    assert!(
        partial > 15,
        "braille must follow the silhouette instead of filling every cell"
    );
}

#[test]
fn animation_preserves_the_entire_silhouette() {
    let logo = Logo::for_os(Os::Mac).unwrap();
    let app = app(GlyphMode::Solid);
    let mut engine = Engine::default();
    let first = render(&app, &logo, &engine);
    engine.advance(
        Duration::from_millis(250),
        Some(90.0),
        80.0,
        Some(1 << 20),
        1.0,
    );
    let second = render(&app, &logo, &engine);
    assert_ne!(first, second);
    assert!(
        first
            .content()
            .iter()
            .zip(second.content())
            .all(|(a, b)| a.symbol() == b.symbol())
    );
}
