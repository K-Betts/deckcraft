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

1. Merge your changes to `main`.
2. GitHub ▸ **Actions** ▸ **Publish Mac update** ▸ **Run workflow**. Type a line about what is new
   (teachers see it in the update dialog). It builds on a free GitHub Mac and publishes a release
   named `build-<commit>` with the zip and its checksum. About 10–20 minutes.
3. Everyone who has update checks on sees *Update available* the next time they start DeckCraft.

Nothing is published until you press that button. The release is public (the repository is).

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
