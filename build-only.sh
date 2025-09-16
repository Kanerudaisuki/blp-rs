#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/build-settings.sh"

need cargo; need rustup; need jq; need lipo; need file; need hdiutil

PROJECT_NAME="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')"

# --- clean dist ---
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# ===== macOS universal =====
echo "📦 macOS universal…"
rustup target add aarch64-apple-darwin x86_64-apple-darwin &>/dev/null || true
cargo build --release --target aarch64-apple-darwin --bin "$BIN_NAME" --locked
cargo build --release --target x86_64-apple-darwin --bin "$BIN_NAME" --locked

MAC_UNI="$DIST_DIR/$PROJECT_NAME-macos"
lipo -create \
  -output "$MAC_UNI" \
  "target/aarch64-apple-darwin/release/$BIN_NAME" \
  "target/x86_64-apple-darwin/release/$BIN_NAME"
chmod +x "$MAC_UNI"
strip_safe "$MAC_UNI" macos
file "$MAC_UNI"

# macOS .app -> zip + dmg
APP_NAME="$PROJECT_NAME"
APP_TMP="$(mktemp -d)/$APP_NAME-macos.app"
APP_MACOS="$APP_TMP/Contents/MacOS"; APP_RES="$APP_TMP/Contents/Resources"
mkdir -p "$APP_MACOS" "$APP_RES"
cp "$MAC_UNI" "$APP_MACOS/$APP_NAME"; chmod +x "$APP_MACOS/$APP_NAME"

ICON_SRC="assets/generated/AppIcon.icns"; [[ -f "$ICON_SRC" ]] || ICON_SRC="assets/icon.icns"
ICON_KEY=""
if [[ -f "$ICON_SRC" ]]; then
  cp "$ICON_SRC" "$APP_RES/app.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>app</string>"
else
  echo "⚠️  icns не найден — .app без иконки"
fi

# версия как плейсхолдер — реальную подставит build-publish
cat > "$APP_TMP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key>                <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>          <string>$APP_ID_BUNDLE.$PROJECT_NAME</string>
  <key>CFBundleExecutable</key>          <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>         <string>APPL</string>
  <key>CFBundleShortVersionString</key>  <string>0.0.0</string>
  <key>CFBundleVersion</key>             <string>0.0.0</string>
  <key>LSMinimumSystemVersion</key>      <string>10.13</string>
  <key>NSHighResolutionCapable</key>     <true/>
  $ICON_KEY
</dict></plist>
PLIST

if command -v codesign &>/dev/null; then codesign --force --deep --sign - "$APP_TMP" || true; fi
ZIP="$DIST_DIR/$PROJECT_NAME-macos.zip"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$APP_TMP" "$ZIP"
DMG="$DIST_DIR/$PROJECT_NAME-macos.dmg"
hdiutil create -quiet -fs HFS+ -imagekey zlib-level=9 -volname "$APP_NAME" -srcfolder "$APP_TMP" -ov -format UDZO "$DMG"

# ===== Linux (musl) =====
echo "🐧 Linux…"
rustup target add x86_64-unknown-linux-musl &>/dev/null || true
cargo build --release --target x86_64-unknown-linux-musl --bin "$BIN_NAME" --locked
LIN_BIN="$DIST_DIR/$PROJECT_NAME-linux"
cp "target/x86_64-unknown-linux-musl/release/$BIN_NAME" "$LIN_BIN"
chmod +x "$LIN_BIN"
strip_safe "$LIN_BIN" linux
file "$LIN_BIN"

# ===== Windows (gnu) =====
echo "🪟 Windows…"
rustup target add x86_64-pc-windows-gnu &>/dev/null || true
cargo build --release --target x86_64-pc-windows-gnu --bin "$BIN_NAME" --locked
WIN_EXE="$DIST_DIR/$PROJECT_NAME-windows.exe"
cp "target/x86_64-pc-windows-gnu/release/$BIN_NAME.exe" "$WIN_EXE"
strip_safe "$WIN_EXE" windows
maybe_upx "$WIN_EXE"
file "$WIN_EXE"

# --- checksums ---
echo "🔐 Checksums…"
(
  cd "$DIST_DIR"
  rm -f SHA256SUMS.txt
  if command -v shasum &>/dev/null; then
    find . -maxdepth 1 -type f ! -name 'SHA256SUMS.txt' -exec shasum -a 256 {} \; > SHA256SUMS.txt
  else
    find . -maxdepth 1 -type f ! -name 'SHA256SUMS.txt' -exec sha256sum {} \; > SHA256SUMS.txt
  fi
)

# --- summary ---
echo -e "\n✅ Готово. Содержимое '$DIST_DIR':"
ls -lh "$DIST_DIR"
