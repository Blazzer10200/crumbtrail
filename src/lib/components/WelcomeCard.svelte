<script lang="ts">
  import { fade } from "svelte/transition";
  import { app } from "$lib/app.svelte";
  import { pop } from "$lib/motion";
  import Logo from "./Logo.svelte";

  const STEPS = [
    [
      "Only known junk",
      "Crumbtrail only looks in temp, cache and leftover folders. Your documents, photos and games are never scanned.",
    ],
    ["You decide", "Nothing is deleted until you check items and press Clean. You'll see a summary first."],
    [
      "Nothing forced",
      "Files in use are skipped, and every deletion is logged so you can see exactly what happened.",
    ],
  ];
</script>

<div class="scrim" transition:fade={{ duration: 350 }}>
  <div class="card" role="dialog" aria-modal="true" aria-labelledby="welcome-title" transition:pop>
    <Logo size={36} dots={3} />
    <div id="welcome-title" class="title display">Welcome to Crumbtrail</div>
    <p class="sub">A quick, safe clean-up for your PC. Here's how it works:</p>
    <div class="steps">
      {#each STEPS as [head, body], i (head)}
        <div class="step">
          <span class="num">{i + 1}</span>
          <div>
            <div class="head">{head}</div>
            <div class="body">{body}</div>
          </div>
        </div>
      {/each}
    </div>
    <button class="btn-primary got" onclick={() => app.closeWelcome()}>Got it</button>
  </div>
</div>

<style>
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 30;
    background: var(--scrim-welcome);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
  }
  .card {
    width: min(420px, calc(100% - 48px));
    background: var(--surface);
    border: 1px solid var(--line-3);
    border-radius: 18px;
    padding: 24px;
    box-shadow: var(--shadow-welcome);
  }
  .title {
    font-weight: 700;
    font-size: 21px;
    margin-top: 14px;
  }
  .sub {
    margin: 4px 0 18px;
    color: var(--muted);
    font-size: 13px;
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .step {
    display: flex;
    gap: 12px;
  }
  .num {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 11.5px;
    font-weight: 700;
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }
  .head {
    font-weight: 600;
    font-size: 13.5px;
  }
  .body {
    font-size: 12.5px;
    color: var(--muted);
    margin-top: 2px;
    line-height: 1.45;
  }
  .got {
    width: 100%;
    height: 42px;
    margin-top: 22px;
  }
  .got:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .got:active:not(:disabled) {
    transform: scale(0.97);
  }
</style>
