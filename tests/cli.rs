use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/cli-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(args: &[&str], root: &Scratch) -> Output {
    Command::new(env!("CARGO_BIN_EXE_osbeat"))
        .args(args)
        .env("XDG_CONFIG_HOME", &root.0)
        .output()
        .unwrap()
}
#[test]
fn explicit_config_missing_or_invalid_fails_before_terminal_setup() {
    let root = Scratch::new();
    let missing = root.0.join("typo.toml");
    let output = run(&["--config", missing.to_str().unwrap()], &root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("typo.toml"));
    assert!(!missing.exists());
    fs::write(&missing, "fps = 1").unwrap();
    let output = run(&["--config", missing.to_str().unwrap()], &root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("fps"));
    assert!(!output.stdout.contains(&27));
}
#[test]
fn initialization_is_explicit_and_never_overwrites() {
    let root = Scratch::new();
    let path = root.0.join("prefs.toml");
    let args = ["--init-config", "--config", path.to_str().unwrap()];
    assert!(run(&args, &root).status.success());
    let original = fs::read(&path).unwrap();
    assert!(!run(&args, &root).status.success());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!run(&["--init-config", "--debug"], &root).status.success());
}
#[test]
fn cli_overrides_validate_and_implicit_missing_config_stays_read_only() {
    use osbeat::cli::{Launch, Options};
    use osbeat::config::{Config, LayoutMode, ThemeName};
    let parse = |args: &[&str]| Options::parse(args.iter().map(|arg| arg.to_string()));
    let Options::Run(launch) = parse(&[
        "--theme",
        "phosphor",
        "--effect",
        "liquid",
        "--dashboard",
        "--demo",
        "--no-animation",
        "--debug",
    ])
    .unwrap() else {
        panic!("run expected")
    };
    let config = launch.apply(Config::default()).unwrap();
    assert_eq!(config.theme, ThemeName::Phosphor);
    assert_eq!(config.effect, "liquid");
    assert_eq!(config.layout, LayoutMode::Dashboard);
    assert!(config.reduced_motion && launch.demo && launch.diagnostics);
    for args in [
        &["--effect", "missing"][..],
        &["--theme", "missing"],
        &["--config"],
        &["--theme", "--demo"],
    ] {
        assert!(parse(args).is_err());
    }
    assert!(
        Launch {
            effect: Some("bad".into()),
            ..Default::default()
        }
        .apply(Config::default())
        .is_err()
    );
    let root = Scratch::new();
    assert!(!run(&[], &root).status.success());
    assert!(!root.0.join("osbeat/config.toml").exists());
}
