use crate::config::{Config, LayoutMode, ThemeName};
use std::{io, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum Options {
    Help,
    Version,
    InitConfig { path: Option<PathBuf> },
    Run(Launch),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Launch {
    pub diagnostics: bool,
    pub demo: bool,
    pub config: Option<PathBuf>,
    pub theme: Option<ThemeName>,
    pub effect: Option<String>,
    pub layout: Option<LayoutMode>,
    pub reduced_motion: bool,
}

impl Launch {
    pub fn apply(&self, config: Config) -> io::Result<Config> {
        let result = Config {
            theme: self.theme.unwrap_or(config.theme),
            effect: self.effect.clone().unwrap_or(config.effect),
            layout: self.layout.unwrap_or(config.layout),
            reduced_motion: self.reduced_motion || config.reduced_motion,
            ..config
        };
        result.validate()?;
        Ok(result)
    }
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> io::Result<Self> {
        let mut launch = Launch::default();
        let mut args = args.into_iter();
        let mut init = false;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--help" | "-h" => {
                    return if args.next().is_none() && launch == Launch::default() && !init {
                        Ok(Self::Help)
                    } else {
                        Err(invalid("--help must be used alone"))
                    };
                }
                "--version" | "-V" => {
                    return if args.next().is_none() && launch == Launch::default() && !init {
                        Ok(Self::Version)
                    } else {
                        Err(invalid("--version must be used alone"))
                    };
                }
                "--debug" => launch.diagnostics = true,
                "--demo" => launch.demo = true,
                "--no-animation" => launch.reduced_motion = true,
                "--focus" => launch.layout = Some(LayoutMode::Focus),
                "--dashboard" => launch.layout = Some(LayoutMode::Dashboard),
                "--config" => launch.config = Some(PathBuf::from(value(&mut args, &flag)?)),
                "--theme" => {
                    launch.theme = Some(match value(&mut args, &flag)?.as_str() {
                        "default" => ThemeName::Default,
                        "catppuccin" => ThemeName::Catppuccin,
                        "tokyonight" => ThemeName::Tokyonight,
                        "gruvbox" => ThemeName::Gruvbox,
                        "phosphor" => ThemeName::Phosphor,
                        _ => return Err(invalid("unknown theme")),
                    })
                }
                "--effect" => {
                    let effect = value(&mut args, &flag)?;
                    if !["aurora", "liquid", "spectrum", "original"].contains(&effect.as_str()) {
                        return Err(invalid("unknown logo effect"));
                    }
                    launch.effect = Some(effect);
                }
                "--init-config" => init = true,
                _ => return Err(invalid("unrecognized arguments; use osbeat --help")),
            }
        }
        if init {
            if launch.diagnostics
                || launch.demo
                || launch.theme.is_some()
                || launch.effect.is_some()
                || launch.layout.is_some()
                || launch.reduced_motion
            {
                return Err(invalid("--init-config supports only --config PATH"));
            }
            Ok(Self::InitConfig {
                path: launch.config,
            })
        } else {
            Ok(Self::Run(launch))
        }
    }
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> io::Result<String> {
    args.next()
        .filter(|value| !value.is_empty() && !value.starts_with("--"))
        .ok_or_else(|| invalid(&format!("{flag} needs a value")))
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub const HELP: &str = "osbeat — your system, with a heartbeat.

Usage: osbeat [OPTIONS]

  --focus / --dashboard  Living canvas (default) or detailed dashboard
  --theme NAME           default, catppuccin, tokyonight, gruvbox, phosphor
  --effect NAME          aurora, liquid, spectrum, original
  --demo                 Demonstrate logo reactions; stats remain real
  --no-animation         Start with motion paused
  --config PATH          Read a specific TOML configuration
  --init-config          Create config without overwriting existing files
  --debug                Show osbeat's own CPU and resident memory
  -h, --help             Print help
  -V, --version          Print version

Keys: Tab view · c customize · t theme · e effect · g glyphs
      a pause · m demo · s save · ? help · d diagnostics · q quit
Collectors update every 500 ms; the logo animates independently at 30 fps.";
