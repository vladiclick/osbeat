#!/bin/sh
# Build and package both macOS release archives plus SHA256SUMS into dist/.
# Local paths are remapped so the binaries carry no home-directory names.
set -eu

fail() { printf 'osbeat: %s\n' "$*" >&2; exit 1; }
root=$(cd "$(dirname "$0")/.." && pwd)
[ "$(uname -s)" = Darwin ] || fail 'Release builds run on macOS only'
version=v$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
cargo_home=${CARGO_HOME:-$HOME/.cargo}
rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
export RUSTFLAGS="--remap-path-prefix=$cargo_home/registry/src=/cargo --remap-path-prefix=$rustup_home=/rustup --remap-path-prefix=$root=/osbeat"

rm -rf "$root/dist"
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    (cd "$root" && cargo build --release --locked --target "$target")
    if strings -a "$root/target/$target/release/osbeat" | grep -q "$HOME"; then
        fail "$target binary still contains $HOME"
    fi
    sh "$root/scripts/package_release.sh" --target "$target" --version "$version" --out-dir "$root/dist"
done
cat "$root"/dist/*.sha256 > "$root/dist/SHA256SUMS"
cat "$root/dist/SHA256SUMS"
