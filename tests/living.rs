use osbeat::{
    animation::Engine,
    app::{Action, App, SETTING_COUNT},
    config::{Config, GlyphMode, LayoutMode, ThemeName},
    logo::Logo,
    model::{HostInfo, Os, Snapshot},
    ui::live::Scene,
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::time::Duration;

fn app() -> App {
    let host = HostInfo {
        os: Os::Mac,
        os_name: "macOS 26".into(),
        kernel: "25.0".into(),
        hostname: "living-mac".into(),
        cpu_name: "Apple M4".into(),
        cores: 10,
        arch: "arm64".into(),
    };
    App::new(host).with_snapshot(Snapshot {
        cpu: Some(35.0),
        used_memory: 8 << 30,
        total_memory: 16 << 30,
        ..Default::default()
    })
}
fn render(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    // Draw normally so the test backend receives the diff.
    terminal
        .draw(|frame| osbeat::ui::draw(frame, app, &Logo::for_os(Os::Mac).unwrap()))
        .unwrap();
    terminal.backend().buffer().clone()
}
fn text(buffer: &Buffer) -> String {
    buffer.content().iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn logo_changes_between_collector_samples_without_adding_history() {
    let app = app();
    let logo = Logo::for_os(Os::Mac).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
    let mut scene = Scene::default();
    let mut engine = Engine::default();
    engine.advance(
        Duration::from_millis(200),
        app.snapshot.cpu,
        50.0,
        Some(100000),
        1.0,
    );
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
        .unwrap();
    let first = terminal.backend().buffer().clone();
    engine.advance(
        Duration::from_millis(200),
        app.snapshot.cpu,
        50.0,
        Some(100000),
        1.0,
    );
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, false))
        .unwrap();
    assert_ne!(first, terminal.backend().buffer().clone());
    assert_eq!(app.history.cpu.len(), 1);
    assert!(scene.has_logo());
}
#[test]
fn cached_frame_and_paused_engine_are_identical_without_new_data() {
    let app = app();
    let logo = Logo::for_os(Os::Mac).unwrap();
    let engine = Engine::default();
    let mut scene = Scene::default();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
        .unwrap();
    let first = terminal.backend().buffer().clone();
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, false))
        .unwrap();
    assert_eq!(first, terminal.backend().buffer().clone());
}
#[test]
fn controls_and_customization_do_not_mutate_previous_state() {
    let original = app();
    let open = original.apply(Action::Customize);
    assert!(!original.customization && open.customization);
    let changed = open.apply(Action::Adjust(1));
    assert_ne!(changed.config.theme, original.config.theme);
    let last = open.apply(Action::Select(-1));
    assert_eq!(last.selected, SETTING_COUNT - 1);
    assert!(!last.demo);
    assert!(last.apply(Action::Adjust(1)).demo);
    let mut current = original.clone();
    for _ in 0..5 {
        current = current.apply(Action::Theme);
    }
    assert_eq!(current.config.theme, original.config.theme);
    assert_ne!(
        original.apply(Action::View).config.layout,
        original.config.layout
    );
    assert!(original.apply(Action::Motion).config.reduced_motion);
    assert_ne!(
        original.apply(Action::Effect).config.effect,
        original.config.effect
    );
    assert_ne!(
        original.apply(Action::Glyphs).config.glyphs,
        original.config.glyphs
    );
}
#[test]
fn help_is_modal_and_escape_only_closes_the_panel() {
    let original = app().apply(Action::Help);
    let after = original.apply(Action::Theme);
    assert_eq!(original.config, after.config);
    assert!(!after.apply(Action::Close).help);
    assert_eq!(after.config, after.apply(Action::Close).config);
}
#[test]
fn every_theme_view_glyph_and_small_viewport_renders_safely() {
    let mut theme = ThemeName::Default;
    for _ in 0..5 {
        for layout in [LayoutMode::Focus, LayoutMode::Dashboard] {
            for glyphs in [GlyphMode::Solid, GlyphMode::Ascii, GlyphMode::Dots] {
                let mut a = app().with_config(Config {
                    theme,
                    layout,
                    glyphs,
                    ..Default::default()
                });
                for (w, h) in [(0, 0), (1, 1), (30, 8), (76, 24), (100, 30), (120, 36)] {
                    render(&a, w, h);
                }
                a = a.apply(Action::Customize);
                assert!(text(&render(&a, 100, 30)).contains("CUSTOMIZE"));
                a = a.apply(Action::Close).apply(Action::Help);
                assert!(text(&render(&a, 100, 30)).contains("CONTROLS"));
            }
        }
        theme = theme.next();
    }
}
#[test]
fn demo_badge_is_visible_and_demo_never_rewrites_measured_stats() {
    let original = app();
    let demo = original.apply(Action::Demo);
    assert_eq!(original.snapshot.cpu, demo.snapshot.cpu);
    assert_eq!(original.history.cpu, demo.history.cpu);
    assert!(text(&render(&demo, 120, 36)).contains("DEMO LOGO"));
}
#[test]
fn all_settings_are_adjustable_with_valid_bounds() {
    let mut a = app().apply(Action::Customize);
    for index in 0..SETTING_COUNT {
        a.selected = index;
        for _ in 0..30 {
            a = a.apply(Action::Adjust(1));
            a.config.validate().unwrap();
        }
        for _ in 0..30 {
            a = a.apply(Action::Adjust(-1));
            a.config.validate().unwrap();
        }
    }
}

#[test]
fn direct_logo_updates_resynchronize_with_full_frames_and_resizes() {
    let app = app();
    let logo = Logo::for_os(Os::Mac).unwrap();
    let mut engine = Engine::default();
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
    let mut scene = Scene::default();
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
        .unwrap();
    for _ in 0..30 {
        engine.advance(
            Duration::from_millis(33),
            Some(65.0),
            50.0,
            Some(1 << 20),
            1.0,
        );
        scene
            .animate(terminal.backend_mut(), &app, &logo, &engine)
            .unwrap();
    }
    let animated = terminal.backend().buffer().clone();
    let mut reference = Terminal::new(TestBackend::new(120, 36)).unwrap();
    reference
        .draw(|frame| Scene::default().draw(frame, &app, &logo, &engine, true))
        .unwrap();
    assert_eq!(animated, reference.backend().buffer().clone());
    scene.sync_before_draw(&mut terminal);
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
        .unwrap();
    assert_eq!(animated, terminal.backend().buffer().clone());
    terminal.backend_mut().resize(80, 24);
    terminal.autoresize().unwrap();
    scene.sync_before_draw(&mut terminal);
    terminal
        .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
        .unwrap();
    engine.advance(Duration::from_millis(33), Some(80.0), 80.0, Some(0), 1.0);
    scene
        .animate(terminal.backend_mut(), &app, &logo, &engine)
        .unwrap();
    let mut reference = Terminal::new(TestBackend::new(80, 24)).unwrap();
    reference
        .draw(|frame| Scene::default().draw(frame, &app, &logo, &engine, true))
        .unwrap();
    assert_eq!(terminal.backend().buffer(), reference.backend().buffer());
}

#[test]
fn animation_never_paints_over_diagnostics_or_settings() {
    let logo = Logo::for_os(Os::Mac).unwrap();
    for layout in [LayoutMode::Focus, LayoutMode::Dashboard] {
        for customize in [false, true] {
            for height in [14, 20, 25, 26, 30, 36] {
                let mut app = app()
                    .with_config(Config {
                        layout,
                        ..Default::default()
                    })
                    .with_diagnostics(true);
                app.customization = customize;
                let mut terminal = Terminal::new(TestBackend::new(120, height)).unwrap();
                let mut scene = Scene::default();
                let mut engine = Engine::default();
                terminal
                    .draw(|frame| scene.draw(frame, &app, &logo, &engine, true))
                    .unwrap();
                engine.advance(
                    Duration::from_millis(200),
                    Some(90.0),
                    80.0,
                    Some(1 << 20),
                    1.0,
                );
                scene
                    .animate(terminal.backend_mut(), &app, &logo, &engine)
                    .unwrap();
                let mut reference = Terminal::new(TestBackend::new(120, height)).unwrap();
                reference
                    .draw(|frame| Scene::default().draw(frame, &app, &logo, &engine, true))
                    .unwrap();
                assert_eq!(
                    terminal.backend().buffer(),
                    reference.backend().buffer(),
                    "occlusion at {layout:?}, customize={customize}, h={height}"
                );
            }
        }
    }
}
