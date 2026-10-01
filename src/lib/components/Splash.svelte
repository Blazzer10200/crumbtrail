<script lang="ts">
  import { fade } from "svelte/transition";
  import Logo from "./Logo.svelte";

  let { title, sub = "" }: { title: string; sub?: string } = $props();
</script>

<div class="splash" role="status" aria-live="polite" out:fade={{ duration: 260 }}>
  <div class="mark"><Logo size={72} dots={3} /></div>
  <div class="t">{title}</div>
  {#if sub}<div class="s">{sub}</div>{/if}
  <div class="bar" aria-hidden="true"><i></i></div>
</div>

<style>
  .splash {
    position: absolute;
    inset: 32px 0 0 0; /* leave the title bar usable */
    z-index: 40;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: var(--bg);
  }
  .mark {
    margin-bottom: 14px;
    animation: breathe 2.4s ease-in-out infinite;
  }
  .t {
    font-size: 15px;
    font-weight: 600;
    color: var(--ink);
  }
  .s {
    font-size: 12.5px;
    color: var(--muted);
  }
  .bar {
    width: 120px;
    height: 3px;
    margin-top: 18px;
    border-radius: 99px;
    background: var(--line-2);
    overflow: hidden;
  }
  .bar i {
    display: block;
    width: 40%;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    animation: slide 1.3s var(--ease-out) infinite;
  }
  @keyframes breathe {
    50% {
      transform: scale(0.94);
      opacity: 0.85;
    }
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(250%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mark,
    .bar i {
      animation: none;
    }
  }
</style>
