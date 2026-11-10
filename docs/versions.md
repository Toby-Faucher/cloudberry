# Versions and releases

Cloudberry uses `vMAJOR.MINOR.PATCH`. One number covers the whole app: the server, the web app,
the WASM modules and the wire protocol ship together.

| Part | Meaning |
|---|---|
| MAJOR | A change that breaks people or data: a database change an older image cannot read, a changed wire message, a removed setting. Needs a backup and a note on how to roll back. |
| MINOR | New features and visible changes. Database changes must only add things (a new table, a new nullable column). |
| PATCH | Bug fixes and docs only. No database change. |

The version lives in two places, always the same number: `version` in `Cargo.toml`
(`[workspace.package]`, which all three crates inherit) and `web/package.json` (plus its
`package-lock.json`, which `npm version` updates).

## Where we are

- **v1.0.0**: everything built so far, deployed on 2026-11-09. A shared station synced to a
  server clock, YouTube links and playlist import through yt-dlp with loudness normalising,
  queue, skip, seek, reorder and remove with undo, chat, reactions, pokes, presence and DJ of the
  day, synced lyrics, the Rust→WASM visualizer (Halo, Bloom, Bars) and lean-back shader pack, the
  smear motion engine, the phase 6 hardware look, saved shared playlists, and the version
  handshake (below). Not verified: a YouTube download on the live instance after the deploy.
- **v1.1.0**: mute for you only. The fader's Vol label is now a mute switch (it reads Muted, in
  accent ink, while on) and `M` toggles it. Remembered like the volume; moving the fader unmutes.
  Not on phones, where the fader is hidden. No database change. Not verified: dark mode, a screen
  reader, and listening on real speakers.
- **v1.2.0**: pick your accent colour. Click the header berry and six berry logos fan out in a
  quarter circle (Cloudberry, Pink, Lime, Cyan, Violet, Gold); arrow keys and Enter work too. Just
  for you, remembered in the browser. The logo, favicons, visualizer and per-track tint follow it.
  No database change. Not verified: a screen reader, and the favicon swap in Safari and Firefox.

## How a release is cut

1. Work lands on `main` the usual way. Before a release, run the checks in
   [CONTRIBUTING.md](../CONTRIBUTING.md#tests). There is no CI, so the local run is the one that counts.
2. Make a commit `chore(release): vX.Y.Z` that sets the version in `Cargo.toml` and
   (`cd web && npm version X.Y.Z --no-git-tag-version`) `web/package.json`, refreshes `Cargo.lock`
   (`cargo check`), and adds a line for the release under "Where we are" above. That line says what
   changed, whether the database changed, and what was not verified.
3. Tag it: `git tag -a vX.Y.Z -m "Cloudberry vX.Y.Z" && git push origin main vX.Y.Z`.
4. Create the GitHub release from the tag (`gh release create vX.Y.Z --notes-from-tag` or
   `--generate-notes`). There is no CHANGELOG file: this page and the releases are the record.
5. Deploy from the tag, never from a branch or a dirty tree: `git checkout vX.Y.Z && scripts/deploy.sh`
   (see [deploy.md](deploy.md)). The script tags the running image `cloudberry-cloudberry:previous`
   before it replaces it. Then `git checkout main`.
6. Check it: `curl http://<host>:<port>/api/version` shows the new version and build, the page
   loads, and a second device tunes in and hears the same track.

## The version handshake

`/api/version` and every `welcome` message carry `version` and `build`. The build id is set at
compile time by [`crates/server/build.rs`](../crates/server/build.rs): `CLOUDBERRY_BUILD_ID` if set,
else the git commit, else the package version. A tab that later gets a `welcome` with a different
build id (it reconnected to a redeployed server) closes its socket and shows "Cloudberry was
updated" with a Reload button. It never reloads by itself, so a mismatch can not loop.

The Docker build from source (Option A in [deploy.md](deploy.md)) has no `.git`, so its build id
is the version: two builds of the same version look the same to open tabs. `scripts/deploy.sh`
builds on your machine and uses the commit.

## Rolling back

The schema is created with `CREATE TABLE IF NOT EXISTS` and additive column checks, so an older
image starts on a database a newer MINOR or PATCH used. To roll back on the host:

```sh
cd /opt/stacks/cloudberry
docker tag cloudberry-cloudberry:previous cloudberry-cloudberry:latest
docker compose up -d --no-build
```

The next `scripts/deploy.sh` builds again from whatever you check out. After a MAJOR release that
changed the database, roll back together with the backup taken before it (see
[deploy.md](deploy.md#backups)); everything written after the upgrade is lost.

## Not an agent's decision

The version number of a release, the license, and making the repository public are the owner's
calls. An agent may prepare the release commit and the tag, and waits for a go before pushing the
tag or deploying.
