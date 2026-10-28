#[cfg(not(target_os = "macos"))]
compile_error!("osbeat runs on macOS only");

pub mod animation;
pub mod app;
pub mod cli;
pub mod collectors;
pub mod config;
pub mod logo;
pub mod model;
pub mod telemetry;
pub mod ui;
