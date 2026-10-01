<script lang="ts">
  import { app } from "$lib/app.svelte";
  import Logo from "./Logo.svelte";

  const updLabel = $derived.by(() => {
    switch (app.updateStatus) {
      case "checking":
        return "Checking…";
      case "available":
        return `Update · ${app.updateVersion}`;
      case "downloading":
        return app.dlPct ? `Updating · ${app.dlPct}%` : "Updating…";
      case "ready":
        return "Restart to update";
      case "restarting":
        return "Restarting…";
      case "current":
        return "✓ Up to date";
      case "failed":
        return "Couldn't check";
      default:
        return app.version ? `v${app.version}` : "Updates";
    }
  });
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <Logo size={26} dots={2} />
    <span class="name display">Crumbtrail</span>
  </div>

  <div class="tabs" role="tablist">
    <div class="thumb" style:transform="translateX({app.tab === 'clean' ? 0 : 84}px)"></div>
    <button role="tab" aria-selected={app.tab === "clean"} class:on={app.tab === "clean"} onclick={() => app.setTab("clean")}>
      Clean
    </button>
    <button role="tab" aria-selected={app.tab === "space"} class:on={app.tab === "space"} onclick={() => app.setTab("space")}>
      Space
    </button>
  </div>

  <div class="actions">
    {#if app.admin}
      <span class="pill admin on" title="Running with admin rights">
        <span class="adot"></span>Admin
      </span>
    {:else}
      <button
        class="pill admin"
        title="System temp and Windows Update cleanup need admin rights"
        disabled={app.busy}
        onclick={() => app.relaunchAdmin()}
      >
        <span class="adot"></span>Restart as admin
      </button>
    {/if}
    <button
      class="pill ghost"
      class:ok={app.updateStatus === "current"}
      class:bad={app.updateStatus === "failed"}
      class:upd={app.updateStatus === "available"}
      aria-label="Version and updates: {updLabel}"
      title={app.updateStatus === "failed" ? app.updateError : "Check for updates"}
      onclick={() => app.checkForUpdates(true)}
    >
      {#if app.updateStatus === "available"}<span class="udot"></span>{/if}{updLabel}
    </button>
    <button class="round" title="How Crumbtrail works" onclick={() => app.openWelcome()}>?</button>
    <button class="round theme" aria-label="Toggle theme" title="Toggle theme" onclick={() => app.toggleTheme()}>
      {app.theme === "dark" ? "☀" : "☾"}
    </button>
  </div>
</header>

<style>
  header {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 24px 12px;
    gap: 16px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    font-weight: 700;
    font-size: 17px;
    letter-spacing: -0.01em;
  }

  .tabs {
    position: relative;
    display: flex;
    background: var(--surface);
    border: 1px solid var(--line-2);
    border-radius: 999px;
    padding: 4px;
  }
  .thumb {
    position: absolute;
    top: 4px;
    left: 4px;
    width: 84px;
    height: 32px;
    border-radius: 999px;
    background: var(--ink);
    transition: transform 0.45s var(--spring);
  }
  .tabs button {
    position: relative;
    width: 84px;
    height: 32px;
    border: none;
    background: transparent;
    border-radius: 999px;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    color: var(--muted);
    transition: color 0.3s;
  }
  .tabs button.on {
    color: var(--bg);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    align-items: center;
  }
  .pill {
    height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--line-2);
    background: transparent;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    transition: all 0.3s;
  }
  .admin {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--ink);
  }
  .admin:hover:not(:disabled):not(.on) {
    border-color: var(--accent);
  }
  .admin:disabled {
    opacity: 0.45;
  }
  .admin.on {
    background: var(--accent-soft);
    border-color: var(--accent-line);
    color: var(--accent);
    cursor: default;
  }
  .adot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted-4);
    transition: background 0.3s;
  }
  .admin.on .adot {
    background: var(--accent);
  }
  .ghost {
    color: var(--muted);
  }
  .ghost.ok {
    color: var(--accent);
  }
  .ghost.bad {
    color: var(--amber);
  }
  .ghost.upd {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--sky);
    border-color: var(--sky-line);
  }
  .udot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--sky);
  }
  .ghost:hover,
  .round:hover {
    color: var(--ink);
    border-color: var(--line-4);
  }
  .round {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 1px solid var(--line-2);
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.3s;
  }
  .round.theme {
    font-size: 14px;
    font-weight: 400;
  }
</style>
