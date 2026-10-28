use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use osbeat::{
    app::{Action, App},
    logo::Logo,
    model::{HostInfo, Os, Snapshot},
    telemetry::{BatteryStatus, DiskUsage, Telemetry},
    ui,
};
use ratatui::{Terminal, backend::TestBackend};

fn sample() -> Snapshot {
    Snapshot {
        cpu: Some(25.0),
        used_memory: 6 << 30,
        total_memory: 16 << 30,
        uptime: 90061,
        own_cpu: Some(0.4),
        own_memory: 8 << 20,
        telemetry: Telemetry {
            per_core: vec![12.0, 25.0, 37.0, 50.0, 62.0, 75.0, 87.0, 20.0],
            swap_used: 1 << 30,
            swap_total: 4 << 30,
            network_down: Some(1 << 20),
            network_up: Some(32 << 10),
            network_interfaces: vec!["en0".into()],
            root_disk: Some(DiskUsage {
                used: 120 << 30,
                total: 256 << 30,
                label: "Macintosh HD".into(),
            }),
            battery: Some(BatteryStatus {
                percent: 82.0,
                charging: false,
                on_ac: false,
            }),
            load_average: [0.8, 1.2, 1.5],
        },
    }
}

#[test]
fn logo_parser_preserves_spaces_and_color_state() {
    let logo =
        Logo::parse("name = 'Test'\ncolors = ['#112233', '#aabbcc']\nart = '''  $1XX\n$2Y Z'''\n")
            .unwrap();
    assert_eq!(logo.width, 4);
    assert_eq!(logo.rows.len(), 2);
    assert_eq!(
        logo.rows[0].iter().map(|g| g.character).collect::<String>(),
        "  XX"
    );
    assert_eq!(
        logo.rows[0][2].color,
        ratatui::style::Color::Rgb(17, 34, 51)
    );
    assert_eq!(
        logo.rows[1][0].color,
        ratatui::style::Color::Rgb(170, 187, 204)
    );
}

#[test]
fn bad_logo_data_is_rejected() {
    for data in [
        "",
        "colors = []\nname='a'\nart='x'",
        "name='a'\ncolors=['oops']\nart='x'",
        "name='a'\ncolors=['#ffffff']\nart='$9x'",
        "name='a'\ncolors=['#ffffff']\nart='x$'",
        "name='a'\ncolors=['#ffffff']\nart='\u{1b}'",
        "name='a'\ncolors=['#ffffff']\nart=''",
    ] {
        assert!(Logo::parse(data).is_err(), "accepted {data:?}");
    }
}

#[test]
fn bundled_logos_load_without_external_files() {
    for os in [Os::Mac, Os::Linux, Os::Other] {
        let logo = Logo::for_os(os).unwrap();
        assert!(logo.width > 0 && !logo.rows.is_empty());
        assert!(!logo.rows.iter().flatten().any(|g| g.character == '$'));
    }
}

#[test]
fn keys_have_predictable_actions() {
    let key = |code, modifiers| Event::Key(KeyEvent::new(code, modifiers));
    assert_eq!(
        Action::from_event(&key(KeyCode::Char('q'), KeyModifiers::NONE)),
        Action::Quit
    );
    assert_eq!(
        Action::from_event(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Action::Quit
    );
    assert_eq!(
        Action::from_event(&key(KeyCode::Char('d'), KeyModifiers::NONE)),
        Action::Diagnostics
    );
    assert_eq!(
        Action::from_event(&key(KeyCode::Char('z'), KeyModifiers::NONE)),
        Action::None
    );
    assert_eq!(Action::from_event(&Event::Resize(80, 24)), Action::Redraw);
    let release = Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    assert_eq!(Action::from_event(&release), Action::None);
}

fn render(width: u16, height: u16, diagnostics: bool) -> String {
    render_snapshot(width, height, diagnostics, sample())
}

fn render_snapshot(width: u16, height: u16, diagnostics: bool, snapshot: Snapshot) -> String {
    let host = HostInfo {
        os: Os::Mac,
        os_name: "macOS 26".into(),
        kernel: "Darwin 25.0".into(),
        hostname: "test-mac".into(),
        cpu_name: "Apple M2".into(),
        cores: 8,
        arch: "aarch64".into(),
    };
    let app = App::new(host)
        .with_snapshot(snapshot)
        .with_config(osbeat::config::Config {
            layout: osbeat::config::LayoutMode::Dashboard,
            ..Default::default()
        })
        .with_diagnostics(diagnostics);
    let logo = Logo::for_os(Os::Mac).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &app, &logo)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn dashboard_and_diagnostics_render() {
    let text = render(120, 36, false);
    for expected in [
        "osbeat",
        "CPU",
        "MEMORY",
        "UPTIME",
        "test-mac",
        "macOS 26",
        "25.0%",
        "6.0 / 16.0 GiB",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    assert!(!text.contains("SELF CPU"));
    assert!(render(120, 36, true).contains("SELF CPU"));
}

#[test]
fn resizing_never_panics_and_small_layout_keeps_stats() {
    for (w, h) in [
        (0, 0),
        (1, 1),
        (8, 3),
        (30, 8),
        (45, 18),
        (70, 24),
        (80, 24),
        (100, 30),
        (240, 60),
    ] {
        let text = render(w, h, true);
        if w >= 30 && h >= 8 {
            assert!(text.contains("CPU"));
        }
    }
}

#[test]
fn formatters_cover_units_and_long_uptime() {
    assert_eq!(ui::uptime(0), "0m");
    assert_eq!(ui::uptime(3600), "1h 00m");
    assert_eq!(ui::uptime(90061), "1d 01h 01m");
    assert_eq!(ui::gib(16 << 30), "16.0");
}

#[test]
fn operating_system_detection_is_explicit() {
    assert_eq!(Os::from_id("macos"), Os::Mac);
    assert_eq!(Os::from_id("linux"), Os::Linux);
    assert_eq!(Os::from_id("freebsd"), Os::Other);
    assert!(!HostInfo::detect().hostname.is_empty());
}

#[test]
fn external_labels_cannot_inject_terminal_controls() {
    assert_eq!(osbeat::model::clean("a\n\r\t\u{1b}b"), "ab");
    assert_eq!(osbeat::model::clean(&"a".repeat(200)).len(), 160);
}

#[test]
fn command_line_options_are_strict() {
    use osbeat::cli::Options;
    let parse = |args: &[&str]| Options::parse(args.iter().map(|arg| arg.to_string()));
    assert_eq!(parse(&[]).unwrap(), Options::Run(Default::default()));
    assert_eq!(
        parse(&["--debug"]).unwrap(),
        Options::Run(osbeat::cli::Launch {
            diagnostics: true,
            ..Default::default()
        })
    );
    for flag in ["-h", "--help"] {
        assert_eq!(parse(&[flag]).unwrap(), Options::Help);
    }
    for flag in ["-V", "--version"] {
        assert_eq!(parse(&[flag]).unwrap(), Options::Version);
    }
    assert!(parse(&["--wat"]).is_err());
    assert!(parse(&["--help", "--debug"]).is_err());
}

#[test]
fn richer_dashboard_shows_live_resources_at_laptop_sizes() {
    for (w, h) in [(100, 30), (120, 36)] {
        let text = render(w, h, false);
        for label in [
            "CPU",
            "MEMORY",
            "SWAP",
            "NETWORK",
            "STORAGE",
            "POWER",
            "C01",
            "en0",
            "1.0 MiB/s",
            "32.0 KiB/s",
            "82%",
            "120.0",
            "256.0",
        ] {
            assert!(text.contains(label), "missing {label} at {w}x{h}");
        }
    }
}

#[test]
fn throughput_formatting_preserves_warmup_and_units() {
    assert_eq!(ui::rate(None), "sampling");
    assert_eq!(ui::rate(Some(0)), "0 B/s");
    assert_eq!(ui::rate(Some(1023)), "1023 B/s");
    assert_eq!(ui::rate(Some(1024)), "1.0 KiB/s");
    assert_eq!(ui::rate(Some(1 << 20)), "1.0 MiB/s");
    assert_eq!(ui::rate(Some(1 << 30)), "1.0 GiB/s");
}

#[test]
fn unavailable_resources_are_explicit_and_never_invented() {
    let text = render_snapshot(120, 36, false, Snapshot::default());
    assert_eq!(text.matches("Not available").count(), 2);
    assert!(text.contains("No active interface"));
    assert!(text.contains("sampling"));
    assert!(!text.contains("Discharging"));
}

#[test]
fn layout_thresholds_and_large_core_counts_remain_bounded() {
    let snapshot = Snapshot {
        telemetry: Telemetry {
            per_core: vec![100.0; 128],
            ..Telemetry::default()
        },
        ..sample()
    };
    for width in [29, 30, 51, 52, 73, 74, 75, 76, 77, 99, 100, 101, 120] {
        for height in [7, 8, 18, 19, 23, 24, 27, 28, 30, 36] {
            let text = render_snapshot(width, height, false, snapshot.clone());
            if width >= 30 && height >= 8 {
                assert!(text.contains("CPU"));
            }
        }
    }
}

#[test]
fn charging_and_external_power_have_distinct_labels() {
    for (charging, on_ac, expected) in [
        (true, true, "Charging"),
        (false, true, "Plugged in"),
        (false, false, "Discharging"),
    ] {
        let snapshot = Snapshot {
            telemetry: Telemetry {
                battery: Some(BatteryStatus {
                    percent: 50.0,
                    charging,
                    on_ac,
                }),
                ..Telemetry::default()
            },
            ..sample()
        };
        assert!(render_snapshot(120, 36, false, snapshot).contains(expected));
    }
}

#[test]
fn unfamiliar_logo_dimensions_do_not_break_responsive_layout() {
    let app = App::new(HostInfo::detect()).with_snapshot(sample());
    let logo = Logo::parse(&format!(
        "name='wide'\ncolors=['#ffffff']\nart='{}'",
        "X".repeat(80)
    ))
    .unwrap();
    for (width, height) in [(80, 24), (120, 36), (240, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| ui::draw(frame, &app, &logo)).unwrap();
    }
}
