#!/bin/sh
# Optional. Pinned OSS release. No global hook installation.
set -eu
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) ;;
    *) printf '%s\n' 'This installer supports Linux x86_64. See rtk-ai/rtk for other builds.' >&2; exit 1 ;;
esac
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT HUP INT TERM
asset=rtk-oss-x86_64-unknown-linux-musl.tar.gz
curl --fail --location --proto '=https' --tlsv1.2 \
    "https://github.com/rtk-ai/rtk/releases/download/v1.2.0/$asset" -o "$staging/$asset"
(cd "$staging" && printf '%s  %s\n' \
    a28407dc13038bdc9c2a1f5a047b0c636e0c1d495b15523576e2adb992a06a17 "$asset" | sha256sum -c -)
tar -xzf "$staging/$asset" -C "$staging" rtk LICENSE
mkdir -p "$app_dir/dist"
install -m 755 "$staging/rtk" "$app_dir/dist/rtk.new"
mv "$app_dir/dist/rtk.new" "$app_dir/dist/rtk"
install -m 644 "$staging/LICENSE" "$app_dir/dist/RTK-LICENSE"
printf '%s\n' 'RTK installed. Restart Mini. Enable Cargo log filtering under Usage / Savings.'
