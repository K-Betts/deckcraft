#!/usr/bin/env bash
# Build the private web app with a curriculum built in and copy it to a server running
# serve-private.py (see docs/web-hosting.md). Run from anywhere in the repository:
#
#   packaging/web/deploy-private.sh user@host ~/maths-workspace/curriculum/ringmer-2026-27.json
#
# The site lands in ~/deckcraft-web/site on the server; the server picks it up at once.
set -euo pipefail

host=${1:?usage: deploy-private.sh user@host CURRICULUM.json}
curriculum=${2:?usage: deploy-private.sh user@host CURRICULUM.json}
[ -f "$curriculum" ] || { echo "no such curriculum file: $curriculum" >&2; exit 1; }
curriculum=$(cd "$(dirname "$curriculum")" && pwd)/$(basename "$curriculum")

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
out="$root/dist/web-private"

(cd "$root/apps/deckcraft-web" && DECKCRAFT_WEB_CURRICULUM="$curriculum" trunk build --release --dist "$out")
# Compressed copies for the server to send to browsers that accept gzip (most of the download).
(cd "$out" && for f in *.wasm *.js index.html; do gzip -kf9 "$f"; done)

ssh "$host" 'mkdir -p ~/deckcraft-web/site'
rsync -a --delete "$out/" "$host:deckcraft-web/site/"
rsync -a "$root/packaging/web/serve-private.py" "$host:deckcraft-web/"
echo "Published to $host:~/deckcraft-web/site"
