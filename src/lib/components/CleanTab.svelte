<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { GROUPS } from "$lib/copy";
  import { fmt, gb1, plural, signed, snapLabel } from "$lib/format";
  import GroupCard from "./GroupCard.svelte";
  import Hero from "./Hero.svelte";
  import Presets from "./Presets.svelte";

  const groups = $derived(GROUPS.filter((g) => app.cats.some(g.match)));

  function joinNames(names: string[]): string {
    return names.length < 2 ? (names[0] ?? "") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
  }

  const nothing = $derived(!app.done && !app.busy && app.selected.length === 0);
  const footTitle = $derived(
    app.done && app.result
      ? `All done · ${fmt(app.result.freed)} freed`
      : app.scanning
        ? "Scanning your PC…"
        : app.scanError
          ? `Scan incomplete · ${plural(app.selected.length, "measured item", "measured items")} checked · ${fmt(app.selectedBytes)}`
          : `${app.selected.length} of ${app.selectableCats.length} items checked · ${fmt(app.selectedBytes)}`,
  );
  const primaryLabel = $derived(
    app.done
      ? "Scan again"
      : app.cleaning
        ? "Cleaning…"
        : app.scanning
          ? "Clean"
          : nothing
            ? "Nothing selected"
            : `Clean ${fmt(app.selectedBytes)}`,
  );
  const primaryDisabled = $derived(app.busy || (!app.done && app.selected.length === 0));

  const failedNames = $derived(
    joinNames((app.scanError?.ids ?? []).map((id) => app.cats.find((c) => c.id === id)?.name ?? id)),
  );
  const lockedBytes = $derived(app.lockedCats.reduce((a, c) => a + (app.sizes[c.id]?.bytes ?? 0), 0));
  const showAdmin = $derived(app.lockedCats.length > 0 && (!app.adminDismissed || app.adminCancelled));
</script>

<Hero />
<Presets />

<main>
  {#if app.teaser}
    {@const t = app.teaser}
    <button class="teaser" onclick={() => app.openWhatChanged()}>
      <span class="tdot"></span>
      <span class="ttext">
        <strong>{signed(t.net)} since {snapLabel(t.since)}.</strong>
        {t.grew.length
          ? `Biggest: ${t.grew.map((g) => `${g.name} ${signed(g.delta)}`).join(", ")}.`
          : "Scan C: to see which folders grew."}
      </span>
      <span class="tlink">See what changed →</span>
    </button>
  {/if}

  {#if app.scanError && !app.scanning}
    {@const se = app.scanError}
    <div class="alert red">
      <span class="aicon">!</span>
      <div class="atext">
        <div class="atitle">The scan stopped early</div>
        <div class="abody">
          Something went wrong while measuring {failedNames}. Nothing was changed. {app.scanStep} of {app.selectableCats.length}
          items were measured, and you can still clean those.
        </div>
        <div class="adetail mono">{se.message} · full details are in the log</div>
      </div>
      <div class="abtns">
        <button class="btn-secondary sm" title={se.log_path} onclick={() => app.reveal(se.log_path)}>Open log</button>
        <button class="btn-primary sm" onclick={() => app.startScan()}>Try again</button>
      </div>
    </div>
  {/if}

  {#if showAdmin}
    <div class="notice" class:red={app.adminCancelled}>
      <div class="ntext">
        {#if app.adminCancelled}
          <strong>Couldn't restart as admin</strong> · Windows didn't allow it (the permission prompt was cancelled). Crumbtrail
          is still running normally, just without admin. Nothing was changed.
        {:else}
          <strong>Running without admin</strong> · {joinNames(app.lockedCats.map((c) => c.name))}
          {app.lockedCats.length === 1 ? "is" : "are"} locked{lockedBytes ? ` (about ${fmt(lockedBytes)})` : ""} until you restart
          as admin.
        {/if}
      </div>
      <button class="btn-secondary sm" disabled={app.busy} onclick={() => app.relaunchAdmin()}>
        {app.adminCancelled ? "Try again" : "Restart as admin"}
      </button>
      <button class="x" aria-label="Dismiss" onclick={() => app.dismissAdmin()}>✕</button>
    </div>
  {/if}

  {#if app.done && app.result}
    {@const r = app.result}
    {@const partial = r.skipped > 0}
    <div class="result" class:partial>
      <div class="res-head">
        <span class="tick">
          {#if partial}
            !
          {:else}
            <svg width="14" height="14" viewBox="0 0 12 12">
              <polyline points="2.5,6.5 5,9 9.5,3.5" fill="none" stroke="var(--accent-ink)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {/if}
        </span>
        <div class="res-text">
          <div class="res-title display">
            Cleaned {fmt(r.freed)}{partial ? ` · ${plural(r.skipped, "file", "files")} left behind` : ""}
          </div>
          <div class="res-sub">
            {#if partial}
              {r.skipped.toLocaleString()}
              {r.skipped === 1 ? "file was" : "files were"} in use, so Crumbtrail left {r.skipped === 1 ? "it" : "them"} alone. Close
              Chrome and other running apps, then retry. Nothing is ever forced.
            {:else}
              {plural(r.items.length, "item", "items")} cleaned · Crumbtrail has now freed {gb1(app.allTime)} in total.
            {/if}
          </div>
        </div>
        {#if partial}
          <button class="btn-amber sm" disabled={app.busy} onclick={() => app.retrySkipped()}>
            {app.retrying ? "Retrying…" : "Retry skipped"}
          </button>
        {/if}
        <button class="btn-secondary sm" title={r.logPath} onclick={() => app.reveal(r.logPath)}>Open log</button>
      </div>
      <div class="res-items">
        {#each r.items as it (it.id)}
          <div class="res-row">
            <span class="ri-name">{it.name}</span>
            <span class="ri-note">{it.skipped ? `${it.skipped.toLocaleString()} files skipped · in use by running apps` : ""}</span>
            <span class="ri-freed mono">{fmt(it.freed)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#each groups as g (g.key)}
    <GroupCard group={g} />
  {/each}
</main>

<footer>
  <div class="ft-text">
    <div class="ft-title">{footTitle}</div>
    <div class="ft-hint">
      {app.done
        ? "Nothing else was touched. Rescan any time."
        : nothing
          ? "Tick items above or pick a preset, then press Clean."
          : "Only checked items are removed. Press ⓘ on any row to see what it does."}
    </div>
  </div>
  <div class="ft-actions">
    <button class="btn-secondary" disabled={app.busy} onclick={() => app.startScan()}>
      {app.scanning ? "Scanning…" : "Rescan"}
    </button>
    <button class="btn-primary go" disabled={primaryDisabled} onclick={() => app.requestClean()}>
      {primaryLabel}
      <span class="arrow">→</span>
    </button>
  </div>
</footer>

<style>
  main {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 12px 24px 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .result {
    flex-shrink: 0;
    background: var(--card);
    border: 1px solid var(--accent-line);
    border-radius: 12px;
    padding: 14px 16px;
    animation:
      fade-in 0.45s ease,
      drop-12 0.6s var(--spring);
  }
  .res-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tick {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--accent);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }
  .res-text {
    flex: 1;
    min-width: 0;
  }
  .res-title {
    font-weight: 700;
    font-size: 15px;
  }
  .res-sub {
    font-size: 12px;
    color: var(--muted);
    margin-top: 1px;
    text-wrap: pretty;
  }
  .res-items {
    display: flex;
    flex-direction: column;
    margin-top: 10px;
    border-top: 1px solid var(--line);
  }
  .res-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 0;
    border-bottom: 1px solid var(--line-row);
    font-size: 13px;
  }
  .ri-name {
    flex: 1;
    min-width: 0;
  }
  .ri-note {
    font-size: 12px;
    color: var(--amber);
    text-align: right;
  }
  .result.partial {
    border-color: var(--amber-panel-line);
  }
  .result.partial .tick {
    background: var(--amber);
    color: var(--bg);
    font-weight: 800;
    font-size: 15px;
  }
  .btn-amber {
    border: none;
    border-radius: 999px;
    background: var(--amber);
    color: var(--bg);
    font-weight: 600;
    cursor: pointer;
  }
  .btn-amber:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .sm {
    height: 30px;
    padding: 0 13px;
    font-size: 12.5px;
    flex-shrink: 0;
  }

  .teaser {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--card);
    color: var(--muted);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    animation: fade-in 0.35s ease;
  }
  .teaser:hover {
    border-color: var(--line-3);
  }
  .tdot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--amber);
    flex-shrink: 0;
  }
  .ttext {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ttext strong {
    color: var(--ink);
    font-weight: 600;
  }
  .tlink {
    color: var(--accent);
    font-weight: 600;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .alert {
    flex-shrink: 0;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--red-soft);
    border: 1px solid var(--red-line);
    animation: fade-in 0.35s ease;
  }
  .aicon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--red);
    color: var(--bg);
    display: grid;
    place-items: center;
    font-weight: 800;
    flex-shrink: 0;
  }
  .atext {
    flex: 1;
    min-width: 0;
  }
  .atitle {
    font-size: 14px;
    font-weight: 700;
    color: var(--red);
  }
  .abody {
    font-size: 12.5px;
    color: var(--ink-2);
    margin-top: 2px;
    line-height: 1.45;
    text-wrap: pretty;
  }
  .adetail {
    font-size: 11px;
    color: var(--muted-3);
    margin-top: 6px;
    word-break: break-word;
  }
  .abtns {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .notice {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 14px;
    border-radius: 12px;
    background: var(--amber-panel);
    border: 1px solid var(--amber-panel-line);
    font-size: 12.5px;
    color: var(--muted);
  }
  .notice strong {
    color: var(--amber);
    font-weight: 600;
  }
  .notice.red {
    background: var(--red-soft);
    border-color: var(--red-line);
  }
  .notice.red strong {
    color: var(--red);
  }
  .ntext {
    flex: 1;
    min-width: 0;
    text-wrap: pretty;
  }
  .x {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--muted-3);
    cursor: pointer;
    flex-shrink: 0;
  }
  .x:hover {
    background: var(--chip-bg);
    color: var(--ink);
  }
  .ri-freed {
    font-weight: 600;
    color: var(--accent);
    min-width: 84px;
    text-align: right;
  }

  footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 24px;
    border-top: 1px solid var(--line-hero);
    background: var(--surface);
  }
  .ft-text {
    min-width: 0;
  }
  .ft-title {
    font-size: 13.5px;
    font-weight: 600;
  }
  .ft-hint {
    font-size: 12px;
    color: var(--muted);
    margin-top: 2px;
    text-wrap: pretty;
  }
  .ft-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .go {
    padding: 0 8px 0 18px;
    font-weight: 650;
    display: flex;
    align-items: center;
    gap: 10px;
    box-shadow: var(--shadow-primary);
  }
  .go:hover:not(:disabled) {
    transform: translateY(-2px) scale(1.02);
  }
  .arrow {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent-ink);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-size: 13px;
  }
</style>
