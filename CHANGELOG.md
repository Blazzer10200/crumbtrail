# Changelog

## 0.5.0 — 2026-09-29

First release under the Crumbtrail name, with a full redesign.

- **New look.** Rebuilt UI from the Claude Design handoff: Bricolage Grotesque / Geist type, dark theme by default, spring-based motion (respects reduced-motion).
- **Clean tab.** Big animated total, Scan → Choose → Clean steps, per-group colour bar, collapsible groups with select-all switches, and an ⓘ explainer on every row.
- **Review before cleaning.** Every clean now shows a review sheet listing each item and size; permanent items are called out.
- **Clean feedback.** Rows wash away one by one, then a summary card lists what was freed and how many in-use files were skipped, with an "Open log" button. The app no longer rescans automatically afterwards.
- **Space tab.** Drive usage rings, Biggest folders / Largest files / Browse views with breadcrumbs. Still view-only.
- **Custom title bar** that blends into the app instead of the grey Windows one.
- **New logo and icons.**
- Bundle identifier stays `com.blazzer.sweep` so the updater keeps working across the rename.

## 0.4.0

- Signed auto-updater (Tauri updater plugin). Tagged but never published as a GitHub Release.

## 0.3.0

- Space tab usage-ring hero, largest-files view, folder scan, theme toggle, UI polish. Tagged but never published as a GitHub Release.

## 0.2.1 and earlier

- Released as **Sweep**: allowlist-only scanner and cleaner, gaming and developer cache modules.
