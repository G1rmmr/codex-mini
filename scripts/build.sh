#!/bin/sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$app_dir"
cargo build --release --locked "$@"
mkdir -p dist
install -m 755 target/release/codex-mini dist/codex-mini.new
mv dist/codex-mini.new dist/codex-mini
