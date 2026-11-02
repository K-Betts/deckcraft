// The web build's start-up check: serve a built site, open it in headless Chrome with `?smoke`
// (the app then walks through Home, Get Ready, Exam Question, the Question Library, a template,
// Teacher Mode and its maths tools; see src/smoke.rs), and fail on any console error, any failed
// step or a walk that does not finish. Nothing to install: Node 22+ and a Chrome or Chromium.
//
//   node apps/deckcraft-web/tests/smoke.mjs [SITE_DIR]      (default dist/web)
//
// CHROME=/path/to/chrome picks the browser; otherwise the usual places are tried. Each viewport
// in VIEWPORTS is checked (a laptop and a 4K board), and the steps' notes are printed.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join, resolve } from "node:path";

const site = resolve(process.argv[2] || "dist/web");
const VIEWPORTS = [
  [1920, 1080],
  [3840, 2160],
];
const TIMEOUT_MS = Number(process.env.SMOKE_TIMEOUT_MS || 120_000);
// The app's own notes that are not problems. The service worker cannot register on a plain-http
// test server in every browser; that is not what this checks.
const IGNORED = [/service ?worker/i, /favicon/i];

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png", ".webmanifest": "application/manifest+json" };

function chrome() {
  const tries = [
    process.env.CHROME,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
  ].filter(Boolean);
  const found = tries.find((p) => existsSync(p));
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

async function devtools(profile) {
  const file = join(profile, "DevToolsActivePort");
  for (let i = 0; i < 200; i++) {
    if (existsSync(file)) {
      const [port] = readFileSync(file, "utf8").split("\n");
      if (port) return Number(port);
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("Chrome did not start");
}

/// One page through the DevTools protocol: `send(method, params)` and a stream of events.
async function page(port, onEvent) {
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const target = targets.find((t) => t.type === "page");
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((ok, bad) => ((ws.onopen = ok), (ws.onerror = bad)));
  let id = 0;
  const waiting = new Map();
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && waiting.has(msg.id)) {
      waiting.get(msg.id)(msg);
      waiting.delete(msg.id);
    } else if (msg.method) onEvent(msg);
  };
  const send = (method, params = {}) =>
    new Promise((ok) => {
      waiting.set(++id, ok);
      ws.send(JSON.stringify({ id, method, params }));
    });
  return { send, close: () => ws.close() };
}

async function check(url, [width, height]) {
  const profile = mkdtempSync(join(tmpdir(), "deckcraft-smoke-"));
  const browser = spawn(chrome(), [
    "--headless=new",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--enable-unsafe-swiftshader",
    `--window-size=${width},${height}`,
    "about:blank",
  ], { stdio: "ignore" });
  const problems = [];
  const notes = [];
  let finished;
  const done = new Promise((ok) => (finished = ok));
  const text = (args) => args.map((a) => a.value ?? a.description ?? "").join(" ");
  const note = (level, line) => {
    if (process.env.SMOKE_DEBUG) console.log(`[${level}] ${line}`);
    if (IGNORED.some((r) => r.test(line))) return;
    if (line.includes("smoke:")) notes.push(line.slice(line.indexOf("smoke:")));
    if (level === "error" || line.includes("FAILED") || line.includes("panicked")) problems.push(line);
    if (line.includes("smoke: done")) finished();
  };
  try {
    const port = await devtools(profile);
    const p = await page(port, (msg) => {
      if (msg.method === "Runtime.consoleAPICalled") note(msg.params.type, text(msg.params.args));
      if (msg.method === "Runtime.exceptionThrown") note("error", msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text);
      if (msg.method === "Log.entryAdded") note(msg.params.entry.level, `${msg.params.entry.text} ${msg.params.entry.url || ""}`);
    });
    await p.send("Runtime.enable");
    await p.send("Log.enable");
    await p.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await p.send("Page.navigate", { url });
    const timeout = new Promise((ok) => setTimeout(() => ok("timeout"), TIMEOUT_MS));
    if ((await Promise.race([done, timeout])) === "timeout") problems.push("the walk did not finish in time");
    p.close();
  } finally {
    const exited = new Promise((ok) => browser.once("exit", ok));
    browser.kill();
    await Promise.race([exited, new Promise((ok) => setTimeout(ok, 5000))]);
    try {
      rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
    } catch {
      // A profile folder left in the temporary folder is harmless.
    }
  }
  return { problems, notes };
}

if (!existsSync(join(site, "index.html"))) {
  console.error(`no built site in ${site}: run trunk build --release in apps/deckcraft-web first`);
  process.exit(2);
}
const server = await serve();
const base = `http://127.0.0.1:${server.address().port}/`;
let failed = false;
for (const vp of VIEWPORTS) {
  const { problems, notes } = await check(`${base}?smoke&webgl`, vp);
  console.log(`${vp[0]}x${vp[1]}: ${notes.join(" | ") || "no steps logged"}`);
  for (const p of problems) console.error(`  problem: ${p}`);
  failed ||= problems.length > 0;
}
server.close();
console.log(failed ? "smoke test FAILED" : "smoke test passed");
process.exit(failed ? 1 : 0);
