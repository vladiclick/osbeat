//! A data-driven color field, independent of any logo's silhouette or UI widget.

use std::{f32::consts::TAU, time::Duration};

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Aurora,
    Liquid,
    Spectrum,
    Original,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Engine {
    cpu: f32,
    memory: f32,
    network: f32,
    pulse_phase: f32,
    flow_phase: f32,
}

impl Engine {
    /// Reduced motion still reflects live measurements, without advancing time.
    pub fn sync_metrics(
        &mut self,
        cpu: Option<f32>,
        memory_percent: f32,
        network_down: Option<u64>,
    ) {
        *self = Self {
            cpu: percent(cpu.unwrap_or(0.0)),
            memory: percent(memory_percent),
            network: network_level(network_down),
            ..*self
        };
    }

    pub fn advance(
        &mut self,
        dt: Duration,
        cpu: Option<f32>,
        memory_percent: f32,
        network_down: Option<u64>,
        speed: f32,
    ) {
        // Exponential smoothing depends on elapsed time, not the frame count.
        // A stalled terminal resumes gently instead of integrating a huge jump.
        let seconds = dt.as_secs_f32().min(0.25);
        let easing = 1.0 - (-3.0 * seconds).exp();
        let cpu = lerp(self.cpu, percent(cpu.unwrap_or(0.0)), easing);
        let memory = lerp(self.memory, percent(memory_percent), easing);
        // A logarithmic response makes both KiB/s and fast downloads visible.
        let network_target = network_level(network_down);
        let network = lerp(self.network, network_target, easing);
        let speed = finite(speed, 1.0).clamp(0.25, 3.0);
        *self = Self {
            cpu,
            memory,
            network,
            pulse_phase: (self.pulse_phase + seconds * TAU * (0.18 + 1.55 * cpu) * speed)
                .rem_euclid(TAU),
            flow_phase: (self.flow_phase + seconds * (0.8 + 4.0 * network) * speed).rem_euclid(TAU),
        };
    }

    #[allow(clippy::too_many_arguments)]
    pub fn color(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        base: Color,
        style: Style,
        intensity: f32,
    ) -> Color {
        let intensity = finite(intensity, 1.0).clamp(0.0, 1.0);
        if intensity == 0.0 {
            return base;
        }
        let nx = coordinate(x, width);
        let ny = coordinate(y, height);
        let flow = (nx * 4.8 - ny * 2.2 - self.flow_phase).sin() * 0.5 + 0.5;
        let shimmer = ((nx * 3.5 + ny - self.flow_phase).sin() * 0.5 + 0.5).powi(8);
        let surface = self.surface(x, width);
        let filled = smoothstep(surface - 0.055, surface + 0.055, ny);
        let rim = (1.0 - (ny - surface).abs() / 0.075).max(0.0);
        let pulse = 0.64 + 0.36 * self.pulse();
        let original = rgb(base);
        let effect = match style {
            Style::Aurora => {
                let mist = mix((73, 72, 220), (63, 245, 232), flow);
                let liquid = mix(mist, (134, 156, 255), filled * 0.30);
                let crest = mix(liquid, (198, 255, 251), rim * 0.55);
                scale(crest, pulse * (0.70 + 0.30 * filled) + shimmer * 0.30)
            }
            Style::Liquid => {
                let water = mix((50, 179, 242), (125, 246, 229), flow);
                let liquid = mix((31, 43, 79), water, filled);
                scale(
                    mix(liquid, (214, 255, 250), rim * 0.85),
                    pulse + shimmer * 0.18,
                )
            }
            Style::Spectrum => {
                let rainbow = spectrum(nx * 0.5 + ny * 0.4 - self.flow_phase / TAU);
                scale(mix(rainbow, (239, 247, 255), shimmer * 0.55), pulse)
            }
            Style::Original => scale(original, 0.58 + self.pulse() * 0.42 + shimmer * 0.18),
        };
        let (r, g, b) = mix(original, effect, intensity);
        Color::Rgb(r, g, b)
    }

    pub fn motion(&self) -> (f32, f32, f32) {
        (self.cpu, self.memory, self.network)
    }

    /// Breathing beat in 0..=1; its frequency follows smoothed CPU usage.
    pub fn pulse(&self) -> f32 {
        self.pulse_phase.sin() * 0.5 + 0.5
    }

    /// RAM's liquid boundary, measured from the top of any logo grid.
    pub fn surface(&self, x: usize, width: usize) -> f32 {
        let wobble = (coordinate(x, width) * 7.0 + self.flow_phase).sin() * 0.028;
        // At truly empty/full RAM, the wobble fades to preserve the meaning.
        let amplitude = (self.memory * 8.0).min(1.0) * ((1.0 - self.memory) * 8.0).min(1.0);
        1.0 - self.memory + wobble * amplitude
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn percent(value: f32) -> f32 {
    finite(value, 0.0).clamp(0.0, 100.0) / 100.0
}

fn network_level(bytes: Option<u64>) -> f32 {
    ((bytes.unwrap_or(0) as f32 + 1.0).log2() / 26.0).clamp(0.0, 1.0)
}

fn coordinate(position: usize, length: usize) -> f32 {
    (position as f32 / length.saturating_sub(1).max(1) as f32).clamp(0.0, 1.0)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let position = ((value - low) / (high - low)).clamp(0.0, 1.0);
    position * position * (3.0 - 2.0 * position)
}

type Rgb = (u8, u8, u8);

fn mix(from: Rgb, to: Rgb, amount: f32) -> Rgb {
    let channel = |a, b| lerp(a as f32, b as f32, amount).round().clamp(0.0, 255.0) as u8;
    (
        channel(from.0, to.0),
        channel(from.1, to.1),
        channel(from.2, to.2),
    )
}

fn scale(color: Rgb, amount: f32) -> Rgb {
    let channel = |value| (value as f32 * amount).round().clamp(0.0, 255.0) as u8;
    (channel(color.0), channel(color.1), channel(color.2))
}

fn spectrum(position: f32) -> Rgb {
    let hue = position.rem_euclid(1.0) * 6.0;
    let rising = (55.0 + (hue.fract() * 200.0)) as u8;
    let falling = 255 - rising + 55;
    match hue as u8 {
        0 => (255, rising, 55),
        1 => (falling, 255, 55),
        2 => (55, 255, rising),
        3 => (55, falling, 255),
        4 => (rising, 55, 255),
        _ => (255, 55, falling),
    }
}

fn rgb(color: Color) -> Rgb {
    use Color::*;
    match color {
        Rgb(r, g, b) => (r, g, b),
        Indexed(index @ 16..=231) => {
            let cube = index - 16;
            let channel = |value| if value == 0 { 0 } else { 55 + value * 40 };
            (channel(cube / 36), channel(cube / 6 % 6), channel(cube % 6))
        }
        Indexed(index @ 232..=255) => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
        Indexed(index) => rgb([
            Black,
            Red,
            Green,
            Yellow,
            Blue,
            Magenta,
            Cyan,
            Gray,
            DarkGray,
            LightRed,
            LightGreen,
            LightYellow,
            LightBlue,
            LightMagenta,
            LightCyan,
            White,
        ][index as usize]),
        Black => (0, 0, 0),
        Red => (170, 0, 0),
        Green => (0, 170, 0),
        Yellow => (170, 170, 0),
        Blue => (0, 0, 170),
        Magenta => (170, 0, 170),
        Cyan => (0, 170, 170),
        Gray => (170, 170, 170),
        DarkGray => (85, 85, 85),
        LightRed => (255, 85, 85),
        LightGreen => (85, 255, 85),
        LightYellow => (255, 255, 85),
        LightBlue => (85, 85, 255),
        LightMagenta => (255, 85, 255),
        LightCyan => (85, 255, 255),
        White | Reset => (255, 255, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::{Engine, Style};
    use ratatui::style::Color;
    use std::time::Duration;

    fn settled(cpu: f32, ram: f32, down: u64) -> Engine {
        (0..180).fold(Engine::default(), |mut engine, _| {
            engine.advance(
                Duration::from_secs_f32(1.0 / 30.0),
                Some(cpu),
                ram,
                Some(down),
                1.0,
            );
            engine
        })
    }

    #[test]
    fn samples_ease_instead_of_jumping() {
        let mut engine = Engine::default();
        engine.advance(
            Duration::from_millis(33),
            Some(100.0),
            100.0,
            Some(64 << 20),
            1.0,
        );
        let (cpu, memory, network) = engine.motion();
        assert!(cpu > 0.0 && cpu < 0.2);
        assert!(memory > 0.0 && memory < 0.2);
        assert!(network > 0.0 && network < 0.2);
        let (cpu, memory, network) = settled(100.0, 100.0, 64 << 20).motion();
        assert!(cpu > 0.99 && memory > 0.99 && network > 0.99);
    }

    #[test]
    fn idle_logo_remains_visibly_alive() {
        let first = settled(0.0, 50.0, 0);
        let mut second = first;
        second.advance(Duration::from_millis(250), Some(0.0), 50.0, Some(0), 1.0);
        for style in [
            Style::Aurora,
            Style::Liquid,
            Style::Spectrum,
            Style::Original,
        ] {
            let changed = (0..20).any(|x| {
                first.color(x, 8, 20, 16, Color::White, style, 1.0)
                    != second.color(x, 8, 20, 16, Color::White, style, 1.0)
            });
            assert!(changed, "{style:?} should animate at idle");
        }
    }

    #[test]
    fn liquid_is_brighter_below_the_memory_boundary() {
        let engine = settled(0.0, 50.0, 0);
        let brightness = |color| match color {
            Color::Rgb(r, g, b) => r as u32 + g as u32 + b as u32,
            _ => panic!("animated color must be RGB"),
        };
        let top = engine.color(10, 0, 20, 20, Color::White, Style::Liquid, 1.0);
        let bottom = engine.color(10, 19, 20, 20, Color::White, Style::Liquid, 1.0);
        assert!(brightness(bottom) > brightness(top) * 2);
    }

    #[test]
    fn disabling_effect_preserves_every_original_color() {
        let engine = settled(100.0, 80.0, 123456);
        for base in [
            Color::Reset,
            Color::Red,
            Color::Indexed(201),
            Color::Rgb(1, 2, 3),
        ] {
            for style in [
                Style::Aurora,
                Style::Liquid,
                Style::Spectrum,
                Style::Original,
            ] {
                assert_eq!(engine.color(1, 1, 10, 10, base, style, 0.0), base);
            }
        }
    }

    #[test]
    fn smoothing_is_independent_of_render_frequency() {
        let run = |frames: u32| {
            (0..frames).fold(Engine::default(), |mut engine, _| {
                engine.advance(
                    Duration::from_secs_f64(1.0 / frames as f64),
                    Some(80.0),
                    60.0,
                    Some(10000),
                    1.0,
                );
                engine
            })
        };
        let slow = run(10).motion();
        let fast = run(60).motion();
        assert!((slow.0 - fast.0).abs() < 0.0001);
        assert!((slow.1 - fast.1).abs() < 0.0001);
        assert!((slow.2 - fast.2).abs() < 0.0001);
    }

    #[test]
    fn malformed_inputs_and_tiny_logos_are_safe() {
        let mut engine = Engine::default();
        engine.advance(
            Duration::MAX,
            Some(f32::NAN),
            f32::INFINITY,
            Some(u64::MAX),
            f32::NAN,
        );
        let (cpu, memory, network) = engine.motion();
        assert!(cpu.is_finite() && memory.is_finite() && network.is_finite());
        for dimension in [0, 1, 2, usize::MAX] {
            for style in [
                Style::Aurora,
                Style::Liquid,
                Style::Spectrum,
                Style::Original,
            ] {
                assert!(matches!(
                    engine.color(0, 0, dimension, dimension, Color::White, style, f32::NAN),
                    Color::Rgb(..)
                ));
            }
        }
    }

    #[test]
    fn paused_time_does_not_change_the_image() {
        let first = settled(20.0, 50.0, 1000);
        let mut second = first;
        second.advance(Duration::ZERO, Some(90.0), 90.0, Some(0), 2.0);
        assert_eq!(first.motion(), second.motion());
        assert_eq!(
            first.color(5, 5, 20, 20, Color::White, Style::Aurora, 1.0),
            second.color(5, 5, 20, 20, Color::White, Style::Aurora, 1.0)
        );
    }

    #[test]
    fn metric_sync_preserves_animation_time_and_updates_live_levels() {
        let original = settled(10.0, 20.0, 0);
        let mut engine = original;
        engine.sync_metrics(Some(80.0), 75.0, Some(64 << 20));
        let (cpu, memory, network) = engine.motion();
        assert_eq!(cpu, 0.8);
        assert_eq!(memory, 0.75);
        assert!(network > 0.999);
        assert_eq!(original.pulse(), engine.pulse());
        assert_eq!(original.pulse_phase, engine.pulse_phase);
        assert_eq!(original.flow_phase, engine.flow_phase);
        assert!(engine.surface(1, 20) < original.surface(1, 20));
        engine.sync_metrics(Some(f32::NAN), f32::INFINITY, None);
        assert_eq!(engine.motion(), (0.0, 0.0, 0.0));
        assert_eq!(original.pulse(), engine.pulse());
    }

    #[test]
    fn cpu_changes_breathing_speed_and_network_changes_flow_speed() {
        let idle = settled(0.0, 50.0, 0);
        let busy = settled(100.0, 50.0, 64 << 20);
        let advance = |mut engine: Engine| {
            let before = (engine.pulse_phase, engine.flow_phase);
            let (cpu, ram, network) = engine.motion();
            engine.advance(
                Duration::from_millis(50),
                Some(cpu * 100.0),
                ram * 100.0,
                Some(if network > 0.5 { 64 << 20 } else { 0 }),
                1.0,
            );
            (
                (engine.pulse_phase - before.0).rem_euclid(super::TAU),
                (engine.flow_phase - before.1).rem_euclid(super::TAU),
            )
        };
        let idle_step = advance(idle);
        let busy_step = advance(busy);
        assert!(busy_step.0 > idle_step.0 * 5.0);
        assert!(busy_step.1 > idle_step.1 * 4.0);
    }

    #[test]
    fn ram_fill_rises_as_memory_usage_increases_on_any_grid() {
        let low = settled(0.0, 10.0, 0);
        let high = settled(0.0, 90.0, 0);
        for width in [1, 9, 37, 1000] {
            for x in [0, width / 2, width - 1] {
                assert!(low.surface(x, width) > 0.85);
                assert!(high.surface(x, width) < 0.15);
            }
        }
    }

    #[test]
    fn speed_and_color_inputs_are_bounded_and_reproducible() {
        let mut fast = Engine::default();
        let mut bounded = Engine::default();
        fast.advance(Duration::from_millis(50), Some(-1.0), -100.0, None, 1000.0);
        bounded.advance(Duration::from_millis(50), Some(0.0), 0.0, Some(0), 3.0);
        assert_eq!(fast.motion(), bounded.motion());
        assert_eq!(fast.flow_phase, bounded.flow_phase);
        for index in 0..=255 {
            let base = Color::Indexed(index);
            for style in [
                Style::Aurora,
                Style::Liquid,
                Style::Spectrum,
                Style::Original,
            ] {
                assert!(matches!(
                    fast.color(1, 1, 7, 7, base, style, 1.0),
                    Color::Rgb(..)
                ));
                assert_eq!(fast.color(1, 1, 7, 7, base, style, -10.0), base);
                assert_eq!(
                    fast.color(1, 1, 7, 7, base, style, 10.0),
                    fast.color(1, 1, 7, 7, base, style, 1.0)
                );
            }
        }
        for section in 0..6 {
            let color = super::spectrum(section as f32 / 6.0);
            assert_ne!(color, (0, 0, 0));
        }
    }
}
