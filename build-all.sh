#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

# Ensure helper scripts are executable
chmod +x "$SCRIPT_DIR/build-only.sh" "$SCRIPT_DIR/build-publish.sh"

# Прокинутые переменные окружения (например, UPX=1 BUMP_KIND=minor) попадут в оба скрипта
"$SCRIPT_DIR/build-only.sh"
"$SCRIPT_DIR/build-publish.sh"
