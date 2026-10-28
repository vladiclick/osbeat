#!/bin/sh
# Package a release build as the archive install.sh accepts: three flat regular files
# (osbeat, LICENSE, THIRD_PARTY_NOTICES) plus a one-line SHA256 file.
set -eu

fail() { printf 'osbeat: %s\n' "$*" >&2; exit 1; }
root=$(cd "$(dirname "$0")/.." && pwd)
target=
version=
binary=
out_dir=$root/dist
while [ "$#" -gt 0 ]; do
    case "$1" in
        --target|--version|--binary|--out-dir)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail "Missing value for $1"
            case "$1" in
                --target) target=$2 ;;
                --version) version=$2 ;;
                --binary) binary=$2 ;;
                --out-dir) out_dir=$2 ;;
            esac
            shift 2 ;;
        *) fail "Unknown argument: $1" ;;
    esac
done
case "$target" in
    aarch64-apple-darwin|x86_64-apple-darwin) ;;
    *) fail '--target must be aarch64-apple-darwin or x86_64-apple-darwin' ;;
esac
crate_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
[ "$version" = "v$crate_version" ] || fail "--version must be v$crate_version to match Cargo.toml"
binary=${binary:-$root/target/$target/release/osbeat}
[ -f "$binary" ] || fail "No binary at $binary"

archive_name=osbeat-$version-$target.tar.gz
stage=$(mktemp -d "${TMPDIR:-/tmp}/osbeat-package.XXXXXX")
trap 'rm -rf "$stage"' EXIT
cp "$binary" "$stage/osbeat"
cp "$root/LICENSE" "$root/THIRD_PARTY_NOTICES" "$stage/"
chmod 755 "$stage/osbeat"
chmod 644 "$stage/LICENSE" "$stage/THIRD_PARTY_NOTICES"
mkdir -p "$out_dir"
# No AppleDouble files, extended attributes or local user names in the archive.
COPYFILE_DISABLE=1 tar --no-mac-metadata --no-xattrs --uid 0 --gid 0 --uname root --gname wheel \
    -czf "$out_dir/$archive_name" -C "$stage" osbeat LICENSE THIRD_PARTY_NOTICES
(cd "$out_dir" && shasum -a 256 "$archive_name" > "$archive_name.sha256")
printf '%s\n' "$out_dir/$archive_name"
