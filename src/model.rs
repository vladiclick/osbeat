use sysinfo::System;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Linux,
    Other,
}

impl Os {
    pub fn from_id(id: &str) -> Self {
        match id {
            "macos" => Self::Mac,
            "linux" => Self::Linux,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HostInfo {
    pub os: Os,
    pub os_name: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu_name: String,
    pub cores: usize,
    pub arch: String,
}

impl HostInfo {
    pub fn detect() -> Self {
        let mut system = System::new();
        system.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
        Self {
            os: Os::from_id(std::env::consts::OS),
            os_name: clean(&System::long_os_version().unwrap_or_else(|| "Unknown OS".into())),
            kernel: clean(&System::kernel_version().unwrap_or_else(|| "Unknown".into())),
            hostname: clean(&System::host_name().unwrap_or_else(|| "localhost".into())),
            cpu_name: clean(
                system
                    .cpus()
                    .first()
                    .map_or("Unknown CPU", |cpu| cpu.brand()),
            ),
            cores: system.cpus().len(),
            arch: std::env::consts::ARCH.into(),
        }
    }
}

// Hostnames and OS labels are external strings; never pass terminal controls through.
pub fn clean(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .take(160)
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub telemetry: crate::telemetry::Telemetry,
    pub cpu: Option<f32>,
    pub used_memory: u64,
    pub total_memory: u64,
    pub uptime: u64,
    /// Percentage of ONE logical core, deliberately not divided by core count.
    pub own_cpu: Option<f32>,
    pub own_memory: u64,
}

impl Snapshot {
    pub fn memory_percent(&self) -> f32 {
        if self.total_memory == 0 {
            return 0.0;
        }
        (self.used_memory as f64 / self.total_memory as f64 * 100.0).clamp(0.0, 100.0) as f32
    }

    pub fn cpu_percent(value: f32) -> Option<f32> {
        value.is_finite().then(|| value.clamp(0.0, 100.0))
    }
}

pub const HISTORY_LEN: usize = 60;

#[derive(Clone, Debug, Default)]
pub struct History {
    pub cpu: Vec<u64>,
    pub ram: Vec<u64>,
    pub down: Vec<u64>,
    pub up: Vec<u64>,
}

impl History {
    pub fn with_sample(&self, snapshot: &Snapshot) -> Self {
        Self {
            cpu: snapshot
                .cpu
                .map_or_else(|| self.cpu.clone(), |cpu| append(&self.cpu, cpu as u64)),
            ram: append(&self.ram, snapshot.memory_percent() as u64),
            down: snapshot
                .telemetry
                .network_down
                .map_or_else(|| self.down.clone(), |value| append(&self.down, value)),
            up: snapshot
                .telemetry
                .network_up
                .map_or_else(|| self.up.clone(), |value| append(&self.up, value)),
        }
    }
}

fn append(values: &[u64], value: u64) -> Vec<u64> {
    values
        .iter()
        .skip(values.len().saturating_sub(HISTORY_LEN - 1))
        .copied()
        .chain([value])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{History, Snapshot};

    #[test]
    fn network_history_skips_warmup_and_keeps_bounded_samples() {
        let empty = History::default().with_sample(&Snapshot::default());
        assert!(empty.down.is_empty() && empty.up.is_empty());
        let measured = Snapshot {
            telemetry: crate::telemetry::Telemetry {
                network_down: Some(1234),
                network_up: Some(42),
                ..Default::default()
            },
            ..Default::default()
        };
        let full = (0..80).fold(empty.clone(), |history, _| history.with_sample(&measured));
        assert_eq!(full.down, vec![1234; 60]);
        assert_eq!(full.up, vec![42; 60]);
        assert!(empty.down.is_empty());
        assert_eq!(full.with_sample(&Snapshot::default()).down, full.down);
    }

    #[test]
    fn percentages_are_safe_at_boundaries() {
        assert_eq!(Snapshot::default().memory_percent(), 0.0);
        assert_eq!(
            Snapshot {
                used_memory: 20,
                total_memory: 10,
                ..Snapshot::default()
            }
            .memory_percent(),
            100.0
        );
        for (value, expected) in [
            (f32::NAN, None),
            (f32::INFINITY, None),
            (-10.0, Some(0.0)),
            (110.0, Some(100.0)),
        ] {
            assert_eq!(Snapshot::cpu_percent(value), expected);
        }
        assert_eq!(
            Snapshot {
                used_memory: 6,
                total_memory: 16,
                ..Snapshot::default()
            }
            .memory_percent(),
            37.5
        );
    }

    #[test]
    fn history_updates_are_bounded_and_immutable() {
        let original = History::default();
        let filled = (0..80).fold(original.clone(), |h, n| {
            h.with_sample(&Snapshot {
                cpu: Some(n as f32),
                ..Snapshot::default()
            })
        });
        assert!(original.cpu.is_empty());
        assert_eq!(filled.cpu.len(), 60);
        assert_eq!(filled.cpu[0], 20);
        assert_eq!(filled.cpu[59], 79);
        assert_eq!(filled.ram.len(), 60);
        assert!(original.with_sample(&Snapshot::default()).cpu.is_empty());
    }
}
