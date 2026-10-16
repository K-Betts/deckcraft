#!/usr/bin/env bash
# Bring the app on this Mac up to date with `main` on GitHub: fetch, fast-forward `main`, then
# rebuild and reinstall with install-local.sh. Your own branch and uncommitted work are left alone
# (the update happens in a separate checkout of `main`).
#
# Usage: packaging/macos/update-local.sh [--check]
#   --check   only say whether a newer version is available
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

git fetch -q origin main
LOCAL="$(git rev-parse --short HEAD)"
LATEST="$(git rev-parse --short origin/main)"
INSTALLED_FILE="${INSTALL_DIR:-/Applications}/DeckCraft.app/Contents/Resources/.built-from"
INSTALLED="$(cat "$INSTALLED_FILE" 2>/dev/null || echo none)"

if [ "$INSTALLED" = "$LATEST" ]; then
  echo "up to date ($LATEST)"
  exit 0
fi
echo "installed: $INSTALLED   latest on main: $LATEST"
[ "${1:-}" = "--check" ] && { echo "an update is available; run packaging/macos/update-local.sh"; exit 0; }

# Build from a clean checkout of origin/main so the current branch is never touched.
WT="$(mktemp -d)/deckcraft-main"
git worktree add -q --detach "$WT" origin/main
trap 'git worktree remove --force "$WT" >/dev/null 2>&1 || true' EXIT

case "$(uname -m)" in
  arm64) ARCH=aarch64 ;;
  x86_64) ARCH=x86_64 ;;
  *) echo "unsupported architecture $(uname -m)" >&2; exit 2 ;;
esac

# Reuse this checkout's build cache so only changed crates are rebuilt.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
"$WT/packaging/macos/package.sh" --arch "$ARCH"

DEST="${INSTALL_DIR:-/Applications}"
APP="$CARGO_TARGET_DIR/macos-package/DeckCraft.app"
[ -d "$APP" ] || { echo "no app at $APP" >&2; exit 1; }
osascript -e 'tell application "DeckCraft" to quit' >/dev/null 2>&1 || true
rm -rf "$DEST/DeckCraft.app"
ditto "$APP" "$DEST/DeckCraft.app"
echo "$LATEST" > "$DEST/DeckCraft.app/Contents/Resources/.built-from"
echo "updated to $LATEST ($DEST/DeckCraft.app)"
