#!/bin/sh
# Optional. Pinned OSS release. No global hook installation.
set -eu
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=x86_64-unknown-linux-musl; checksum=a28407dc13038bdc9c2a1f5a047b0c636e0c1d495b15523576e2adb992a06a17 ;;
    Darwin-arm64) target=aarch64-apple-darwin; checksum=1959bd6d89a729ccdee176b7141490bbe750820f032c74f29a752576f2495254 ;;
    Darwin-x86_64) target=x86_64-apple-darwin; checksum=48f8cee63ae0b4853f9a77d7839e62ffcca18ef88c408f8a93d4e69986b1ab82 ;;
    *) printf '%s\n' 'Supported: Linux x86_64, macOS arm64 / x86_64.' >&2; exit 1 ;;
esac
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT HUP INT TERM
asset="rtk-oss-$target.tar.gz"
curl --fail --location --proto '=https' --tlsv1.2 \
    "https://github.com/rtk-ai/rtk/releases/download/v1.2.0/$asset" -o "$staging/$asset"
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$staging/$asset" | cut -d ' ' -f 1)
else
    actual=$(shasum -a 256 "$staging/$asset" | cut -d ' ' -f 1)
fi
[ "$actual" = "$checksum" ] || { printf '%s\n' 'Checksum mismatch.' >&2; exit 1; }
tar -xzf "$staging/$asset" -C "$staging" rtk LICENSE
mkdir -p "$app_dir/dist"
install -m 755 "$staging/rtk" "$app_dir/dist/rtk.new"
mv "$app_dir/dist/rtk.new" "$app_dir/dist/rtk"
install -m 644 "$staging/LICENSE" "$app_dir/dist/RTK-LICENSE"
printf '%s\n' 'RTK installed. Restart Mini. Enable Cargo log filtering under Usage / Savings.'
