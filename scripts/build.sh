#!/bin/sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$app_dir"
target=${CARGO_BUILD_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}
case "$target" in
    x86_64-unknown-linux-gnu|aarch64-apple-darwin) ;;
    *) printf '%s\n' "Unsupported release target: $target" >&2; exit 1 ;;
esac
# Keep compiler-generated panic/source paths from exposing the builder's home.
separator=$(printf '\037')
release_flags=${CARGO_ENCODED_RUSTFLAGS:-}
if [ -z "$release_flags" ] && [ -n "${RUSTFLAGS:-}" ]; then
    set -f
    for flag in $RUSTFLAGS; do release_flags="${release_flags}${release_flags:+$separator}$flag"; done
    set +f
fi
if [ -n "${HOME:-}" ]; then
    release_flags="${release_flags}${release_flags:+$separator}--remap-path-prefix=$HOME=/build/home"
fi
release_flags="${release_flags}${release_flags:+$separator}--remap-path-prefix=$app_dir=/build/codex-mini"
if [ -n "${CARGO_HOME:-}" ]; then release_flags="$release_flags$separator--remap-path-prefix=$CARGO_HOME=/build/cargo"; fi
if [ -n "${RUSTUP_HOME:-}" ]; then release_flags="$release_flags$separator--remap-path-prefix=$RUSTUP_HOME=/build/rustup"; fi
export CARGO_ENCODED_RUSTFLAGS="$release_flags"
cargo build --release --locked --target "$target" "$@"
mkdir -p dist
install -m 755 "target/$target/release/codex-mini" dist/codex-mini.new
mv dist/codex-mini.new dist/codex-mini
