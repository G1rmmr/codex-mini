#!/bin/sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$app_dir/dist/codex-mini" ]; then
    exec "$app_dir/dist/codex-mini" "$@"
fi
if [ -x "$app_dir/target/release/codex-mini" ]; then
    exec "$app_dir/target/release/codex-mini" "$@"
fi
printf '%s\n' 'Build first: cargo build --release --locked' >&2
exit 1
