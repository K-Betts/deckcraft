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

## Hosting on a Raspberry Pi (free, your own hardware)

Any Pi from a Pi 3 up will do: it only hands out files, and the app runs in each teacher's
browser. The one thing a Pi cannot give you on its own is **https**, and without https the
browser turns off offline use, installing the app, the clipboard and WebGPU (it still runs, more
slowly, on WebGL2). So pick how people will reach it first:

| Who opens it | How | https? | Cost |
|---|---|---|---|
| Only you, on your home network | `http://deckcraft-pi.local` | No (works, but no offline/install) | Free |
| Anyone with the link, anywhere | Tailscale Funnel gives the Pi a fixed `https://….ts.net` address | Yes | Free |
| Only people on your Tailscale network | Tailscale (no Funnel) | Yes | Free |

Cloudflare Tunnel also works but needs a domain name of your own, which costs money.

### 1. Set up the Pi (once)

1. Flash **Raspberry Pi OS Lite (64-bit)** with Raspberry Pi Imager. In its settings (the cog), set
   the hostname to `deckcraft-pi`, turn on SSH, and set your username (these steps use `pi`).
2. Boot it on your network, then from the Mac:
   ```bash
   ssh pi@deckcraft-pi.local
   ```
3. On the Pi, install Caddy (a small web server) and make the site folder:
   ```bash
   sudo apt update && sudo apt install -y caddy
   sudo mkdir -p /var/www/deckcraft && sudo chown pi:pi /var/www/deckcraft
   ```
4. Replace Caddy's settings with these (`sudo nano /etc/caddy/Caddyfile`, paste, save):
   ```
   :80 {
       root * /var/www/deckcraft
       encode zstd gzip
       file_server
       header X-Content-Type-Options nosniff
       @hashed path_regexp \-[0-9a-f]{16}(_bg)?\.(wasm|js)$
       header @hashed Cache-Control "public, max-age=31536000, immutable"
       @fresh path / /index.html /sw.js /manifest.webmanifest
       header @fresh Cache-Control "no-cache"
   }
   ```
   then `sudo systemctl reload caddy`. (`sw.js` must not be cached, or updates stop arriving.)

### 2. Publish a version (each time)

On the Mac, from the repository:

```bash
cd apps/deckcraft-web && trunk build --release && cd ../..
rsync -av --delete dist/web/ pi@deckcraft-pi.local:/var/www/deckcraft/
```

Open `http://deckcraft-pi.local` to check it. Teachers who already have it open get the new
version the next time they open the app.

### 3. Give it an https address with Tailscale (optional)

1. Make a free account at tailscale.com, then on the Pi:
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   sudo tailscale up
   ```
   (open the link it prints and sign in).
2. In the Tailscale admin console, turn on **HTTPS certificates** (DNS page) and allow **Funnel**
   for this machine (Access controls; the console offers a one-click button).
3. On the Pi:
   ```bash
   sudo tailscale funnel --bg 80
   ```
   It prints the address, e.g. `https://deckcraft-pi.<your-tailnet>.ts.net/`. That is the link to
   share. `sudo tailscale funnel reset` takes it off the internet again. Leave out step 3 to keep
   it to devices signed in to your Tailscale.

The Pi needs to stay on for the address to work; GitHub Pages does not need anything running.

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
