#!/bin/bash
set -euo pipefail

# --- deps ---
if ! command -v jq &>/dev/null; then
  echo "❌ Требуется 'jq'. Установи: brew install jq"
  exit 1
fi

# --- names/paths ---
PROJECT_NAME=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')
BIN_NAME="blp_rs"
VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
BUNDLE_ID_DEFAULT="com.blp.${PROJECT_NAME}"
BUNDLE_ID=$(cargo metadata --no-deps --format-version 1 | jq -r \
  --arg def "$BUNDLE_ID_DEFAULT" '.packages[0].metadata.bundle.identifier // $def')

DIST_DIR="bin"

# --- clean dist ---
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo "📦 Building universal macOS binary…"
rustup target add aarch64-apple-darwin x86_64-apple-darwin &>/dev/null || true
cargo build --release --target aarch64-apple-darwin --bin "$BIN_NAME"
cargo build --release --target x86_64-apple-darwin --bin "$BIN_NAME"

MAC_UNI="$DIST_DIR/$PROJECT_NAME-macos"
lipo -create \
  -output "$MAC_UNI" \
  "target/aarch64-apple-darwin/release/$BIN_NAME" \
  "target/x86_64-apple-darwin/release/$BIN_NAME"
chmod +x "$MAC_UNI"
file "$MAC_UNI" || true

echo "🍏 Creating .app bundle…"
APP_NAME="$PROJECT_NAME"
APP_DIR="$DIST_DIR/$APP_NAME-macos.app"
APP_MACOS="$APP_DIR/Contents/MacOS"
APP_RES="$APP_DIR/Contents/Resources"
mkdir -p "$APP_MACOS" "$APP_RES"

# put universal binary inside .app
cp "$MAC_UNI" "$APP_MACOS/$APP_NAME"
chmod +x "$APP_MACOS/$APP_NAME"

# optional icon
ICON_SRC="assets/icon.icns"
ICON_KEY=""
if [[ -f "$ICON_SRC" ]]; then
  cp "$ICON_SRC" "$APP_RES/icon.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>icon</string>"
fi

# Info.plist
cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key>                <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>          <string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key>          <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>         <string>APPL</string>
  <key>CFBundleShortVersionString</key>  <string>$VERSION</string>
  <key>CFBundleVersion</key>             <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>      <string>10.13</string>
  <key>NSHighResolutionCapable</key>     <true/>
  $ICON_KEY
  <!-- Сними коммент, если нужна агент-аппа без иконки в Dock/меню:
  <key>LSUIElement</key><true/>
  -->
</dict></plist>
PLIST

# ad-hoc sign if available
if command -v codesign &>/dev/null; then
  codesign --force --deep --sign - "$APP_DIR" || true
fi

echo "🗜  Zipping .app…"
ZIP="$DIST_DIR/$PROJECT_NAME-macos.zip"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$APP_DIR" "$ZIP"

echo "💿 Creating .dmg…"
DMG="$DIST_DIR/$PROJECT_NAME-macos.dmg"
hdiutil create -quiet \
  -fs HFS+ -imagekey zlib-level=9 \
  -volname "$APP_NAME" \
  -srcfolder "$APP_DIR" \
  -ov -format UDZO "$DMG"

echo "🐧 Building for Linux (x86_64-unknown-linux-musl)…"
rustup target add x86_64-unknown-linux-musl &>/dev/null || true
cargo build --release --target x86_64-unknown-linux-musl --bin "$BIN_NAME"
cp "target/x86_64-unknown-linux-musl/release/$BIN_NAME" "$DIST_DIR/$PROJECT_NAME-linux"
chmod +x "$DIST_DIR/$PROJECT_NAME-linux"

echo "🪟 Building for Windows (x86_64-pc-windows-gnu)…"
rustup target add x86_64-pc-windows-gnu &>/dev/null || true
cargo build --release --target x86_64-pc-windows-gnu --bin "$BIN_NAME"
cp "target/x86_64-pc-windows-gnu/release/$BIN_NAME.exe" "$DIST_DIR/$PROJECT_NAME-windows.exe"

echo ""
echo "✅ Build complete:"
ls -lh "$DIST_DIR"
echo ""
echo "👉 macOS console binary:  $MAC_UNI"
echo "👉 macOS app bundle:      $APP_DIR   (run: open \"$APP_DIR\")"
echo "👉 macOS .zip:            $ZIP"
echo "👉 macOS .dmg:            $DMG"
echo "👉 Linux binary:          $DIST_DIR/$PROJECT_NAME-linux"
echo "👉 Windows exe:           $DIST_DIR/$PROJECT_NAME-windows.exe"
