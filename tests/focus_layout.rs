use osbeat::{
    app::App,
    config::{Config, LayoutMode, Panels},
    logo::Logo,
    model::{HostInfo, Os, Snapshot},
    telemetry::{BatteryStatus, DiskUsage, Telemetry},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

fn app() -> App {
    App::new(HostInfo {
        os: Os::Mac,
        os_name: "macOS".into(),
        kernel: "Darwin".into(),
        hostname: "test-mac".into(),
        cpu_name: "Apple M4".into(),
        cores: 10,
        arch: "arm64".into(),
    })
    .with_config(Config {
        layout: LayoutMode::Focus,
        ..Default::default()
    })
    .with_snapshot(Snapshot {
        cpu: Some(27.5),
        used_memory: 8 << 30,
        total_memory: 16 << 30,
        telemetry: Telemetry {
            swap_used: 1 << 30,
            swap_total: 2 << 30,
            network_down: Some(3 << 20),
            network_up: Some(128 << 10),
            network_interfaces: vec!["en0".into()],
            battery: Some(BatteryStatus {
                percent: 83.0,
                charging: true,
                on_ac: true,
            }),
            root_disk: Some(DiskUsage {
                used: 384 << 30,
                total: 512 << 30,
                label: "/".into(),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn render(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let logo = Logo::for_os(Os::Mac).unwrap();
    terminal
        .draw(|frame| osbeat::ui::draw(frame, app, &logo))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn title_row(buffer: &Buffer, label: &str) -> u16 {
    (3..buffer.area.height - 4)
        .find(|&y| row(buffer, y).contains(label))
        .unwrap()
}

#[test]
fn focus_rails_have_three_aligned_complete_readouts() {
    for (width, height) in [(100, 30), (120, 36)] {
        let buffer = render(&app(), width, height);
        for (left, right) in [("CPU", "NETWORK"), ("MEMORY", "POWER"), ("SWAP", "STORAGE")] {
            assert_eq!(title_row(&buffer, left), title_row(&buffer, right));
        }
        let content = (0..height)
            .map(|y| row(&buffer, y))
            .collect::<Vec<_>>()
            .join("\n");
        for value in [
            "27.5%",
            "8.0 / 16.0 GiB",
            "1.0 GiB used",
            "↓ 3.0 MiB/s",
            "↑ 128.0 KiB/s",
            "83%",
            "128.0 GiB free",
            "384.0 / 512.0 GiB",
            "en0",
        ] {
            assert!(
                content.contains(value),
                "missing {value} at {width}x{height}"
            );
        }
        assert!(!content.contains("measured / second"));
        assert!(!content.contains("non-loopback total"));
    }
}

#[test]
fn charts_show_latest_measurements_and_have_empty_gutters() {
    let buffer = render(&app(), 120, 36);
    let chart_y = (3..32)
        .find(|&y| {
            let line = row(&buffer, y);
            line.contains("CPU") && line.contains("MEMORY") && line.contains("NETWORK")
        })
        .unwrap();
    let line = row(&buffer, chart_y);
    for value in ["27.5%", "50.0%", "3.0 MiB/s"] {
        assert!(line.contains(value));
    }
    for x in [39, 40, 79, 80] {
        for y in chart_y..chart_y + 4 {
            assert_eq!(buffer[(x, y)].symbol(), " ", "gutter at {x},{y}");
        }
    }
}

#[test]
fn compact_focus_retains_measured_stats_and_honors_panel_toggles() {
    for width in [60, 80] {
        let buffer = render(&app(), width, 24);
        let content = (0..24)
            .map(|y| row(&buffer, y))
            .collect::<Vec<_>>()
            .join("\n");
        for value in ["27.5%", "50.0%", "3.0 MiB/s", "128.0 KiB/s"] {
            assert!(content.contains(value), "missing {value} at width {width}");
        }
    }
    let hidden = app().with_config(Config {
        panels: Panels {
            cpu: false,
            memory: false,
            network: false,
            storage: false,
            power: false,
        },
        ..app().config
    });
    for (width, height) in [(80, 24), (120, 36)] {
        let buffer = render(&hidden, width, height);
        let content = (3..height - 4)
            .map(|y| row(&buffer, y))
            .collect::<Vec<_>>()
            .join("\n");
        for label in ["CPU", "MEMORY", "NETWORK", "SWAP", "POWER", "STORAGE"] {
            assert!(!content.contains(label));
        }
    }
}
