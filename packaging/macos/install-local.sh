#!/usr/bin/env bash
# Build the app for this Mac and install it to /Applications (or $INSTALL_DIR).
# Ad-hoc signed, so it runs on the Mac that built it; use package.sh with a real identity to share it.
#
# Usage: packaging/macos/install-local.sh [--skip-build]
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="${INSTALL_DIR:-/Applications}"

case "$(uname -m)" in
  arm64) ARCH=aarch64 ;;
  x86_64) ARCH=x86_64 ;;
  *) echo "unsupported architecture $(uname -m)" >&2; exit 2 ;;
esac

if [ "${1:-}" != "--skip-build" ]; then
  "$ROOT/packaging/macos/package.sh" --arch "$ARCH"
fi

APP="$ROOT/target/macos-package/DeckCraft.app"
[ -d "$APP" ] || { echo "no app at $APP; run without --skip-build" >&2; exit 1; }

osascript -e 'tell application "DeckCraft" to quit' >/dev/null 2>&1 || true
rm -rf "$DEST/DeckCraft.app"
ditto "$APP" "$DEST/DeckCraft.app"
echo "installed $DEST/DeckCraft.app"
