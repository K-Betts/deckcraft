# Updating DeckCraft

## For teachers: the in-app update

1. **File ▸ Options ▸ Updates**: tick *Check for a newer version when DeckCraft starts* (it is off
   by default: DeckCraft makes no network calls unless you ask), or press *Check for updates now…*.
2. When a newer version exists, a blue **Update available** pill appears in the status bar (and a
   card on the Home screen). Click it, read what is new, press **Update now**.
3. It downloads, checks the file, and swaps itself in (a few seconds). Your open presentations are
   not touched. Press **Restart now** when you are ready (it asks you to save first).
4. The previous version is kept beside it as `DeckCraft.app.previous` in case you need to go back
   (drag it over `DeckCraft.app`).

What is sent: one request to `api.github.com` for the latest release, then the download from
`github.com`. No account, no id, no personal information.

Only the installed Mac app (`/Applications/DeckCraft.app`) can update itself. A build run from a
terminal with `cargo run` says so and does nothing.

## For you: publishing an update

You do not run anything. Merging a pull request into `main` starts two GitHub workflows by itself:

- **Publish Mac update** builds the Mac app on a free GitHub Mac and publishes a release named
  `build-<commit>` with the zip and its checksum (about 10–20 minutes). Everyone with update checks
  on sees *Update available* the next time they start DeckCraft.
- **Publish web app** builds the web version and puts it on GitHub Pages. The installed web app
  picks up the new version the next time it is opened online.

Merges that only change Markdown files or `docs/` publish nothing. The release text is the pull
request's title. To publish without a merge, or with your own wording, use GitHub ▸ **Actions** ▸
the workflow ▸ **Run workflow**. Releases are public (the repository is).

Check it worked: GitHub ▸ **Actions** shows both workflows going green, then **Releases** lists the
new `build-…` entry.

## How the app decides

The app knows the commit it was built from. It fetches the latest release, reads the commit out of
the asset name (`DeckCraft-macos-aarch64-<commit>.zip`) and offers the update if that commit is
different. The checksum file must match the zip or the download is thrown away. The new app's code
signature is checked before it replaces the old one, and if any step fails the old app stays.

## Not done yet

- Windows and Linux (the app says "Mac-only for now").
- Intel Macs (the workflow builds Apple Silicon; add `--arch x86_64` or `universal` to extend).
- A signed and notarised app, which would remove the first-install warning on other people's
  Macs (needs an Apple Developer account).
