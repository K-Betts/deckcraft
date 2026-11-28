#!/usr/bin/env python3
"""Capture the staff guide's screenshots from a clean copy of the app (no school data):
   python3 docs/staff-guide/capture.py target/release/deckcraft
Runs the app with a throwaway HOME so nothing of the real settings or curriculum shows."""
import json, os, socket, subprocess, sys, tempfile, time

app = sys.argv[1]
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "img")
home = tempfile.mkdtemp()
env = {**os.environ, "HOME": home}
port = 7991
proc = subprocess.Popen([app, "--control", str(port)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(60):
        try:
            sock = socket.create_connection(("127.0.0.1", port)); break
        except OSError:
            time.sleep(0.5)
    else:
        sys.exit("the app did not start")
    f = sock.makefile("rw")
    n = 0
    def call(method, params=None):
        global n
        n += 1
        f.write(json.dumps({"id": n, "method": method, "params": params or {}}) + "\n"); f.flush()
        r = json.loads(f.readline())
        return r
    def shot(name, wait=1.0):
        time.sleep(wait)
        r = call("ui.screenshot", {"path": os.path.join(out, name + ".png")})
        print(name, "ok" if r.get("ok") else r.get("error"))
    call("ui.resize", {"width": 1280, "height": 800})
    time.sleep(2)
    shot("home")
    call("app.pack", {"id": "maths", "on": True})
    call("engine.execute", {"command": "app.home", "params": {"on": False}})
    call("ui.set", {"tab": "Insert"})
    call("ui.dialog.open", {"id": "getReady", "fields": {"mode": "choose"}})
    shot("get-ready-dialog")
    call("ui.dialog.close", {"id": "getReady"})
    call("ui.dialog.open", {"id": "examQuestion"})
    shot("exam-question")
    call("ui.dialog.close", {"id": "examQuestion"})
finally:
    proc.terminate()
