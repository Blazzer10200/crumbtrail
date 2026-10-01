<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "$lib/app.svelte";
  import { SYSTEM_FILES, TYPES } from "$lib/copy";
  import { baseName, fmtS, parentPath } from "$lib/format";
  import FileRow from "./FileRow.svelte";

  const r = $derived(app.spaceResult!);
  const total = $derived(Math.max(1, r.bytes));
  const segs = $derived(
    TYPES.map((t) => {
      const bytes = r.types.find((x) => x.key === t.key)?.bytes ?? 0;
      return { ...t, bytes, pct: (bytes / total) * 100 };
    }),
  );
  const sel = $derived(segs.find((s) => s.key === app.typeSel) ?? null);
  const files = $derived(sel ? (r.type_top[sel.key] ?? []) : []);
  const max = $derived(files[0]?.bytes ?? 1);

  // Segments grow in from zero on view load.
  let grown = $state(false);
  onMount(() => {
    const id = requestAnimationFrame(() => (grown = true));
    return () => cancelAnimationFrame(id);
  });

  function pick(key: (typeof TYPES)[number]["key"]) {
    app.typeSel = app.typeSel === key ? null : key;
  }

  function pctLabel(p: number): string {
    return p > 0 && p < 1 ? "<1%" : `${Math.round(p)}%`;
  }
</script>

<div class="stack" role="group" aria-label="Space by file type">
  {#each segs.filter((s) => s.bytes > 0) as s (s.key)}
    {@const on = app.typeSel === s.key}
    {@const dim = !!app.typeSel && !on}
    <button
      class="seg"
      class:dim
      style:flex-grow={grown ? s.bytes : 0}
      style:--c={s.color}
      aria-pressed={on}
      title="{s.label} · {fmtS(s.bytes)}"
      onclick={() => pick(s.key)}
    >
      {s.pct >= 12 ? s.label : s.pct >= 5 ? pctLabel(s.pct) : ""}
    </button>
  {/each}
</div>

<div class="legend">
  {#each segs as s (s.key)}
    <button class="lg" class:on={app.typeSel === s.key} aria-pressed={app.typeSel === s.key} onclick={() => pick(s.key)}>
      <span class="dot" style:background={s.color}></span>
      <span class="ll">{s.label}</span>
      <span class="mono lv">{fmtS(s.bytes)}</span>
      <span class="lp">{pctLabel(s.pct)}</span>
    </button>
  {/each}
</div>

{#if sel}
  <div class="sect">
    Biggest {sel.plural}
    <span class="mono sv">{fmtS(sel.bytes)} of {fmtS(r.bytes)} used</span>
  </div>
  {#key sel.key}
    <div class="rows">
      {#each files as f, i (f.path)}
        {@const name = baseName(f.path) || f.name}
        <FileRow
          rank={i + 1}
          {name}
          path={parentPath(f.path)}
          bytes={f.bytes}
          {max}
          color={sel.color}
          sys={SYSTEM_FILES.has(name.toLowerCase())}
          delay={i * 45}
          onrow={() => app.reveal(f.path)}
          onopen={() => app.reveal(f.path)}
        />
      {/each}
    </div>
  {/key}
{:else}
  <div class="pickhint">Pick a segment above to list that type's biggest files.</div>
{/if}

<style>
  .stack {
    display: flex;
    gap: 3px;
    height: 40px;
    margin: 0 8px;
  }
  .seg {
    flex-basis: 0;
    min-width: 0;
    border: none;
    border-radius: 6px;
    background: var(--c);
    color: var(--bg);
    font-size: 11.5px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    cursor: pointer;
    transition:
      flex-grow 0.8s var(--ease-out),
      background 0.25s,
      color 0.25s;
  }
  .seg.dim {
    background: color-mix(in srgb, var(--c) 28%, transparent);
    color: var(--ink);
  }
  .legend {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 4px;
    margin: 10px 4px 8px;
  }
  .lg {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: 9px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .lg:hover {
    background: var(--row-hover);
  }
  .lg.on {
    border-color: var(--line-3);
    color: var(--ink);
  }
  .ll {
    flex: 1;
  }
  .lv {
    color: var(--ink);
    font-weight: 600;
  }
  .lp {
    width: 36px;
    text-align: right;
    font-size: 11.5px;
    color: var(--muted-3);
  }
  .sect {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 10px;
    border-top: 1px solid var(--line);
    font-size: 12.5px;
    font-weight: 600;
  }
  .sv {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted-3);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pickhint {
    padding: 10px;
    border-top: 1px solid var(--line);
    font-size: 12.5px;
    color: var(--muted-3);
  }
</style>
