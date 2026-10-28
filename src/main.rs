mod runtime;
use osbeat::{
    app::App,
    cli::{HELP, Launch, Options},
    config,
    logo::Logo,
    model::HostInfo,
};
use std::{
    io::{self, IsTerminal},
    path::Path,
};

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
        Options::InitConfig { path } => {
            let path = path.map_or_else(config::default_path, Ok)?;
            config::Config::init(&path)?;
            println!("Created {}", path.display());
            Ok(())
        }
        Options::Run(launch) => launch_app(launch).await,
    }
}

async fn launch_app(launch: Launch) -> io::Result<()> {
    let path = launch
        .config
        .clone()
        .map_or_else(config::default_path, Ok)?;
    let loaded = if launch.config.is_some() {
        config::load_required(&path)?
    } else {
        config::load(&path)?
    };
    let settings = launch.apply(loaded)?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "osbeat needs an interactive terminal; run it directly in Terminal, Ghostty, or iTerm",
        ));
    }
    let mut app = App::new(HostInfo::detect())
        .with_config(settings)
        .with_diagnostics(launch.diagnostics);
    app.demo = launch.demo;
    let logo = match &app.config.logo {
        Some(custom) => {
            let resolved = if custom.is_absolute() {
                custom.clone()
            } else {
                path.parent().unwrap_or(Path::new(".")).join(custom)
            };
            Logo::load(&resolved)?
        }
        None => Logo::for_os(app.host.os)?,
    };
    // Arm cleanup before initialization, which can fail after raw mode is enabled.
    let _restore = TerminalRestore;
    let mut terminal = runtime::init_terminal()?;
    runtime::run(&mut terminal, app, &logo, &path).await
}

struct TerminalRestore;
impl Drop for TerminalRestore {
    fn drop(&mut self) {
        if let Err(error) = ratatui::try_restore() {
            eprintln!("osbeat: could not fully restore the terminal: {error}");
        }
    }
}
