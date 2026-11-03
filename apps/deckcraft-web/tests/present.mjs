// The presenter tab and its presentation window, in headless Chrome: `?sample&presenttest` starts
// a show with Presenter View on; the tab must open a second window, hand it the deck, and show
// the presenter view while that window plays the show; Escape in the window ends it, closes it
// and brings the editor back. Fails on any console error or a step that does not happen.
//
//   node apps/deckcraft-web/tests/present.mjs [SITE_DIR]      (default dist/web)
//
// CHROME=/path/to/chrome picks the browser.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join, resolve } from "node:path";

const site = resolve(process.argv[2] || "dist/web");
const TIMEOUT_MS = Number(process.env.PRESENT_TIMEOUT_MS || 90_000);
const IGNORED = [/service ?worker/i, /favicon/i];
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png", ".webmanifest": "application/manifest+json" };

function chrome() {
  const found = [process.env.CHROME, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/usr/bin/chromium", "/usr/bin/chromium-browser"]
    .filter(Boolean)
    .find((p) => existsSync(p));
  if (!found) throw new Error("no Chrome found: set CHROME=/path/to/chrome");
  return found;
}
function serve() {
  const server = createServer((req, res) => {
    let path = join(site, decodeURIComponent(new URL(req.url, "http://x").pathname));
    if (!path.startsWith(site)) return res.writeHead(403).end();
    if (existsSync(path) && statSync(path).isDirectory()) path = join(path, "index.html");
    if (!existsSync(path)) return res.writeHead(404).end();
    res.writeHead(200, { "Content-Type": TYPES[extname(path)] || "application/octet-stream" });
    res.end(readFileSync(path));
  });
  return new Promise((ok) => server.listen(0, "127.0.0.1", () => ok(server)));
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function devtools(profile) {
  const file = join(profile, "DevToolsActivePort");
  for (let i = 0; i < 200; i++) {
    if (existsSync(file)) {
      const [port] = readFileSync(file, "utf8").split("\n");
      if (port) return Number(port);
    }
    await sleep(100);
  }
  throw new Error("Chrome did not start");
}
async function attach(target, onLine) {
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((ok, bad) => ((ws.onopen = ok), (ws.onerror = bad)));
  let id = 0;
  const waiting = new Map();
  const text = (args) => args.map((a) => a.value ?? a.description ?? "").join(" ");
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && waiting.has(msg.id)) return waiting.get(msg.id)(msg), waiting.delete(msg.id);
    if (msg.method === "Runtime.consoleAPICalled") onLine(msg.params.type, text(msg.params.args));
    if (msg.method === "Runtime.exceptionThrown") onLine("error", msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text);
    if (msg.method === "Log.entryAdded") onLine(msg.params.entry.level, `${msg.params.entry.text} ${msg.params.entry.url || ""}`);
  };
  const send = (method, params = {}) => new Promise((ok) => (waiting.set(++id, ok), ws.send(JSON.stringify({ id, method, params }))));
  await send("Runtime.enable");
  await send("Log.enable");
  return { send, close: () => ws.close() };
}

if (!existsSync(join(site, "index.html"))) {
  console.error(`no built site in ${site}: run trunk build --release in apps/deckcraft-web first`);
  process.exit(2);
}
const server = await serve();
const url = `http://127.0.0.1:${server.address().port}/?sample&presenttest&webgl`;
const profile = mkdtempSync(join(tmpdir(), "deckcraft-present-"));
const browser = spawn(chrome(), ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", "--enable-unsafe-swiftshader", "--disable-popup-blocking", "--window-size=1600,900", "about:blank"], { stdio: "ignore" });
const lines = [];
const problems = [];
const attached = new Set();
const sessions = [];
const onLine = (who) => (level, line) => {
  if (process.env.PRESENT_DEBUG) console.log(`[${who} ${level}] ${line}`);
  if (IGNORED.some((r) => r.test(line))) return;
  lines.push(line);
  if (level === "error" || line.includes("panicked")) problems.push(`${who}: ${line}`);
};
const seen = (text) => lines.some((l) => l.includes(text));
async function pages(port) {
  const targets = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).filter((t) => t.type === "page" && !t.url.startsWith("about:"));
  for (const t of targets) {
    if (attached.has(t.id)) continue;
    attached.add(t.id);
    const who = t.url.includes("deckcraft-audience") ? "window" : "tab";
    sessions.push({ who, id: t.id, ...(await attach(t, onLine(who))) });
  }
  return targets;
}
async function waitFor(what, port, ok) {
  const end = Date.now() + TIMEOUT_MS;
  while (Date.now() < end) {
    const targets = await pages(port);
    if (ok(targets)) return true;
    await sleep(300);
  }
  problems.push(`did not happen in time: ${what}`);
  return false;
}
try {
  const port = await devtools(profile);
  const first = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === "page");
  const tab = await attach(first, onLine("tab"));
  attached.add(first.id);
  sessions.push({ who: "tab", id: first.id, ...tab });
  await tab.send("Page.navigate", { url });
  const steps = [
    ["a second window opens", (t) => t.filter((x) => x.url.includes("deckcraft-audience")).length === 1],
  ];
  if (await waitFor(steps[0][0], port, steps[0][1])) {
    await waitFor("the window starts the show", port, () => seen("presentation window: show started"));
    await waitFor("the tab shows the presenter view", port, () => seen("presenter view: connected"));
  }
  const popup = sessions.find((s) => s.who === "window");
  if (popup && !problems.length) {
    // The audience presses Escape: the show ends, the window closes, the tab is the editor again.
    for (const type of ["rawKeyDown", "keyUp"]) await popup.send("Input.dispatchKeyEvent", { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await waitFor("Escape closes the presentation window", port, (t) => !t.some((x) => x.url.includes("deckcraft-audience")));
  }
} finally {
  const exited = new Promise((ok) => browser.once("exit", ok));
  browser.kill();
  await Promise.race([exited, sleep(5000)]);
  try {
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  } catch {}
  server.close();
}
for (const p of problems) console.error(`  problem: ${p}`);
console.log(problems.length ? "present test FAILED" : "present test passed");
process.exit(problems.length ? 1 : 0);
