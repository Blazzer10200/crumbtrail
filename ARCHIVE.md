# Sweep archival snapshot — 2026-09-03

> **Unarchived 2026-09-29 — development has resumed, and the project was renamed
> from Sweep to Crumbtrail** (repo `Blazzer10200/crumbtrail`). This file is kept
> as a historical record. Since 0.6.0, updates ship through Velopack (see README,
> "Releasing a new version"); the Tauri updater, its manifest helper and signing
> key described below are retired.

## Status

Sweep is archived at the owner's request. Application source remains at version
0.4.0; this archival documentation does not publish a new installer or update.
The repository retains the source, dependency lockfiles, icons, build configuration,
updater-manifest helper, optional CDP tools, and Git history.

The last application commit before archival documentation was
`1eb0c1245afbbde6009711b5f0a34084f6431148`.

## Preserved behavior

- Tauri 2, Svelte 5, and Rust Windows desktop application.
- Clean: explicit known-junk allowlist, user selection and confirmation, locked-file
  skipping, a 48-hour minimum age for temporary files, and per-deletion logs.
- Space: view-only drive usage, largest files, hotspots, folder browsing, and a
  native folder picker. It must not acquire deletion behavior.
- Symlinks and junctions are not followed. Registry cleaning is prohibited.
- Light/dark themes and signed Tauri update verification were implemented.
- Optional WebView2 CDP inspection uses the de-elevated development launcher;
  admin-only cleanup categories are unavailable in that development instance.

## Restore development

1. Unarchive this repository if development is to resume, then clone it.
2. Install Node.js 20+, Rust stable, and the Tauri 2 Windows prerequisites.
3. Run `npm ci` and `npm run check`.
4. Run the Rust format, lint, and test commands documented in `AGENTS.md`.
5. Run `npm run tauri dev` for development or `npm run tauri build` to package.

Dependency caches, generated frontend/build output, screenshots, local logs, and
machine-specific assistant notes are deliberately not part of the public archive.
Their durable technical guidance is represented here, in the README, and in
`AGENTS.md`; dependencies and generated output can be rebuilt from the lockfiles.

## Release and credential boundaries

The updater public key and release-manifest workflow remain in source. The private
updater signing key must remain outside Git and must not be uploaded to GitHub.
The existing protected local signing key is retained separately for future recovery.
If it is unavailable when development resumes, do not assume existing installations
will accept updates signed with a newly generated key.

Archival does not claim a new runtime acceptance test. Historical validation is
recorded in project history; the archival change is documentation only.
