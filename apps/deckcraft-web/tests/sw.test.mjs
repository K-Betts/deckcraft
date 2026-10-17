// Tests for sw.js against a fake browser: `node apps/deckcraft-web/tests/sw.test.mjs`.
// (No browser or test framework needed. It plays the roles of the cache, the network and the page.)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const code = readFileSync(new URL("../sw.js", import.meta.url), "utf8");
const BASE = "https://school.example/deckcraft/";

function fakeBrowser(site) {
  const store = new Map();
  const cache = {
    async put(req, res) { store.set(typeof req === "string" ? req : req.url, res.clone()); },
    async match(req) { const r = store.get(typeof req === "string" ? req : req.url); return r ? r.clone() : undefined; },
    async keys() { return [...store.keys()].map((url) => ({ url })); },
    async delete(req) { return store.delete(typeof req === "string" ? req : req.url); },
  };
  const state = { online: true, fetched: [] };
  const listeners = {};
  const self = {
    registration: { scope: BASE },
    location: { origin: new URL(BASE).origin },
    addEventListener: (t, f) => (listeners[t] = f),
    skipWaiting: async () => {},
    clients: { claim: async () => {} },
  };
  const caches = { open: async () => cache, keys: async () => ["deckcraft-app"] };
  const fetchFn = async (input) => {
    const url = typeof input === "string" ? input : input.url;
    state.fetched.push(url);
    if (!state.online) throw new TypeError("offline");
    const hit = site.files[url];
    return hit === undefined ? new Response("nope", { status: 404 }) : new Response(hit);
  };
  new Function("self", "caches", "fetch", "URL", "Response", code)(self, caches, fetchFn, URL, Response);
  const run = async (type, init = {}) => {
    const waits = [];
    let responded;
    const event = { ...init, waitUntil: (p) => waits.push(p), respondWith: (p) => (responded = p) };
    listeners[type](event);
    await Promise.all(waits);
    const res = responded ? await responded : undefined;
    await Promise.all(waits);
    return res;
  };
  return { state, store, run };
}

const page = (hash) => `<link rel="manifest" href="manifest.webmanifest"><link rel="icon" href="icon-192.png">
<script>navigator.serviceWorker.register("./sw.js")</script>
<script type="module">import init from './deckcraft-web-${hash}.js'; init('./deckcraft-web-${hash}_bg.wasm');</script>`;
const build = (hash) => ({
  files: {
    [BASE]: page(hash),
    [BASE + `deckcraft-web-${hash}.js`]: "js " + hash,
    [BASE + `deckcraft-web-${hash}_bg.wasm`]: "wasm " + hash,
    [BASE + "manifest.webmanifest"]: "{}",
    [BASE + "icon-192.png"]: "png192",
    [BASE + "icon-512.png"]: "png512",
    [BASE + "sw.js"]: code,
  },
});
const get = (url, extra = {}) => ({ request: { method: "GET", url, mode: "same-origin", ...extra } });
const nav = (url) => get(url, { mode: "navigate" });
const cachedUrls = (b) => [...b.store.keys()].map((u) => u.replace(BASE, "")).sort();

// 1. Installing caches the page and exactly what it names (and the 512 icon), never the worker itself.
const site = build("aaa");
const b = fakeBrowser(site);
await b.run("install");
assert.deepEqual(cachedUrls(b), ["", "deckcraft-web-aaa.js", "deckcraft-web-aaa_bg.wasm", "icon-192.png", "icon-512.png", "manifest.webmanifest"]);

// 2. Offline: the page and its files still load, from the cache.
b.state.online = false;
b.state.fetched.length = 0;
assert.equal(await (await b.run("fetch", nav(BASE))).text(), page("aaa"));
assert.equal(await (await b.run("fetch", get(BASE + "deckcraft-web-aaa_bg.wasm"))).text(), "wasm aaa");
assert.equal(b.state.fetched.filter((u) => u.endsWith(".wasm")).length, 0, "the cached wasm is not fetched again");

// 3. Online with a new build published: the page is taken from the network, the cache moves to
//    the new files and drops the old ones.
Object.assign(site.files, build("bbb").files);
delete site.files[BASE + "deckcraft-web-aaa.js"];
delete site.files[BASE + "deckcraft-web-aaa_bg.wasm"];
b.state.online = true;
const res = await b.run("fetch", nav(BASE));
assert.equal(await res.text(), page("bbb"), "the update is served as soon as the app is opened online");
assert.deepEqual(cachedUrls(b), ["", "deckcraft-web-bbb.js", "deckcraft-web-bbb_bg.wasm", "icon-192.png", "icon-512.png", "manifest.webmanifest"]);

// 4. Other sites' requests and non-GETs are left alone.
assert.equal(await b.run("fetch", { request: { method: "POST", url: BASE, mode: "same-origin" } }), undefined);
assert.equal(await b.run("fetch", get("https://elsewhere.example/x.js")), undefined);

// 5. A failed install (server down) leaves nothing half-cached and does not crash the worker.
const down = fakeBrowser(build("ccc"));
down.state.online = false;
await assert.rejects(down.run("install"));
assert.deepEqual(cachedUrls(down), []);

console.log("sw.js: all checks passed");
