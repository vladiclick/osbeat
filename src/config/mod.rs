mod themes;

pub use themes::{Palette, ThemeName};

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_CONFIG_BYTES: u64 = 128 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: ThemeName,
    pub effect: String,
    pub glyphs: GlyphMode,
    pub fps: u16,
    pub speed: f32,
    pub intensity: f32,
    pub reduced_motion: bool,
    pub layout: LayoutMode,
    pub panels: Panels,
    pub logo: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::Default,
            effect: "aurora".into(),
            glyphs: GlyphMode::Solid,
            fps: 30,
            speed: 1.0,
            intensity: 0.85,
            reduced_motion: false,
            layout: LayoutMode::Focus,
            panels: Panels::default(),
            logo: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    Dashboard,
    #[default]
    Focus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlyphMode {
    #[default]
    Solid,
    Ascii,
    Dots,
}

impl GlyphMode {
    pub fn next(self) -> Self {
        match self {
            Self::Solid => Self::Ascii,
            Self::Ascii => Self::Dots,
            Self::Dots => Self::Solid,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Ascii => "ascii",
            Self::Dots => "dots",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Panels {
    pub cpu: bool,
    pub memory: bool,
    pub network: bool,
    pub storage: bool,
    pub power: bool,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            network: true,
            storage: true,
            power: true,
        }
    }
}

/// Resolve paths separately from the environment so tests never change process-wide state.
fn resolve_path(xdg: Option<&Path>, home: Option<&Path>) -> io::Result<PathBuf> {
    if let Some(root) = xdg.filter(|root| root.is_absolute()) {
        return Ok(root.join("osbeat/config.toml"));
    }
    home.filter(|root| root.is_absolute())
        .map(|root| root.join(".config/osbeat/config.toml"))
        .ok_or_else(|| invalid("Cannot locate config: set an absolute XDG_CONFIG_HOME or HOME"))
}

pub fn default_path() -> io::Result<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    resolve_path(xdg.as_deref(), home.as_deref())
}

/// Loading is read-only; a first launch does not create files in the user's home.
pub fn load(path: &Path) -> io::Result<Config> {
    read_config(path, true)
}

/// An explicitly selected config must exist so a path typo never silently changes behavior.
pub fn load_required(path: &Path) -> io::Result<Config> {
    read_config(path, false)
}

fn read_config(path: &Path, allow_missing: bool) -> io::Result<Config> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if allow_missing && error.kind() == io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(error) => return Err(context(path, error)),
    };
    let mut source = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut source)
        .map_err(|error| context(path, error))?;
    if source.len() as u64 > MAX_CONFIG_BYTES {
        return Err(context(path, invalid("config must be at most 128 KiB")));
    }
    let config: Config =
        toml::from_str(&source).map_err(|error| context(path, invalid(error.to_string())))?;
    config.validate().map_err(|error| context(path, error))?;
    Ok(config)
}

impl Config {
    pub fn validate(&self) -> io::Result<()> {
        if !["aurora", "liquid", "spectrum", "original"].contains(&self.effect.as_str()) {
            return Err(invalid(
                "effect must be aurora, liquid, spectrum, or original",
            ));
        }
        if !(10..=60).contains(&self.fps) {
            return Err(invalid("fps must be between 10 and 60"));
        }
        if !self.speed.is_finite() || !(0.25..=3.0).contains(&self.speed) {
            return Err(invalid("speed must be a finite number between 0.25 and 3"));
        }
        if !self.intensity.is_finite() || !(0.0..=1.0).contains(&self.intensity) {
            return Err(invalid("intensity must be a finite number between 0 and 1"));
        }
        if self
            .logo
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            return Err(invalid("logo path must not be empty"));
        }
        Ok(())
    }

    /// Called only for an explicit --init-config request. create_new prevents overwrites.
    pub fn init(path: &Path) -> io::Result<()> {
        let contents = Config::default().document()?;
        prepare_parent(path)?;
        let mut file = private_file(path).map_err(|error| context(path, error))?;
        file.write_all(contents.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| context(path, error))
    }

    /// Save only after an explicit user action; rename publishes the complete document atomically.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let contents = self.document()?;
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(context(
                path,
                invalid("refusing to replace a symbolic-link config"),
            ));
        }
        prepare_parent(path)?;
        let (temporary, mut file) = temporary_file(parent(path))?;
        let result = file
            .write_all(contents.as_bytes())
            .and_then(|()| file.sync_all())
            .and_then(|()| fs::rename(&temporary, path));
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|error| context(path, error))
    }

    fn document(&self) -> io::Result<String> {
        self.validate()?;
        let body = toml::to_string_pretty(self).map_err(|error| invalid(error.to_string()))?;
        Ok(format!(
            "# osbeat — your system, with a heartbeat.\n# Save in the app with s. Edit this TOML and restart to apply changes.\n# Themes: default, catppuccin, tokyonight, gruvbox, phosphor\n# Effects: aurora, liquid, spectrum, original\n# Optional custom logo: logo = \"/absolute/path/logo.toml\"\n\n{body}"
        ))
    }
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn prepare_parent(path: &Path) -> io::Result<()> {
    fs::create_dir_all(parent(path)).map_err(|error| context(path, error))
}

fn private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    options.open(path)
}

fn temporary_file(directory: &Path) -> io::Result<(PathBuf, File)> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let sequence = SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(".osbeat-{}-{sequence}.tmp", std::process::id()));
        match private_file(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(context(&path, error)),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "cannot create temporary config file",
    ))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn context(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn scratch() -> PathBuf {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-configs")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn resolves_xdg_before_home_without_environment_mutation() {
        assert_eq!(
            resolve_path(
                Some(Path::new("/custom/config")),
                Some(Path::new("/home/me"))
            )
            .unwrap(),
            Path::new("/custom/config/osbeat/config.toml")
        );
        assert_eq!(
            resolve_path(None, Some(Path::new("/home/me"))).unwrap(),
            Path::new("/home/me/.config/osbeat/config.toml")
        );
        assert!(resolve_path(None, None).is_err());
        assert!(resolve_path(Some(Path::new("relative")), None).is_err());
    }

    #[test]
    fn missing_files_use_defaults_without_writing() {
        let dir = scratch();
        let path = dir.join("missing.toml");
        assert_eq!(load(&path).unwrap(), Config::default());
        assert!(!path.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn required_missing_file_fails_without_writing() {
        let dir = scratch();
        let path = dir.join("explicit.toml");
        let error = load_required(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("explicit.toml"));
        assert!(!path.exists());
        Config::init(&path).unwrap();
        assert_eq!(load_required(&path).unwrap(), Config::default());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn minimal_config_and_roundtrip() {
        let config: Config =
            toml::from_str("theme = 'gruvbox'\n[panels]\nnetwork = false").unwrap();
        assert_eq!(config.theme, ThemeName::Gruvbox);
        assert!(!config.panels.network);
        assert!(config.panels.cpu);
        assert_eq!(config.fps, 30);
        assert_eq!(
            toml::from_str::<Config>(&toml::to_string_pretty(&config).unwrap()).unwrap(),
            config
        );
    }

    #[test]
    fn rejects_unknown_and_invalid_values() {
        for invalid in [
            "theem = 'gruvbox'",
            "theme = 'missing'",
            "[panels]\nnetwrok = false",
            "fps = 9",
            "fps = 61",
            "speed = 0.2",
            "speed = 3.1",
            "speed = nan",
            "intensity = 1.1",
            "intensity = -0.1",
            "intensity = inf",
            "effect = 'missing'",
            "glyphs = 'missing'",
        ] {
            let parsed = toml::from_str::<Config>(invalid);
            assert!(
                parsed.is_err() || parsed.unwrap().validate().is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn invalid_file_error_names_path_and_preserves_file() {
        let dir = scratch();
        let path = dir.join("invalid.toml");
        fs::write(&path, "fps = 1").unwrap();
        let error = load(&path).unwrap_err().to_string();
        assert!(error.contains("invalid.toml"));
        assert!(error.contains("fps"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "fps = 1");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn initialization_never_overwrites_and_save_roundtrips() {
        let dir = scratch();
        let path = dir.join("nested/config.toml");
        Config::init(&path).unwrap();
        assert!(Config::init(&path).is_err());
        let config = Config {
            theme: ThemeName::Phosphor,
            effect: "liquid".into(),
            ..Config::default()
        };
        config.save(&path).unwrap();
        assert_eq!(load(&path).unwrap(), config);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        let invalid = Config {
            fps: 0,
            ..config.clone()
        };
        assert!(invalid.save(&path).is_err());
        assert_eq!(load(&path).unwrap(), config);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_rejects_symlink_targets() {
        let dir = scratch();
        let original = dir.join("original.toml");
        Config::init(&original).unwrap();
        let link = dir.join("symlink.toml");
        std::os::unix::fs::symlink(&original, &link).unwrap();
        assert!(Config::default().save(&link).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn glyph_cycle_visits_all_styles() {
        let solid = GlyphMode::default();
        assert_eq!(solid.label(), "solid");
        assert_eq!(solid.next().label(), "ascii");
        assert_eq!(solid.next().next().label(), "dots");
        assert_eq!(solid.next().next().next(), solid);
        assert_eq!(Config::default().glyphs, GlyphMode::Solid);
    }

    #[test]
    fn theme_cycle_visits_each_theme() {
        let mut theme = ThemeName::Default;
        let mut labels = Vec::new();
        for _ in 0..5 {
            labels.push(theme.label());
            let palette = theme.palette();
            assert_ne!(palette.bg, palette.fg);
            theme = theme.next();
        }
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), 5);
        assert_eq!(theme, ThemeName::Default);
    }
}
