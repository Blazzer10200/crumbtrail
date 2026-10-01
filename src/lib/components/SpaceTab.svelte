<script lang="ts">
  // View-only by contract (AGENTS.md): this tab may reveal paths in Explorer, never delete.
  import { tick } from "svelte";
  import { app } from "$lib/app.svelte";
  import { MORE, MORE_W, SYSTEM_FILES, VIEWS } from "$lib/copy";
  import { baseName, fmtS, parentPath, rootLabel } from "$lib/format";
  import type { Folder, SpaceView } from "$lib/types";
  import ChangedView from "./ChangedView.svelte";
  import FileRow from "./FileRow.svelte";
  import GamesView from "./GamesView.svelte";
  import InstallersView from "./InstallersView.svelte";
  import TypesView from "./TypesView.svelte";

  const RING = 201.06; // 2πr for r = 32
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  type Row = { key: string; name: string; path: string; bytes: number; sys: boolean; folder?: Folder };

  const root = $derived(rootLabel(app.spaceResult?.root ?? app.spaceRoot));
  const viewMeta = $derived([...VIEWS, ...MORE].find((v) => v.key === app.view) ?? VIEWS[0]);
  // Folder scans hide the More slot: games, installers and file types are drive-wide.
  const showMore = $derived(!!app.spaceResult?.is_drive);
  const moreActive = $derived(MORE.find((v) => v.key === app.view) ?? null);
  const slots = $derived([
    ...VIEWS.map((v) => ({ key: v.key as SpaceView | "more", label: v.label, w: v.w })),
    ...(showMore ? [{ key: "more" as const, label: moreActive?.label ?? "More", w: moreActive?.w ?? MORE_W }] : []),
  ]);
  const thumb = $derived.by(() => {
    const key = moreActive ? "more" : app.view;
    const i = Math.max(0, slots.findIndex((s) => s.key === key));
    return { x: slots.slice(0, i).reduce((a, s) => a + s.w, 0), w: slots[i]?.w ?? 0 };
  });
  const hasSnap = $derived((app.spaceResult?.snapshots.older.length ?? 0) > 0);

  function moreMeta(key: SpaceView): string {
    const r = app.spaceResult;
    if (!r) return "";
    if (key === "games") return `${r.games.length} installed`;
    if (key === "installers") {
      const n = r.installers && r.installers.dir.slice(0, 2).toUpperCase() === r.drive ? r.installers.old.length : 0;
      return `${n} ${n === 1 ? "file" : "files"}`;
    }
    return `${r.types.filter((t) => t.bytes > 0).length} types`;
  }

  function unreadableText(paths: string[], count: number): string {
    const shown = paths.length < 2 ? (paths[0] ?? "") : `${paths.slice(0, -1).join(", ")} and ${paths[paths.length - 1]}`;
    const rest = count - paths.length;
    return rest > 0 ? `${shown} and ${rest.toLocaleString()} more` : shown;
  }

  // Bring fresh results into view once per finished scan (not on tab remount).
  let resultEl = $state<HTMLDivElement>();
  let seenTick = app.spaceTick;
  $effect(() => {
    const t = app.spaceTick;
    if (t === seenTick) return;
    seenTick = t;
    void tick().then(() => resultEl?.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "start" }));
  });

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
    <div class="result" bind:this={resultEl}>
      {#if r.unreadable.count > 0}
        <div class="unread">
          <span class="utext">
            {r.unreadable.count === 1 ? "1 folder" : `${r.unreadable.count.toLocaleString()} folders`} couldn't be read (access
            denied): {unreadableText(r.unreadable.paths, r.unreadable.count)}. Totals here may be a little low.
          </span>
          {#if !app.admin}
            <button class="btn-secondary sm" onclick={() => app.relaunchAdmin()}>Restart as admin</button>
          {/if}
        </div>
      {/if}

      <div class="rhead">
        <div class="rtext">
          <div class="rt display">What's using space on {root}</div>
          <div class="rs">{r.files.toLocaleString()} files · {fmtS(r.bytes)} seen</div>
        </div>
        <span class="spacer"></span>
        <div class="views" role="tablist">
          <div class="vthumb" style:transform="translateX({thumb.x}px)" style:width="{thumb.w}px"></div>
          {#each slots as s (s.key)}
            {#if s.key === "more"}
              <div class="more-wrap" data-pop>
                <button
                  role="tab"
                  aria-selected={!!moreActive}
                  aria-haspopup="menu"
                  aria-expanded={app.popover === "more"}
                  class:on={!!moreActive}
                  style:width="{s.w}px"
                  onclick={() => app.togglePopover("more")}>{s.label} ▾</button
                >
                {#if app.popover === "more"}
                  <div class="menu" role="menu">
                    {#each MORE as m (m.key)}
                      <button class="mi" role="menuitem" class:cur={app.view === m.key} onclick={() => app.setView(m.key)}>
                        <span class="dot" style:background={m.color}></span>
                        <span class="mtext">
                          <span class="ml">{m.label}</span>
                          <span class="md">{m.desc}</span>
                        </span>
                        <span class="mono mm">{moreMeta(m.key)}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {:else}
              <button
                role="tab"
                aria-selected={app.view === s.key}
                class:on={app.view === s.key}
                style:width="{s.w}px"
                onclick={() => app.setView(s.key as SpaceView)}
              >
                {s.label}
                {#if s.key === "changed" && hasSnap && app.view !== "changed"}<span class="adot"></span>{/if}
              </button>
            {/if}
          {/each}
        </div>
      </div>

      <div class="panel">
        <div class="hint">{viewMeta.hint}</div>
        {#if app.view === "changed"}
          <ChangedView />
        {:else if app.view === "games"}
          <GamesView />
        {:else if app.view === "installers"}
          <InstallersView />
        {:else if app.view === "types"}
          <TypesView />
        {:else}
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
                <FileRow
                  rank={i + 1}
                  name={row.name}
                  path={row.path}
                  bytes={row.bytes}
                  {max}
                  color={viewMeta.color}
                  sys={row.sys}
                  kid={!!row.folder}
                  delay={i * 45}
                  onrow={() => onRow(row)}
                  onopen={() => app.reveal(fullPath(row))}
                />
              {/each}
            </div>
          {/key}
        {/if}
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
    height: 28px;
    border-radius: 999px;
    background: var(--line-2);
    transition:
      transform 0.45s var(--spring),
      width 0.45s var(--spring);
  }
  .adot {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-left: 4px;
    border-radius: 50%;
    background: var(--amber);
    vertical-align: middle;
  }
  .more-wrap {
    position: relative;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 8px);
    z-index: 10;
    width: 310px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--line-3);
    border-radius: 14px;
    box-shadow: var(--shadow-welcome);
    animation: fade-in 0.18s ease;
  }
  .mi {
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
  .mi:hover,
  .mi.cur {
    background: var(--row-hover);
  }
  .mtext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .ml {
    font-size: 13px;
    font-weight: 600;
  }
  .md {
    font-size: 11.5px;
    color: var(--muted-3);
  }
  .mm {
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .unread {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 9px 9px 14px;
    border-radius: 12px;
    background: var(--amber-panel);
    border: 1px solid var(--amber-panel-line);
    color: var(--amber);
    font-size: 12.5px;
  }
  .utext {
    flex: 1;
    min-width: 0;
    text-wrap: pretty;
    word-break: break-word;
  }
  .views button {
    position: relative;
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
</style>
