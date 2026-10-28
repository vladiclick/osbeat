/// Extra readings remain optional when the OS cannot provide a trustworthy value.
#[derive(Clone, Debug, Default)]
pub struct Telemetry {
    pub per_core: Vec<f32>,
    pub swap_used: u64,
    pub swap_total: u64,
    /// Bytes per second, measured across non-loopback interfaces. None during warmup.
    pub network_down: Option<u64>,
    pub network_up: Option<u64>,
    /// Interfaces able to pass packets; this is not an internet connectivity check.
    pub network_interfaces: Vec<String>,
    pub root_disk: Option<DiskUsage>,
    pub battery: Option<BatteryStatus>,
    pub load_average: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct DiskUsage {
    pub used: u64,
    pub total: u64,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct BatteryStatus {
    pub percent: f32,
    pub charging: bool,
    pub on_ac: bool,
}
