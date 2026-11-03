"""The school question bank: staff pages served by serve-private.py under /bank/.

Everyone with the staff password (the whole site's password) can browse the bank and *suggest*
changes. People with an editor login (made by an admin) edit the live bank directly: add
questions, replace or retire built-in ones. Admins also manage logins. The DeckCraft web app reads
the live bank from /bank/api/live.json (staff password only) and applies it on top of its built-in
questions.

Python standard library only. Data: $DECKCRAFT_BANK_DATA (default ~/deckcraft-web/bank-data):
bank.json (live bank, suggestions, history) and users.json (logins, salted PBKDF2).
"""

import hmac
import json
import os
import re
import secrets
import threading
import time

DATA_DIR = os.environ.get("DECKCRAFT_BANK_DATA", os.path.expanduser("~/deckcraft-web/bank-data"))
HERE = os.path.dirname(os.path.abspath(__file__))
LEGACY_ADMIN = os.path.join(HERE, "bank-admin.json")  # from the first version: one admin password
PAGE = os.path.join(HERE, "bank_page.html")
TOPICS = {"Number", "Algebra", "Ratio", "Geometry", "Statistics", "Probability"}
# The department's strands and skills, and the bank's own topic each strand belongs to.
STRAND_TOPIC = {"01_Number": "Number", "02_Algebra": "Algebra", "03_Ratio_Proportion_Rates": "Ratio",
                "04_Geometry_Measures": "Geometry", "05_Probability": "Probability", "06_Statistics": "Statistics"}


def _skills():
    try:
        with open(os.path.join(HERE, "bank_taxonomy.json"), encoding="utf-8") as f:
            return {k["id"]: (s["id"], k["name"]) for s in json.load(f) for k in s["skills"]}
    except (OSError, ValueError):
        return {}
YEARS = {"Y7", "Y8", "Y9", "Y10", "Y11"}
ACTIONS = {"retire", "wrong", "reword", "retag"}
ROLES = {"editor", "admin"}
MAX_BODY = 64 * 1024
SESSION_SECONDS = 12 * 3600
LOCK = threading.Lock()
SESSIONS = {}  # token -> (username, expires)
derive = None  # set by serve-private.py (its password hashing)


def clean(v, limit):
    v = re.sub(r"[\x00-\x08\x0b-\x1f]", "", str(v or "")).strip()
    return v[:limit]


def _read(name, default):
    try:
        with open(os.path.join(DATA_DIR, name), encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def _write(name, obj):
    os.makedirs(DATA_DIR, mode=0o700, exist_ok=True)
    tmp = os.path.join(DATA_DIR, name + ".tmp")
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=1)
    os.replace(tmp, os.path.join(DATA_DIR, name))


def load():
    db = _read("bank.json", {})
    db.setdefault("next", 1)
    db.setdefault("items", [])  # suggestions from anyone
    db.setdefault("added", {})  # staff-<n> -> question
    db.setdefault("replaced", {})  # built-in id -> question that takes its place
    db.setdefault("retired", [])  # built-in ids taken out
    db.setdefault("recat", {})  # built-in id -> skill id it was moved to
    db.setdefault("tags", {})  # curriculum objective -> skill keys that practise it
    db.setdefault("history", [])
    db.setdefault("revision", 0)
    return db


def users():
    u = _read("users.json", None)
    if u is None and os.path.isfile(LEGACY_ADMIN):
        with open(LEGACY_ADMIN) as f:
            a = json.load(f)
        u = {"admin": {"name": "Admin", "role": "admin", "salt": a["salt"], "hash": a["hash"]}}
    return u or {}


def save_user(username, name, role, password):
    salt = secrets.token_bytes(16)
    u = users()
    u[username] = {"name": name, "role": role, "salt": salt.hex(), "hash": derive(password, salt)}
    _write("users.json", u)


# ---- questions --------------------------------------------------------------------------------

def read_question(d):
    try:
        diff = int(d.get("difficulty"))
    except (TypeError, ValueError):
        diff = 0
    kw = d.get("keywords") or []
    if isinstance(kw, str):
        kw = kw.split(",")
    cat = clean(d.get("category"), 8).upper()
    skills = _skills()
    topic = clean(d.get("topic"), 40)
    sub = clean(d.get("subtopic"), 120)
    if cat in skills:
        # The strand and skill the teacher chose give the topic and the skill name the app lists.
        topic = STRAND_TOPIC.get(skills[cat][0], topic)
        sub = sub or skills[cat][1]
    else:
        cat = ""
    return {
        "category": cat,
        "topic": topic,
        "subtopic": sub,
        "difficulty": diff,
        "year": clean(d.get("year"), 4).upper(),
        "text": clean(d.get("text"), 2000),
        "answer": clean(d.get("answer"), 500),
        "keywords": [clean(k, 40) for k in kw if clean(k, 40)][:10] if isinstance(kw, list) else [],
    }


def question_problem(q):
    """What is wrong with a question a teacher typed, or None."""
    if len(q["text"]) < 5:
        return "The question needs some text."
    if not q["answer"]:
        return "The question needs an answer."
    if not q["category"]:
        return "Choose a strand and a skill."
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


def live_export(db):
    """What the DeckCraft apps load: added and replacing questions, and the built-ins taken out."""
    mine = [{"id": i, **q} for i, q in db["added"].items()]
    removed = list(db["retired"])
    for target, q in db["replaced"].items():
        mine.append({"id": f"staff-r-{target}", **q})
        removed.append(target)
    return {"version": 1, "revision": db["revision"], "questions": mine, "templates": [],
            "myQuestions": mine, "removedQuestions": sorted(set(removed)), "curriculumTags": db["tags"]}


# ---- people -----------------------------------------------------------------------------------

def session(handler):
    token = handler.headers.get("X-Token", "")
    s = SESSIONS.get(token)
    if not s or s[1] < time.time():
        SESSIONS.pop(token, None)
        return None
    u = users().get(s[0])
    return {"username": s[0], **u} if u else None


def login(body):
    username = clean(body.get("username"), 40).lower()
    u = users().get(username)
    given = str(body.get("password") or "")
    ok = bool(u) and hmac.compare_digest(derive(given, bytes.fromhex(u["salt"])), u["hash"])
    if not ok:
        time.sleep(1)
        return 403, {"error": "Wrong user name or password."}
    token = secrets.token_hex(24)
    SESSIONS[token] = (username, time.time() + SESSION_SECONDS)
    return 200, {"token": token, "name": u["name"], "role": u["role"], "username": username}


def log(db, who, what, target=""):
    db["history"].append({"at": time.strftime("%Y-%m-%d %H:%M"), "by": who, "what": what, "target": target})
    del db["history"][:-500]
    db["revision"] += 1


# ---- the API ----------------------------------------------------------------------------------

def api(handler, method, path, body):
    """Returns (status, json-able). `path` is what follows /bank/api/."""
    if method == "POST" and path == "login":
        return login(body)
    with LOCK:
        db = load()
        me = session(handler)
        editor = bool(me)  # every login may edit; admins also manage logins
        admin = bool(me) and me["role"] == "admin"
        if method == "GET":
            if path == "live.json":
                return 200, live_export(db)
            if path == "me":
                return 200, {"name": me["name"], "role": me["role"], "username": me["username"]} if me else {"name": None}
            if path == "items":
                return 200, {"items": db["items"], "added": db["added"] if editor else {}, "replaced": db["replaced"],
                             "recat": db["recat"], "tags": db["tags"],
                             "retired": db["retired"], "history": db["history"][-50:] if editor else []}
            if path == "users" and admin:
                return 200, {"users": [{"username": k, "name": v["name"], "role": v["role"]} for k, v in users().items()]}
            return 404, {"error": "not found"}
        if method != "POST":
            return 404, {"error": "not found"}
        now = time.strftime("%Y-%m-%d %H:%M")
        who = me["name"] if me else clean(handler.headers.get("X-Teacher", ""), 40)

        # Anyone with the staff password may suggest.
        if path in ("suggest/new", "suggest/flag"):
            if len(who) < 2:
                return 400, {"error": "Type your name at the top first."}
            if path == "suggest/new":
                q = read_question(body)
                problem = question_problem(q)
                if problem:
                    return 400, {"error": problem}
                q["addedBy"] = who
                item = {"id": db["next"], "kind": "new", "status": "pending", "by": who, "at": now, "question": q}
            else:
                action = body.get("action")
                target = clean(body.get("target"), 80)
                if action not in ACTIONS or not target:
                    return 400, {"error": "Choose what is wrong."}
                item = {"id": db["next"], "kind": "flag", "status": "pending", "by": who, "at": now, "target": target,
                        "action": action, "note": clean(body.get("note"), 500)}
                if action in ("reword", "retag"):
                    q = read_question(body.get("question") or {})
                    problem = question_problem(q)
                    if problem:
                        return 400, {"error": problem}
                    q["addedBy"], q["replaces"] = who, target
                    item["question"] = q
            db["next"] += 1
            db["items"].append(item)
            _write("bank.json", db)
            return 200, {"ok": True, "id": item["id"]}

        # Everything below needs a login.
        if not editor:
            return 403, {"error": "Log in as an editor first."}

        if path == "edit/save":  # add a question, or change one of ours
            q = read_question(body)
            problem = question_problem(q)
            if problem:
                return 400, {"error": problem}
            qid = clean(body.get("id"), 40)
            q["addedBy"] = me["name"]
            if qid and qid.startswith("staff-") and qid in db["added"]:
                db["added"][qid] = q
                log(db, me["name"], "edited", qid)
            else:
                qid = f"staff-{db['next']}"
                db["next"] += 1
                db["added"][qid] = q
                log(db, me["name"], "added", qid)
        elif path == "edit/replace":  # a fixed question takes the place of a built-in one
            target = clean(body.get("target"), 80)
            q = read_question(body)
            problem = question_problem(q)
            if not target or problem:
                return 400, {"error": problem or "No question chosen."}
            q["addedBy"], q["replaces"] = me["name"], target
            db["replaced"][target] = q
            log(db, me["name"], "replaced", target)
        elif path == "edit/retire":
            target = clean(body.get("target"), 80)
            if not target:
                return 400, {"error": "No question chosen."}
            if target not in db["retired"]:
                db["retired"].append(target)
            log(db, me["name"], "retired", target)
        elif path == "edit/restore":  # undo a replacement or a retirement
            target = clean(body.get("target"), 80)
            db["replaced"].pop(target, None)
            if target in db["retired"]:
                db["retired"].remove(target)
            log(db, me["name"], "restored", target)
        elif path == "edit/move":  # put questions in another skill (built-in ones, or our own)
            cat = clean(body.get("category"), 8).upper()
            targets = [clean(t, 80) for t in (body.get("targets") or []) if clean(t, 80)][:300]
            if cat not in _skills() or not targets:
                return 400, {"error": "Choose a skill to move them to."}
            for t in targets:
                if t in db["added"]:
                    db["added"][t]["category"] = cat
                    db["added"][t]["topic"] = STRAND_TOPIC.get(_skills()[cat][0], db["added"][t]["topic"])
                else:
                    db["recat"][t] = cat
            log(db, me["name"], f"moved {len(targets)} to {cat}:", targets[0])
        elif path == "tags/save":  # which questions practise a curriculum objective
            key = clean(body.get("key"), 400)
            keys = [clean(k, 200) for k in (body.get("skills") or []) if clean(k, 200)][:40]
            if not key:
                return 400, {"error": "No objective chosen."}
            if keys:
                db["tags"][key] = keys
            else:
                db["tags"].pop(key, None)
            log(db, me["name"], "tagged", key[:60])
        elif path == "edit/delete":
            qid = clean(body.get("id"), 40)
            if db["added"].pop(qid, None) is None:
                return 404, {"error": "No such question."}
            log(db, me["name"], "deleted", qid)
        elif path == "review":  # approve or reject a suggestion
            for it in db["items"]:
                if it["id"] == body.get("id") and it["status"] == "pending" and body.get("decision") in ("approved", "rejected"):
                    it["status"], it["reviewed"], it["reviewer"] = body["decision"], now, me["name"]
                    if it["status"] == "approved":
                        _apply(db, it, me["name"])
                    _write("bank.json", db)
                    return 200, {"ok": True}
            return 404, {"error": "No such suggestion."}
        elif admin and path == "users/save":
            username = clean(body.get("username"), 40).lower()
            name, role, password = clean(body.get("name"), 40), body.get("role"), str(body.get("password") or "")
            if not re.fullmatch(r"[a-z0-9._-]{2,40}", username) or len(name) < 2 or role not in ROLES:
                return 400, {"error": "Give a user name (letters and digits), a name and a role."}
            if len(password) < 8 and username not in users():
                return 400, {"error": "A new login needs a password of at least 8 characters."}
            if password:
                save_user(username, name, role, password)
            else:
                u = users()
                u[username].update(name=name, role=role)
                _write("users.json", u)
            return 200, {"ok": True}
        elif admin and path == "users/remove":
            username = clean(body.get("username"), 40).lower()
            u = users()
            if username == me["username"] or username not in u:
                return 400, {"error": "You cannot remove yourself."}
            del u[username]
            _write("users.json", u)
            return 200, {"ok": True}
        else:
            return 404, {"error": "not found"}
        _write("bank.json", db)
        return 200, {"ok": True, "id": qid if path == "edit/save" else None, "revision": db["revision"]}


def _apply(db, it, who):
    """An approved suggestion goes into the live bank."""
    if it["kind"] == "new":
        qid = f"staff-{db['next']}"
        db["next"] += 1
        db["added"][qid] = it["question"]
        log(db, who, f"approved suggestion #{it['id']} by {it['by']}: added", qid)
    elif it["action"] in ("retire", "wrong"):
        if it["target"] not in db["retired"]:
            db["retired"].append(it["target"])
        log(db, who, f"approved suggestion #{it['id']} by {it['by']}: retired", it["target"])
    elif it.get("question"):
        db["replaced"][it["target"]] = it["question"]
        log(db, who, f"approved suggestion #{it['id']} by {it['by']}: replaced", it["target"])
