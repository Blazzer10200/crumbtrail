<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
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
  type Drive = { letter: string; total: number; free: number };
  type Folder = { path: string; name: string; bytes: number };

  let tab = $state<"clean" | "space">("clean");
  let admin = $state(false);
  let theme = $state<"light" | "dark">("light");

  function applyTheme(t: "light" | "dark") {
    theme = t;
    document.documentElement.dataset.theme = t;
    try {
      localStorage.setItem("crumbtrail-theme", t);
    } catch {}
  }

  function toggleTheme() {
    applyTheme(theme === "dark" ? "light" : "dark");
  }

  // ---- Auto-update ----
  let update = $state<Update | null>(null);
  let updateStatus = $state<
    "idle" | "checking" | "available" | "current" | "installing" | "error"
  >("idle");
  let updateErr = $state("");

  async function checkForUpdates(manual = false) {
    if (updateStatus === "checking" || updateStatus === "installing") return;
    updateStatus = "checking";
    try {
      const u = await check();
      if (u) {
        update = u;
        updateStatus = "available";
      } else {
        updateStatus = manual ? "current" : "idle";
      }
    } catch (e) {
      updateErr = String(e);
      // Stay quiet on the silent launch check; only surface errors on a manual click.
      updateStatus = manual ? "error" : "idle";
    }
    if (updateStatus === "current" || updateStatus === "error") {
      setTimeout(() => {
        if (updateStatus === "current" || updateStatus === "error") updateStatus = "idle";
      }, 4000);
    }
  }

  async function installUpdate() {
    if (!update) return;
    updateStatus = "installing";
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      updateErr = String(e);
      updateStatus = "error";
    }
  }

  // ---- Clean tab ----
  let cats = $state<Cat[]>([]);
  let sizes = $state<Record<string, ScanRes>>({});
  let checked = $state<Record<string, boolean>>({});
  let scanning = $state(false);
  let cleaning = $state(false);
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

  // ---- Space tab ----
  let drives = $state<Drive[]>([]);
  let spaceScanning = $state(false);
  let spaceProgress = $state<{ files: number; bytes: number } | null>(null);
  let spaceResult = $state<{
    root: string;
    files: number;
    bytes: number;
    hotspots: Folder[];
    top: Folder[];
    biggest: Folder[];
  } | null>(null);
  let crumbs = $state<{ path: string; name: string }[]>([]);
  let browseEntries = $state<Folder[]>([]);

  function fmt(b: number): string {
    if (b >= 1024 ** 3) return (b / 1024 ** 3).toFixed(2) + " GB";
    if (b >= 1024 ** 2) return (b / 1024 ** 2).toFixed(1) + " MB";
    if (b >= 1024) return (b / 1024).toFixed(0) + " KB";
    return b + " B";
  }

  // Windows-managed files that show up huge but aren't user-deletable — tag them
  // so nobody wonders why "deleting" them does nothing.
  const SYSTEM_FILES = new Set([
    "pagefile.sys",
    "hiberfil.sys",
    "swapfile.sys",
    "dumpstack.log.tmp",
  ]);
  function isSystemFile(p: string): boolean {
    return SYSTEM_FILES.has(p.slice(p.lastIndexOf("\\") + 1).toLowerCase());
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
    if (careSelected.length > 0) confirmOpen = true;
    else doClean();
  }

  async function doClean() {
    confirmOpen = false;
    cleaning = true;
    cleanSkipped = 0;
    await invoke("clean", { ids: selectedIds });
  }

  async function scanRoot(root: string) {
    spaceResult = null;
    crumbs = [];
    browseEntries = [];
    spaceProgress = null;
    spaceScanning = true;
    await invoke("space_scan", { root });
  }

  function scanDrive(letter: string) {
    return scanRoot(letter + "\\");
  }

  async function scanFolder() {
    const picked = await openDialog({ directory: true, title: "Scan a folder" });
    if (typeof picked === "string") scanRoot(picked);
  }

  function setModule(key: string, on: boolean) {
    const next = { ...checked };
    for (const c of cats) if (c.module === key && selectable(c)) next[c.id] = on;
    checked = next;
  }

  async function drillInto(f: Folder) {
    const entries = await invoke<Folder[]>("space_children", { dir: f.path });
    if (entries.length === 0) return;
    crumbs = [...crumbs, { path: f.path, name: f.name }];
    browseEntries = entries;
  }

  async function jumpTo(index: number) {
    if (index < 0) {
      crumbs = [];
      browseEntries = spaceResult ? spaceResult.top : [];
      return;
    }
    const target = crumbs[index];
    crumbs = crumbs.slice(0, index + 1);
    browseEntries = await invoke<Folder[]>("space_children", { dir: target.path });
  }

  onMount(() => {
    const stored = (() => {
      try {
        return localStorage.getItem("crumbtrail-theme");
      } catch {
        return null;
      }
    })();
    if (stored === "light" || stored === "dark") {
      applyTheme(stored);
    } else {
      applyTheme(matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
    }

    // Quiet check for a newer release on launch; errors stay silent.
    checkForUpdates(false);

    const unlisteners: Array<() => void> = [];
    (async () => {
      admin = await invoke<boolean>("elevated");
      cats = await invoke<Cat[]>("get_categories");
      drives = await invoke<Drive[]>("drives");
      const defaults: Record<string, boolean> = {};
      for (const c of cats) defaults[c.id] = c.risk === "safe" && selectable(c);
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
        }),
        await listen<{ files: number; bytes: number }>("space:progress", (e) => {
          spaceProgress = e.payload;
        }),
        await listen<{
          root: string;
          files: number;
          bytes: number;
          hotspots: Folder[];
          top: Folder[];
          biggest: Folder[];
        }>("space:done", (e) => {
          spaceScanning = false;
          spaceResult = e.payload;
          browseEntries = e.payload.top;
          crumbs = [];
        })
      );

      await startScan();
    })();
    return () => unlisteners.forEach((u) => u());
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && confirmOpen) confirmOpen = false;
  }}
/>

<div class="app">
  {#if updateStatus === "available" || updateStatus === "installing"}
    <div class="update-bar">
      <span class="update-msg">
        {#if updateStatus === "installing"}
          Downloading and installing update…
        {:else}
          A new version is ready — <strong>v{update?.version}</strong>
        {/if}
      </span>
      <button class="update-btn" onclick={installUpdate} disabled={updateStatus === "installing"}>
        {updateStatus === "installing" ? "Installing…" : "Install & restart"}
      </button>
    </div>
  {/if}
  <header>
    <div class="brand">
      <svg class="logo" width="34" height="34" viewBox="0 0 32 32" aria-hidden="true">
        <rect width="32" height="32" rx="9" fill="var(--accent)" />
        <g stroke="var(--accent-ink)" stroke-width="2.4" stroke-linecap="round">
          <line x1="9" y1="11" x2="23" y2="11" />
          <line x1="9" y1="16" x2="20" y2="16" />
          <line x1="9" y1="21" x2="16" y2="21" />
        </g>
      </svg>
      <div>
        <h1>Crumbtrail</h1>
        <p class="tagline">Scan first. Nothing is deleted until you say so.</p>
      </div>
    </div>
    <div class="header-actions">
      {#if admin}
        <span class="admin-chip on">Admin</span>
      {:else}
        <button
          class="ghost"
          onclick={() => invoke("relaunch_admin")}
          title="System temp and Windows Update cleanup need admin rights"
        >
          Restart as admin
        </button>
      {/if}
      <button
        class="ghost"
        onclick={() => checkForUpdates(true)}
        disabled={updateStatus === "checking" || updateStatus === "installing"}
        title="Check for updates"
      >
        {#if updateStatus === "checking"}Checking…{:else if updateStatus === "current"}✓ Up to date{:else if updateStatus === "error"}Check failed{:else}Check for updates{/if}
      </button>
      <button
        class="ghost icon"
        onclick={toggleTheme}
        title={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
        aria-label="Toggle theme"
      >
        {theme === "dark" ? "☀" : "☾"}
      </button>
    </div>
  </header>

  <nav class="tabs">
    <button class:active={tab === "clean"} onclick={() => (tab = "clean")}>Clean</button>
    <button class:active={tab === "space"} onclick={() => (tab = "space")}>Space</button>
  </nav>

  {#if tab === "clean"}
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
          {@const anyOn = group.some((c) => checked[c.id] && selectable(c))}
          <section>
            <div class="section-head">
              <h2>{mod.label}</h2>
              <button
                class="link"
                disabled={busy || !group.some((c) => selectable(c))}
                onclick={() => setModule(mod.key, !anyOn)}
              >
                {anyOn ? "Deselect all" : "Select all"}
              </button>
            </div>
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
                    {#if c.needs_admin}<span class="chip admin" class:off={!admin}>admin</span
                      >{/if}
                  </div>
                  <div class="row-desc">
                    {#if !c.available}Not found on this PC{:else}{c.description}{/if}
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
      <div class="footer-actions">
        <button class="ghost" onclick={startScan} disabled={busy}>
          {scanning ? "Scanning…" : "Rescan"}
        </button>
        <button class="clean" onclick={requestClean} disabled={busy || totalSelected === 0}>
          {cleaning
            ? "Cleaning…"
            : `Clean ${selectedIds.length} ${selectedIds.length === 1 ? "category" : "categories"}`}
        </button>
      </div>
    </footer>
  {:else if tab === "space"}
    <main>
      <section>
        <div class="section-head">
          <h2>Drives</h2>
          <button class="link" disabled={spaceScanning} onclick={scanFolder}>
            Scan a folder…
          </button>
        </div>
        <div class="drive-grid">
          {#each drives as d (d.letter)}
            {@const used = d.total - d.free}
            {@const pct = Math.round((used / d.total) * 100)}
            <button
              class="drive"
              onclick={() => scanDrive(d.letter)}
              disabled={spaceScanning}
            >
              <div class="donut" class:hot={used / d.total > 0.9} style="--pct:{pct}">
                <div class="donut-hole">
                  <span class="donut-pct">{pct}%</span>
                  <span class="donut-sub">used</span>
                </div>
              </div>
              <div class="drive-body">
                <div class="drive-title">
                  <span class="drive-letter">{d.letter}</span>
                  <span class="drive-kind">Local disk</span>
                </div>
                <div class="drive-stats">
                  <span class="stat"><span class="stat-v">{fmt(used)}</span><span class="stat-l">used</span></span>
                  <span class="stat"><span class="stat-v">{fmt(d.free)}</span><span class="stat-l">free</span></span>
                  <span class="stat"><span class="stat-v">{fmt(d.total)}</span><span class="stat-l">total</span></span>
                </div>
              </div>
              <span class="drive-cta">
                {spaceScanning ? "Scanning…" : "Scan " + d.letter}
                <span class="cta-arrow">→</span>
              </span>
            </button>
          {/each}
        </div>
      </section>

      {#if spaceScanning}
        <section>
          <h2>Scanning</h2>
          <div class="scan-progress">
            <span class="pulse">Walking the drive…</span>
            {#if spaceProgress}
              <span class="dim">
                {spaceProgress.files.toLocaleString()} files · {fmt(spaceProgress.bytes)} so far
              </span>
            {/if}
          </div>
        </section>
      {/if}

      {#if spaceResult}
        <section>
          <h2>Space hotspots</h2>
          <p class="section-sub">
            Folders where space piles up — {spaceResult.root} · {spaceResult.files.toLocaleString()}
            files, {fmt(spaceResult.bytes)} seen
          </p>
          {#each spaceResult.hotspots as h, i (h.path)}
            {@const max = spaceResult.hotspots[0]?.bytes ?? 1}
            <div class="folder-row">
              <button class="folder-main" onclick={() => invoke("reveal", { path: h.path })} title="Open in Explorer">
                <div class="folder-name">{h.path}</div>
                <div class="size-bar">
                  <div class="size-fill" style="width: {Math.max(2, (h.bytes / max) * 100)}%"></div>
                </div>
              </button>
              <span class="folder-bytes">{fmt(h.bytes)}</span>
            </div>
          {/each}
          {#if spaceResult.hotspots.length === 0}
            <p class="dim">No single folder over 1 GB found.</p>
          {/if}
        </section>

        {#if spaceResult.biggest.length > 0}
          <section>
            <h2>Largest files</h2>
            <p class="section-sub">The biggest individual files on the drive.</p>
            {#each spaceResult.biggest.slice(0, 25) as f, i (f.path)}
              {@const max = spaceResult.biggest[0]?.bytes ?? 1}
              <div class="folder-row">
                <button
                  class="folder-main"
                  onclick={() => invoke("reveal", { path: f.path })}
                  title="Show in Explorer"
                >
                  <div class="file-line">
                    <span class="file-name">{f.name}</span>
                    {#if isSystemFile(f.path)}<span class="chip sys">system</span>{/if}
                  </div>
                  <div class="file-path">{f.path}</div>
                  <div class="size-bar">
                    <div class="size-fill" style="width: {Math.max(2, (f.bytes / max) * 100)}%"></div>
                  </div>
                </button>
                <span class="folder-bytes">{fmt(f.bytes)}</span>
              </div>
            {/each}
          </section>
        {/if}

        <section>
          <h2>Browse</h2>
          <p class="section-sub">Drill into any folder. View-only — nothing here deletes anything.</p>
          <div class="crumbs">
            <button class="crumb" onclick={() => jumpTo(-1)}>{spaceResult.root}</button>
            {#each crumbs as c, i (c.path)}
              <span class="dim">›</span>
              <button class="crumb" onclick={() => jumpTo(i)}>{c.name}</button>
            {/each}
          </div>
          {#each browseEntries as f (f.path)}
            {@const max = browseEntries[0]?.bytes ?? 1}
            <div class="folder-row">
              <button class="folder-main" onclick={() => drillInto(f)} title="Open folder">
                <div class="folder-name">{f.name}</div>
                <div class="size-bar">
                  <div class="size-fill" style="width: {Math.max(2, (f.bytes / max) * 100)}%"></div>
                </div>
              </button>
              <span class="folder-bytes">{fmt(f.bytes)}</span>
              <button class="ghost mini" onclick={() => invoke("reveal", { path: f.path })}>
                Open
              </button>
            </div>
          {/each}
        </section>
      {:else if !spaceScanning}
        <div class="empty">
          <svg width="52" height="52" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M3 7.5 A9 4.5 0 0 0 21 7.5 M3 7.5 A9 4.5 0 0 1 21 7.5 M3 7.5 v9 A9 4.5 0 0 0 21 16.5 v-9"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <div class="empty-title">Map where your space went</div>
          <p class="empty-sub">
            Pick a drive above (or scan a folder) to see the biggest folders and files. View-only —
            nothing here deletes anything.
          </p>
        </div>
      {/if}
    </main>
  {/if}

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
    /* Center content at a comfortable max width; bars/borders stay full-bleed. */
    --gutter: max(24px, calc((100% - 1060px) / 2));
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
  /* Explicit theme override wins over the OS preference in both directions. */
  :global(:root[data-theme="dark"] body) {
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
  :global(:root[data-theme="light"] body) {
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

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  /* Smooth the light/dark swap — fade colors instead of snapping. */
  :global(body),
  :global(.app *) {
    transition: background-color 0.28s ease, color 0.28s ease, border-color 0.28s ease;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(body),
    :global(.app *) {
      transition: none;
    }
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px var(--gutter) 8px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .logo {
    flex-shrink: 0;
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

  .tabs {
    display: flex;
    gap: 4px;
    padding: 6px var(--gutter) 10px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    background: transparent;
    border: none;
    color: var(--muted);
    font-weight: 600;
    font-size: 13px;
    padding: 7px 14px;
    border-radius: 6px;
  }
  .tabs button:hover {
    background: var(--accent-soft);
    color: var(--ink);
  }
  .tabs button.active {
    background: var(--accent-soft);
    color: var(--accent);
  }

  main {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--gutter) 16px;
    animation: fadeIn 0.2s ease;
  }
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    main {
      animation: none;
    }
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
  .section-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6px;
  }
  .section-head h2 {
    margin: 0 0 0 2px;
  }
  .link {
    background: transparent;
    border: none;
    color: var(--accent);
    font-weight: 600;
    font-size: 12px;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .link:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .section-sub {
    color: var(--muted);
    font-size: 12.5px;
    margin: -2px 0 9px 2px;
    max-width: 72ch;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 4px;
    padding: 52px 24px;
    color: var(--muted);
  }
  .empty svg {
    opacity: 0.55;
    margin-bottom: 4px;
  }
  .empty-title {
    font-size: 15px;
    font-weight: 650;
    color: var(--ink);
  }
  .empty-sub {
    font-size: 13px;
    max-width: 44ch;
    margin: 0;
    line-height: 1.5;
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
    flex-wrap: wrap;
    word-break: break-all;
  }
  .row-desc {
    color: var(--muted);
    font-size: 12px;
    margin-top: 1px;
    word-break: break-all;
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
  .chip.sys {
    background: var(--line);
    color: var(--muted);
    flex-shrink: 0;
  }
  .admin-chip.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 99px;
  }

  .drive-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .drive {
    display: flex;
    align-items: center;
    gap: 18px;
    width: 100%;
    background: var(--surface);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 16px 20px;
    text-align: left;
    transition: background-color 0.28s ease, color 0.28s ease, border-color 0.12s,
      box-shadow 0.12s;
  }
  .drive:hover:not(:disabled) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  /* Usage ring — conic fill to --pct%, hollowed by the surface-colored center. */
  .donut {
    --fillcolor: var(--accent);
    width: 88px;
    height: 88px;
    border-radius: 50%;
    background: conic-gradient(var(--fillcolor) calc(var(--pct) * 1%), var(--line) 0);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    transition: background 0.5s ease;
  }
  .donut.hot {
    --fillcolor: var(--warn);
  }
  .donut-hole {
    width: 66px;
    height: 66px;
    border-radius: 50%;
    background: var(--surface);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .donut-pct {
    font-size: 19px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }
  .donut-sub {
    font-size: 10px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }
  .drive-body {
    flex: 1;
    min-width: 0;
  }
  .drive-title {
    display: flex;
    align-items: baseline;
    gap: 9px;
    margin-bottom: 9px;
  }
  .drive-letter {
    font-size: 21px;
    font-weight: 700;
  }
  .drive-kind {
    font-size: 12px;
    color: var(--muted);
  }
  .drive-stats {
    display: flex;
    gap: 26px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stat-v {
    font-weight: 650;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }
  .stat-l {
    font-size: 11px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .drive-cta {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent);
    font-weight: 650;
    font-size: 13px;
    flex-shrink: 0;
  }
  .cta-arrow {
    transition: transform 0.15s ease;
  }
  .drive:hover:not(:disabled) .cta-arrow {
    transform: translateX(3px);
  }

  .scan-progress {
    display: flex;
    gap: 12px;
    align-items: baseline;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 12px 14px;
    font-variant-numeric: tabular-nums;
  }

  .folder-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 4px;
  }
  .folder-main {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 8px 12px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: background-color 0.28s ease, color 0.28s ease, border-color 0.12s;
  }
  .folder-main:hover {
    border-color: var(--accent);
  }
  .folder-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .file-line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .file-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-path {
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .size-bar {
    height: 5px;
    background: var(--line);
    border-radius: 3px;
    overflow: hidden;
  }
  .size-fill {
    height: 100%;
    background: var(--accent);
    opacity: 0.85;
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .folder-bytes {
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    min-width: 84px;
    text-align: right;
    flex-shrink: 0;
  }
  .crumbs {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .crumb {
    background: transparent;
    border: none;
    color: var(--accent);
    font-weight: 600;
    font-size: 13px;
    padding: 2px 4px;
    border-radius: 4px;
  }
  .crumb:hover {
    background: var(--accent-soft);
  }

  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px var(--gutter);
    border-top: 1px solid var(--line);
    background: var(--surface);
  }
  .footer-actions {
    display: flex;
    gap: 8px;
    align-items: center;
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
    color: inherit;
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
  .ghost.mini {
    padding: 4px 10px;
    font-size: 12px;
    flex-shrink: 0;
  }
  .ghost.icon {
    padding: 5px 9px;
    font-size: 15px;
    line-height: 1;
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
    margin: 12px var(--gutter) 0;
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

  .update-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 9px var(--gutter);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 13px;
    flex-wrap: wrap;
  }
  .update-msg strong {
    font-weight: 700;
  }
  .update-btn {
    background: var(--accent-ink);
    color: var(--accent);
    border: none;
    font-weight: 650;
    padding: 5px 14px;
    border-radius: 6px;
  }
  .update-btn:hover:not(:disabled) {
    filter: brightness(0.95);
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
    max-width: 480px;
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
    word-break: break-all;
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
