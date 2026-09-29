<script lang="ts">
  import { fade } from "svelte/transition";
  import { app } from "$lib/app.svelte";
  import { RISK_LABEL, riskOf } from "$lib/copy";
  import { fmt, plural } from "$lib/format";
  import { sheetSlide } from "$lib/motion";

  const perm = $derived(app.selected.filter((c) => riskOf(c.id) === "perm"));
</script>

<div class="scrim" role="presentation" onclick={() => app.closeSheet()} transition:fade={{ duration: 350 }}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="sheet-title" transition:sheetSlide>
  <div class="grip"></div>
  <div class="head">
    <div id="sheet-title" class="title display">Clean {fmt(app.selectedBytes)}?</div>
    <p class="sub">
      You're about to clean {plural(app.selected.length, "item", "items")}. Nothing else on your PC is touched, and files in use are
      skipped.
    </p>
  </div>
  <div class="list">
    {#each app.selected as c (c.id)}
      {@const risk = riskOf(c.id)}
      <div class="item">
        <span class="nm">{c.name}</span>
        <span class="chip {risk}">{RISK_LABEL[risk]}</span>
        <span class="mono size">{fmt(app.sizes[c.id]?.bytes ?? 0)}</span>
      </div>
    {/each}
  </div>
  {#if perm.length}
    <div class="warn">{perm.map((c) => c.name).join(", ")} can't be restored once cleaned.</div>
  {/if}
  <div class="actions">
    <button class="btn-secondary cancel" onclick={() => app.closeSheet()}>Cancel</button>
    <button class="btn-primary" onclick={() => app.doClean()}>Clean now</button>
  </div>
</div>

<style>
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 20;
    background: var(--scrim);
  }
  .sheet {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 21;
    max-height: 72%;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-top: 1px solid var(--line-3);
    border-radius: 18px 18px 0 0;
    padding: 10px 24px 18px;
    box-shadow: var(--shadow-sheet);
  }
  .grip {
    width: 36px;
    height: 4px;
    border-radius: 4px;
    background: var(--line-4);
    margin: 0 auto 14px;
    flex-shrink: 0;
  }
  .head {
    flex-shrink: 0;
  }
  .title {
    font-weight: 700;
    font-size: 19px;
  }
  .sub {
    margin: 4px 0 12px;
    color: var(--muted);
    font-size: 13px;
    text-wrap: pretty;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 12px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--line-row);
    font-size: 13px;
  }
  .nm {
    flex: 1;
    min-width: 0;
    font-weight: 500;
  }
  .size {
    font-weight: 600;
    min-width: 84px;
    text-align: right;
  }
  .warn {
    flex-shrink: 0;
    margin-top: 10px;
    padding: 10px 14px;
    border-radius: 12px;
    background: var(--amber-panel);
    border: 1px solid var(--amber-panel-line);
    color: var(--amber);
    font-size: 12.5px;
    text-wrap: pretty;
  }
  .actions {
    flex-shrink: 0;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .cancel {
    padding: 0 18px;
  }
</style>
