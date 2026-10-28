use crossterm::event::EventStream;
use futures_util::StreamExt;
use osbeat::{
    animation::Engine,
    app::{Action, App},
    collectors,
    logo::Logo,
    ui::live::Scene,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};
use tokio::{
    signal::unix::{SignalKind, signal},
    time::{MissedTickBehavior, interval},
};

type BufferedTerminal = Terminal<CrosstermBackend<std::io::BufWriter<std::io::Stdout>>>;

pub fn init_terminal() -> io::Result<BufferedTerminal> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        previous(info);
    }));
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    // SGR color commands contain many tiny writes. One frame buffer avoids
    // locking stdout for every number in every animated pixel's RGB sequence.
    let writer = std::io::BufWriter::with_capacity(64 * 1024, std::io::stdout());
    Terminal::new(CrosstermBackend::new(writer))
}

pub async fn run(
    terminal: &mut BufferedTerminal,
    mut app: App,
    logo: &Logo,
    config_path: &Path,
) -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut events = EventStream::new();
    let (mut samples, worker) = collectors::start();
    let mut scene = Scene::default();
    let mut engine = Engine::default();
    let mut frames = interval(period(app.config.fps));
    frames.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let started = Instant::now();
    let mut last = Instant::now();
    let result = async {
        terminal.draw(|frame| scene.draw(frame, &app, logo, &engine, true))?;
        loop {
            let dirty = tokio::select! {
                _ = terminate.recv() => break,
                _ = hangup.recv() => break,
                _ = interrupt.recv() => break,
                changed = samples.changed() => {
                    changed.map_err(|_| io::Error::other("system collector stopped unexpectedly"))?;
                    app = app.with_snapshot(samples.borrow_and_update().clone());
                    if app.config.reduced_motion { engine.sync_metrics(app.snapshot.cpu, app.snapshot.memory_percent(), app.snapshot.telemetry.network_down); }
                    true
                }
                _ = frames.tick(), if !app.config.reduced_motion && app.config.intensity > 0.0 && !app.help && scene.has_logo() => {
                    let now = Instant::now();
                    let (cpu, memory, network) = logo_inputs(&app, started.elapsed());
                    engine.advance(now.duration_since(last), cpu, memory, network, app.config.speed);
                    last = now;
                    false
                }
                event = events.next() => {
                    let event = event.ok_or_else(|| io::Error::other("terminal input closed"))??;
                    let action = Action::from_event(&event);
                    let previous_fps = app.config.fps;
                    if action == Action::Quit { break; }
                    if action == Action::None { continue; }
                    if action == Action::Save && !app.help {
                        app = match app.config.save(config_path) { Ok(()) => app.with_notice(format!("Saved {}", config_path.display())), Err(error) => app.with_notice(format!("Could not save: {error}")) };
                    } else { app = app.apply(action); }
                    if previous_fps != app.config.fps {
                        frames = interval(period(app.config.fps));
                        frames.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    }
                    frames.reset_after(period(app.config.fps));
                    last = Instant::now();
                    true
                }
            };
            if dirty {
                terminal.autoresize()?;
                scene.sync_before_draw(terminal);
                terminal.draw(|frame| scene.draw(frame, &app, logo, &engine, true))?;
            } else {
                scene.animate(terminal.backend_mut(), &app, logo, &engine)?;
            }
        }
        Ok(())
    }.await;
    drop(samples);
    worker.await.map_err(io::Error::other)?;
    result
}

fn period(fps: u16) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(fps.max(1)))
}

fn logo_inputs(app: &App, elapsed: Duration) -> (Option<f32>, f32, Option<u64>) {
    if !app.demo {
        return (
            app.snapshot.cpu,
            app.snapshot.memory_percent(),
            app.snapshot.telemetry.network_down,
        );
    }
    // Demo drives only the logo, never the measured dashboard or its histories.
    let t = elapsed.as_secs_f32();
    let cpu = 50.0 + (t * 0.65).sin() * 47.0;
    let memory = 50.0 + (t * 0.3).sin() * 43.0;
    let network = (((t * 0.9).sin() * 0.5 + 0.5).powi(3) * (16 << 20) as f32) as u64;
    (Some(cpu), memory, Some(network))
}
