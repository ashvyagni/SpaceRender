#!/usr/bin/env bash
# Build Cosmogon.app and a distributable .dmg.
#
#   scripts/package-macos.sh              # native architecture
#   UNIVERSAL=1 scripts/package-macos.sh  # arm64 + x86_64 universal binary
#   SIGN_ID="Developer ID Application: …" scripts/package-macos.sh   # real signing
#
# Without SIGN_ID the app is ad-hoc signed: it runs on this Mac; on other Macs users must
# right-click → Open the first time. Public distribution needs a Developer ID certificate
# and notarisation (see BUILDING.md).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(cargo metadata --no-deps --format-version 1 | python3 -c "import json,sys; print([p['version'] for p in json.load(sys.stdin)['packages'] if p['name']=='cosmogon'][0])")
DIST=dist
APP="$DIST/Cosmogon.app"

echo "==> Building release binary (v$VERSION)"
if [[ "${UNIVERSAL:-0}" == "1" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release -p cosmogon --target aarch64-apple-darwin
  cargo build --release -p cosmogon --target x86_64-apple-darwin
  mkdir -p target/universal
  lipo -create -output target/universal/cosmogon target/aarch64-apple-darwin/release/cosmogon target/x86_64-apple-darwin/release/cosmogon
  BIN=target/universal/cosmogon
else
  cargo build --release -p cosmogon
  BIN=target/release/cosmogon
fi

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/cosmogon"
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"

echo "==> Building icon"
ICONSET=$(mktemp -d)/Cosmogon.iconset
mkdir -p "$ICONSET"
SRC=packaging/icons/cosmogon-1024.png
[[ -f "$SRC" ]] || python3 scripts/make_icon.py
for s in 16 32 128 256 512; do
  sips -z $s $s "$SRC" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s*2)) $((s*2)) "$SRC" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Cosmogon.icns"

echo "==> Signing"
if [[ -n "${SIGN_ID:-}" ]]; then
  codesign --force --deep --options runtime --timestamp --sign "$SIGN_ID" "$APP"
else
  codesign --force --deep --sign - "$APP"
fi
codesign --verify --deep --strict "$APP"

echo "==> Creating disk image"
DMG="$DIST/Cosmogon-$VERSION-macOS.dmg"
STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "Cosmogon $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"
if [[ -n "${SIGN_ID:-}" ]]; then
  codesign --force --sign "$SIGN_ID" "$DMG"
fi

echo
echo "Done:"
du -sh "$APP" "$DMG"
