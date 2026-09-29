<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { GROUPS } from "$lib/copy";
  import { fmt, plural } from "$lib/format";
  import GroupCard from "./GroupCard.svelte";
  import Hero from "./Hero.svelte";

  const groups = $derived(GROUPS.filter((g) => app.cats.some(g.match)));

  const footTitle = $derived(
    app.done && app.result
      ? `All done · ${fmt(app.result.freed)} freed`
      : app.scanning
        ? "Scanning your PC…"
        : `${app.selected.length} of ${app.selectableCats.length} items checked · ${fmt(app.selectedBytes)}`,
  );
  const primaryLabel = $derived(
    app.done ? "Scan again" : app.cleaning ? "Cleaning…" : app.scanning ? "Clean" : `Clean ${fmt(app.selectedBytes)}`,
  );
  const primaryDisabled = $derived(app.busy || (!app.done && app.selected.length === 0));
</script>

<Hero />

<main>
  {#if app.done && app.result}
    {@const r = app.result}
    <div class="result">
      <div class="res-head">
        <span class="tick">
          <svg width="14" height="14" viewBox="0 0 12 12">
            <polyline points="2.5,6.5 5,9 9.5,3.5" fill="none" stroke="var(--accent-ink)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </span>
        <div class="res-text">
          <div class="res-title display">Cleaned {fmt(r.freed)}</div>
          <div class="res-sub">
            {plural(r.items.length, "item", "items")} cleaned{r.skipped
              ? ` · ${r.skipped.toLocaleString()} files were in use, so they were left alone (that's normal)`
              : ""}
          </div>
        </div>
        <button class="btn-secondary sm" title={r.logPath} onclick={() => app.reveal(r.logPath)}>Open log</button>
      </div>
      <div class="res-items">
        {#each r.items as it (it.id)}
          <div class="res-row">
            <span class="ri-name">{it.name}</span>
            <span class="ri-note">{it.skipped ? `${it.skipped.toLocaleString()} files skipped · in use` : ""}</span>
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
    color: var(--amber-text);
    text-align: right;
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
