#!/bin/sh
# Run after build.sh and license generation. Native host only.
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$app_dir"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
arch=$(uname -m)
case "$arch" in arm64) arch=aarch64 ;; esac
case "$(uname -s)" in Darwin) platform=macos ;; Linux) platform=linux ;; *) exit 1 ;; esac
name="codex-mini-$version-$platform-$arch"
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT HUP INT TERM
package="$staging/$name"
mkdir -p "$package" dist
# The notice file is generated for this target by cargo-about. Fail if absent.
cp dist/THIRD_PARTY_LICENSES.html "$package/"
cp LICENSE README.md THIRD_PARTY_NOTICES.md "$package/"
cp -R licenses docs "$package/"
mkdir -p "$package/scripts"
cp scripts/install-rtk.sh "$package/scripts/"
if [ "$platform" = macos ]; then
    bundle="$package/Codex Mini.app/Contents"
    mkdir -p "$bundle/MacOS" "$bundle/Resources"
    cp dist/codex-mini "$bundle/MacOS/"
    cat > "$bundle/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Codex Mini</string>
<key>CFBundleIdentifier</key><string>io.github.G1rmmr.codex-mini</string>
<key>CFBundleExecutable</key><string>codex-mini</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleIconFile</key><string>codex-mini</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
    iconset="$staging/codex-mini.iconset"
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" assets/codex-mini-source.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
        double=$((size * 2))
        sips -z "$double" "$double" assets/codex-mini-source.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$bundle/Resources/codex-mini.icns"
    # Ad-hoc signing satisfies the local arm64 loader. This is not notarization.
    codesign --force --sign - "$package/Codex Mini.app"
    codesign --verify --deep --strict "$package/Codex Mini.app"
    ditto -c -k --keepParent "$package" "dist/$name.zip"
else
    cp dist/codex-mini "$package/"
    tar -czf "dist/$name.tar.gz" -C "$staging" "$name"
fi
