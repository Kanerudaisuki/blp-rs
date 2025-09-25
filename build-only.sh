#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/build-settings.sh"

need cargo; need rustup; need jq; need lipo; need file; need hdiutil

PROJECT_NAME="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')"

# --- clean dist ---
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

build_variant() {
  local bin_name="$1"
  local feature_spec="$2"
  local packaging="${3:-binary}"
  local feature_label="${feature_spec:-none}"
  local cargo_features=(--no-default-features)
  if [[ -n "$feature_spec" && "$feature_spec" != "-" ]]; then
    cargo_features+=(--features "$feature_spec")
  fi

  # shellcheck disable=SC2028
  echo "\n=== 🔨 Building $bin_name (features: $feature_label, packaging: $packaging) ==="

  # ===== macOS universal =====
  echo "📦 macOS universal…"
  rustup target add aarch64-apple-darwin x86_64-apple-darwin &>/dev/null || true
  cargo build --release --target aarch64-apple-darwin --bin "$bin_name" --locked "${cargo_features[@]}"
  cargo build --release --target x86_64-apple-darwin --bin "$bin_name" --locked "${cargo_features[@]}"

  local mac_uni="$DIST_DIR/${bin_name}-macos"
  lipo -create \
    -output "$mac_uni" \
    "target/aarch64-apple-darwin/release/$bin_name" \
    "target/x86_64-apple-darwin/release/$bin_name"
  chmod +x "$mac_uni"
  strip_safe "$mac_uni" macos
  file "$mac_uni"

  if [[ "$packaging" == "app" ]]; then
    local app_name="$PROJECT_NAME"
    # shellcheck disable=SC2155
    local app_tmp="$(mktemp -d)/$app_name-macos.app"
    local app_macos="$app_tmp/Contents/MacOS"
    local app_res="$app_tmp/Contents/Resources"
    mkdir -p "$app_macos" "$app_res"
    cp "$mac_uni" "$app_macos/$app_name"; chmod +x "$app_macos/$app_name"

    local icon_src="assets/generated/AppIcon.icns"
    [[ -f "$icon_src" ]] || icon_src="assets/icon.icns"
    local icon_key=""
    if [[ -f "$icon_src" ]]; then
      cp "$icon_src" "$app_res/app.icns"
      icon_key="<key>CFBundleIconFile</key><string>app</string>"
    else
      echo "⚠️  icns не найден — .app без иконки"
    fi

    cat > "$app_tmp/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key>                <string>$app_name</string>
  <key>CFBundleIdentifier</key>          <string>$APP_ID_BUNDLE.$PROJECT_NAME</string>
  <key>CFBundleExecutable</key>          <string>$app_name</string>
  <key>CFBundlePackageType</key>         <string>APPL</string>
  <key>CFBundleShortVersionString</key>  <string>0.0.0</string>
  <key>CFBundleVersion</key>             <string>0.0.0</string>
  <key>LSMinimumSystemVersion</key>      <string>10.13</string>
  <key>NSHighResolutionCapable</key>     <true/>
  $icon_key
</dict></plist>
PLIST

    if command -v codesign &>/dev/null; then codesign --force --deep --sign - "$app_tmp" || true; fi
    local zip_path="$DIST_DIR/${bin_name}-macos.zip"
    /usr/bin/ditto -c -k --sequesterRsrc --keepParent "$app_tmp" "$zip_path"
    local dmg_path="$DIST_DIR/${bin_name}-macos.dmg"
    hdiutil create -quiet -fs HFS+ -imagekey zlib-level=9 -volname "$app_name" -srcfolder "$app_tmp" -ov -format UDZO "$dmg_path"
  fi

  # ===== Linux (musl) =====
  echo "🐧 Linux…"
  rustup target add x86_64-unknown-linux-musl &>/dev/null || true
  cargo build --release --target x86_64-unknown-linux-musl --bin "$bin_name" --locked "${cargo_features[@]}"
  local lin_bin="$DIST_DIR/${bin_name}-linux"
  cp "target/x86_64-unknown-linux-musl/release/$bin_name" "$lin_bin"
  chmod +x "$lin_bin"
  strip_safe "$lin_bin" linux
  file "$lin_bin"

  # ===== Windows (gnu) =====
  echo "🪟 Windows…"
  rustup target add x86_64-pc-windows-gnu &>/dev/null || true
  cargo build --release --target x86_64-pc-windows-gnu --bin "$bin_name" --locked "${cargo_features[@]}"
  local win_exe="$DIST_DIR/${bin_name}-windows.exe"
  cp "target/x86_64-pc-windows-gnu/release/$bin_name.exe" "$win_exe"
  strip_safe "$win_exe" windows
  maybe_upx "$win_exe"
  file "$win_exe"
}

while IFS= read -r spec; do
  spec="${spec//$'\r'/}"
  [[ -z "${spec//[[:space:]]/}" ]] && continue
  IFS=':' read -r bin_name feature_spec packaging <<<"$spec"
  build_variant "$bin_name" "$feature_spec" "$packaging"
done <<<"$BUILD_VARIANTS"

# --- checksums ---
# shellcheck disable=SC2028
echo "\n🔐 Checksums…"
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
