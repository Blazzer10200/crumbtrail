# Sweep

A small Windows app that shows you exactly what junk is eating your disk — temp files, stale caches, update leftovers, gaming and dev caches — and clears only what you approve.

**Scan first. Nothing is deleted until you check the boxes and hit Clean.**

## Install

Grab the installer from [Releases](../../releases), run it, done. No accounts, no background services, no runtime deps (uses the WebView2 already on Windows 10/11).

> **First run:** Windows SmartScreen shows "Windows protected your PC" because the installer isn't code-signed. Click **More info → Run anyway**. That's normal for unsigned indie tools.

## Two tabs

**Clean** — pick categories, see real sizes, delete only what you check:

| Group | Categories |
|---|---|
| System & apps | User/system temp (48h+ old only), Windows Update leftovers, browser caches (Chrome/Edge/Firefox), thumbnail caches, crash dumps, Recycle Bin |
| Gaming | DirectX shader cache, NVIDIA shader caches, Steam shader cache, FiveM/RedM cache |
| Developer | npm cache, pip cache, Cargo registry cache |

Categories that don't exist on your PC show as "Not found" and are left alone. Shader caches rebuild automatically — first game launch afterward is slightly slower, then back to normal.

**Space** — maps where your disk actually went. Pick a drive (or point it at any folder), and get:

- a **usage ring** per drive with used / free / total at a glance,
- **space hotspots** — the folders where space actually piles up,
- **largest files** — the biggest individual files on the drive, with Windows-managed files (like `pagefile.sys`) tagged as *system*,
- a **browse** view to drill into any folder and open it in Explorer.

Full 2 TB drive scans in under 20 seconds. **View-only — the Space tab never deletes anything.**

Light and dark themes are both supported — toggle with the sun/moon button in the header (it remembers your choice; otherwise it follows Windows).

## Safety rules (baked into the core)

- **Allowlist only** — the scanner can only ever see its built-in list of known-junk paths. There is no "clean anything" mode.
- **Dry-run by default** — scan shows real sizes; nothing is touched until you click Clean.
- **Locked files are skipped, never forced** — files in use by running apps are left alone and reported.
- **Symlinks and junctions are never followed.**
- **Temp files younger than 48 hours are always kept** — they may belong to running apps.
- **Every deletion is logged** to `%LOCALAPPDATA%\Sweep\logs`.
- **No registry cleaning. Ever.**

Riskier items (Windows Update leftovers, Recycle Bin) are marked **confirm** and get an extra confirmation step. System temp and Windows Update cleanup need admin — use the "Restart as admin" button.

## Building from source

Requires Node 20+, Rust stable, and the Tauri 2 prerequisites for Windows.

```
npm install
npm run tauri dev     # run in dev mode
npm run tauri build   # produce the installer (src-tauri/target/release/bundle)
```

Stack: Tauri 2 + Svelte 5 (frontend), Rust (scanner/cleaner core).

### CDP dev tooling (optional)

`scripts/cdp/` holds a WebView2 DevTools-Protocol harness for inspecting the running app (screenshots, DOM/console reads, driving the UI) without manual screenshotting. WebView2 150.x won't expose the debug port under an elevated process, so launch dev de-elevated:

```
npm run cdp:dev      # launch dev at medium integrity + wait for CDP :9222
npm run cdp:serve    # start the wrapper (background), then:
bash scripts/cdp/c.sh look
npm run cdp:doctor   # diagnose if CDP won't come up
```
