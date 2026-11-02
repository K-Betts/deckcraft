"""The staff question-bank pages, served by serve-private.py under /bank/ (same staff password).

Teachers browse the built-in bank (bank-catalog.json, made by deploy-private.sh), flag or suggest
edits to a question, and add questions of their own. Nothing changes the live bank until an
admin approves it; approved changes are exported at /bank/api/approved.json in the bank file
format (import it with Manage Questions, or publish it with deploy-private.sh).

Python standard library only. Data: $DECKCRAFT_BANK_DATA (default ~/deckcraft-web/bank-data).
"""

import hmac
import json
import os
import re
import threading
import time

DATA_DIR = os.environ.get("DECKCRAFT_BANK_DATA", os.path.expanduser("~/deckcraft-web/bank-data"))
ADMIN_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bank-admin.json")
TOPICS = {"Number", "Algebra", "Ratio", "Geometry", "Statistics", "Probability"}
YEARS = {"Y7", "Y8", "Y9", "Y10", "Y11"}
ACTIONS = {"retire", "wrong", "reword", "retag"}
MAX_BODY = 64 * 1024
LOCK = threading.Lock()
derive = None  # set by serve-private.py (its password hashing)
PAGE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bank_page.html")


def clean(v, limit):
    v = re.sub(r"[\x00-\x08\x0b-\x1f]", "", str(v or "")).strip()
    return v[:limit]


def load():
    try:
        with open(os.path.join(DATA_DIR, "bank.json"), encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return {"next": 1, "items": []}


def save(db):
    os.makedirs(DATA_DIR, mode=0o700, exist_ok=True)
    tmp = os.path.join(DATA_DIR, "bank.json.tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(db, f, ensure_ascii=False, indent=1)
    os.replace(tmp, os.path.join(DATA_DIR, "bank.json"))


def question_problem(q):
    """What is wrong with a question a teacher typed, or None."""
    if len(q["text"]) < 5:
        return "The question needs some text."
    if not q["answer"]:
        return "The question needs an answer."
    if q["topic"] not in TOPICS:
        return "Choose a topic."
    if q["year"] not in YEARS:
        return "Choose a year group (Y7 to Y11)."
    if q["difficulty"] not in (1, 2, 3, 4):
        return "Choose a level (1 Support, 2 Core, 3 Stretch, 4 Exam style)."
    for field in ("text", "answer"):
        s = q[field]
        if s.count("$") % 2:
            return f"The {field} has an odd number of $ signs: each equation goes between a pair of $."
        if "[[" in s or "]]" in s:
            return f"The {field} has [[ ]], which is for random-number templates; write a fixed question."
        if s.count("{") != s.count("}"):
            return f"The {field} has unbalanced curly brackets."
    return None


def read_question(d):
    try:
        diff = int(d.get("difficulty"))
    except (TypeError, ValueError):
        diff = 0
    kw = d.get("keywords") or []
    if isinstance(kw, str):
        kw = kw.split(",")
    return {
        "topic": clean(d.get("topic"), 40),
        "subtopic": clean(d.get("subtopic"), 120),
        "difficulty": diff,
        "year": clean(d.get("year"), 4).upper(),
        "text": clean(d.get("text"), 2000),
        "answer": clean(d.get("answer"), 500),
        "keywords": [clean(k, 40) for k in kw if clean(k, 40)][:10] if isinstance(kw, list) else [],
    }


def teacher(handler):
    name = clean(handler.headers.get("X-Teacher", ""), 40)
    return name if len(name) >= 2 else None


def is_admin(handler):
    try:
        with open(ADMIN_FILE) as f:
            a = json.load(f)
    except (OSError, ValueError):
        return False
    given = handler.headers.get("X-Admin-Password", "")
    ok = hmac.compare_digest(derive(given, bytes.fromhex(a["salt"])), a["hash"])
    if not ok:
        time.sleep(1)
    return ok


def approved_export(db):
    questions, removed = [], []
    for it in db["items"]:
        if it["status"] != "approved":
            continue
        if it["kind"] == "new":
            questions.append({"id": f"staff-{it['id']}", **it["question"]})
        elif it["kind"] == "flag":
            if it["action"] in ("retire", "wrong"):
                removed.append(it["target"])
            elif it["action"] in ("reword", "retag") and it.get("question"):
                removed.append(it["target"])
                questions.append({"id": f"staff-{it['id']}", **it["question"]})
    return {"version": 1, "questions": questions, "templates": [], "removedQuestions": removed, "myQuestions": questions}


def api(handler, method, path, body):
    """Returns (status, json-able). `path` is after /bank/api/."""
    who = teacher(handler)
    with LOCK:
        db = load()
        if method == "GET" and path == "items":
            return 200, {"items": db["items"], "admin": is_admin(handler) if handler.headers.get("X-Admin-Password") else False}
        if method == "GET" and path == "approved.json":
            return 200, approved_export(db)
        if method != "POST":
            return 404, {"error": "not found"}
        if path in ("new", "flag") and not who:
            return 400, {"error": "Type your name at the top first."}
        now = time.strftime("%Y-%m-%d %H:%M")
        if path == "new":
            q = read_question(body)
            problem = question_problem(q)
            if problem:
                return 400, {"error": problem}
            q["source"] = f"Staff question · {who}"
            item = {"id": db["next"], "kind": "new", "status": "pending", "by": who, "at": now, "question": q}
        elif path == "flag":
            action = body.get("action")
            if action not in ACTIONS:
                return 400, {"error": "Choose what is wrong."}
            target = clean(body.get("target"), 80)
            if not target:
                return 400, {"error": "No question chosen."}
            item = {"id": db["next"], "kind": "flag", "status": "pending", "by": who, "at": now, "target": target,
                    "action": action, "note": clean(body.get("note"), 500)}
            if action in ("reword", "retag"):
                q = read_question(body.get("question") or {})
                problem = question_problem(q)
                if problem:
                    return 400, {"error": problem}
                q["source"] = f"Staff edit of {target} · {who}"
                item["question"] = q
        elif path == "review":
            if not is_admin(handler):
                return 403, {"error": "Admin password needed."}
            for it in db["items"]:
                if it["id"] == body.get("id") and body.get("decision") in ("approved", "rejected"):
                    it["status"], it["reviewed"] = body["decision"], now
                    save(db)
                    return 200, {"ok": True}
            return 404, {"error": "No such item."}
        else:
            return 404, {"error": "not found"}
        db["next"] += 1
        db["items"].append(item)
        save(db)
        return 200, {"ok": True, "id": item["id"]}
