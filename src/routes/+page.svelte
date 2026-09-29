<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "$lib/app.svelte";
  import CleanTab from "$lib/components/CleanTab.svelte";
  import Header from "$lib/components/Header.svelte";
  import ReviewSheet from "$lib/components/ReviewSheet.svelte";
  import SpaceTab from "$lib/components/SpaceTab.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import UpdateBar from "$lib/components/UpdateBar.svelte";
  import WelcomeCard from "$lib/components/WelcomeCard.svelte";

  onMount(() => app.init());

  function onKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (app.welcomeOpen) app.closeWelcome();
    else if (app.sheetOpen) app.closeSheet();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="shell">
  <TitleBar />
  <Header />
  <UpdateBar />
  {#key app.tab}
    <div class="pane">
      {#if app.tab === "clean"}
        <CleanTab />
      {:else}
        <SpaceTab />
      {/if}
    </div>
  {/key}
  {#if app.sheetOpen}<ReviewSheet />{/if}
  {#if app.welcomeOpen}<WelcomeCard />{/if}
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    min-height: 560px;
    position: relative;
    overflow: hidden;
  }
  .pane {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    animation:
      fade-in 0.35s ease,
      rise-10 0.5s var(--ease-out);
  }
</style>
