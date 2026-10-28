#!/bin/sh
# Download only a verified release. No sudo, shell-profile edits, or remote code execution.
set -eu

fail() { printf 'osbeat: %s\n' "$*" >&2; exit 1; }
usage() {
    cat <<'EOF'
Install osbeat from a GitHub release (macOS, Apple Silicon or Intel).

  sh install.sh [--version v0.2.0] [--bin-dir PATH]
  sh install.sh --archive FILE --checksum SHA256SUMS [--bin-dir PATH]

--repo       GitHub repository (default: vladiclick/osbeat, or OSBEAT_REPO).
--version    Release tag, or latest (default).
--bin-dir    Destination directory (default: ~/.local/bin).
--prefix     Install under PREFIX/bin instead.
--archive    Offline release archive; requires --checksum.
--checksum   Local SHA256SUMS file (the archive's exact basename must occur once).
--help       Show this help.
EOF
}
repo=${OSBEAT_REPO:-vladiclick/osbeat}
version=latest
bin_dir=${HOME:?HOME must be set}/.local/bin
archive=
checksum=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --help|-h) usage; exit 0 ;;
        --repo|--version|--bin-dir|--prefix|--archive|--checksum)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail "Missing value for $1"
            case "$1" in
                --repo) repo=$2 ;;
                --version) version=$2 ;;
                --bin-dir) bin_dir=$2 ;;
                --prefix) bin_dir=$2/bin ;;
                --archive) archive=$2 ;;
                --checksum) checksum=$2 ;;
            esac
            shift 2 ;;
        *) fail "Unknown argument: $1 (see --help)" ;;
    esac
done
case "$bin_dir" in /*) ;; *) fail '--bin-dir / --prefix must be absolute paths' ;; esac

# Reject untrusted URL components before any request.
case "$repo$version" in
    *'
'*|*'
'*) fail 'Repository and version must each be one line' ;;
esac
if [ -n "$repo" ]; then
    printf '%s\n' "$repo" | LC_ALL=C grep -Eq '^[A-Za-z0-9][A-Za-z0-9_-]*/[A-Za-z0-9][A-Za-z0-9_.-]*$' || fail 'Use --repo OWNER/REPO'
    case "$repo" in */.|*/..) fail 'Invalid repository' ;; esac
fi
validate_version() {
    printf '%s\n' "$1" | LC_ALL=C grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9]+([.-][A-Za-z0-9]+)*)?$' || fail 'Version must be a release tag such as v0.2.0'
}
[ "$version" = latest ] || validate_version "$version"
if [ -n "$archive" ] || [ -n "$checksum" ]; then
    [ -n "$archive" ] && [ -n "$checksum" ] || fail '--archive and --checksum must be supplied together'
    [ -f "$archive" ] && [ -f "$checksum" ] || fail 'Archive and checksum must be readable regular files'
else
    [ -n "$repo" ] || fail 'Empty repository: use --repo OWNER/REPO'
fi

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/osbeat-install.XXXXXX")
staged=
cleanup() {
    [ -z "$staged" ] || rm -f "$staged"
    rm -rf "$work_dir"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

if [ -z "$archive" ]; then
    command -v curl >/dev/null 2>&1 || fail 'curl is required for downloads'
    [ "$(uname -s)" = Darwin ] || fail 'osbeat runs on macOS only'
    case "$(uname -m)" in
        x86_64|amd64) arch=x86_64 ;;
        aarch64|arm64) arch=aarch64 ;;
        *) fail 'Supported Macs: Apple Silicon (arm64) and Intel (x86_64)' ;;
    esac
    if [ "$version" = latest ]; then
        resolved=$(curl --fail --silent --show-error --location --head --proto '=https' --proto-redir '=https' \
            --connect-timeout 15 --max-time 60 --output /dev/null --write-out '%{url_effective}' \
            "https://github.com/$repo/releases/latest") || fail 'Could not resolve the latest release'
        case "$resolved" in "https://github.com/$repo/releases/tag/"*) version=${resolved##*/} ;;
            *) fail 'The repository has no matching latest release' ;;
        esac
        validate_version "$version"
    fi
    asset="osbeat-$version-$arch-apple-darwin.tar.gz"
    archive=$work_dir/$asset
    checksum=$work_dir/SHA256SUMS
    base_url="https://github.com/$repo/releases/download/$version"
    for name in "$asset" SHA256SUMS; do
        curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
            --connect-timeout 15 --max-time 300 --output "$work_dir/$name" "$base_url/$name" \
            || fail "Could not download $name"
    done
fi
asset=${archive##*/}
# Only accept one exact filename, with no extra fields. Never hand an untrusted
# checksum manifest to a tool that might read arbitrary paths from it.
expected=$(awk -v name="$asset" '$2 == name { if (NF != 2 || length($1) != 64 || $1 ~ /[^0-9a-fA-F]/) bad=1; hash=tolower($1); count++ } END { if (count != 1 || bad) exit 1; print hash }' "$checksum") \
    || fail 'Checksum manifest must contain one valid SHA256 entry for this archive'
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$archive" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$archive" | awk '{print $1}')
else
    fail 'SHA256 verification requires sha256sum or shasum'
fi
[ "$actual" = "$expected" ] || fail 'SHA256 mismatch; the existing installation was preserved'

# Accept only the three flat, regular files produced by package_release.sh.
# Extract the binary to stdout rather than letting tar create archive-controlled paths.
tar -tzf "$archive" > "$work_dir/members" || fail 'Invalid release archive'
LC_ALL=C sort "$work_dir/members" > "$work_dir/sorted"
printf 'LICENSE\nTHIRD_PARTY_NOTICES\nosbeat\n' > "$work_dir/expected"
cmp -s "$work_dir/sorted" "$work_dir/expected" || fail 'Unexpected, missing, or duplicate archive member'
tar -tvzf "$archive" > "$work_dir/types" || fail 'Could not inspect archive members'
awk 'substr($0, 1, 1) != "-" { exit 1 } END { if (NR != 3) exit 1 }' "$work_dir/types" || fail 'Archive members must be regular files (no links)'
[ ! -d "$bin_dir/osbeat" ] || fail 'The destination osbeat is a directory'
mkdir -p "$bin_dir" || fail 'Cannot create installation directory'
staged=$(mktemp "$bin_dir/.osbeat.XXXXXX")
tar -xzOf "$archive" osbeat > "$staged" || fail 'Could not extract the verified binary'
[ -s "$staged" ] || fail 'The release binary is empty'
chmod 755 "$staged"
mv -f "$staged" "$bin_dir/osbeat" || fail 'Could not replace the installation'
staged=
printf 'Installed osbeat to %s/osbeat\n' "$bin_dir"
case ":${PATH:-}:" in
    *":$bin_dir:"*) printf 'Run: osbeat\n' ;;
    *) printf 'Run now: "%s/osbeat"\nAdd this directory to your PATH: %s\n' "$bin_dir" "$bin_dir" ;;
esac
