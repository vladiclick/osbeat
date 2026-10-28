use std::io;

#[derive(Debug, PartialEq, Eq)]
pub enum Options {
    Help,
    Version,
    Run { diagnostics: bool },
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> io::Result<Self> {
        let args: Vec<String> = args.into_iter().collect();
        match args.as_slice() {
            [] => Ok(Self::Run { diagnostics: false }),
            [flag] if flag == "--debug" => Ok(Self::Run { diagnostics: true }),
            [flag] if flag == "--help" || flag == "-h" => Ok(Self::Help),
            [flag] if flag == "--version" || flag == "-V" => Ok(Self::Version),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unrecognized arguments; use osbeat --help",
            )),
        }
    }
}

pub const HELP: &str = "osbeat — your system, with a heartbeat.

Usage: osbeat [--debug]

  --debug       Show osbeat's own CPU and resident memory
  -h, --help    Print this help
  -V, --version Print version

Keys: q quit · d toggle diagnostics · Ctrl-C quit
Requires an interactive terminal. Samples every 500 ms.";
