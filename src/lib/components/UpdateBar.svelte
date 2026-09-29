<script lang="ts">
  // Not in the design handoff — styled to match the result card so it reads as part of the system.
  import { app } from "$lib/app.svelte";

  const installing = $derived(app.updateStatus === "installing");
</script>

{#if app.update}
  <div class="bar">
    <span class="dot" class:breathe={installing} style:background="var(--accent)"></span>
    <div class="text">
      {#if installing}
        Downloading and installing update…
      {:else}
        A new version is ready — <strong class="mono">v{app.update.version}</strong>
      {/if}
      {#if app.updateError && !installing}
        <div class="err">Update failed: {app.updateError}</div>
      {/if}
    </div>
    <button class="btn-primary sm" disabled={installing} onclick={() => app.installUpdate()}>
      {installing ? "Installing…" : "Install & restart"}
    </button>
  </div>
{/if}

<style>
  .bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 24px 10px;
    padding: 8px 8px 8px 14px;
    background: var(--card);
    border: 1px solid var(--accent-line);
    border-radius: 12px;
    font-size: 13px;
    animation:
      fade-in 0.35s ease,
      drop-12 0.5s var(--spring);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  strong {
    font-weight: 600;
    color: var(--accent);
  }
  .err {
    font-size: 12px;
    color: var(--amber);
    margin-top: 2px;
    word-break: break-word;
  }
  .btn-primary.sm {
    height: 32px;
    padding: 0 14px;
    font-size: 12.5px;
  }
</style>
