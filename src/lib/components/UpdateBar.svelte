<script lang="ts">
  import { slide } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { app } from "$lib/app.svelte";

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const MB = 1024 ** 2;

  // First line of the release notes, if the release has any.
  const notes = $derived.by(() => {
    const line = (app.update?.body ?? "")
      .split("\n")
      .map((l) => l.replace(/^[#\-*\s]+/, "").trim())
      .find(Boolean);
    return line ? (line.length > 140 ? line.slice(0, 139) + "…" : line) : "";
  });

  const pct = $derived(app.dlTotal ? Math.min(100, Math.round((app.dlDone / app.dlTotal) * 100)) : 0);

  type Banner = { icon: string; tone: string; title: string; sub: string };

  const banner = $derived.by((): Banner | null => {
    if (app.updateHidden && app.updateStatus !== "restarting") return null;
    const v = app.update?.version ?? "";
    switch (app.updateStatus) {
      case "available":
        return {
          icon: "↑",
          tone: "sky",
          title: `Crumbtrail ${v} is available`,
          sub: notes || "A newer version of Crumbtrail is ready to download. Your presets and snapshots stay as they are.",
        };
      case "downloading":
        return {
          icon: "↓",
          tone: "sky",
          title: `Downloading ${v}${app.dlTotal ? ` · ${pct}%` : ""}`,
          sub: `${app.dlTotal ? `${(app.dlDone / MB).toFixed(1)} of ${(app.dlTotal / MB).toFixed(0)} MB` : `${(app.dlDone / MB).toFixed(1)} MB so far`}. You can keep using Crumbtrail. Nothing restarts until you say so.`,
        };
      case "ready":
        return {
          icon: "✓",
          tone: "accent",
          title: "Update ready to install",
          sub: "Crumbtrail closes and reopens in a few seconds. Your presets and snapshots stay as they are.",
        };
      case "restarting":
        return { icon: "↻", tone: "sky", title: "Restarting Crumbtrail…", sub: `Installing ${v}. This takes a few seconds.` };
      case "current":
        return {
          icon: "✓",
          tone: "soft",
          title: "You're up to date",
          sub: `Crumbtrail ${app.version} is the latest version. Checked just now.`,
        };
      case "failed":
        return {
          icon: "!",
          tone: "amber",
          title: "Couldn't check for updates",
          sub: "You might be offline. Crumbtrail works fine without updates, and nothing else is affected.",
        };
      default:
        return null;
    }
  });
</script>

{#if banner}
  <div class="wrap" transition:slide={{ duration: reduced ? 0 : 450, easing: cubicOut }}>
    <div class="bar">
      <span class="icon {banner.tone}" class:spin={app.updateStatus === "restarting"}>{banner.icon}</span>
      <div class="text">
        <div class="title">{banner.title}</div>
        <div class="sub">{banner.sub}</div>
        {#if app.updateStatus === "downloading"}
          <div class="track"><div class="fill" style:width="{app.dlTotal ? pct : 8}%"></div></div>
        {/if}
        {#if app.updateError && (app.updateStatus === "available" || app.updateStatus === "ready")}
          <div class="err">Update failed: {app.updateError}</div>
        {/if}
      </div>
      <div class="btns">
        {#if app.updateStatus === "available"}
          <button class="btn-secondary sm" onclick={() => app.dismissUpdate()}>Later</button>
          <button class="btn-primary sm" onclick={() => app.downloadUpdate()}>Download update</button>
        {:else if app.updateStatus === "downloading"}
          <!-- The updater can't abort a download, so this only hides the banner; the chip keeps showing progress. -->
          <button class="btn-secondary sm" onclick={() => app.dismissUpdate()}>Hide</button>
        {:else if app.updateStatus === "ready"}
          <button class="btn-secondary sm" onclick={() => app.dismissUpdate()}>Not now</button>
          <button
            class="btn-primary sm"
            disabled={app.busy}
            title={app.busy ? "Wait for the scan or clean to finish" : ""}
            onclick={() => app.installUpdate()}>{app.busy ? "Finish cleaning first" : "Install & restart"}</button
          >
        {:else if app.updateStatus === "current"}
          <button class="btn-secondary sm" onclick={() => app.dismissUpdate()}>Dismiss</button>
        {:else if app.updateStatus === "failed"}
          <button class="btn-secondary sm" onclick={() => app.dismissUpdate()}>Dismiss</button>
          <button class="btn-primary sm" onclick={() => app.checkForUpdates(true)}>Try again</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .wrap {
    flex-shrink: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 24px 10px;
    padding: 9px 10px 9px 12px;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 12px;
  }
  .icon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    font-size: 14px;
    font-weight: 700;
  }
  .icon.sky {
    background: var(--sky-soft);
    color: var(--sky);
  }
  .icon.accent {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .icon.soft {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .icon.amber {
    background: var(--amber-soft);
    color: var(--amber);
  }
  .icon.spin {
    animation: spin 1.2s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 13.5px;
    font-weight: 600;
  }
  .sub {
    font-size: 12px;
    color: var(--muted);
    margin-top: 1px;
    text-wrap: pretty;
  }
  .track {
    height: 4px;
    border-radius: 4px;
    background: var(--chip-bg);
    margin-top: 7px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 4px;
    background: var(--sky);
    transition: width 0.3s ease;
  }
  .err {
    font-size: 12px;
    color: var(--amber);
    margin-top: 3px;
    word-break: break-word;
  }
  .btns {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .sm {
    height: 32px;
    padding: 0 14px;
    font-size: 12.5px;
  }
</style>
