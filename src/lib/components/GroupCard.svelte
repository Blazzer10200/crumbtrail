<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { INFO, RISK_LABEL, riskOf, type Group } from "$lib/copy";
  import { fmt } from "$lib/format";
  import type { Cat } from "$lib/types";

  let { group }: { group: Group } = $props();

  const rows = $derived(app.cats.filter(group.match));
  const on = $derived(rows.filter((c) => app.isSelectable(c) && app.checked[c.id]));
  const anyOn = $derived(on.length > 0);
  const isOpen = $derived(!!app.open[group.key]);
  const total = $derived(on.filter((c) => !app.swept[c.id]).reduce((a, c) => a + (app.sizes[c.id]?.bytes ?? 0), 0));

  function onRowKey(e: KeyboardEvent, c: Cat) {
    if (e.target !== e.currentTarget) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      app.toggle(c);
    }
  }

  function onRowClick(e: MouseEvent, c: Cat) {
    // Clicks inside the explainer panel shouldn't toggle the row.
    if ((e.target as HTMLElement).closest(".explain")) return;
    app.toggle(c);
  }
</script>

<div class="card">
  <div
    class="head"
    role="button"
    tabindex="0"
    aria-expanded={isOpen}
    onclick={() => app.toggleOpen(group.key)}
    onkeydown={(e) => {
      if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        app.toggleOpen(group.key);
      }
    }}
  >
    <span class="chev-box">
      <svg width="12" height="12" viewBox="0 0 12 12" class="chev" class:open={isOpen}>
        <polyline points="4,2 8,6 4,10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </span>
    <span class="swatch" style:background={group.color}></span>
    <span class="label display">{group.label}</span>
    <span class="sel">
      {group.key === "adv" ? "Can't be undone · off by default" : `${on.length} of ${rows.length} selected`}
    </span>
    <span class="spacer"></span>
    <span class="total mono">{fmt(total)}</span>
    <button
      class="switch"
      class:on={anyOn}
      title="Select all"
      aria-label="Select all in {group.label}"
      disabled={app.busy}
      onclick={(e) => {
        e.stopPropagation();
        app.setGroup(rows);
      }}
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="collapse" class:open={isOpen}>
    <div class="clip">
      <div class="body">
        {#each rows as c (c.id)}
          {@const ok = app.isSelectable(c)}
          {@const ck = ok && !!app.checked[c.id]}
          {@const size = app.sizes[c.id]}
          {@const sw = !!app.swept[c.id]}
          {@const risk = riskOf(c.id)}
          {@const infoOpen = !!app.info[c.id]}
          <div
            class="row"
            class:dim={!ok}
            class:clickable={ok && !app.busy}
            role="checkbox"
            aria-checked={ck}
            aria-disabled={!ok || app.busy}
            tabindex={ok ? 0 : -1}
            onclick={(e) => onRowClick(e, c)}
            onkeydown={(e) => onRowKey(e, c)}
          >
            <div class="wash" class:on={app.washing[c.id]}></div>
            <span class="box" class:ck>
              <svg width="12" height="12" viewBox="0 0 12 12">
                <polyline points="2.5,6.5 5,9 9.5,3.5" fill="none" stroke="var(--accent-ink)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </span>

            <div class="main">
              <div class="title">
                <span class="nm">{c.name}</span>
                <span class="chip {risk}">{RISK_LABEL[risk]}</span>
                {#if c.needs_admin}<span class="admin-chip" class:ok={app.admin}>admin</span>{/if}
                <button
                  class="info"
                  class:open={infoOpen}
                  title="What happens if I clean this?"
                  aria-expanded={infoOpen}
                  onclick={(e) => {
                    e.stopPropagation();
                    app.toggleInfo(c.id);
                  }}>i</button
                >
              </div>
              <div class="desc">{c.available ? c.description : "Not found on this PC"}</div>
              {#if INFO[c.id]}
                <div class="collapse" class:open={infoOpen}>
                  <div class="clip">
                    <div class="explain" class:open={infoOpen}>
                      <div class="ex-h">What happens if I clean this?</div>
                      <div class="ex-what">{INFO[c.id][0]}</div>
                      <div class="ex-where mono">{INFO[c.id][1]}</div>
                    </div>
                  </div>
                </div>
              {/if}
            </div>

            <div class="size">
              {#if !ok}
                <span class="dimlabel">{c.available ? "needs admin" : "—"}</span>
              {:else}
                {#if app.scanning && !size}<div class="shimmer"></div>{/if}
                <div class="sz" class:in={!!size}>
                  <div class="mono amt" class:swept={sw}>{sw ? "cleaned" : fmt(size?.bytes ?? 0)}</div>
                  <div class="files">{sw ? "all clear" : `${(size?.files ?? 0).toLocaleString()} files`}</div>
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .card {
    flex-shrink: 0;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 12px;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    cursor: pointer;
    user-select: none;
  }
  .head:hover {
    background: var(--surface);
  }
  .chev-box {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    background: var(--chip-bg);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--muted);
  }
  .chev {
    transition: transform 0.4s var(--spring);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .label {
    font-weight: 600;
    font-size: 14px;
  }
  .sel {
    font-size: 12px;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .total {
    font-size: 13px;
    font-weight: 600;
  }
  .switch {
    width: 38px;
    height: 22px;
    border-radius: 99px;
    border: none;
    padding: 0;
    background: var(--line-3);
    position: relative;
    cursor: pointer;
    transition: background 0.3s;
    flex-shrink: 0;
  }
  .switch.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--ink);
    transition: transform 0.35s var(--spring);
  }
  .switch.on .knob {
    transform: translateX(16px);
  }

  .collapse {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows 0.45s var(--ease-out);
  }
  .collapse.open {
    grid-template-rows: 1fr;
  }
  .clip {
    overflow: hidden;
    min-height: 0;
  }
  .body {
    padding: 0 8px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    opacity: 0;
    transform: translateY(-10px);
    transition:
      opacity 0.35s ease,
      transform 0.45s var(--ease-out);
  }
  .open > .clip > .body {
    opacity: 1;
    transform: none;
  }

  .row {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 9px;
    cursor: default;
    overflow: hidden;
  }
  .row.clickable {
    cursor: pointer;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .row.dim {
    opacity: 0.5;
  }
  .wash {
    position: absolute;
    inset: 0;
    background: var(--accent);
    opacity: 0.1;
    transform-origin: left;
    transform: scaleX(0);
    transition: transform 0.7s var(--sweep);
    pointer-events: none;
  }
  .wash.on {
    transform: scaleX(1);
  }
  .box {
    position: relative;
    margin-top: 2px;
    width: 17px;
    height: 17px;
    border-radius: 5px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    background: transparent;
    border: 1.5px solid var(--line-4);
    transform: scale(0.92);
    transition:
      background 0.2s,
      border-color 0.2s,
      transform 0.3s var(--spring);
  }
  .box.ck {
    background: var(--accent);
    border-color: var(--accent);
    transform: scale(1);
  }
  .box svg {
    opacity: 0;
    transform: scale(0);
    transition: all 0.25s var(--spring);
  }
  .box.ck svg {
    opacity: 1;
    transform: scale(1);
  }
  .main {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    font-weight: 600;
    font-size: 13.5px;
  }
  .nm {
    white-space: nowrap;
  }
  .admin-chip {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 99px;
    border: 1px solid var(--line-3);
    color: var(--muted-3);
  }
  .admin-chip.ok {
    border-color: var(--accent-line);
    color: var(--accent);
  }
  .info {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--line-4);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    font-weight: 700;
    font-style: italic;
    font-family: Georgia, serif;
    padding: 0;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: all 0.2s;
  }
  .info:hover {
    border-color: var(--muted);
    color: var(--ink);
  }
  .info.open {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--bg);
  }
  .desc {
    color: var(--muted-2);
    font-size: 12px;
    margin-top: 1px;
    text-wrap: pretty;
  }
  .main .collapse {
    transition-duration: 0.4s;
  }
  .explain {
    margin-top: 8px;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--bg);
    border: 1px solid var(--line-2);
    cursor: default;
    opacity: 0;
    transition: opacity 0.3s ease;
  }
  .explain.open {
    opacity: 1;
  }
  .ex-h {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .ex-what {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--ink-2);
    margin-top: 3px;
    text-wrap: pretty;
  }
  .ex-where {
    margin-top: 6px;
    font-size: 11px;
    color: var(--muted-3);
    word-break: break-all;
  }

  .size {
    position: relative;
    text-align: right;
    flex-shrink: 0;
    min-width: 104px;
    min-height: 32px;
  }
  .shimmer {
    position: absolute;
    right: 0;
    top: 4px;
    width: 72px;
    height: 12px;
    border-radius: 6px;
    background: linear-gradient(90deg, var(--shimmer-a) 0%, var(--shimmer-b) 50%, var(--shimmer-a) 100%);
    background-size: 120px 12px;
    animation: shimmer 1.1s linear infinite;
  }
  .sz {
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity 0.4s ease,
      transform 0.5s var(--spring);
  }
  .sz.in {
    opacity: 1;
    transform: none;
  }
  .amt {
    font-weight: 600;
    font-size: 13px;
    color: var(--ink);
  }
  .amt.swept {
    color: var(--accent);
  }
  .files {
    font-size: 11.5px;
    color: var(--muted-3);
    margin-top: 1px;
  }
  .dimlabel {
    font-size: 12.5px;
    color: var(--muted-4);
  }
</style>
