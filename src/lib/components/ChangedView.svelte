<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { dayCount, fmtS, parentPath, shortDate, signed, snapLabel, snapTitle } from "$lib/format";
  import WeeklySwitch from "./WeeklySwitch.svelte";

  let shrankOpen = $state(true);

  const r = $derived(app.spaceResult!);
  const older = $derived([...r.snapshots.older].sort((a, b) => b.taken_at - a.taken_at));
  const snap = $derived(app.compareSnap);
  const d = $derived(app.diff);
  const net = $derived(d?.net ?? snap?.net ?? 0);
  const grewSum = $derived(d?.grew.reduce((a, x) => a + x.delta, 0) ?? 0);
  const shrankSum = $derived(d?.shrank.reduce((a, x) => a - x.delta, 0) ?? 0);
  const grewMax = $derived(Math.max(1, ...(d?.grew.map((x) => x.now) ?? [])));
  const shrankMax = $derived(Math.max(1, ...(d?.shrank.map((x) => x.then) ?? [])));

  function pct(b: number, max: number): string {
    return Math.max(0, Math.min(100, (b / max) * 100)).toFixed(1) + "%";
  }
</script>

{#if !older.length}
  <div class="first">
    <div class="trail" aria-hidden="true">
      {#each [1, 0.7, 0.45, 0.22] as o, i (i)}<span style:opacity={o}></span>{/each}
    </div>
    {#if r.is_drive}
      <div class="fh display">Scan again later to see what changed</div>
      <div class="fb">
        Crumbtrail just saved its first snapshot of {r.drive}. Next time you scan, this tab shows which folders grew or shrank
        since then. A week or two apart works best.
      </div>
      {#if r.snapshots.saved}
        <span class="saved">
          Snapshot saved · {shortDate(r.snapshots.saved.taken_at)} · {fmtS(r.snapshots.saved.used_bytes)} used
        </span>
      {/if}
      {#if r.drive === "C:"}
        <div class="weekly-card"><WeeklySwitch /></div>
      {/if}
    {:else}
      <div class="fh display">Nothing to compare with yet</div>
      <div class="fb">
        What changed compares folders with a saved scan of the whole {r.drive} drive. Scan {r.drive} once now, then again later,
        and this view shows what grew inside this folder.
      </div>
    {/if}
    <button class="btn-secondary sm" onclick={() => app.setView("hot")}>See biggest folders now</button>
  </div>
{:else}
  <div class="head">
    <div class="hl">
      <div class="big">
        <span class="num display" class:grow={net > 0} class:shrink={net < 0}>{signed(net)}</span>
        {#if snap}<span class="since">since {snapLabel(snap.taken_at)}</span>{/if}
      </div>
      {#if snap}
        <div class="hsub">
          {#if d}Grew {fmtS(grewSum)} · shrank {fmtS(shrankSum)} ·{/if}
          compared with the scan from {shortDate(snap.taken_at)} ({dayCount(snap.taken_at)})
        </div>
      {/if}
    </div>
    <div class="pick-wrap" data-pop>
      <button class="btn-secondary sm" aria-haspopup="listbox" aria-expanded={app.popover === "snap"} onclick={() => app.togglePopover("snap")}>
        Compare with <strong>{snap ? snapTitle(snap.taken_at) : "…"}</strong> ▾
      </button>
      {#if app.popover === "snap"}
        <div class="pop" role="listbox" aria-label="Compare with">
          {#each older as s (s.id)}
            {@const on = s.id === app.diffId}
            <button class="opt" role="option" aria-selected={on} onclick={() => app.compareWith(s.id)}>
              <span class="radio" class:on></span>
              <span class="otext">
                <span class="ot">{snapTitle(s.taken_at)}</span>
                <span class="os">{shortDate(s.taken_at)} · {fmtS(s.used_bytes)} used</span>
              </span>
              {#if s.net !== null}
                <span class="mono onet" class:grow={s.net > 0} class:shrink={s.net < 0}>{signed(s.net)}</span>
              {/if}
            </button>
          {/each}
          <div class="pfoot">
            Kept on this PC in <span class="mono">%LOCALAPPDATA%\Crumbtrail\snapshots</span>. Only folder names and sizes are saved.
          </div>
          <div class="sw-row"><WeeklySwitch /></div>
        </div>
      {/if}
    </div>
  </div>

  {#if app.diffError}
    <div class="err">{app.diffError}</div>
  {:else if app.diffLoading && !d}
    <div class="loading">Comparing…</div>
  {:else if d}
    <div class="sect">Grew the most · {d.grew.length} {d.grew.length === 1 ? "folder" : "folders"}</div>
    {#if !d.grew.length}
      <div class="none">No folder grew by more than 50 MB.</div>
    {/if}
    <div class="rows">
      {#each d.grew as g, i (g.path)}
        <div class="row">
          <span class="rank mono">{i + 1}</span>
          <div class="rmain">
            <div class="rname">
              <span class="nm">{g.name}</span>
              {#if g.is_new}<span class="new">new</span>{/if}
            </div>
            <div class="rpath"><bdi dir="ltr">{parentPath(g.path)}</bdi></div>
          </div>
          <div class="cmp">
            <div class="bar">
              <span class="was" style:width={pct(g.then, grewMax)}></span>
              <span class="add" style:width={pct(g.delta, grewMax)} style:animation-delay="{i * 45}ms"></span>
            </div>
            <div class="cap">{g.is_new ? "new folder" : `was ${fmtS(g.then)}`}</div>
          </div>
          <div class="right">
            <div class="mono delta">{signed(g.delta)}</div>
            <div class="now">{fmtS(g.now)} now</div>
          </div>
          <button class="pill-open" onclick={() => app.reveal(g.path)}>Open</button>
        </div>
      {/each}
    </div>

    {#if d.shrank.length}
      <button class="shrank-head" aria-expanded={shrankOpen} onclick={() => (shrankOpen = !shrankOpen)}>
        <span class="chev" class:open={shrankOpen}>›</span>
        Shrank · {d.shrank.length} {d.shrank.length === 1 ? "folder" : "folders"}
        <span class="mono">{signed(-shrankSum)}</span>
      </button>
      {#if shrankOpen}
        <div class="rows small">
          {#each d.shrank as s (s.path)}
            <div class="row">
              <div class="rmain">
                <span class="nm">{s.name}</span>
                <div class="rpath"><bdi dir="ltr">{parentPath(s.path)}</bdi></div>
              </div>
              <div class="cmp">
                <div class="bar thin">
                  <span class="cur" style:width={pct(s.now, shrankMax)}></span>
                  <span class="gone" style:width={pct(-s.delta, shrankMax)}></span>
                </div>
              </div>
              <div class="right">
                <div class="mono delta quiet">{signed(s.delta)}</div>
                <div class="now">{fmtS(s.now)} now</div>
              </div>
              <button class="pill-open" onclick={() => app.reveal(s.path)}>Open</button>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
{/if}

<style>
  .first {
    border: 1px dashed var(--line-3);
    border-radius: 12px;
    padding: 28px 24px;
    margin: 0 4px 4px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
  }
  .trail {
    display: flex;
    gap: 7px;
    margin-bottom: 4px;
  }
  .trail span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  .fh {
    font-weight: 700;
    font-size: 16px;
  }
  .fb {
    font-size: 13px;
    color: var(--muted);
    max-width: 52ch;
    line-height: 1.5;
    text-wrap: pretty;
  }
  .saved {
    margin: 4px 0;
    padding: 4px 11px;
    border-radius: 99px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 2px 8px 12px;
  }
  .hl {
    min-width: 0;
  }
  .big {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }
  .num {
    font-weight: 800;
    font-size: 40px;
    line-height: 1;
    letter-spacing: -0.03em;
  }
  .grow {
    color: var(--amber);
  }
  .shrink {
    color: var(--accent);
  }
  .since {
    font-size: 18px;
    font-weight: 600;
    color: var(--muted);
  }
  .hsub {
    margin-top: 8px;
    font-size: 12.5px;
    color: var(--muted);
    text-wrap: pretty;
  }
  .pick-wrap {
    position: relative;
    flex-shrink: 0;
  }
  .pick-wrap strong {
    font-weight: 700;
  }
  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 10;
    width: 330px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--line-3);
    border-radius: 14px;
    box-shadow: var(--shadow-welcome);
    animation: fade-in 0.18s ease;
  }
  .opt {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .opt:hover {
    background: var(--row-hover);
  }
  .radio {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    border: 1.5px solid var(--line-4);
    flex-shrink: 0;
  }
  .radio.on {
    border: 4.5px solid var(--accent);
  }
  .otext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .ot {
    font-size: 13px;
    font-weight: 600;
  }
  .os {
    font-size: 11.5px;
    color: var(--muted-3);
  }
  .onet {
    font-size: 12px;
    font-weight: 600;
  }
  .pfoot {
    margin-top: 6px;
    padding: 8px 10px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--muted-3);
    line-height: 1.45;
  }
  .sw-row {
    margin: 4px 0 0;
    padding-top: 4px;
    border-top: 1px solid var(--line);
  }
  .weekly-card {
    width: min(400px, 100%);
    padding: 2px;
    border: 1px solid var(--line-2);
    border-radius: 12px;
  }

  .err {
    margin: 0 8px 8px;
    font-size: 12.5px;
    color: var(--amber);
  }
  .loading,
  .none {
    padding: 6px 10px 10px;
    font-size: 12.5px;
    color: var(--muted-3);
  }
  .sect {
    padding: 4px 10px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
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
  .rank {
    width: 22px;
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    flex-shrink: 0;
  }
  .rmain {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rname {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .nm {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .new {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--amber-soft);
    color: var(--amber);
    flex-shrink: 0;
  }
  .rpath {
    font-size: 11px;
    color: var(--muted-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .cmp {
    width: 150px;
    flex-shrink: 0;
  }
  .bar {
    display: flex;
    height: 6px;
    border-radius: 6px;
    background: var(--chip-bg);
    overflow: hidden;
  }
  .bar.thin {
    height: 4px;
    overflow: visible;
    background: transparent;
  }
  .was {
    background: var(--line-4);
  }
  .add {
    background: var(--amber);
    animation: grow-width 0.8s var(--ease-out) both;
  }
  .cur {
    background: var(--line-4);
    border-radius: 4px 0 0 4px;
  }
  .gone {
    border: 1px dashed var(--line-4);
    border-left: none;
    border-radius: 0 4px 4px 0;
  }
  .cap {
    font-size: 10.5px;
    color: var(--muted-3);
    margin-top: 4px;
  }
  .right {
    width: 90px;
    text-align: right;
    flex-shrink: 0;
  }
  .delta {
    font-size: 13px;
    font-weight: 600;
    color: var(--amber);
  }
  .delta.quiet {
    color: var(--muted);
  }
  .now {
    font-size: 11px;
    color: var(--muted-3);
  }
  .shrank-head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin-top: 8px;
    padding: 8px 10px;
    border: none;
    border-top: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .chev {
    display: inline-block;
    transition: transform 0.3s var(--spring);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .small .row {
    padding: 6px 10px 6px 44px;
  }
  .small .nm {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--muted);
  }
</style>
