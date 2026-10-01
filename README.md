# Crumbtrail

A small Windows app that shows you exactly what junk is eating your disk — temp files, stale caches, update leftovers, gaming and dev caches — and clears only what you approve.

**Scan first. Nothing is deleted until you check the boxes and hit Clean.**

## Install

Grab `Crumbtrail.App-win-Setup.exe` from the latest [Release](../../releases/latest), run it, done. It installs per-user (no admin prompt), adds Start menu and desktop shortcuts, and uses the WebView2 already on Windows 10/11. No accounts.

> **First run:** Windows SmartScreen shows "Windows protected your PC" because the installer isn't code-signed. Click **More info → Run anyway**. That's normal for unsigned indie tools.

**Automatic updates** (v0.6.0+): Crumbtrail checks GitHub for a new version on launch and downloads it quietly in the background. When it's ready you get a "Restart now" banner. Ignore it and the update installs by itself the next time you close the app. You can also check by hand with the version button in the header. Updates are delivered with [Velopack](https://velopack.io).

> **Coming from 0.5.0 or older?** Those used a different installer. Uninstall the old Crumbtrail from *Settings → Apps*, then install 0.6.0 once. Your presets and snapshots are kept. Every update after that is automatic.

## Two tabs

**Clean** — pick categories, see real sizes, delete only what you check:

| Group | Categories |
|---|---|
| System & apps | User/system temp (48h+ old only), Windows Update leftovers, browser caches (Chrome/Edge/Firefox), thumbnail caches, crash dumps, Recycle Bin |
| Gaming | DirectX shader cache, NVIDIA shader caches, Steam shader cache, FiveM/RedM cache |
| Developer | npm cache, pip cache, Cargo registry cache |
| GPU | Old NVIDIA/AMD driver installer folders (the installed version is always kept) |

Save any selection as a **preset** to re-apply it later. The hero also shows how much Crumbtrail has freed all-time.

Categories that don't exist on your PC show as "Not found" and are left alone. Shader caches rebuild automatically — first game launch afterward is slightly slower, then back to normal.

**Space** — maps where your disk actually went. Pick a drive (or point it at any folder), and get:

- a **usage ring** per drive with used / free / total at a glance,
- **space hotspots** — the folders where space actually piles up,
- **largest files** — the biggest individual files on the drive, with Windows-managed files (like `pagefile.sys`) tagged as *system*,
- a **browse** view to drill into any folder and open it in Explorer,
- **what changed** — every drive scan saves a small snapshot, so the next scan shows which folders grew or shrank. Turn on **Save a snapshot every week** and a quiet Windows scheduled task keeps the history going without you (folder sizes only; switch it off any time),
- under **More**: installed **games** (Steam, Epic), **forgotten installers** in Downloads, and a **file types** breakdown.

Full 2 TB drive scans in under 20 seconds. **View-only — the Space tab never deletes anything.**

Dark theme by default, with a light theme on the sun/moon button in the header (it remembers your choice). The window uses its own title bar so the whole app is one continuous surface.

## Safety rules (baked into the core)

- **Allowlist only** — the scanner can only ever see its built-in list of known-junk paths. There is no "clean anything" mode.
- **Dry-run by default** — scan shows real sizes; nothing is touched until you click Clean.
- **Locked files are skipped, never forced** — files in use by running apps are left alone and reported.
- **Symlinks and junctions are never followed.**
- **Temp files younger than 48 hours are always kept** — they may belong to running apps.
- **Every deletion is logged** to `%LOCALAPPDATA%\Crumbtrail\logs`.
- **No registry cleaning. Ever.**

Every row has an **ⓘ** button that explains, in plain words, what cleaning it does and where it lives. Riskier items (Windows Update leftovers, Recycle Bin) sit under **More options** and are off by default. Before anything is deleted you always get a review sheet listing each item and its size, with anything permanent (Recycle Bin) called out. System temp and Windows Update cleanup need admin — use the "Restart as admin" button.

## Building from source

Requires Node 20+, Rust stable, and the Tauri 2 prerequisites for Windows.

```
npm install
npm run tauri dev     # run in dev mode
npm run tauri build -- --no-bundle   # release exe only; installers come from scripts/release.ps1
```

Stack: Tauri 2 + Svelte 5 (frontend), Rust (scanner/cleaner core), Velopack (installer + updates).

A dev build can't update itself; the update button says so. Only installed copies update.

### Releasing a new version

Needs the [Velopack CLI](https://docs.velopack.io) (`dotnet tool install -g vpk`) and a logged-in `gh`.

1. Bump the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Add a dated `## X.Y.Z — YYYY-MM-DD` section to `CHANGELOG.md`. It becomes the release notes.
3. Commit and push `main`, then run:

```
pwsh -NoProfile -File scripts\release.ps1             # build, pack, publish GitHub Release vX.Y.Z
pwsh -NoProfile -File scripts\release.ps1 -NoUpload   # or: build + pack only, to try the installer first
```

The script builds the exe, packs it with `vpk` (full package, plus a delta against the previous release), and uploads everything to a GitHub Release. Installed apps find it on their next launch. Build output lives in the git-ignored `.release/` folder.

### CDP dev tooling (optional)

`scripts/cdp/` holds a WebView2 DevTools-Protocol harness for inspecting the running app (screenshots, DOM/console reads, driving the UI) without manual screenshotting. WebView2 150.x won't expose the debug port under an elevated process, so launch dev de-elevated:

```
npm run cdp:dev      # launch dev at medium integrity + wait for CDP :9222
npm run cdp:serve    # start the wrapper (background), then:
bash scripts/cdp/c.sh look
npm run cdp:doctor   # diagnose if CDP won't come up
```
