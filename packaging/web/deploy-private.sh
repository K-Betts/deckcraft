#!/usr/bin/env bash
# Publish the private web app as a copy of what the Mac app has now, to a server running
# serve-private.py (see docs/web-hosting.md). Run from anywhere in the repository:
#
#   packaging/web/deploy-private.sh kieran@100.74.218.18
#   packaging/web/deploy-private.sh --local                # build only, into dist/web-private
#
# From the Mac app's settings folder (~/Library/Application Support/DeckCraft, or the folder
# given as a second argument) it builds in the curriculum, own questions, subject packs, Teacher
# Mode tools and lesson templates (apps/deckcraft-web/build.rs reads only those). The exam
# question library the Mac app uses (Insert > Question Library) goes beside the site in exam/,
# behind the same password; DECKCRAFT_EXAM_LIBRARY=/path picks another library folder, and
# DECKCRAFT_EXAM_LIBRARY=none leaves it out. Only counts are printed, never contents.
#
# The site lands in ~/deckcraft-web/site on the server; the server picks it up at once.
set -euo pipefail

usage="usage: deploy-private.sh user@host|--local [SETTINGS_FOLDER]"
host=${1:?$usage}
settings=${2:-"$HOME/Library/Application Support/DeckCraft"}
[ -f "$settings/ui.json" ] || { echo "no DeckCraft settings in $settings (expected ui.json there)" >&2; exit 1; }

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
out="$root/dist/web-private"

# The exam question library: the folder chosen in the Mac app's Question Library.
library=${DECKCRAFT_EXAM_LIBRARY:-$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("examLibraryDir", ""))' "$settings/ui.json")}
[ "$library" = none ] && library=""
if [ -n "$library" ] && [ ! -d "$library/items/exam" ]; then
    echo "the exam library $library has no items/exam folder; publishing without it" >&2
    library=""
fi

exam=0
[ -n "$library" ] && exam=1
(cd "$root/apps/deckcraft-web" && DECKCRAFT_WEB_SCHOOL="$settings" DECKCRAFT_WEB_EXAM=$exam trunk build --release --dist "$out")

if [ -n "$library" ]; then
    # Same layout as the folder on the Mac: the questions in one file (one download), the
    # pictures one file each (fetched as they are shown), and the list of picture names.
    mkdir -p "$out/exam/items/exam" "$out/exam/derived/exam_crops"
    python3 - "$library" "$out/exam" <<'PY'
import glob, json, os, sys
lib, dest = sys.argv[1], sys.argv[2]
items = []
for f in sorted(glob.glob(os.path.join(lib, "items/exam/*.json"))):
    with open(f, encoding="utf-8") as fh:
        items.extend(json.load(fh).get("items", []))
with open(os.path.join(dest, "items/exam/library.json"), "w", encoding="utf-8") as fh:
    json.dump({"items": items}, fh, ensure_ascii=False, separators=(",", ":"))
crops = sorted(n for n in os.listdir(os.path.join(lib, "derived/exam_crops")) if n.lower().endswith(".png")) \
    if os.path.isdir(os.path.join(lib, "derived/exam_crops")) else []
with open(os.path.join(dest, "crops.json"), "w") as fh:
    json.dump(crops, fh)
print(f"exam question library: {len(items)} questions, {len(crops)} pictures")
PY
    if [ -d "$library/derived/exam_crops" ]; then
        rsync -a --delete --include='*.png' --include='*.PNG' --exclude='*' "$library/derived/exam_crops/" "$out/exam/derived/exam_crops/"
    fi
fi

# The staff question-bank page (/bank/): the built-in bank with one worked example each.
(cd "$root" && cargo build -q -p deckcraft-cli)
python3 "$root/packaging/web/make_catalog.py" "$root/target/debug/deckcraft-cli" "$root/assets/questions" "$out/bank-catalog.json"

# Compressed copies for the server to send to browsers that accept gzip (most of the download).
(cd "$out" && for f in *.wasm *.js index.html exam/items/exam/library.json; do if [ -f "$f" ]; then gzip -kf9 "$f"; fi; done)

if [ "$host" = --local ]; then
    echo "Built in $out (not published). Try it: python3 -m http.server 8765 --directory $out (then http://localhost:8765/)"
    exit 0
fi
ssh "$host" 'mkdir -p ~/deckcraft-web/site'
rsync -a --delete "$out/" "$host:deckcraft-web/site/"
rsync -a "$root/packaging/web/serve-private.py" "$root/packaging/web/bank_portal.py" "$root/packaging/web/bank_page.html" "$root/packaging/web/bank_taxonomy.json" "$host:deckcraft-web/"
echo "Published to $host:~/deckcraft-web/site"
