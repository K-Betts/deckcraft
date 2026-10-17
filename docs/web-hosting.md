# Putting DeckCraft on the web (free)

DeckCraft's web version is a folder of static files (no server code), so any free static host
works. Nothing about a deck is ever sent to the host: decks open and save as files on the
teacher's own device, and the only thing the browser stores is the teacher's preferences.

## What you get

- **One link for everyone**, on any device with a modern browser (Chrome, Edge, Safari, Firefox).
  No install, no admin rights, no Apple or Windows signing.
- **Installable and offline.** Chrome and Edge offer *Install DeckCraft* (a desktop-style window);
  on iPad, Share ▸ Add to Home Screen. After one visit it works with no internet.
- **Updates by themselves.** Publish a new version and the next time anyone opens the app with
  internet they get it (the previous version keeps working until then). Nothing to press.

## The catches (be honest with staff)

- **Saving is a download.** The browser can't write over the file you opened. *Save* downloads a
  fresh `.pptx` to Downloads, which you then move or replace. (Chrome and Edge could later be
  given true "save in place"; Safari and iPad can't.) On a Mac or Windows PC with Google Drive
  for Desktop, saving to the Drive folder works well.
- **No Google Drive sign-in in the web version yet.** The Drive feature in the Mac app uses the
  system keychain; the web version needs its own browser sign-in (later).
- **First load is big:** about 36 MB, about 14 MB once the host compresses it. After that it is cached.
- **Safari / iPad:** works in principle (WebGPU or WebGL2), but I have only tested Chromium so far.

## Where to host it, free

| Host | Free? | Who can open it | Notes |
|---|---|---|---|
| **GitHub Pages** (recommended to start) | Yes | Anyone with the link (the repository is public) | Workflow included: `.github/workflows/web.yml`. Address: `https://k-betts.github.io/deckcraft/`. Compresses the wasm for you. |
| **Cloudflare Pages** | Yes | Anyone, **or only people you allow** with *Cloudflare Access* (free for up to 50 users, e.g. only school Google accounts) | Best if the school wants it not to be public. Connect the repo, build command below, output `dist/web`. |
| **Netlify / Firebase Hosting** | Yes (generous limits) | Anyone | Same idea. `packaging/web/_headers` is already in the Netlify/Cloudflare format. |
| **The school's own web server** | Their cost | Whoever IT chooses | Unzip `deckcraft-web-<version>.zip` (from `packaging/web/package.sh`) and see `packaging/web/README.md` for the two settings it needs. |

The app contains no pupil data and no secrets, so a public address is not a data risk, but it
does mean anyone who finds the link can use the app. If that matters, use Cloudflare Access.

## Publishing to GitHub Pages (one time, then one click)

1. GitHub ▸ the repository ▸ **Settings ▸ Pages ▸ Build and deployment ▸ Source: GitHub Actions**.
   (Until you do this, nothing can be published. I did not change your repository settings.)
2. **Actions ▸ Publish web app ▸ Run workflow.** About 10 minutes the first time. When it finishes
   the address appears in the run summary.
3. To publish an update later: merge to `main`, then run the workflow again.

## Building it yourself

```bash
cd apps/deckcraft-web && trunk build --release     # output: dist/web
python3 -m http.server 8765 --directory dist/web   # then open http://localhost:8765/
```

The offline and update logic is `apps/deckcraft-web/sw.js`; check it with
`node apps/deckcraft-web/tests/sw.test.mjs`.

## What this does not do yet

- The iPad-draws-and-the-board-shows idea needs a live link between two devices (a small free relay
  or a direct browser-to-browser connection). That is the next web phase (W3 in `MATHS.md`).
- Save in place and Google Drive in the browser (see above).
