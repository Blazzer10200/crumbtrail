<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { shortDate } from "$lib/format";

  const on = $derived(!!app.weekly?.enabled);
  const hint = $derived(
    on
      ? `Runs quietly in the background, folder sizes only.${app.weekly?.next_run ? ` Next: ${shortDate(app.weekly.next_run)}.` : ""}`
      : "Snapshots are only saved when you scan.",
  );
</script>

<button
  class="row"
  role="switch"
  aria-checked={on}
  disabled={!app.weekly || app.weeklyBusy}
  onclick={() => app.setWeekly(!on)}
>
  <span class="text">
    <span class="t">Save a snapshot every week</span>
    <span class="s">{hint}</span>
    {#if app.weeklyError}<span class="err">{app.weeklyError}</span>{/if}
  </span>
  <span class="track" class:on><span class="knob"></span></span>
</button>

<style>
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .row:hover:not(:disabled) {
    background: var(--row-hover);
  }
  .row:disabled {
    cursor: progress;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .t {
    font-size: 12.5px;
    font-weight: 600;
  }
  .s {
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--muted-3);
  }
  .err {
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--amber);
  }
  .track {
    position: relative;
    width: 38px;
    height: 22px;
    border-radius: 99px;
    background: var(--line-3);
    flex-shrink: 0;
    transition: background 0.3s;
  }
  .track.on {
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
  .track.on .knob {
    transform: translateX(16px);
    background: var(--bg);
  }
</style>
