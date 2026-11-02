#!/usr/bin/env python3
"""Serve a private DeckCraft web build (e.g. one with the school's curriculum built in) behind a
staff password, with nothing to install beyond Python 3.

    python3 serve-private.py --set-password            # once: choose the user name and password
    python3 serve-private.py SITE_DIR [--port 8790]    # serve SITE_DIR on 127.0.0.1

It listens on 127.0.0.1 only; put https in front of it (Tailscale Funnel, see
docs/web-hosting.md). Every request needs the password (HTTP Basic). Files with a `.gz` copy
beside them are sent compressed; `.wasm` gets the type browsers need to compile it; the
content-hashed files are cached for good and the page, service worker and manifest never are.
"""

import argparse
import base64
import getpass
import hashlib
import hmac
import http.server
import json
import os
import re
import secrets
import sys
from functools import partial

import bank_portal

AUTH_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "password.json")
ITERATIONS = 200_000
HASHED = re.compile(r"-[0-9a-f]{16}(_bg)?\.(wasm|js|png)$")
# The exam question library's list files change when it is published again (its pictures do not).
NO_CACHE = {"/", "/index.html", "/sw.js", "/manifest.webmanifest", "/exam/items/exam/library.json", "/exam/crops.json"}


def derive(password, salt):
    return hashlib.pbkdf2_hmac("sha256", password.encode(), salt, ITERATIONS).hex()


def set_password():
    user = input("User name staff will type [staff]: ").strip() or "staff"
    password = getpass.getpass("Password: ")
    if len(password) < 8 or password != getpass.getpass("Again: "):
        sys.exit("The passwords differ or are shorter than 8 characters; nothing changed.")
    salt = secrets.token_bytes(16)
    fd = os.open(AUTH_FILE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as f:
        json.dump({"user": user, "salt": salt.hex(), "hash": derive(password, salt)}, f)
    print(f"Saved in {AUTH_FILE}. Restart the server to use it.")


def set_admin():
    password = getpass.getpass("Admin password for approving question changes: ")
    if len(password) < 8 or password != getpass.getpass("Again: "):
        sys.exit("The passwords differ or are shorter than 8 characters; nothing changed.")
    salt = secrets.token_bytes(16)
    fd = os.open(bank_portal.ADMIN_FILE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as f:
        json.dump({"salt": salt.hex(), "hash": derive(password, salt)}, f)
    print(f"Saved in {bank_portal.ADMIN_FILE}.")


bank_portal.derive = derive


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".webmanifest": "application/manifest+json",
    }

    def __init__(self, *args, auth, accepted, **kwargs):
        self.auth, self.accepted = auth, accepted
        super().__init__(*args, **kwargs)

    def allowed(self):
        header = self.headers.get("Authorization", "")
        if header in self.accepted:
            return True
        if not header.startswith("Basic "):
            return False
        try:
            user, _, password = base64.b64decode(header[6:]).decode().partition(":")
        except ValueError:
            return False
        ok = hmac.compare_digest(user, self.auth["user"]) & hmac.compare_digest(
            derive(password, bytes.fromhex(self.auth["salt"])), self.auth["hash"]
        )
        # The key derivation is slow on purpose; remember a right answer (bounded) rather than
        # repeat it for each of the page's requests.
        if ok and len(self.accepted) < 64:
            self.accepted.add(header)
        return ok

    def do_GET(self):
        self.serve(head=False)

    def do_HEAD(self):
        self.serve(head=True)

    def do_POST(self):
        if not self.allowed():
            return self.deny()
        url = self.path.split("?", 1)[0]
        # A custom header makes a browser ask permission first, so another site cannot post here.
        if not url.startswith("/bank/api/") or not self.headers.get("X-Teacher", "").strip():
            return self.send_error(404)
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= bank_portal.MAX_BODY:
                return self.send_error(413)
            body = json.loads(self.rfile.read(length))
            if not isinstance(body, dict):
                raise ValueError
        except ValueError:
            return self.send_error(400)
        self.reply_json(*bank_portal.api(self, "POST", url[len("/bank/api/"):], body))

    def deny(self):
        self.send_response(401)
        self.send_header("WWW-Authenticate", 'Basic realm="DeckCraft", charset="UTF-8"')
        self.send_header("Content-Length", "0")
        self.end_headers()

    def reply_json(self, status, obj):
        data = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        self.wfile.write(data)

    def serve(self, head):
        if not self.allowed():
            return self.deny()
        url = self.path.split("?", 1)[0]
        if url.startswith("/bank/api/"):
            return self.reply_json(*bank_portal.api(self, "GET", url[len("/bank/api/"):], None))
        if url in ("/bank", "/bank/"):
            with open(bank_portal.PAGE, "rb") as f:
                body = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            return None if head else self.wfile.write(body)
        path = self.translate_path(self.path)
        if os.path.isdir(path):
            path = os.path.join(path, "index.html")
        gz = path + ".gz"
        if not os.path.isfile(path) or os.path.islink(path):
            self.send_error(404)
            return
        compressed = "gzip" in self.headers.get("Accept-Encoding", "") and os.path.isfile(gz)
        with open(gz if compressed else path, "rb") as f:
            body = f.read()
        self.send_response(200)
        self.send_header("Content-Type", self.guess_type(path))
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Vary", "Accept-Encoding")
        self.send_header("X-Content-Type-Options", "nosniff")
        if compressed:
            self.send_header("Content-Encoding", "gzip")
        url = self.path.split("?", 1)[0]
        if url in NO_CACHE:
            self.send_header("Cache-Control", "no-cache")
        elif HASHED.search(url):
            self.send_header("Cache-Control", "private, max-age=31536000, immutable")
        self.end_headers()
        if not head:
            self.wfile.write(body)

    def log_message(self, fmt, *args):
        # The request line and status only: no addresses or credentials in the log.
        sys.stderr.write(f"{self.log_date_time_string()} {fmt % args}\n")


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("site", nargs="?", help="the built site (trunk's dist folder)")
    p.add_argument("--port", type=int, default=8790)
    p.add_argument("--set-admin", action="store_true", help="choose the password that approves question changes")
    p.add_argument("--set-password", action="store_true", help="choose the user name and password")
    a = p.parse_args()
    if a.set_password:
        return set_password()
    if a.set_admin:
        return set_admin()
    if not a.site or not os.path.isfile(os.path.join(a.site, "index.html")):
        sys.exit("Give the folder of a built site (it must hold index.html).")
    try:
        with open(AUTH_FILE) as f:
            auth = json.load(f)
    except OSError:
        sys.exit(f"No password yet: run `python3 {sys.argv[0]} --set-password` first.")
    handler = partial(Handler, auth=auth, accepted=set(), directory=os.path.abspath(a.site))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", a.port), handler)
    print(f"Serving {a.site} on http://127.0.0.1:{a.port}/ (password required)", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
