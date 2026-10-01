<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { EXT_COLOR } from "$lib/copy";
  import { ageDays, fmtS } from "$lib/format";

  const r = $derived(app.spaceResult!);
  const inst = $derived(r.installers);
  const dlDrive = $derived(inst ? inst.dir.slice(0, 2).toUpperCase() : "");
  const list = $derived(inst ? [...inst.old].sort((a, b) => b.bytes - a.bytes) : []);
  const total = $derived(list.reduce((a, i) => a + i.bytes, 0));
  const max = $derived(Math.max(1, ...list.map((i) => i.bytes)));

  function ext(name: string): string {
    return name.slice(name.lastIndexOf(".") + 1).toLowerCase();
  }
</script>

{#if !inst}
  <div class="none">Couldn't find your Downloads folder, so there are no installers to list.</div>
{:else if dlDrive !== r.drive}
  <div class="none">Only the Downloads folder on {dlDrive} is checked for old installers, so there's nothing to show for {r.drive}.</div>
{:else if !list.length}
  <div class="none">No installers older than 30 days in Downloads.</div>
  {#if inst.newer}
    <div class="newer">{inst.newer} newer {inst.newer === 1 ? "installer" : "installers"} (under 30 days) not listed.</div>
  {/if}
{:else}
  <div class="tools">
    <span class="count">{list.length} {list.length === 1 ? "file" : "files"} · {fmtS(total)} · older than 30 days</span>
    {#if inst.newer}
      <span class="newer">{inst.newer} newer {inst.newer === 1 ? "installer" : "installers"} (under 30 days) not listed.</span>
    {/if}
  </div>
  <div class="rows">
    {#each list as f, i (f.path)}
      {@const e = ext(f.name)}
      <div class="row">
        <span class="badge mono" style:color={EXT_COLOR[e] ?? "var(--muted)"}>.{e}</span>
        <span class="nm" title={f.path}>{f.name}</span>
        <div class="rbar">
          <div class="fill" style:width="{Math.max(2, (f.bytes / max) * 100).toFixed(1)}%" style:animation-delay="{i * 45}ms"></div>
        </div>
        <span class="mono size">{fmtS(f.bytes)}</span>
        <span class="age">{ageDays(f.age_days)}</span>
        <button class="pill-open" title="Show in Explorer" onclick={() => app.reveal(f.path)}>Show</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .none {
    padding: 14px 10px 6px;
    font-size: 13px;
    color: var(--muted);
  }
  .tools {
    display: flex;
    align-items: baseline;
    gap: 14px;
    flex-wrap: wrap;
    padding: 0 8px 8px;
  }
  .count {
    font-size: 12.5px;
    font-weight: 600;
  }
  .newer {
    font-size: 12px;
    color: var(--muted-3);
    padding: 0 10px 8px;
  }
  .tools .newer {
    padding: 0;
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
    padding: 7px 10px;
    border-radius: 9px;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .badge {
    width: 46px;
    height: 24px;
    border-radius: 7px;
    background: var(--chip-bg);
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .nm {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rbar {
    width: 120px;
    height: 5px;
    border-radius: 5px;
    background: var(--chip-bg);
    overflow: hidden;
    flex-shrink: 0;
  }
  .fill {
    height: 100%;
    border-radius: 5px;
    background: var(--amber);
    animation: grow-width 0.8s var(--ease-out) both;
  }
  .size {
    font-weight: 600;
    font-size: 13px;
    min-width: 72px;
    text-align: right;
    flex-shrink: 0;
  }
  .age {
    width: 100px;
    font-size: 12px;
    color: var(--muted);
    flex-shrink: 0;
  }
</style>
