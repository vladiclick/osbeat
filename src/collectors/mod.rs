use std::{thread, time::Duration};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{sync::watch, task::JoinHandle};

use crate::model::Snapshot;

mod extras;

pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

pub fn start() -> (watch::Receiver<Snapshot>, JoinHandle<()>) {
    // A watch channel retains only the latest snapshot: a slow UI can't build a queue.
    let (sender, receiver) = watch::channel(Snapshot::default());
    let worker = tokio::task::spawn_blocking(move || collect(sender));
    (receiver, worker)
}

fn collect(sender: watch::Sender<Snapshot>) {
    // sysinfo keeps the previous counters internally to calculate CPU deltas.
    // Its mutable state stays on this one blocking thread, outside the async UI.
    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    let refresh = ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .without_tasks();
    let mut warmed = false;
    let mut extras = extras::Extras::new();
    loop {
        let started = std::time::Instant::now();
        system.refresh_cpu_usage();
        system.refresh_memory();
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, refresh);
        let process = system.process(pid);
        let snapshot = Snapshot {
            // The first CPU refresh establishes a baseline, not a meaningful reading.
            cpu: warmed
                .then(|| Snapshot::cpu_percent(system.global_cpu_usage()))
                .flatten(),
            used_memory: system.used_memory(),
            total_memory: system.total_memory(),
            uptime: System::uptime(),
            own_cpu: process.filter(|_| warmed).and_then(|p| {
                let value = p.cpu_usage();
                value.is_finite().then(|| value.max(0.0))
            }),
            own_memory: process.map_or(0, |p| p.memory()),
            telemetry: extras.sample(&system, warmed),
        };
        if sender.send(snapshot).is_err() {
            break;
        }
        warmed = true;
        // Receiver shutdown is checked after the wait: exit takes at most one tick.
        thread::sleep(SAMPLE_INTERVAL.saturating_sub(started.elapsed()));
        if sender.is_closed() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collector_publishes_live_snapshots_and_stops_on_receiver_drop() {
        let (mut receiver, worker) = start();
        tokio::time::timeout(Duration::from_secs(5), receiver.changed())
            .await
            .expect("first collection deadline")
            .expect("collector remains open");
        // A watch channel can coalesce the warmup update while the test is paused.
        // Wait for a valid CPU reading rather than assuming which update we saw.
        let sample = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let sample = receiver.borrow_and_update().clone();
                if sample.cpu.is_some() {
                    break sample;
                }
                receiver.changed().await.expect("collector remains open");
            }
        })
        .await
        .expect("CPU warmup deadline");
        assert!(sample.total_memory > 0);
        assert!(
            sample
                .cpu
                .is_some_and(|value| (0.0..=100.0).contains(&value))
        );
        assert!(!sample.telemetry.per_core.is_empty());
        assert!(sample.own_memory > 0);
        drop(receiver);
        tokio::time::timeout(Duration::from_secs(2), worker)
            .await
            .expect("collector shuts down")
            .expect("collector worker succeeds");
    }
}
