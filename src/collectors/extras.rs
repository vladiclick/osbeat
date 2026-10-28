use std::{
    collections::BTreeMap,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use sysinfo::{InterfaceOperationalState, Networks, System};
use tokio::sync::watch;

use crate::{
    model::{Snapshot, clean},
    telemetry::{BatteryStatus, DiskUsage, Telemetry},
};

const SLOW_INTERVAL: Duration = Duration::from_secs(10);
const SHUTDOWN_GRACE: Duration = Duration::from_millis(750);
type Counters = BTreeMap<String, (u64, u64)>;

pub(super) struct Extras {
    networks: Networks,
    previous: Option<(Instant, Counters)>,
    peripherals: PeripheralWorker,
}

impl Extras {
    pub(super) fn new() -> Self {
        Self {
            networks: Networks::new(),
            previous: None,
            peripherals: PeripheralWorker::start(|| PeripheralSnapshot {
                root_disk: read_root_disk(),
                battery: read_battery(),
            }),
        }
    }

    pub(super) fn sample(&mut self, system: &System, warmed: bool) -> Telemetry {
        let (rate, interfaces) = self.sample_network();
        let peripherals = self.peripherals.snapshot();
        let load = System::load_average();
        Telemetry {
            per_core: if warmed {
                system
                    .cpus()
                    .iter()
                    .filter_map(|cpu| Snapshot::cpu_percent(cpu.cpu_usage()))
                    .collect()
            } else {
                Vec::new()
            },
            swap_used: system.used_swap(),
            swap_total: system.total_swap(),
            network_down: rate.map(|value| value.0),
            network_up: rate.map(|value| value.1),
            network_interfaces: interfaces,
            root_disk: peripherals.root_disk,
            battery: peripherals.battery,
            load_average: [load.one, load.five, load.fifteen],
        }
    }

    fn sample_network(&mut self) -> (Option<(u64, u64)>, Vec<String>) {
        self.networks.refresh(true);
        let now = Instant::now();
        let counters: Counters = self
            .networks
            .iter()
            .filter(|(name, network)| {
                !is_loopback(name) && network.operational_state() == InterfaceOperationalState::Up
            })
            .map(|(name, network)| {
                (
                    name.clone(),
                    (network.total_received(), network.total_transmitted()),
                )
            })
            .collect();
        let rate = self.previous.as_ref().and_then(|(last, previous)| {
            rates(&counters, Some(previous), now.duration_since(*last))
        });
        let interfaces = rank_interfaces(
            &counters,
            self.previous.as_ref().map(|(_, previous)| previous),
        );
        self.previous = Some((now, counters));
        (rate, interfaces)
    }
}

#[derive(Clone, Default)]
struct PeripheralSnapshot {
    root_disk: Option<DiskUsage>,
    battery: Option<BatteryStatus>,
}

struct PeripheralWorker {
    latest: watch::Receiver<PeripheralSnapshot>,
    stop: Option<mpsc::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
    finished: mpsc::Receiver<()>,
}

impl PeripheralWorker {
    fn start(mut poll: impl FnMut() -> PeripheralSnapshot + Send + 'static) -> Self {
        let (sender, latest) = watch::channel(PeripheralSnapshot::default());
        let (stop, shutdown) = mpsc::channel();
        let (finished_sender, finished) = mpsc::channel();
        let worker = thread::spawn(move || {
            // Disconnecting this channel signals completion even if the poll panics.
            let _completion = finished_sender;
            loop {
                if matches!(shutdown.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
                    break;
                }
                if sender.send(poll()).is_err() {
                    break;
                }
                // Dropping the stop sender interrupts this wait immediately at shutdown.
                if !matches!(
                    shutdown.recv_timeout(SLOW_INTERVAL),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    break;
                }
            }
        });
        Self {
            latest,
            stop: Some(stop),
            worker: Some(worker),
            finished,
        }
    }

    fn snapshot(&self) -> PeripheralSnapshot {
        self.latest.borrow().clone()
    }
}

impl Drop for PeripheralWorker {
    fn drop(&mut self) {
        self.stop.take();
        let Some(worker) = self.worker.take() else {
            return;
        };
        // OS calls cannot be cancelled safely. Keep terminal restoration bounded
        // even if the kernel unexpectedly stalls; process exit reclaims the thread.
        if self.finished.recv_timeout(SHUTDOWN_GRACE) == Err(mpsc::RecvTimeoutError::Timeout) {
            eprintln!(
                "osbeat: peripheral query is still blocked; detached collector so shutdown can continue"
            );
            return;
        }
        if let Err(error) = worker.join() {
            eprintln!("osbeat: peripheral collector stopped unexpectedly: {error:?}");
        }
    }
}

fn read_root_disk() -> Option<DiskUsage> {
    // Query exactly this path. Enumerating mounts can touch unavailable SMB/NFS
    // volumes even when their capacity refresh is disabled.
    let stat = rustix::fs::statvfs("/").ok()?;
    let block_size = if stat.f_frsize > 0 {
        stat.f_frsize
    } else {
        stat.f_bsize
    };
    let total = stat.f_blocks.saturating_mul(block_size);
    (total > 0).then(|| DiskUsage {
        used: stat
            .f_blocks
            .saturating_sub(stat.f_bfree)
            .saturating_mul(block_size)
            .min(total),
        total,
        label: "/".into(),
    })
}

fn rank_interfaces(current: &Counters, previous: Option<&Counters>) -> Vec<String> {
    let mut ranked: Vec<_> = current
        .iter()
        .map(|(name, &(rx, tx))| {
            let (old_rx, old_tx) = previous
                .and_then(|values| values.get(name))
                .copied()
                .unwrap_or((rx, tx));
            let activity = rx
                .saturating_sub(old_rx)
                .saturating_add(tx.saturating_sub(old_tx));
            (name, activity)
        })
        .collect();
    ranked.sort_by(|(name_a, activity_a), (name_b, activity_b)| {
        activity_b.cmp(activity_a).then_with(|| name_a.cmp(name_b))
    });
    ranked.into_iter().map(|(name, _)| clean(name)).collect()
}

fn is_loopback(name: &str) -> bool {
    name == "lo"
        || name.strip_prefix("lo").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
        })
}

fn rates(current: &Counters, previous: Option<&Counters>, elapsed: Duration) -> Option<(u64, u64)> {
    let previous = previous?;
    if elapsed.is_zero() || current.is_empty() {
        return None;
    }
    // Compare interfaces independently: hotplug and counter resets must not create spikes.
    let (down, up) = current
        .iter()
        .fold((0u64, 0u64), |(down, up), (name, &(rx, tx))| {
            let (old_rx, old_tx) = previous.get(name).copied().unwrap_or((rx, tx));
            (
                down.saturating_add(rx.saturating_sub(old_rx)),
                up.saturating_add(tx.saturating_sub(old_tx)),
            )
        });
    Some((
        (down as f64 / elapsed.as_secs_f64()) as u64,
        (up as f64 / elapsed.as_secs_f64()) as u64,
    ))
}

fn read_battery() -> Option<BatteryStatus> {
    use std::{
        process::{Command, Stdio},
        thread,
    };

    // This read-only, fixed command avoids private APIs and never requests privileges.
    // Bound its lifetime so a system utility cannot stall collection indefinitely.
    let mut child = Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let output = child.wait_with_output().ok()?;
                return parse_battery(std::str::from_utf8(&output.stdout).ok()?);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn parse_battery(text: &str) -> Option<BatteryStatus> {
    let on_ac = if text.contains("'AC Power'") {
        true
    } else if text.contains("'Battery Power'") {
        false
    } else {
        return None;
    };
    let line = text
        .lines()
        .find(|line| line.contains("InternalBattery") && line.contains('%'))?;
    let before_percent = line.split_once('%')?.0;
    let percent: f32 = before_percent.split_whitespace().last()?.parse().ok()?;
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return None;
    }
    let state = line.split(';').nth(1)?.trim();
    let charging = match state {
        "charging" => true,
        "discharging" | "charged" | "not charging" | "finishing charge" => false,
        _ => return None,
    };
    Some(BatteryStatus {
        percent,
        charging,
        on_ac,
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, time::Duration};

    use super::*;

    #[test]
    fn blocked_peripheral_poll_cannot_hold_shutdown_indefinitely() {
        struct Completed(mpsc::Sender<()>);
        impl Drop for Completed {
            fn drop(&mut self) {
                let _ = self.0.send(());
            }
        }
        let (entered, entered_receiver) = mpsc::channel();
        let (release, release_receiver) = mpsc::channel();
        let (completed, completed_receiver) = mpsc::channel();
        let completion = Completed(completed);
        let worker = PeripheralWorker::start(move || {
            let _keep_until_worker_exits = &completion;
            entered.send(()).expect("poll started");
            let _ = release_receiver.recv_timeout(Duration::from_secs(3));
            PeripheralSnapshot::default()
        });
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("poll entered");
        let started = Instant::now();
        drop(worker);
        let elapsed = started.elapsed();
        let _ = release.send(());
        completed_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("released worker exits");
        assert!(
            elapsed < Duration::from_millis(1500),
            "shutdown took {elapsed:?}"
        );
    }

    #[test]
    fn interfaces_sort_by_recent_activity_then_name_without_lifetime_bias() {
        let previous = BTreeMap::from([
            ("anpi0".into(), (90_000, 90_000)),
            ("en0".into(), (100, 200)),
            ("utun0".into(), (10, 20)),
            ("en1".into(), (0, 0)),
        ]);
        let current = BTreeMap::from([
            ("anpi0".into(), (90_000, 90_000)),
            ("en0".into(), (300, 300)),
            ("utun0".into(), (210, 120)),
            ("en1".into(), (0, 0)),
        ]);
        assert_eq!(
            rank_interfaces(&current, Some(&previous)),
            ["en0", "utun0", "anpi0", "en1"]
        );
        assert_eq!(
            rank_interfaces(&current, None),
            ["anpi0", "en0", "en1", "utun0"]
        );
        assert_eq!(
            rank_interfaces(&counters("new0", u64::MAX, u64::MAX), Some(&previous)),
            ["new0"]
        );
    }

    #[tokio::test]
    async fn peripheral_worker_publishes_and_stops_without_waiting_for_its_interval() {
        let mut worker = PeripheralWorker::start(|| PeripheralSnapshot {
            root_disk: Some(DiskUsage {
                used: 12,
                total: 40,
                label: "root".into(),
            }),
            battery: None,
        });
        tokio::time::timeout(Duration::from_secs(1), worker.latest.changed())
            .await
            .expect("peripheral sample deadline")
            .expect("peripheral worker alive");
        assert_eq!(worker.snapshot().root_disk.unwrap().used, 12);
        let started = Instant::now();
        drop(worker);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn slow_peripheral_poll_does_not_block_fast_samples() {
        let (started_sender, started_receiver) = std::sync::mpsc::channel();
        let (release_sender, release_receiver) = std::sync::mpsc::channel();
        let peripherals = PeripheralWorker::start(move || {
            started_sender.send(()).expect("sampler started");
            release_receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("sampler released");
            PeripheralSnapshot::default()
        });
        let mut extras = Extras {
            networks: Networks::new(),
            previous: None,
            peripherals,
        };
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("slow sampler entered");
        let sample = extras.sample(&System::new(), false);
        // The slow poll is still waiting here; the fast sampler returned independently.
        release_sender
            .send(())
            .expect("slow sampler remains available");
        assert!(sample.root_disk.is_none());
        assert!(sample.battery.is_none());
        drop(extras);
    }

    fn counters(name: &str, down: u64, up: u64) -> BTreeMap<String, (u64, u64)> {
        BTreeMap::from([(name.into(), (down, up))])
    }

    #[test]
    fn network_rates_use_actual_elapsed_time_and_have_a_warmup() {
        let baseline = counters("en0", 100, 40);
        assert_eq!(rates(&baseline, None, Duration::from_millis(500)), None);
        assert_eq!(
            rates(
                &counters("en0", 400, 100),
                Some(&baseline),
                Duration::from_millis(750)
            ),
            Some((400, 80))
        );
        assert_eq!(rates(&baseline, Some(&baseline), Duration::ZERO), None);
        assert_eq!(
            rates(&BTreeMap::new(), Some(&baseline), Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn network_counter_reset_and_new_interfaces_do_not_spike() {
        let old = counters("en0", 10_000, 10_000);
        assert_eq!(
            rates(&counters("en0", 40, 20), Some(&old), Duration::from_secs(1)),
            Some((0, 0))
        );
        assert_eq!(
            rates(
                &counters("utun0", 90_000, 90_000),
                Some(&old),
                Duration::from_secs(1)
            ),
            Some((0, 0))
        );
    }

    #[test]
    fn network_rates_add_only_matching_interfaces_without_overflow() {
        let old = BTreeMap::from([("en0".into(), (0, 0)), ("en1".into(), (0, 0))]);
        let new = BTreeMap::from([("en0".into(), (u64::MAX, 4)), ("en1".into(), (1, 8))]);
        assert_eq!(
            rates(&new, Some(&old), Duration::from_secs(1)),
            Some((u64::MAX, 12))
        );
    }

    #[test]
    fn battery_parser_distinguishes_charging_from_discharging() {
        let discharge = parse_battery("Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t63%; discharging; 2:12 remaining present: true").unwrap();
        assert_eq!(discharge.percent, 63.0);
        assert!(!discharge.charging);
        assert!(!discharge.on_ac);
        let charging = parse_battery("Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t45%; charging; 1:20 remaining present: true").unwrap();
        assert!(charging.charging);
        assert!(charging.on_ac);
        let charged = parse_battery("Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t100%; charged; 0:00 remaining present: true").unwrap();
        assert!(!charged.charging);
        assert_eq!(charged.percent, 100.0);
    }

    #[test]
    fn absent_or_invalid_battery_is_unavailable() {
        for text in [
            "",
            "Now drawing from 'AC Power'",
            "101%; charging",
            "NaN%; charging",
            "50%; unknown",
            "-4%; charging",
        ] {
            assert!(parse_battery(text).is_none(), "{text}");
        }
        for state in [
            "101%; charging",
            "NaN%; charging",
            "50%; unknown",
            "-4%; charging",
        ] {
            let text = format!(
                "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t{state}; remaining"
            );
            assert!(parse_battery(&text).is_none(), "{text}");
        }
    }

    #[test]
    fn loopback_names_are_excluded_without_excluding_unrelated_interfaces() {
        for name in ["lo", "lo0", "lo123"] {
            assert!(is_loopback(name));
        }
        for name in ["en0", "eth0", "local", "utun0"] {
            assert!(!is_loopback(name));
        }
    }

    #[test]
    fn live_extra_readings_are_bounded_and_optional() {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();
        let mut extras = Extras::new();
        let baseline = extras.sample(&system, false);
        assert!(baseline.per_core.is_empty());
        assert_eq!(baseline.network_down, None);
        let sample = extras.sample(&system, true);
        assert_eq!(sample.per_core.len(), system.cpus().len());
        assert!(
            sample
                .per_core
                .iter()
                .all(|usage| usage.is_finite() && (0.0..=100.0).contains(usage))
        );
        assert!(sample.swap_used <= sample.swap_total);
        assert!(
            sample
                .network_interfaces
                .iter()
                .all(|name| !is_loopback(name))
        );
        assert!(
            sample
                .load_average
                .iter()
                .all(|value| value.is_finite() && *value >= 0.0)
        );
        if let Some(disk) = sample.root_disk {
            assert!(disk.total > 0 && disk.used <= disk.total);
            assert!(!disk.label.chars().any(char::is_control));
        }
        if let Some(battery) = sample.battery {
            assert!((0.0..=100.0).contains(&battery.percent));
        }
    }

    #[test]
    fn native_peripheral_readings_are_valid_when_supported() {
        if let Some(disk) = read_root_disk() {
            assert!(disk.total > 0 && disk.used <= disk.total);
            assert!(!disk.label.chars().any(char::is_control));
        }
        if let Some(battery) = read_battery() {
            assert!((0.0..=100.0).contains(&battery.percent));
        }
    }
}
