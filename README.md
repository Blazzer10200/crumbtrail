# Sweep

A small Windows app that shows you exactly what junk is eating your disk — temp files, stale caches, update leftovers, gaming and dev caches — and clears only what you approve.

**Scan first. Nothing is deleted until you check the boxes and hit Clean.**

## Install

Grab the installer from [Releases](../../releases), run it, done. No accounts, no background services, no runtime deps (uses the WebView2 already on Windows 10/11).

> **First run:** Windows SmartScreen shows "Windows protected your PC" because the installer isn't code-signed. Click **More info → Run anyway**. That's normal for unsigned indie tools.

## What it cleans

| Group | Categories |
|---|---|
| System & apps | User/system temp (48h+ old only), Windows Update leftovers, browser caches (Chrome/Edge/Firefox), thumbnail caches, crash dumps, Recycle Bin |
| Gaming | DirectX shader cache, NVIDIA shader caches, Steam shader cache, FiveM/RedM cache |
| Developer | npm cache, pip cache, Cargo registry cache |

Categories that don't exist on your PC show as "Not found" and are left alone. Shader caches rebuild automatically — first game launch afterward is slightly slower, then back to normal.

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
