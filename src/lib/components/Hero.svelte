<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { app } from "$lib/app.svelte";
  import { MODS } from "$lib/copy";
  import { fmt, plural } from "$lib/format";

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const shown = new Tween(app.heroTarget, { duration: reduced ? 0 : 650, easing: cubicOut });
  $effect(() => {
    shown.target = app.heroTarget;
  });

  const GB = 1024 ** 3;
  const bigNum = $derived(shown.current >= GB ? (shown.current / GB).toFixed(2) : (shown.current / 1024 ** 2).toFixed(0));
  const bigUnit = $derived(shown.current >= GB ? "GB" : "MB");

  const status = $derived.by(() => {
    if (app.error) return { label: "Something went wrong", color: "var(--amber)", anim: "", sub: app.error };
    if (app.scanning)
      return {
        label: `Scanning · ${app.scanStep} of ${app.selectableCats.length}`,
        color: "var(--amber)",
        anim: "breathe",
        sub: "Measuring known-junk folders. Nothing is touched yet.",
      };
    if (app.cleaning)
      return {
        label: `Cleaning · ${app.cleanStep} of ${app.cleanTotal}`,
        color: "var(--accent)",
        anim: "breathe fast",
        sub: "Anything locked by a running app is skipped, never forced.",
      };
    if (app.done)
      return {
        label: "Done · space freed",
        color: "var(--accent)",
        anim: "",
        sub: "See the summary below for what was removed.",
      };
    return {
      label: "Ready · choose what to clean",
      color: "var(--accent)",
      anim: "",
      sub: `selected across ${plural(app.selected.length, "category", "categories")} · ${app.selectedFiles.toLocaleString()} files`,
    };
  });

  const cur = $derived(app.scanning ? 0 : app.cleaning ? 2 : app.done ? 3 : 1);
  const steps = ["Scan", "Choose", "Clean"];

  const segs = $derived.by(() => {
    const all = app.selectableCats.reduce((a, c) => a + (app.sizes[c.id]?.bytes ?? 0), 0) || 1;
    const res = app.done ? app.result : null;
    return MODS.map((m) => {
      const b = res
        ? (res.byMod[m.key] ?? 0)
        : app.selected
            .filter((c) => c.module === m.key && !app.swept[c.id])
            .reduce((a, c) => a + app.sizes[c.id].bytes, 0);
      return { ...m, b, w: ((b / (res ? res.freed || 1 : all)) * 100).toFixed(2) + "%" };
    });
  });

  // Confetti: fires once per finished clean (not on remount when switching tabs).
  let burstEl: HTMLDivElement;
  let seenTick = app.celebrateTick;
  $effect(() => {
    const tick = app.celebrateTick;
    if (tick === seenTick) return;
    seenTick = tick;
    if (!reduced) burst();
  });

  function burst() {
    const cols = ["#5ee6a0", "#b79cff", "#6cc8f5", "#f4c160", "#ececee"];
    for (let i = 0; i < 46; i++) {
      const p = document.createElement("span");
      const sz = 5 + Math.random() * 7;
      const round = Math.random() > 0.5;
      Object.assign(p.style, {
        position: "absolute",
        left: "90px",
        top: "60px",
        width: sz + "px",
        height: (round ? sz : sz * 0.45) + "px",
        borderRadius: round ? "50%" : "2px",
        background: cols[i % cols.length],
      });
      burstEl.appendChild(p);
      const a = Math.random() * Math.PI * 2;
      const d = 90 + Math.random() * 260;
      const dx = Math.cos(a) * d * 1.6;
      const dy = Math.sin(a) * d * 0.6 - 40;
      p.animate(
        [
          { transform: "translate(0,0) rotate(0) scale(.4)", opacity: 1 },
          { transform: `translate(${dx * 0.8}px,${dy}px) rotate(${Math.random() * 360}deg) scale(1)`, opacity: 1, offset: 0.55 },
          { transform: `translate(${dx}px,${dy + 70}px) rotate(${Math.random() * 720}deg) scale(.8)`, opacity: 0 },
        ],
        { duration: 1100 + Math.random() * 700, easing: "cubic-bezier(.2,.8,.3,1)" },
      ).onfinish = () => p.remove();
    }
  }
</script>

<section class="hero">
  <div class="burst" bind:this={burstEl}></div>
  <div class="glow" style:opacity={app.glow}></div>

  <div class="top">
    <div class="left">
      <div class="status">
        <span class="dot {status.anim}" style:background={status.color}></span>{status.label}
      </div>
      <div class="big">
        <span class="num display" class:done={app.done}>{bigNum}</span>
        <span class="unit display">{bigUnit}</span>
      </div>
      <div class="sub">{status.sub}</div>
    </div>

    <div class="steps">
      {#each steps as label, i (label)}
        {@const done = i < cur}
        {@const active = i === cur}
        <div class="step">
          {#if i > 0}<span class="line" class:lit={i <= cur}></span>{/if}
          <span class="mark" class:done class:active>{done ? "✓" : i + 1}</span>
          <span class="slabel" class:lit={done || active}>{label}</span>
        </div>
      {/each}
    </div>
  </div>

  <div class="segbar">
    {#each segs as s (s.key)}
      <div class="seg" style:width={s.w} style:background={s.color}></div>
    {/each}
  </div>
  <div class="legend">
    {#each segs as s (s.key)}
      <div class="lg">
        <span class="sw" style:background={s.color}></span>{s.label}
        <span class="mono val">{fmt(s.b)}</span>
      </div>
    {/each}
  </div>
</section>

<style>
  .hero {
    flex-shrink: 0;
    margin: 4px 24px 0;
    position: relative;
    background: var(--surface);
    border: 1px solid var(--line-hero);
    border-radius: 16px;
    padding: 16px 18px 14px;
    overflow: hidden;
  }
  .burst {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 3;
  }
  .glow {
    position: absolute;
    inset: 0;
    background: var(--accent);
    transition: opacity 0.8s ease;
    pointer-events: none;
  }
  .top {
    position: relative;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    flex-wrap: wrap;
  }
  .left {
    min-width: 0;
    flex: 1;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
  }
  .big {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: 4px;
  }
  .num {
    font-weight: 800;
    font-size: 44px;
    line-height: 1;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    transition: color 0.5s;
  }
  .num.done {
    color: var(--accent);
  }
  .unit {
    font-weight: 600;
    font-size: 18px;
    color: var(--muted);
  }
  .sub {
    margin-top: 10px;
    font-size: 13px;
    color: var(--muted);
    text-wrap: pretty;
  }

  .steps {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .step {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .line {
    width: 22px;
    height: 2px;
    border-radius: 2px;
    background: var(--line-3);
    transition: background 0.4s;
  }
  .line.lit {
    background: var(--accent);
  }
  .mark {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 11.5px;
    font-weight: 700;
    background: transparent;
    color: var(--muted-4);
    border: 1.5px solid var(--line-4);
    transition: all 0.35s var(--spring);
  }
  .mark.active {
    color: var(--accent);
    border-color: var(--accent);
    transform: scale(1.08);
  }
  .mark.done {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  .slabel {
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    color: var(--muted-4);
    transition: color 0.3s;
  }
  .slabel.lit {
    color: var(--ink);
  }

  .segbar {
    position: relative;
    display: flex;
    gap: 3px;
    height: 6px;
    margin-top: 14px;
    border-radius: 99px;
    overflow: hidden;
    background: var(--line);
  }
  .seg {
    height: 100%;
    border-radius: 99px;
    transition: width 0.7s var(--ease-out);
  }
  .legend {
    position: relative;
    display: flex;
    gap: 18px;
    margin-top: 10px;
    flex-wrap: wrap;
  }
  .lg {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--muted);
  }
  .sw {
    width: 8px;
    height: 8px;
    border-radius: 3px;
  }
  .val {
    color: var(--ink);
  }
</style>
