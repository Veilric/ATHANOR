#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
python3 scripts/cargo-local.py build --release --locked -p athanor --bin athanor
system=$(uname -s)
arch=$(uname -m)
build_dir=target/release
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    build_dir="target/$CARGO_BUILD_TARGET/release"
    arch=$CARGO_BUILD_TARGET
fi
label="athanor-0.1.0-$system-$arch"
stage="artifacts/$label"
if [ -e "$stage" ]; then
    printf '%s\n' "Package staging folder already exists: $stage. Move it aside before packaging again." >&2
    exit 1
fi
mkdir -p "$stage"
if [ "$system" = Darwin ]; then
    app="$stage/Athanor.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    cp "$build_dir/athanor" "$app/MacOS/athanor"
    cp assets/icon.svg "$app/Resources/icon.svg"
    cat > "$app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>dev.veilric.athanor</string><key>CFBundleName</key><string>Athanor</string><key>CFBundleExecutable</key><string>athanor</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>0.1.0</string><key>LSUIElement</key><true/><key>NSHighResolutionCapable</key><true/></dict></plist>
PLIST
else
    cp "$build_dir/athanor" "$stage/athanor"
    cp assets/icon.svg "$stage/athanor.svg"
    cat > "$stage/dev.veilric.athanor.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Athanor
Comment=Search, calculate, convert
Exec=athanor
Icon=athanor
Terminal=false
Categories=Utility;
DESKTOP
fi
cp README.md "$stage/README.md"
cp -R docs "$stage/docs"
python3 scripts/dependency-notices.py "$stage/licenses"
printf '%s\n' 'Version: 0.1.0' 'Rune: bb310fc9e2c4a1a1937814c71efc08924a5c2885' 'Unsigned local preview; see docs/platform-support.md' > "$stage/BUILD_INFO.txt"
tar -czf "artifacts/$label.tar.gz" -C artifacts "$label"
printf '%s\n' "artifacts/$label.tar.gz"
