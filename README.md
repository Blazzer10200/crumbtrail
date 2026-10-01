# Crumbtrail

A small Windows app that shows you exactly what junk is eating your disk — temp files, stale caches, update leftovers, gaming and dev caches — and clears only what you approve.

**Scan first. Nothing is deleted until you check the boxes and hit Clean.**

## Install

Grab the installer from [Releases](../../releases), run it, done. No accounts, no background services, no runtime deps (uses the WebView2 already on Windows 10/11).

> **First run:** Windows SmartScreen shows "Windows protected your PC" because the installer isn't code-signed. Click **More info → Run anyway**. That's normal for unsigned indie tools.

**Automatic updates** (v0.5.0+): Crumbtrail checks for a new version on launch and shows an "Install & restart" banner when one's available — or check manually with the version button in the header. Updates are cryptographically signed and verified before installing, so they're safe even though the app isn't Authenticode-signed. (v0.5.0 is the first published release with the updater; if you're on v0.2.x or older, install it manually once, then future updates are automatic.)

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
- **what changed** — every drive scan saves a small snapshot, so the next scan shows which folders grew or shrank,
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
npm run tauri build   # produce the installer (src-tauri/target/release/bundle)
```

Stack: Tauri 2 + Svelte 5 (frontend), Rust (scanner/cleaner core).

### Releasing a new version (with auto-update)

Updates are signed with a minisign keypair (the public key lives in `tauri.conf.json`; the private key stays off-repo). To cut a release:

```
# 1. bump the version in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml
# 2. build the signed installer + .sig
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$env:USERPROFILE\.tauri\sweep-updater.key" -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run tauri build
# 3. generate the update manifest
pwsh -NoProfile -File scripts\make-update-manifest.ps1 -Version X.Y.Z -Notes "What changed"
# 4. create a GitHub Release tagged vX.Y.Z and upload the -setup.exe + latest.json
```

Installed apps fetch `releases/latest/download/latest.json`, compare versions, and download + verify the signed installer before applying. **Keep `~/.tauri/sweep-updater.key` safe and private** — it's what proves an update is genuinely from you.

### CDP dev tooling (optional)

`scripts/cdp/` holds a WebView2 DevTools-Protocol harness for inspecting the running app (screenshots, DOM/console reads, driving the UI) without manual screenshotting. WebView2 150.x won't expose the debug port under an elevated process, so launch dev de-elevated:

```
npm run cdp:dev      # launch dev at medium integrity + wait for CDP :9222
npm run cdp:serve    # start the wrapper (background), then:
bash scripts/cdp/c.sh look
npm run cdp:doctor   # diagnose if CDP won't come up
```
