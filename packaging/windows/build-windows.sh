#!/usr/bin/env bash
# Build DeckCraft for Windows (x86_64-pc-windows-gnu).
# Cross-compiling from macOS requires the MinGW-w64 toolchain (for example, `brew install mingw-w64`).
#
# Usage: packaging/windows/build-windows.sh [--skip-build]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET=x86_64-pc-windows-gnu
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT/dist/windows}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
case "$CARGO_TARGET_DIR" in
  /*) ;;
  *) CARGO_TARGET_DIR="$ROOT/$CARGO_TARGET_DIR" ;;
esac

if [ "${1:-}" != "--skip-build" ]; then
  if [ "$(uname -s)" = "Darwin" ] && ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "missing x86_64-w64-mingw32-gcc; install MinGW-w64 (for example, brew install mingw-w64)" >&2
    exit 1
  fi
  if ! rustup target list --installed | grep -Fxq "$TARGET"; then
    echo "missing Rust target; install it with: rustup target add $TARGET" >&2
    exit 1
  fi
  (cd "$ROOT" && CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo build --release --locked -p deckcraft --target "$TARGET")
fi

EXE="$CARGO_TARGET_DIR/$TARGET/release/deckcraft.exe"
[ -f "$EXE" ] || { echo "no executable at $EXE; run without --skip-build" >&2; exit 1; }

mkdir -p "$OUTPUT_DIR"
cp "$EXE" "$OUTPUT_DIR/DeckCraft.exe"
echo "built $OUTPUT_DIR/DeckCraft.exe"
