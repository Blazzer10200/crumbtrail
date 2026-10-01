# Codex project instructions

This file is the Codex runtime authority for `crumbtrail` (renamed from Sweep 2026-09-29). `CLAUDE.md` may supply product history, but its slash-command claims do not define Codex behavior.

## Project

Windows disk cleaner built with Tauri 2, Svelte 5, and Rust. The Clean tab performs allowlisted cleanup; the Space tab is view-only.

## Canonical commands

Run from the project root.

| Task | Command |
|---|---|
| Install | `npm install` |
| Frontend dev | `npm run dev` |
| Desktop dev | `npm run tauri dev` |
| Frontend build | `npm run build` |
| Release exe | `npm run tauri build -- --no-bundle` |
| Package + publish | `pwsh -NoProfile -File scripts/release.ps1` (`-NoUpload` to pack only) |
| Frontend typecheck | `npm run check` |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` |
| Rust format check | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` |

For cross-stack changes, verify in this order: frontend typecheck, Rust format check, Rust lint, Rust tests. The global `$check` skill covers the Svelte check only; it does not run Cargo commands.

## Load-bearing safety rules

- Scanner scope remains an explicit known-junk allowlist. Do not introduce arbitrary-path cleaning.
- Deletion remains opt-in after scan and selection; locked files are skipped and symlinks/junctions are never followed.
- Preserve the 48-hour floor for temp files, per-deletion logs, and the ban on registry cleaning.
- The Space tab is view-only and must never acquire deletion behavior.
- Keep cleaner safety invariants covered by Rust tests when modifying backend behavior.
- Updates ship through Velopack (`src-tauri/src/updater.rs`). The pack id stays `Crumbtrail.App`: Velopack installs to and uninstall-wipes `%LOCALAPPDATA%\<packId>`, and `%LOCALAPPDATA%\Crumbtrail` holds user snapshots and logs.
- `velopack::VelopackApp` must run first in `main`; the uninstall hook removes the weekly snapshot task.
- Tokens and keys (e.g. `gh auth token`) must never appear in output.
