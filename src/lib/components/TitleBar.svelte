<script lang="ts">
  // Custom title bar (window decorations are off) so the chrome matches the app instead of Windows'.
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const win = getCurrentWindow();
  let maximized = $state(false);

  $effect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const sync = () =>
      win.isMaximized().then(
        (m) => (maximized = m),
        (e) => console.error("isMaximized failed:", e),
      );
    sync();
    win.onResized(sync).then(
      (u) => (disposed ? u() : (unlisten = u)),
      (e) => console.error("onResized failed:", e),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function act(fn: () => Promise<void>, what: string) {
    fn().catch((e) => console.error(`${what} failed:`, e));
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <button class="ctl" aria-label="Minimize" title="Minimize" onclick={() => act(() => win.minimize(), "minimize")}>
    <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="currentColor" /></svg>
  </button>
  <button
    class="ctl"
    aria-label={maximized ? "Restore" : "Maximize"}
    title={maximized ? "Restore" : "Maximize"}
    onclick={() => act(() => win.toggleMaximize(), "toggleMaximize")}
  >
    {#if maximized}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor">
        <rect x="0.5" y="2.5" width="7" height="7" rx="1" /><path d="M2.5 2.5v-1a1 1 0 0 1 1-1h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-1" />
      </svg>
    {:else}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor"><rect x="0.5" y="0.5" width="9" height="9" rx="1.5" /></svg>
    {/if}
  </button>
  <button class="ctl close" aria-label="Close" title="Close" onclick={() => act(() => win.close(), "close")}>
    <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="currentColor" stroke-width="1.1" /></svg>
  </button>
</div>

<style>
  .titlebar {
    flex-shrink: 0;
    height: 32px;
    display: flex;
    justify-content: flex-end;
    background: var(--bg);
    user-select: none;
  }
  .ctl {
    width: 46px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: default;
    transition:
      background 0.15s,
      color 0.15s;
  }
  .ctl:hover {
    background: var(--row-hover);
    color: var(--ink);
  }
  .ctl.close:hover {
    background: #e81123;
    color: #fff;
  }
  .ctl:focus-visible {
    outline-offset: -2px;
  }
</style>
