#!/bin/bash
set -euo pipefail

# Проверка наличия jq
if ! command -v jq &> /dev/null; then
    echo "❌ Требуется 'jq'. Установи: brew install jq"
    exit 1
fi

# Имя проекта (для имени артефакта)
PROJECT_NAME=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')

# Имя бинарника из [[bin]]
BIN_NAME="blp_rs"

DIST_DIR="bin"
mkdir -p "$DIST_DIR"

echo "📦 Building universal macOS binary..."

cargo build --release --target aarch64-apple-darwin --bin "$BIN_NAME"
cargo build --release --target x86_64-apple-darwin --bin "$BIN_NAME"

lipo -create \
  -output "$DIST_DIR/$PROJECT_NAME-macos" \
  "target/aarch64-apple-darwin/release/$BIN_NAME" \
  "target/x86_64-apple-darwin/release/$BIN_NAME"

file "$DIST_DIR/$PROJECT_NAME-macos"

echo "🐧 Building for Linux (x86_64-unknown-linux-musl)..."

rustup target add x86_64-unknown-linux-musl &>/dev/null || true
cargo build --release --target x86_64-unknown-linux-musl --bin "$BIN_NAME"

cp "target/x86_64-unknown-linux-musl/release/$BIN_NAME" "$DIST_DIR/$PROJECT_NAME-linux"

echo "🪟 Building for Windows (x86_64-pc-windows-gnu)..."

rustup target add x86_64-pc-windows-gnu &>/dev/null || true
cargo build --release --target x86_64-pc-windows-gnu --bin "$BIN_NAME"

cp "target/x86_64-pc-windows-gnu/release/$BIN_NAME.exe" "$DIST_DIR/$PROJECT_NAME-windows.exe"

echo ""
echo "✅ Build complete:"
ls -lh "$DIST_DIR"
