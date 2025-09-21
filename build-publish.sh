#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/build-settings.sh"

need jq; need git; need gh

BUMP_KIND="${BUMP_KIND:-patch}"   # patch | minor | major

git diff --quiet || { echo "❌ В репо есть незакоммиченные изменения"; exit 1; }
gh auth status &>/dev/null || { echo "❌ gh не авторизован. Выполни: gh auth login"; exit 1; }

PROJECT_NAME="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')"
CURR_VERSION="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')"

IFS=. read -r MAJ MIN PAT <<<"$CURR_VERSION"
case "$BUMP_KIND" in
  major) NEW_VERSION="$((MAJ+1)).0.0" ;;
  minor) NEW_VERSION="$MAJ.$((MIN+1)).0" ;;
  patch) NEW_VERSION="$MAJ.$MIN.$((PAT+1))" ;;
  *) echo "❌ Неизвестный BUMP_KIND='$BUMP_KIND'"; exit 1 ;;
esac
echo "🔢 Версия: $CURR_VERSION → $NEW_VERSION"
TAG="v$NEW_VERSION"

# 1) Бамп версии в Cargo.toml
if sed --version &>/dev/null; then
  sed -E -i "s/^version *= *\"[0-9]+\.[0-9]+\.[0-9]+([^\"]*)?\"/version = \"$NEW_VERSION\"/" Cargo.toml
else
  sed -E -i '' "s/^version *= *\"[0-9]+\.[0-9]+\.[0-9]+([^\"]*)?\"/version = \"$NEW_VERSION\"/" Cargo.toml
fi
[[ -f Cargo.lock ]] && cargo generate-lockfile >/dev/null

# 2) Коммит + тэг (на новой версии)
git add Cargo.toml Cargo.lock 2>/dev/null || true
git commit -m "chore(release): $TAG"
git tag -a "$TAG" -m "$PROJECT_NAME $NEW_VERSION"
git push origin HEAD --tags

# 3) Чистая сборка артефактов уже с новой версией
# (важно не тащить старые файлы в $DIST_DIR)
rm -rf "$DIST_DIR"
"./build-only.sh"

# 4) Собираем список файлов для релиза
ASSETS=()
while IFS= read -r -d '' f; do ASSETS+=("$f"); done < <(find "$DIST_DIR" -maxdepth 1 -type f -print0)

# 5) Публикуем релиз
gh release create "$TAG" "${ASSETS[@]}" \
  --title "$PROJECT_NAME $NEW_VERSION" \
  --generate-notes

echo "✅ Published $TAG"
