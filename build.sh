#!/usr/bin/env bash
set -euo pipefail

# ============= settings =============
BUMP_KIND="${BUMP_KIND:-patch}"    # patch | minor | major
DIST_DIR="bin"
BIN_NAME="blp_rs"
# ====================================

need() { command -v "$1" &>/dev/null || { echo "❌ Требуется '$1'"; exit 1; }; }
need jq; need git; need cargo; need rustup; need gh; need lipo; need hdiutil

git diff --quiet || { echo "❌ В репо есть незакоммиченные изменения"; exit 1; }
gh auth status &>/dev/null || { echo "❌ gh не авторизован. Выполни: gh auth login"; exit 1; }

PROJECT_NAME=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')
CURR_VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

IFS=. read -r MAJ MIN PAT <<<"$CURR_VERSION"
case "$BUMP_KIND" in
  major) NEW_VERSION="$((MAJ+1)).0.0" ;;
  minor) NEW_VERSION="$MAJ.$((MIN+1)).0" ;;
  patch) NEW_VERSION="$MAJ.$MIN.$((PAT+1))" ;;
  *) echo "❌ Неизвестный BUMP_KIND='$BUMP_KIND'"; exit 1 ;;
esac
echo "🔢 Версия: $CURR_VERSION → $NEW_VERSION"

# bump Cargo.toml
if sed --version &>/dev/null; then
  # GNU sed (Linux)
  sed -E -i "s/^version *= *\"[0-9]+\.[0-9]+\.[0-9]+([^\"]*)?\"/version = \"$NEW_VERSION\"/" Cargo.toml
else
  # BSD sed (macOS)
  sed -E -i '' "s/^version *= *\"[0-9]+\.[0-9]+\.[0-9]+([^\"]*)?\"/version = \"$NEW_VERSION\"/" Cargo.toml
fi
[[ -f Cargo.lock ]] && cargo generate-lockfile >/dev/null

TAG="v$NEW_VERSION"
git add Cargo.toml Cargo.lock 2>/dev/null || true
git commit -m "chore(release): $TAG"
git push origin HEAD
git tag -a "$TAG" -m "$PROJECT_NAME $NEW_VERSION"
git push origin "$TAG"

VERSION="$NEW_VERSION"

# clean dist
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# ===== macOS: универсальный бинарь =====
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

# ===== macOS .app -> zip/dmg (временная .app, не в bin) =====
APP_NAME="$PROJECT_NAME"
APP_TMP="$(mktemp -d)/$APP_NAME-macos.app"
APP_MACOS="$APP_TMP/Contents/MacOS"
APP_RES="$APP_TMP/Contents/Resources"
mkdir -p "$APP_MACOS" "$APP_RES"
cp "$MAC_UNI" "$APP_MACOS/$APP_NAME"
chmod +x "$APP_MACOS/$APP_NAME"

# ---- иконка для .app ----
# 1) сначала пробуем готовую icns, которую генерит твой build.rs
ICON_SRC="assets/generated/AppIcon.icns"
# 2) если её нет — fallback на ручную icns
[[ -f "$ICON_SRC" ]] || ICON_SRC="assets/icon.icns"

ICON_KEY=""
if [[ -f "$ICON_SRC" ]]; then
  cp "$ICON_SRC" "$APP_RES/app.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>app</string>"
else
  echo "⚠️  Не найдено icns: ни assets/generated/AppIcon.icns, ни assets/icon.icns — .app будет без иконки"
fi

cat > "$APP_TMP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key>                <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>          <string>com.blp.$PROJECT_NAME</string>
  <key>CFBundleExecutable</key>          <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>         <string>APPL</string>
  <key>CFBundleShortVersionString</key>  <string>$VERSION</string>
  <key>CFBundleVersion</key>             <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>      <string>10.13</string>
  <key>NSHighResolutionCapable</key>     <true/>
  $ICON_KEY
</dict></plist>
PLIST

if command -v codesign &>/dev/null; then
  codesign --force --deep --sign - "$APP_TMP" || true
fi

# zip + dmg
ZIP="$DIST_DIR/$PROJECT_NAME-macos.zip"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$APP_TMP" "$ZIP"

DMG="$DIST_DIR/$PROJECT_NAME-macos.dmg"
hdiutil create -quiet \
  -fs HFS+ -imagekey zlib-level=9 \
  -volname "$APP_NAME" \
  -srcfolder "$APP_TMP" \
  -ov -format UDZO "$DMG"

# ===== Linux =====
echo "🐧 Linux…"
rustup target add x86_64-unknown-linux-musl &>/dev/null || true
cargo build --release --target x86_64-unknown-linux-musl --bin "$BIN_NAME" --locked
cp "target/x86_64-unknown-linux-musl/release/$BIN_NAME" "$DIST_DIR/$PROJECT_NAME-linux"
chmod +x "$DIST_DIR/$PROJECT_NAME-linux"

# ===== Windows =====
echo "🪟 Windows…"
rustup target add x86_64-pc-windows-gnu &>/dev/null || true
cargo build --release --target x86_64-pc-windows-gnu --bin "$BIN_NAME" --locked
cp "target/x86_64-pc-windows-gnu/release/$BIN_NAME.exe" "$DIST_DIR/$PROJECT_NAME-windows.exe"

# --- checksums (только файлы, без директорий и без самой SHA256SUMS.txt) ---
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

# --- release: явный список ассетов ---
echo "🚀 Release $TAG"
gh release create "$TAG" \
  "$DIST_DIR/$PROJECT_NAME-macos" \
  "$DIST_DIR/$PROJECT_NAME-macos.zip" \
  "$DIST_DIR/$PROJECT_NAME-macos.dmg" \
  "$DIST_DIR/$PROJECT_NAME-linux" \
  "$DIST_DIR/$PROJECT_NAME-windows.exe" \
  "$DIST_DIR/SHA256SUMS.txt" \
  --title "$PROJECT_NAME $VERSION" \
  --generate-notes

echo "✅ Done. Артефакты:"
ls -lh "$DIST_DIR"
