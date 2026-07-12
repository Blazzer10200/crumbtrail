<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type Cat = {
    id: string;
    name: string;
    description: string;
    module: "core" | "gaming" | "dev";
    risk: "safe" | "care";
    needs_admin: boolean;
    available: boolean;
  };
  type ScanRes = { id: string; bytes: number; files: number };
  type CleanRes = { id: string; freed_bytes: number; deleted: number; skipped: number };

  let cats = $state<Cat[]>([]);
  let sizes = $state<Record<string, ScanRes>>({});
  let checked = $state<Record<string, boolean>>({});
  let scanning = $state(false);
  let cleaning = $state(false);
  let admin = $state(false);
  let confirmOpen = $state(false);
  let cleanSkipped = $state(0);
  let result = $state<{ total: number; skipped: number; logPath: string } | null>(null);

  const modules = [
    { key: "core", label: "System & apps" },
    { key: "gaming", label: "Gaming" },
    { key: "dev", label: "Developer" },
  ] as const;

  const totalSelected = $derived(
    cats.filter((c) => checked[c.id]).reduce((s, c) => s + (sizes[c.id]?.bytes ?? 0), 0)
  );
  const selectedIds = $derived(cats.filter((c) => checked[c.id]).map((c) => c.id));
  const careSelected = $derived(
    cats.filter((c) => checked[c.id] && c.risk === "care" && (sizes[c.id]?.bytes ?? 0) > 0)
  );
  const busy = $derived(scanning || cleaning);

  function fmt(b: number): string {
    if (b >= 1024 ** 3) return (b / 1024 ** 3).toFixed(2) + " GB";
    if (b >= 1024 ** 2) return (b / 1024 ** 2).toFixed(1) + " MB";
    if (b >= 1024) return (b / 1024).toFixed(0) + " KB";
    return b + " B";
  }

  function selectable(c: Cat): boolean {
    return c.available && !(c.needs_admin && !admin);
  }

  async function startScan() {
    result = null;
    sizes = {};
    scanning = true;
    await invoke("scan");
  }

  function requestClean() {
    if (careSelected.length > 0) {
      confirmOpen = true;
    } else {
      doClean();
    }
  }

  async function doClean() {
    confirmOpen = false;
    cleaning = true;
    cleanSkipped = 0;
    await invoke("clean", { ids: selectedIds });
  }

  async function relaunchAdmin() {
    await invoke("relaunch_admin");
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    (async () => {
      admin = await invoke<boolean>("elevated");
      cats = await invoke<Cat[]>("get_categories");
      const defaults: Record<string, boolean> = {};
      for (const c of cats) {
        defaults[c.id] = c.risk === "safe" && selectable(c);
      }
      checked = defaults;

      unlisteners.push(
        await listen<ScanRes>("scan:result", (e) => {
          sizes = { ...sizes, [e.payload.id]: e.payload };
        }),
        await listen("scan:done", () => {
          scanning = false;
        }),
        await listen<CleanRes>("clean:result", (e) => {
          cleanSkipped += e.payload.skipped;
        }),
        await listen<{ total_bytes: number; log_path: string }>("clean:done", (e) => {
          cleaning = false;
          result = {
            total: e.payload.total_bytes,
            skipped: cleanSkipped,
            logPath: e.payload.log_path,
          };
          startScan();
        })
      );

      await startScan();
    })();
    return () => unlisteners.forEach((u) => u());
  });
</script>

<div class="app">
  <header>
    <div>
      <h1>Sweep</h1>
      <p class="tagline">Scan first. Nothing is deleted until you say so.</p>
    </div>
    <div class="header-actions">
      {#if admin}
        <span class="admin-chip on">Admin</span>
      {:else}
        <button class="ghost" onclick={relaunchAdmin} title="System temp and Windows Update cleanup need admin rights">
          Restart as admin
        </button>
      {/if}
      <button class="ghost" onclick={startScan} disabled={busy}>
        {scanning ? "Scanning…" : "Rescan"}
      </button>
    </div>
  </header>

  {#if result}
    <div class="banner">
      <strong>{fmt(result.total)} freed.</strong>
      {#if result.skipped > 0}
        {result.skipped} files skipped (in use by other apps — normal).
      {/if}
      <span class="log-path">Log: {result.logPath}</span>
    </div>
  {/if}

  <main>
    {#each modules as mod (mod.key)}
      {@const group = cats.filter((c) => c.module === mod.key)}
      {#if group.length > 0}
        <section>
          <h2>{mod.label}</h2>
          {#each group as c (c.id)}
            {@const size = sizes[c.id]}
            <label class="row" class:disabled={!selectable(c)}>
              <input
                type="checkbox"
                checked={checked[c.id] ?? false}
                disabled={!selectable(c) || busy}
                onchange={(e) => (checked = { ...checked, [c.id]: e.currentTarget.checked })}
              />
              <div class="row-text">
                <div class="row-title">
                  {c.name}
                  {#if c.risk === "care"}<span class="chip care">confirm</span>{/if}
                  {#if c.needs_admin}<span class="chip admin" class:off={!admin}>admin</span>{/if}
                </div>
                <div class="row-desc">
                  {#if !c.available}
                    Not found on this PC
                  {:else}
                    {c.description}
                  {/if}
                </div>
              </div>
              <div class="row-size">
                {#if !c.available}
                  <span class="dim">—</span>
                {:else if c.needs_admin && !admin}
                  <span class="dim">needs admin</span>
                {:else if size}
                  <span class="bytes">{fmt(size.bytes)}</span>
                  <span class="files">{size.files.toLocaleString()} files</span>
                {:else if scanning}
                  <span class="dim pulse">scanning…</span>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </div>
            </label>
          {/each}
        </section>
      {/if}
    {/each}
  </main>

  <footer>
    <div class="total">
      <span class="total-label">Selected</span>
      <span class="total-value">{fmt(totalSelected)}</span>
    </div>
    <button
      class="clean"
      onclick={requestClean}
      disabled={busy || totalSelected === 0}
    >
      {cleaning ? "Cleaning…" : `Clean ${selectedIds.length} ${selectedIds.length === 1 ? "category" : "categories"}`}
    </button>
  </footer>

  {#if confirmOpen}
    <div class="overlay" role="dialog" aria-modal="true">
      <div class="modal">
        <h3>Double-checking these</h3>
        <p>These selections can't be undone once cleaned:</p>
        <ul>
          {#each careSelected as c (c.id)}
            <li>
              <strong>{c.name}</strong> — {fmt(sizes[c.id]?.bytes ?? 0)}
              <div class="modal-desc">{c.description}</div>
            </li>
          {/each}
        </ul>
        <div class="modal-actions">
          <button class="ghost" onclick={() => (confirmOpen = false)}>Cancel</button>
          <button class="clean" onclick={doClean}>Clean anyway</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
  }
  :global(::-webkit-scrollbar) {
    width: 10px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--line);
    border-radius: 5px;
  }
  :global(::-webkit-scrollbar-thumb:hover) {
    background: var(--muted);
  }
  :global(body) {
    background: var(--bg);
    color: var(--ink);
    font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
    font-size: 14px;
    --bg: #f6f7f8;
    --surface: #ffffff;
    --ink: #1c2733;
    --muted: #5b6b7a;
    --accent: #178a58;
    --accent-ink: #ffffff;
    --accent-soft: #e2f2ea;
    --warn: #a86c10;
    --warn-soft: #f8efdd;
    --line: #dfe4e9;
  }
  @media (prefers-color-scheme: dark) {
    :global(body) {
      --bg: #12181f;
      --surface: #1a222b;
      --ink: #e4ebf1;
      --muted: #92a2b0;
      --accent: #2fb87e;
      --accent-ink: #0b1510;
      --accent-soft: #16301f;
      --warn: #dfa23f;
      --warn-soft: #2b2211;
      --line: #29343f;
    }
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 24px 12px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-family: "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif;
  }
  .tagline {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  main {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px 16px;
  }
  section {
    margin-top: 14px;
  }
  h2 {
    font-size: 11.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 0 0 6px 2px;
    font-weight: 600;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 14px;
    margin-bottom: 6px;
    cursor: pointer;
  }
  .row.disabled {
    opacity: 0.55;
    cursor: default;
  }
  .row input {
    accent-color: var(--accent);
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
  .row-text {
    flex: 1;
    min-width: 0;
  }
  .row-title {
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .row-desc {
    color: var(--muted);
    font-size: 12px;
    margin-top: 1px;
  }
  .row-size {
    text-align: right;
    flex-shrink: 0;
    min-width: 110px;
  }
  .bytes {
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    display: block;
  }
  .files {
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .dim {
    color: var(--muted);
  }
  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }

  .chip {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 7px;
    border-radius: 99px;
  }
  .chip.care {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .chip.admin {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .chip.admin.off {
    background: transparent;
    border: 1px solid var(--line);
    color: var(--muted);
  }
  .admin-chip.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 99px;
  }

  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 24px;
    border-top: 1px solid var(--line);
    background: var(--surface);
  }
  .total-label {
    color: var(--muted);
    font-size: 12px;
    display: block;
  }
  .total-value {
    font-size: 20px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  button {
    font-family: inherit;
    font-size: 13px;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid var(--line);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .ghost {
    background: transparent;
    color: var(--ink);
    padding: 6px 12px;
  }
  .ghost:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .clean {
    background: var(--accent);
    color: var(--accent-ink);
    border: none;
    font-weight: 650;
    padding: 10px 22px;
  }
  .clean:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .banner {
    margin: 0 24px;
    background: var(--accent-soft);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 13px;
  }
  .log-path {
    display: block;
    color: var(--muted);
    font-size: 11px;
    margin-top: 3px;
    word-break: break-all;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }
  .modal {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 20px 22px;
    max-width: 440px;
    width: calc(100% - 48px);
  }
  .modal h3 {
    margin: 0 0 6px;
  }
  .modal p {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 13px;
  }
  .modal ul {
    margin: 0 0 16px;
    padding-left: 18px;
  }
  .modal li {
    margin-bottom: 8px;
  }
  .modal-desc {
    color: var(--muted);
    font-size: 12px;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
