<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { LAUNCH } from "$lib/copy";
  import { ago, fmtS } from "$lib/format";

  const games = $derived.by(() => {
    const list = [...(app.spaceResult?.games ?? [])];
    // Last played: longest ago first so forgotten games rise; unknown sinks to the bottom.
    if (app.gameSort === "played") list.sort((a, b) => (a.last_played ?? Infinity) - (b.last_played ?? Infinity));
    else list.sort((a, b) => b.bytes - a.bytes);
    return list;
  });
  const total = $derived(games.reduce((a, g) => a + g.bytes, 0));
  const max = $derived(Math.max(1, ...games.map((g) => g.bytes)));
  const now = Date.now() / 1000;
</script>

{#if !games.length}
  <div class="none">No installed games found on this drive.</div>
{:else}
  <div class="tools">
    <span class="count">{games.length} {games.length === 1 ? "game" : "games"} · {fmtS(total)}</span>
    <span class="spacer"></span>
    <span class="sortl">Sort by</span>
    <div class="seg" role="radiogroup" aria-label="Sort by">
      <button role="radio" aria-checked={app.gameSort === "size"} class:on={app.gameSort === "size"} onclick={() => (app.gameSort = "size")}>
        Size
      </button>
      <button
        role="radio"
        aria-checked={app.gameSort === "played"}
        class:on={app.gameSort === "played"}
        onclick={() => (app.gameSort = "played")}>Last played</button
      >
    </div>
  </div>
  <div class="rows">
    {#each games as g, i (g.path)}
      {@const icon = app.launcherIcons[g.launcher]}
      {@const stale = g.last_played !== null && now - g.last_played >= 180 * 86400}
      <div class="row">
        <span class="lchip" title={LAUNCH[g.launcher]}>
          {#if icon}<img src={icon} alt="" />{:else}{LAUNCH[g.launcher][0]}{/if}
        </span>
        <div class="rmain">
          <span class="nm">{g.name}</span>
          <span class="rpath">{LAUNCH[g.launcher]} · <bdi dir="ltr">{g.path}</bdi></span>
          <div class="rbar">
            <div class="fill" style:width="{Math.max(2, (g.bytes / max) * 100).toFixed(1)}%" style:animation-delay="{i * 45}ms"></div>
          </div>
        </div>
        <span class="mono size">{fmtS(g.bytes)}</span>
        <span class="played" class:stale>
          <span class="pl">last played</span>
          {g.last_played === null ? "Unknown" : ago(g.last_played)}
        </span>
        <button class="pill-open" onclick={() => app.reveal(g.path)}>Open folder</button>
        <button class="pill-open" onclick={() => app.openLauncher(g.launcher)}>Open {LAUNCH[g.launcher]}</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .none {
    padding: 14px 10px;
    font-size: 13px;
    color: var(--muted);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 8px;
  }
  .count {
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .sortl {
    font-size: 12px;
    color: var(--muted-3);
  }
  .seg {
    display: flex;
    padding: 2px;
    border-radius: 99px;
    border: 1px solid var(--line-2);
    background: var(--surface);
  }
  .seg button {
    height: 24px;
    padding: 0 11px;
    border: none;
    border-radius: 99px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--line-2);
    color: var(--ink);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 9px;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .lchip {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--chip-bg);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    font-weight: 700;
    font-size: 13px;
    color: var(--muted);
  }
  .lchip img {
    width: 22px;
    height: 22px;
  }
  .rmain {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .nm {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rpath {
    font-size: 11px;
    color: var(--muted-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rbar {
    height: 5px;
    border-radius: 5px;
    background: var(--chip-bg);
    overflow: hidden;
    margin-top: 2px;
  }
  .fill {
    height: 100%;
    border-radius: 5px;
    background: var(--violet);
    animation: grow-width 0.8s var(--ease-out) both;
  }
  .size {
    font-weight: 600;
    font-size: 13px;
    min-width: 72px;
    text-align: right;
    flex-shrink: 0;
  }
  .played {
    width: 92px;
    display: flex;
    flex-direction: column;
    font-size: 12px;
    color: var(--muted);
    flex-shrink: 0;
  }
  .played.stale {
    color: var(--amber);
  }
  .pl {
    font-size: 10.5px;
    color: var(--muted-3);
  }
</style>
