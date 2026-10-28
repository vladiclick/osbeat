#!/bin/sh
# Build this checkout and install the `osbeat` command with the locked dependencies.
set -eu

fail() { printf 'osbeat: %s\n' "$*" >&2; exit 1; }
root=$(cd "$(dirname "$0")/.." && pwd)
prefix=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail 'Missing value for --prefix'
            prefix=$2
            shift 2 ;;
        --help|-h)
            printf 'Usage: sh scripts/install_local.sh [--prefix PATH]\n'
            printf 'Installs to ~/.cargo/bin, or PREFIX/bin with --prefix.\n'
            exit 0 ;;
        *) fail "Unknown argument: $1" ;;
    esac
done
[ "$(uname -s)" = Darwin ] || fail 'osbeat runs on macOS only'
command -v cargo >/dev/null 2>&1 || fail 'Rust is required: https://rustup.rs'

if [ -n "$prefix" ]; then
    cargo install --locked --path "$root" --root "$prefix" --force
    bin_dir=$prefix/bin
else
    cargo install --locked --path "$root" --force
    bin_dir=${CARGO_HOME:-$HOME/.cargo}/bin
fi
case ":${PATH:-}:" in
    *":$bin_dir:"*) printf 'Run: osbeat\n' ;;
    *) printf 'Installed to %s; add it to your PATH, then run: osbeat\n' "$bin_dir" ;;
esac
