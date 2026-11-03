#!/usr/bin/env python3
"""Make bank-catalog.json for the staff question-bank page: one entry per built-in template with
one worked example (made by `deckcraft-cli bank check`).  usage: make_catalog.py CLI QUESTIONS_DIR OUT"""
import glob, json, os, re, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bank_categories

cli, qdir, out = sys.argv[1:4]
entries = []
for path in sorted(glob.glob(f"{qdir}/*.json")):
    data = json.load(open(path, encoding="utf-8"))
    meta = {t["id"]: t for t in data.get("templates", []) + data.get("questions", [])}
    text = subprocess.run([cli, "bank", "check", path, "--samples", "1"], capture_output=True, text=True).stdout
    for m in re.finditer(r"^(\S+) · (.*?) · (.*?) · level (\d)\n(?:  CHECK THIS:.*\n)*  Q: (.*)\n  A: (.*)$", text, re.M):
        id_, topic, skill, level, q, a = m.groups()
        t = meta.get(id_, {})
        entries.append({"id": id_, "year": t.get("year") or "", "level": int(level), "topic": topic, "skill": skill, "q": q, "a": a,
                        "key": t.get("source") or f"{topic} · {skill}"})
    for q in data.get("questions", []):
        entries.append({"id": q["id"], "year": q.get("year") or "", "level": q["difficulty"], "topic": q["topic"],
                        "skill": q.get("subtopic", ""), "q": q["text"], "a": q.get("answer", ""),
                        "key": q.get("source") or f"{q['topic']} · {q.get('subtopic', '')}"})
for e in entries:
    e["cat"] = bank_categories.categorise(e["topic"], e["skill"]) or ""
loose = sum(1 for e in entries if not e["cat"])
json.dump({"taxonomy": bank_categories.taxonomy(), "items": entries}, open(out, "w", encoding="utf-8"), ensure_ascii=False, separators=(",", ":"))
print(f"staff question bank page: {len(entries)} entries, {loose} not sorted into a skill")
