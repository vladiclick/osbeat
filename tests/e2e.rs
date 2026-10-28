#[test]
fn real_terminal_lifecycle() {
    let output = std::process::Command::new("python3")
        .args([
            "scripts/terminal_check.py",
            "--binary",
            env!("CARGO_BIN_EXE_osbeat"),
        ])
        .output()
        .expect("Python 3 is required for the PTY smoke test");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
