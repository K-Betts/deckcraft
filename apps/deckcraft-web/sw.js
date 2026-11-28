// DeckCraft's service worker: makes the web app work offline and keeps it up to date.
//
// - The page (index.html) is fetched from the network first, so a published update is picked up
//   the next time the app is opened online; offline it comes from the cache.
// - Everything the page needs (the app's .js and .wasm, icons) has a content hash in its name, so
//   once cached it is served from the cache and never re-downloaded until a new build names new files.
// - After each successful page fetch the cache is brought in line with that page: new files are
//   fetched, files the new page no longer uses are dropped.
// Decks and settings are never touched here; they stay in the browser's own storage and files.
"use strict";

const CACHE = "deckcraft-app";
const INDEX = new URL("./", self.registration.scope).href;
// What a build's page refers to: something.js|wasm|png|webmanifest, with or without a ./ (never a URL).
const ASSET = /["']((?:\.\/)?[^"'?#:\s]+\.(?:js|wasm|png|webmanifest))["']/g;

self.addEventListener("install", (event) => {
  event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

// Fetch the page and the files it names; keep exactly those.
async function precache() {
  const cache = await caches.open(CACHE);
  const res = await fetch(INDEX, { cache: "no-store" });
  if (!res.ok) throw new Error("index " + res.status);
  const html = await res.clone().text();
  const wanted = new Set([INDEX]);
  for (const m of html.matchAll(ASSET)) wanted.add(new URL(m[1], INDEX).href);
  // Named by the manifest rather than the page.
  wanted.add(new URL("icon-512.png", INDEX).href);
  // The worker is updated by the browser itself; it does not belong in its own cache.
  wanted.delete(new URL("sw.js", INDEX).href);
  for (const url of wanted) {
    if (url !== INDEX && !(await cache.match(url))) {
      const r = await fetch(url);
      if (r.ok) await cache.put(url, r);
    }
  }
  await cache.put(INDEX, res);
  for (const req of await cache.keys()) {
    if (!wanted.has(req.url)) await cache.delete(req);
  }
}

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET" || new URL(req.url).origin !== self.location.origin) return;
  const isPage = req.mode === "navigate" || new URL(req.url).href === INDEX || new URL(req.url).pathname.endsWith("/index.html");
  event.respondWith(isPage ? networkFirst(event) : cacheFirst(req));
});

async function networkFirst(event) {
  try {
    const res = await fetch(event.request, { cache: "no-store" });
    if (res.ok) event.waitUntil(precache().catch(() => {}));
    return res;
  } catch (_) {
    const cache = await caches.open(CACHE);
    return (await cache.match(INDEX)) || Response.error();
  }
}

async function cacheFirst(req) {
  const cache = await caches.open(CACHE);
  const hit = await cache.match(req);
  if (hit) return hit;
  const res = await fetch(req);
  if (res.ok) cache.put(req, res.clone());
  return res;
}
