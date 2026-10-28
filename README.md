# osbeat

**Your system, with a heartbeat.**

A quiet, logo-centered terminal system monitor. Built in Rust for macOS first.
The long-term goal: an OS logo that breathes, fills, shimmers, and reacts to
what your system is doing.

**Current checkpoint: Phase 1.** The logo is static. CPU and memory are live.

## Run

Requires Rust **1.95+**, macOS, and an interactive terminal with 256-color support.
Apple Silicon is the initial test platform.

```sh
cargo run --release
```

Or install this local checkout:

```sh
cargo install --path . --locked
osbeat
```

This project is not published to crates.io yet. No sudo or account is needed.

| Key | Action |
| --- | --- |
| `q` / `Ctrl-C` | Quit and restore the terminal |
| `d` | Toggle osbeat's CPU/RAM diagnostics |

`osbeat --debug` starts with diagnostics visible. `--help` and `--version`
also work outside an interactive terminal.

## What works

- Fastfetch's rainbow Apple ASCII logo, embedded as data with a per-character palette.
- Total CPU usage and RAM usage, with 30 seconds of history at 500 ms intervals.
- Uptime, OS, kernel, hostname, CPU model, logical core count, and architecture.
- Side-by-side layout at standard sizes; stacked or compact layouts when space is limited.
- Resize handling and restoration on quit, Ctrl-C, SIGINT, SIGTERM, SIGHUP, and panics.
- Self CPU (percent of **one logical core**) and resident memory diagnostics.

CPU displays “warming up” until two samples establish a valid counter delta.
RAM is the operating system's used-memory value reported by sysinfo, not memory
pressure. The self-CPU budget is **under 2% of one core**, with no multicore
normalization that could hide overhead.

## Architecture

```text
src/
  app.rs        immutable dashboard state and key actions
  cli.rs        command-line validation
  collectors/   persistent sysinfo collector on a blocking worker
  logo/         TOML assets → colored character cells
  model.rs      system snapshots, OS detection, bounded histories
  ui/           responsive layout, charts, diagnostics
assets/logos/   embedded fastfetch art and RGB palettes
tests/          core, collector integration, and real-PTY tests
scripts/        standard-library-only terminal checks and CPU measurement
```

The collector owns sysinfo's mutable counter cache and publishes new snapshots
through `tokio::sync::watch`. A slow renderer sees the latest value without
an unbounded queue. The UI never holds a channel lock across a draw or an await.
Only the current process is refreshed for diagnostics, not the entire process list.

Phase 1 draws on samples, keys that change the view, and resize events. Ratatui
diffs frames and writes changed cells. A later 30 fps animation clock will be
independent of collection; Phase 1's low CPU result does not validate that future engine.

## Verify

Python 3 is needed only for real-PTY integration tests and performance checks.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
python3 scripts/terminal_check.py --measure 35
python3 scripts/terminal_check.py --measure 35 --debug
```

The PTY test starts the binary, toggles diagnostics, resizes from 1×1 back to
110×32, tests keys and signals, and checks terminal restoration.
Performance checks warm up for 3 seconds and continuously drain terminal output.
They calculate process CPU-time delta / elapsed wall time and fail at 2% or above.
Terminal emulator rendering cost is outside the process's own CPU budget.

For measured coverage, install `cargo-llvm-cov` and the `llvm-tools-preview`
Rust component, then run:

```sh
cargo llvm-cov --all-targets --fail-under-lines 80
```

Latest local validation, **2026-10-28**: Apple M4, macOS 26.6.2, Rust 1.96.0,
110×32 truecolor PTY, release build. Each CPU run followed a 3-second warm-up
and lasted 35 seconds.

| Check | Result |
| --- | --- |
| Unit + integration + PTY tests | 14 passed |
| Rust line coverage (including CLI and real terminal paths) | 97.0% |
| Release CPU, diagnostics closed | 0.171% of one core |
| Release CPU, diagnostics open | 0.143% of one core |
| Resident memory, closed / open | 6.75 / 6.75 MiB |
| Formatting and all-target Clippy | Passed, warnings denied |
| RustSec dependency audit | No known vulnerabilities or warnings |

These are local measurements, not a guarantee for every host or terminal size.
The animation phase must pass the same budget again.

## Next checkpoints

1. **Current:** static logo, basic live stats, reliable macOS TUI.
2. Logo animation: pulse, liquid fill, temperature color, network shimmer.
3. Events ticker, process/network views, expanded keybindings.
4. Linux distro logos, TOML configuration, selectable themes.
5. Apple Silicon GPU, power, and temperatures without sudo.
6. Releases, package managers, CI packaging, and a demo GIF.

Animation, P/E-core breakdowns, GPU/temperature readings, network statistics,
process management, event tracking, user configuration, and themes are not
implemented at this checkpoint. Other platforms currently use a generic
fastfetch fallback logo; Linux validation belongs to Phase 4.

## Credits and license

MIT. ASCII logo art comes from [fastfetch](https://github.com/fastfetch-cli/fastfetch),
also MIT licensed. The art is pinned to revision
`2f68ce7f7467ddbaaf31519c1d41455781d48492` and kept unmodified; the original
color-slot mapping uses a muted RGB palette here. See
[THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES) for the upstream source paths,
copyright notices, and full license. osbeat does not invoke fastfetch at runtime.
