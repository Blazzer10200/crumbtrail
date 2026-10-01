# Changelog

## Unreleased

- **Smoother launch.** The window opens dark with no white flash, and a short "Getting ready…" screen shows until the first data is in. Light-theme users get a light background instead.
- **Restart as admin** shows a "Restarting as admin…" screen while Windows asks for permission, and no longer flashes a command prompt (debug builds were missing the no-console flag).
- **Fixed:** the footer said "8 of 11 items checked" while 10 boxes were ticked; it now counts every checked item. When nothing in the checked items can be cleaned, it says so.
- **Fixed:** the Clean/Space pill no longer drifts sideways when the update chip changes; the chip now sits next to the app name.
- **Fixed:** the Save-selection and preset menus no longer hang off the edge of the window.

## 0.6.0 — 2026-10-01

- **Updates install themselves.** Crumbtrail now updates through Velopack: it downloads new versions quietly in the background, then offers "Restart now". If you ignore it, the update installs the next time you close the app. Coming from 0.5.0? Uninstall it and install this version once. Your presets and snapshots are kept.
- **Weekly snapshots.** Turn on "Save a snapshot every week" (Space → What changed) and a quiet Windows scheduled task records folder sizes on C: once a week, so "What changed" always has something to compare against. Uninstalling Crumbtrail removes the task.
- **Old GPU driver installers.** New Clean group for leftover NVIDIA/AMD installer folders. The installed driver version is always kept; if it can't be read, nothing is deleted.
- **Presets.** Save the current selection as a preset and re-apply it in one click (none ship by default).
- **All-time freed** stat in the hero, kept across sessions.
- **Clearer errors.** A scan that stops early, unreadable folders, a cancelled admin prompt, and partly-failed cleans all get their own message. "Retry skipped" re-runs just the items that were skipped.
- **Update banner** now shows checking / downloading / ready / failed / up to date, and explains when a copy can't update itself.
- **Space: What changed.** Each drive scan saves a snapshot; the next scan shows which folders grew or shrank. Defaults to comparing with a scan at least 6 days old.
- **Space: More views** — Games (Steam and Epic, by size or last played), Forgotten installers (old .exe/.msi/.zip in Downloads), and File types (stacked bar + biggest files per type).
- **Clean-tab teaser** shows how much C: grew or shrank since the last snapshot (hidden under 50 MB).
- Esc and click-outside close any open menu or popover.

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
