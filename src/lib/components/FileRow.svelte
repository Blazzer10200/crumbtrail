<script lang="ts">
  import { fmtS } from "$lib/format";

  let {
    rank,
    name,
    path = "",
    bytes,
    max,
    color,
    sys = false,
    kid = false,
    delay = 0,
    onrow,
    onopen,
  }: {
    rank: number;
    name: string;
    path?: string;
    bytes: number;
    max: number;
    color: string;
    sys?: boolean;
    kid?: boolean;
    delay?: number;
    onrow: () => void;
    onopen: () => void;
  } = $props();
</script>

<div
  class="row"
  role="button"
  tabindex="0"
  title={kid ? "Look inside" : "Open in Explorer"}
  onclick={onrow}
  onkeydown={(e) => {
    if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      onrow();
    }
  }}
>
  <span class="rank mono">{rank}</span>
  <div class="rmain">
    <div class="rname">
      <span class="nm">{name}</span>
      {#if sys}<span class="sys" title="Managed by Windows. Not a file you can delete.">system</span>{/if}
      {#if kid}<span class="kid">›</span>{/if}
    </div>
    {#if path}
      <div class="rpath"><bdi dir="ltr">{path}</bdi></div>
    {/if}
    <div class="rbar">
      <div
        class="fill"
        style:background={color}
        style:width="{Math.max(2, (bytes / (max || 1)) * 100).toFixed(1)}%"
        style:animation-delay="{delay}ms"
      ></div>
    </div>
  </div>
  <span class="rsize mono">{fmtS(bytes)}</span>
  <button
    class="open"
    title="Open in Explorer"
    onclick={(e) => {
      e.stopPropagation();
      onopen();
    }}>Open</button
  >
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 9px;
    cursor: pointer;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .rank {
    width: 22px;
    font-size: 11px;
    color: var(--faint);
    text-align: right;
    flex-shrink: 0;
  }
  .rmain {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .rname {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .nm {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sys {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--chip-bg);
    color: var(--muted);
    flex-shrink: 0;
  }
  .kid {
    font-size: 12px;
    color: var(--faint);
    flex-shrink: 0;
  }
  .rpath {
    font-size: 11px;
    color: var(--muted-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .rbar {
    height: 5px;
    border-radius: 5px;
    background: var(--chip-bg);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 5px;
    animation: grow-width 0.8s var(--ease-out) both;
  }
  .rsize {
    font-weight: 600;
    font-size: 13px;
    min-width: 78px;
    text-align: right;
    flex-shrink: 0;
  }
  .open {
    height: 26px;
    padding: 0 10px;
    border-radius: 99px;
    border: 1px solid var(--line-3);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }
  .open:hover {
    color: var(--ink);
    border-color: var(--line-4);
  }
</style>
