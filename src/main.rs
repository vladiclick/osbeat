use std::io::{self, IsTerminal};

use crossterm::event::EventStream;
use futures_util::StreamExt;
use osbeat::{
    app::{Action, App},
    cli::{HELP, Options},
    collectors,
    logo::Logo,
    model::HostInfo,
    ui,
};
use ratatui::DefaultTerminal;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    match Options::parse(std::env::args().skip(1))? {
        Options::Help => {
            println!("{HELP}");
            Ok(())
        }
        Options::Version => {
            println!("osbeat {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Options::Run { diagnostics } => launch(diagnostics).await,
    }
}

async fn launch(diagnostics: bool) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "osbeat needs an interactive terminal; run it directly in Terminal, Ghostty, or iTerm",
        ));
    }
    let app = App::new(HostInfo::detect()).with_diagnostics(diagnostics);
    let logo = Logo::for_os(app.host.os)?;
    // Ratatui also installs a panic hook that restores the terminal before printing.
    // Arm cleanup BEFORE initialization, which can fail after enabling raw mode.
    let _restore = TerminalRestore;
    let mut terminal = ratatui::try_init()?;
    run(&mut terminal, app, &logo).await
}

struct TerminalRestore;

impl Drop for TerminalRestore {
    // RAII: leaving this scope restores raw mode and the normal screen on errors too.
    fn drop(&mut self) {
        if let Err(error) = ratatui::try_restore() {
            eprintln!("osbeat: could not fully restore the terminal: {error}");
        }
    }
}

async fn run(terminal: &mut DefaultTerminal, mut app: App, logo: &Logo) -> io::Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut events = EventStream::new();
    let (mut samples, worker) = collectors::start();
    let result = async {
        terminal.draw(|frame| ui::draw(frame, &app, logo))?;
        loop {
            tokio::select! {
                _ = terminate.recv() => break,
                _ = hangup.recv() => break,
                _ = interrupt.recv() => break,
                changed = samples.changed() => {
                    changed.map_err(|_| io::Error::other("system collector stopped unexpectedly"))?;
                    // Release the watch read lock before drawing or awaiting input.
                    let snapshot = samples.borrow_and_update().clone();
                    app = app.with_snapshot(snapshot);
                }
                event = events.next() => {
                    let event = event.ok_or_else(|| io::Error::other("terminal input closed"))??;
                    match Action::from_event(&event) {
                        Action::Quit => break,
                        Action::Diagnostics => app = app.with_diagnostics(!app.diagnostics),
                        Action::Redraw => {},
                        Action::None => continue,
                    }
                }
            }
            terminal.draw(|frame| ui::draw(frame, &app, logo))?;
        }
        Ok(())
    }
    .await;
    // Closing the receiver tells the blocking worker to stop within one sample.
    drop(samples);
    worker.await.map_err(io::Error::other)?;
    result
}
