<script lang="ts">
  // View-only by contract (AGENTS.md): this tab may reveal paths in Explorer, never delete.
  import { app } from "$lib/app.svelte";
  import { SYSTEM_FILES, VIEWS } from "$lib/copy";
  import { baseName, fmtS, parentPath, rootLabel } from "$lib/format";
  import type { Folder } from "$lib/types";

  const RING = 201.06; // 2πr for r = 32

  type Row = { key: string; name: string; path: string; bytes: number; sys: boolean; folder?: Folder };

  const root = $derived(rootLabel(app.spaceResult?.root ?? app.spaceRoot));
  const viewMeta = $derived(VIEWS.find((v) => v.key === app.view) ?? VIEWS[0]);
  const viewIndex = $derived(VIEWS.findIndex((v) => v.key === app.view));

  const rows = $derived.by((): Row[] => {
    const r = app.spaceResult;
    if (!r) return [];
    let list: Row[];
    if (app.view === "hot") {
      list = r.hotspots.map((f) => ({ key: f.path, name: baseName(f.path) || f.name, path: parentPath(f.path), bytes: f.bytes, sys: false }));
    } else if (app.view === "big") {
      list = r.biggest.map((f) => {
        const name = baseName(f.path) || f.name;
        return { key: f.path, name, path: parentPath(f.path), bytes: f.bytes, sys: SYSTEM_FILES.has(name.toLowerCase()) };
      });
    } else {
      list = app.browseEntries.map((f) => ({ key: f.path, name: f.name, path: "", bytes: f.bytes, sys: false, folder: f }));
    }
    return list.sort((a, b) => b.bytes - a.bytes);
  });
  const max = $derived(rows[0]?.bytes || 1);

  function fullPath(row: Row): string {
    return row.folder ? row.folder.path : row.key;
  }

  function onRow(row: Row) {
    if (row.folder) app.drillInto(row.folder);
    else app.reveal(fullPath(row));
  }
</script>

<main>
  <div class="bar">
    <span class="h display">Your drives</span>
    <span class="viewonly"><span class="vdot"></span>View only · nothing here deletes files</span>
    <span class="spacer"></span>
    <button class="btn-secondary sm" disabled={app.spaceScanning} onclick={() => app.scanFolder()}>Scan a folder…</button>
  </div>

  {#if app.spaceError}
    <div class="error">{app.spaceError}</div>
  {/if}

  <div class="drives">
    {#each app.drives as d (d.letter)}
      {@const used = d.total - d.free}
      {@const f = d.total ? used / d.total : 0}
      {@const active = app.spaceRoot === d.letter + "\\" && (app.spaceScanning || !!app.spaceResult)}
      <button class="drive" class:active disabled={app.spaceScanning} onclick={() => app.scanDrive(d.letter)}>
        <span class="ring">
          <svg width="76" height="76" viewBox="0 0 76 76">
            <circle cx="38" cy="38" r="32" fill="none" stroke="var(--line)" stroke-width="8" />
            <circle
              class="arc"
              cx="38"
              cy="38"
              r="32"
              fill="none"
              stroke={f > 0.9 ? "var(--amber)" : "var(--accent)"}
              stroke-width="8"
              stroke-linecap="round"
              stroke-dasharray={RING}
              style:stroke-dashoffset={(RING * (1 - f)).toFixed(2)}
            />
          </svg>
          <span class="pct">
            <span class="pv display">{Math.round(f * 100)}%</span>
            <span class="pl">used</span>
          </span>
        </span>
        <span class="dinfo">
          <span class="dtitle">
            <span class="letter display">{d.letter}</span>
            {#if f > 0.9}<span class="chip perm">Almost full</span>{/if}
          </span>
          <span class="stats">
            <span class="stat"><span class="mono sv">{fmtS(d.free)}</span><span class="sl">free</span></span>
            <span class="stat"><span class="mono sv">{fmtS(used)}</span><span class="sl">used</span></span>
            <span class="stat"><span class="mono sv">{fmtS(d.total)}</span><span class="sl">total</span></span>
          </span>
          <span class="cta">
            {active && app.spaceScanning ? "Scanning…" : active ? "Scan again →" : `Scan ${d.letter} →`}
          </span>
        </span>
      </button>
    {/each}
  </div>

  {#if app.spaceScanning}
    <div class="scanning">
      <div class="st"><span class="dot breathe" style:background="var(--amber)"></span><span>Mapping {root}</span></div>
      <div class="prog">
        <span class="mono pf">{app.spaceProgress.files.toLocaleString()} files</span>
        <span class="pb">{fmtS(app.spaceProgress.bytes)} so far</span>
      </div>
      <div class="track"><div class="runner"></div></div>
      <div class="note">Reading folder sizes only. Nothing is moved or deleted.</div>
    </div>
  {:else if !app.spaceResult}
    <div class="empty">
      <div class="eh display">See where your space went</div>
      <div class="eb">
        Pick a drive above and Crumbtrail maps its biggest folders and files. Takes under 20 seconds, even on a 2 TB drive.
      </div>
    </div>
  {:else}
    {@const r = app.spaceResult}
    <div class="result">
      <div class="rhead">
        <div class="rtext">
          <div class="rt display">What's using space on {root}</div>
          <div class="rs">{r.files.toLocaleString()} files · {fmtS(r.bytes)} seen</div>
        </div>
        <span class="spacer"></span>
        <div class="views" role="tablist">
          <div class="vthumb" style:transform="translateX({viewIndex * 112}px)"></div>
          {#each VIEWS as v (v.key)}
            <button role="tab" aria-selected={app.view === v.key} class:on={app.view === v.key} onclick={() => app.setView(v.key)}>
              {v.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="panel">
        <div class="hint">{viewMeta.hint}</div>
        {#if app.view === "browse"}
          <div class="crumbs">
            {#each [{ name: root, i: -1 }, ...app.crumbs.map((c, i) => ({ name: c.name, i }))] as cr, n (cr.i)}
              {#if n > 0}<span class="sep">›</span>{/if}
              <button class="crumb" class:cur={n === app.crumbs.length} onclick={() => app.jumpTo(cr.i)}>{cr.name}</button>
            {/each}
          </div>
        {/if}
        {#key app.rowsKey}
          <div class="rows">
            {#each rows as row, i (row.key)}
              <div
                class="row"
                role="button"
                tabindex="0"
                title={row.folder ? "Look inside" : "Open in Explorer"}
                onclick={() => onRow(row)}
                onkeydown={(e) => {
                  if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onRow(row);
                  }
                }}
              >
                <span class="rank mono">{i + 1}</span>
                <div class="rmain">
                  <div class="rname">
                    <span class="nm">{row.name}</span>
                    {#if row.sys}<span class="sys" title="Managed by Windows. Not a file you can delete.">system</span>{/if}
                    {#if row.folder}<span class="kid">›</span>{/if}
                  </div>
                  {#if row.path}
                    <div class="rpath"><bdi dir="ltr">{row.path}</bdi></div>
                  {/if}
                  <div class="rbar">
                    <div
                      class="fill"
                      style:background={viewMeta.color}
                      style:width="{Math.max(2, (row.bytes / max) * 100).toFixed(1)}%"
                      style:animation-delay="{i * 45}ms"
                    ></div>
                  </div>
                </div>
                <span class="rsize mono">{fmtS(row.bytes)}</span>
                <button
                  class="open"
                  title="Open in Explorer"
                  onclick={(e) => {
                    e.stopPropagation();
                    app.reveal(fullPath(row));
                  }}>Open</button
                >
              </div>
            {/each}
          </div>
        {/key}
      </div>
    </div>
  {/if}
</main>

<style>
  main {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .h {
    font-weight: 700;
    font-size: 15px;
  }
  .viewonly {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border-radius: 99px;
    background: var(--sky-soft);
    color: var(--sky);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }
  .vdot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--sky);
  }
  .spacer {
    flex: 1;
  }
  .error {
    flex-shrink: 0;
    padding: 10px 14px;
    border-radius: 12px;
    background: var(--amber-panel);
    border: 1px solid var(--amber-panel-line);
    color: var(--amber);
    font-size: 12.5px;
  }

  .drives {
    flex-shrink: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 10px;
  }
  .drive {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--card);
    border: 1px solid var(--line);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.25s,
      transform 0.3s var(--spring),
      box-shadow 0.3s;
  }
  .drive.active {
    border-color: var(--accent-line);
    box-shadow: var(--active-ring);
  }
  .drive:hover:not(:disabled) {
    transform: translateY(-2px);
    border-color: var(--line-4);
  }
  .ring {
    position: relative;
    width: 76px;
    height: 76px;
    flex-shrink: 0;
  }
  .ring svg {
    transform: rotate(-90deg);
  }
  .arc {
    transition: stroke-dashoffset 1.1s var(--ease-out);
    animation: ring-in 1.1s var(--ease-out) 180ms both;
  }
  .pct {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .pv {
    font-weight: 700;
    font-size: 17px;
    line-height: 1;
  }
  .pl {
    font-size: 10px;
    color: var(--muted-3);
    margin-top: 2px;
  }
  .dinfo {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .dtitle {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .letter {
    font-weight: 700;
    font-size: 19px;
  }
  .stats {
    display: flex;
    gap: 18px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .sv {
    font-weight: 600;
    font-size: 12.5px;
  }
  .sl {
    font-size: 11px;
    color: var(--muted-3);
  }
  .cta {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent);
  }

  .scanning {
    flex-shrink: 0;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 16px 18px;
  }
  .st {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .prog {
    display: flex;
    align-items: baseline;
    gap: 16px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .pf {
    font-weight: 600;
    font-size: 20px;
  }
  .pb {
    font-size: 12.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 4px;
    border-radius: 4px;
    background: var(--line);
    overflow: hidden;
    margin-top: 12px;
  }
  .runner {
    position: absolute;
    top: 0;
    left: 0;
    width: 40%;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    animation: slide 1.3s var(--sweep) infinite;
  }
  .note {
    font-size: 12px;
    color: var(--muted-3);
    margin-top: 10px;
  }

  .empty {
    flex-shrink: 0;
    border: 1px dashed var(--line-3);
    border-radius: 14px;
    padding: 36px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
  }
  .eh {
    font-weight: 700;
    font-size: 16px;
  }
  .eb {
    font-size: 13px;
    color: var(--muted);
    max-width: 44ch;
    line-height: 1.5;
    text-wrap: pretty;
  }

  .result {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    animation:
      fade-in 0.4s ease,
      rise-14 0.55s var(--ease-out);
  }
  .rhead {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .rtext {
    min-width: 0;
  }
  .rt {
    font-weight: 700;
    font-size: 15px;
  }
  .rs {
    font-size: 12px;
    color: var(--muted);
    margin-top: 1px;
  }
  .views {
    position: relative;
    display: flex;
    background: var(--surface);
    border: 1px solid var(--line-2);
    border-radius: 999px;
    padding: 3px;
  }
  .vthumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 112px;
    height: 28px;
    border-radius: 999px;
    background: var(--line-2);
    transition: transform 0.45s var(--spring);
  }
  .views button {
    position: relative;
    width: 112px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: 999px;
    font-weight: 600;
    font-size: 12.5px;
    cursor: pointer;
    color: var(--muted);
    transition: color 0.3s;
    white-space: nowrap;
  }
  .views button.on {
    color: var(--ink);
  }

  .panel {
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 8px;
  }
  .hint {
    padding: 4px 8px 8px;
    font-size: 12px;
    color: var(--muted-3);
    text-wrap: pretty;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    padding: 0 4px 8px;
  }
  .sep {
    color: var(--faint);
    font-size: 12px;
  }
  .crumb {
    border: none;
    background: transparent;
    color: var(--accent);
    font-weight: 600;
    font-size: 12.5px;
    padding: 3px 9px;
    border-radius: 99px;
    cursor: pointer;
  }
  .crumb.cur {
    background: var(--chip-bg);
    color: var(--ink);
  }
  .crumb:hover {
    background: var(--chip-bg);
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
    cursor: pointer;
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
    gap: 5px;
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
  .sys {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--chip-bg);
    color: var(--muted);
    flex-shrink: 0;
  }
  .kid {
    font-size: 12px;
    color: var(--faint);
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
  .rbar {
    height: 5px;
    border-radius: 5px;
    background: var(--chip-bg);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 5px;
    animation: grow-width 0.8s var(--ease-out) both;
  }
  .rsize {
    font-weight: 600;
    font-size: 13px;
    min-width: 78px;
    text-align: right;
    flex-shrink: 0;
  }
  .open {
    height: 26px;
    padding: 0 10px;
    border-radius: 99px;
    border: 1px solid var(--line-3);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }
  .open:hover {
    color: var(--ink);
    border-color: var(--line-4);
  }
</style>
