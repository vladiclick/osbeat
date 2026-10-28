# osbeat

**Your system, with a heartbeat.**

A live terminal monitor for macOS with the Apple logo in the middle. The logo
reacts to the system: CPU makes it breathe, memory fills it like liquid, and
network traffic sends a shimmer across it.

![osbeat living logo and live customization](assets/demo.gif)

*Real terminal output, recorded in demo mode: the logo effects are scripted, the
stats are live.*

## Install

macOS on Apple Silicon or Intel, in a truecolor terminal such as Ghostty,
iTerm2 or Terminal. No sudo, no account.

```sh
curl -fsSL https://raw.githubusercontent.com/vladiclick/osbeat/main/install.sh | sh
```

The installer downloads the latest release for your Mac, checks its SHA-256,
and puts `osbeat` in `~/.local/bin`. Use `--bin-dir PATH` to choose another
directory or `--version v0.2.0` to pin a release.

From source, with Rust 1.95+:

```sh
cargo install --git https://github.com/vladiclick/osbeat --locked
```

or from a checkout: `sh scripts/install_local.sh`.

Then run:

```sh
osbeat
```

## Use

| Key | Action |
| --- | --- |
| `Tab` | Living canvas / detailed dashboard |
| `c` | Live customization panel |
| `t` | Cycle themes: default, Catppuccin, Tokyo Night, Gruvbox, Phosphor |
| `e` | Cycle effects: Aurora, Liquid, Spectrum, Original |
| `g` | Solid / ASCII / dot-matrix logo |
| `a` or `Space` | Pause / resume motion |
| `m` | Demo mode for the logo; stats stay real |
| `s` | Save preferences |
| `d` | Show osbeat's own CPU and memory |
| `?` | Help |
| `q` or `Ctrl-C` | Quit and restore the terminal |

```sh
osbeat --demo --effect liquid             # show off the logo effects
osbeat --theme tokyonight --dashboard
osbeat --init-config                      # write a default config, never overwrites
osbeat --config ./preferences.toml
```

Preferences live in `~/.config/osbeat/config.toml` (or
`$XDG_CONFIG_HOME/osbeat/config.toml`). The customization panel previews every
change live and `s` saves it. See [the example config](assets/config.example.toml).
Files are validated strictly, so typos and out-of-range values are errors.

## How the logo works

- **CPU → breathing.** Utilization sets the pulse speed.
- **RAM → liquid.** Used memory sets the fill height, with a moving waterline.
- **Network → shimmer.** Download rate sets the speed of a wave, on a log scale
  so both light browsing and fast downloads are visible.
- Values ease smoothly over time. Stats are sampled every 500 ms; the logo
  animates separately at 30 fps.

The Apple silhouette is a 128×128 coverage mask rendered with antialiased
half-block cells (or braille dots). The engine only does color math on a mask,
so any logo works: point `logo` in your config at a TOML asset with the same
`name`, `colors`, `art` and optional `mask` fields as
[the bundled one](assets/logos/macos.toml).

The dashboard shows per-core CPU, load averages, RAM, swap, network graphs,
disk space, battery and uptime. Missing readings stay blank, and nothing is
estimated: there are no temperature or GPU readings yet.

## Performance

Measured on an Apple M4, macOS 26.6.2, release build, 30 fps, 35 s per run.
CPU is percent of one core.

| Mode | CPU | Memory |
| --- | --- | --- |
| Canvas, 110×32 | 0.97% | 8.9 MiB |
| Canvas, 120×36 | 1.09% | 9.7 MiB |
| Dashboard, 110×32 | 0.97% | 9.0 MiB |
| Canvas + diagnostics, 160×48 | 1.26% | 11.9 MiB |

Animation frames only repaint logo cells that changed. The first frame appears
in under 15 ms.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                                   # 82 tests
python3 -m unittest tests/test_install.py   # installer and packaging
python3 scripts/terminal_check.py --measure 35
```

Releases: `sh scripts/build_release.sh` builds the Apple Silicon and Intel
archives and `SHA256SUMS` into `dist/`; upload those to a GitHub release named
after the version in `Cargo.toml`.

```text
src/
  animation/   logo-independent color fields
  collectors/  500 ms CPU/network worker, 10 s disk/battery worker
  config/      strict TOML, atomic saves, themes
  logo/        logo assets → colored character masks
  ui/          canvas, dashboard, live customization
  runtime.rs   separate animation, collection and input clocks
```

## Status

Works and is tested on macOS. Not done yet: process and event views,
crash/network-drop reactions, temperatures, GPU and power draw, and a Homebrew
formula.

## Credits and license

MIT. The ASCII logo art comes from [fastfetch](https://github.com/fastfetch-cli/fastfetch)
(MIT, revision `2f68ce7f7467ddbaaf31519c1d41455781d48492`); fastfetch is not run
at runtime. The detailed Apple silhouette is from
[Simple Icons](https://github.com/simple-icons/simple-icons/blob/90646e4bf7f3b89df6c37e65baf3e7bc759a517c/icons/apple.svg)
(CC0 1.0). Full notices are in [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES).
