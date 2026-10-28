use std::{thread, time::Duration};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{sync::watch, task::JoinHandle};

use crate::model::Snapshot;

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
