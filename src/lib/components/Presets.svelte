<script lang="ts">
  import { app } from "$lib/app.svelte";
  import { fmt, plural } from "$lib/format";
  import type { Preset } from "$lib/types";

  const FILE = "%APPDATA%\\Crumbtrail\\presets.json";

  let renaming = $state("");
  let renameText = $state("");
  let confirmDel = $state("");
  let newName = $state("");

  const dup = $derived(!!newName.trim() && app.presetNameTaken(newName));
  const canSave = $derived(!!newName.trim() && !dup && app.checkedIds.length > 0);
  const checkedBytes = $derived(
    app.selectableCats.filter((c) => app.checked[c.id]).reduce((a, c) => a + (app.sizes[c.id]?.bytes ?? 0), 0),
  );

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  // Panels hang off their chip, so near a window edge they clip (the Save panel did with no presets). Nudge back inside.
  function inView(node: HTMLElement) {
    const { left, right } = node.getBoundingClientRect();
    const pad = 12;
    const dx = left < pad ? pad - left : right > innerWidth - pad ? innerWidth - pad - right : 0;
    if (dx) node.style.translate = `${dx}px`;
  }

  function startRename(p: Preset) {
    app.popover = null;
    renaming = p.id;
    renameText = p.name;
  }

  function commitRename() {
    if (!renaming) return;
    app.renamePreset(renaming, renameText);
    renaming = "";
  }

  function openMenu(p: Preset) {
    confirmDel = "";
    app.togglePopover(`preset:${p.id}`);
  }

  function openSave() {
    newName = "";
    app.togglePopover("save");
  }

  function chipTitle(p: Preset): string {
    const n = app.presetUnavailable(p).length;
    return n ? `${plural(n, "item isn't", "items aren't")} available right now` : `Apply “${p.name}”`;
  }
</script>

<div class="row">
  <span class="label" title="Saved selections you can re-apply in one click">Presets</span>

  {#each app.presets as p (p.id)}
    {@const active = app.presetActive(p)}
    <div class="chip-wrap" data-pop>
      <div class="chip" class:active>
        {#if renaming === p.id}
          <input
            class="rename"
            maxlength="28"
            bind:value={renameText}
            use:focus
            onblur={commitRename}
            onkeydown={(e) => {
              if (e.key === "Enter") commitRename();
              else if (e.key === "Escape") {
                e.stopPropagation();
                renaming = "";
              }
            }}
          />
        {:else}
          <button class="apply" disabled={app.busy} title={chipTitle(p)} onclick={() => app.applyPreset(p)}>
            <span class="nm">{p.name}</span>
            <span class="mono amt">{app.scanning ? "" : fmt(app.presetBytes(p))}</span>
          </button>
        {/if}
        <button
          class="more"
          aria-label="Preset options for {p.name}"
          aria-expanded={app.popover === `preset:${p.id}`}
          onclick={() => openMenu(p)}>⋯</button
        >
      </div>

      {#if app.popover === `preset:${p.id}`}
        <div class="pop menu" role="menu" use:inView>
          {#if confirmDel === p.id}
            <div class="confirm">
              <div class="ct">Delete “{p.name}”?</div>
              <div class="cs">This only removes the preset. No files are touched.</div>
              <div class="cbtns">
                <button class="btn-secondary sm" onclick={() => (app.popover = null)}>Keep</button>
                <button class="btn-danger sm" onclick={() => app.deletePreset(p.id)}>Delete</button>
              </div>
            </div>
          {:else}
            <button class="mi" role="menuitem" onclick={() => startRename(p)}>Rename</button>
            <button class="mi" role="menuitem" disabled={!app.checkedIds.length} onclick={() => app.updatePreset(p.id)}>
              Update to current selection <span class="meta">{plural(app.checkedIds.length, "item", "items")}</span>
            </button>
            <button class="mi danger" role="menuitem" onclick={() => (confirmDel = p.id)}>Delete preset</button>
            <div class="foot">Saved on this PC in <span class="mono">{FILE}</span></div>
          {/if}
        </div>
      {/if}
    </div>
  {/each}

  <div class="chip-wrap" data-pop>
    <button class="add" aria-expanded={app.popover === "save"} onclick={openSave}>+ Save selection</button>
    {#if app.popover === "save"}
      <div class="pop save" role="dialog" aria-label="Save this selection" use:inView>
        <div class="st">Save this selection</div>
        <div class="si">
          {plural(app.checkedIds.length, "checked item", "checked items")} ({fmt(checkedBytes)} right now) will be saved. Give it a name
          you'll recognise.
        </div>
        <input
          class="name"
          maxlength="28"
          placeholder="e.g. Gamer quick clean"
          bind:value={newName}
          use:focus
          onkeydown={(e) => {
            if (e.key === "Enter" && canSave) app.savePreset(newName);
          }}
        />
        {#if dup}<div class="dup">You already have a preset with that name.</div>{/if}
        <div class="cbtns">
          <button class="btn-secondary sm" onclick={() => (app.popover = null)}>Cancel</button>
          <button class="btn-primary sm" disabled={!canSave} onclick={() => app.savePreset(newName)}>Save preset</button>
        </div>
        <div class="foot">
          Presets live on this PC in <span class="mono">{FILE}</span>. Rename or delete any preset from the ⋯ on its chip.
        </div>
      </div>
    {/if}
  </div>

  {#if app.presetNote}
    <span class="note" class:bad={app.presetNoteBad}>{app.presetNote}</span>
  {/if}
</div>

<style>
  .row {
    flex-shrink: 0;
    position: relative;
    z-index: 5;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 24px 0;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted-3);
    margin-right: 4px;
    cursor: help;
  }
  .chip-wrap {
    position: relative;
  }
  .chip {
    display: flex;
    align-items: center;
    height: 32px;
    border-radius: 999px;
    border: 1px solid var(--line-2);
    background: var(--card);
    transition:
      border-color 0.25s,
      background 0.25s;
  }
  .chip.active {
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .apply {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    padding: 0 4px 0 13px;
    border: none;
    background: transparent;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink);
    cursor: pointer;
    white-space: nowrap;
  }
  .apply:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .chip.active .nm {
    color: var(--accent);
  }
  .amt {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted);
  }
  .more {
    width: 26px;
    height: 26px;
    margin-right: 3px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
  }
  .more:hover {
    background: var(--chip-bg);
    color: var(--ink);
  }
  .rename {
    width: 150px;
    height: 24px;
    margin-left: 6px;
    padding: 0 8px;
    border-radius: 99px;
    border: 1px solid var(--accent-line);
    background: var(--bg);
    color: var(--ink);
    font: inherit;
    font-size: 12.5px;
  }
  .add {
    height: 32px;
    padding: 0 13px;
    border-radius: 999px;
    border: 1px dashed var(--line-4);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .add:hover {
    color: var(--ink);
    border-color: var(--muted-3);
  }
  .note {
    font-size: 12px;
    color: var(--muted);
    margin-left: 6px;
    animation: fade-in 0.3s ease;
  }
  .note.bad {
    color: var(--red);
  }

  .pop {
    position: absolute;
    top: calc(100% + 6px);
    background: var(--surface);
    border: 1px solid var(--line-3);
    border-radius: 14px;
    box-shadow: var(--shadow-welcome);
    animation: fade-in 0.18s ease;
  }
  .menu {
    left: 0;
    width: 270px;
    padding: 6px;
  }
  .save {
    right: 0;
    width: 320px;
    padding: 14px;
  }
  .mi {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--ink);
    font-size: 12.5px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .mi:hover:not(:disabled) {
    background: var(--row-hover);
  }
  .mi:disabled {
    color: var(--faint);
    cursor: default;
  }
  .mi.danger {
    color: var(--red);
  }
  .meta {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted-3);
  }
  .foot {
    margin-top: 6px;
    padding: 8px 10px 4px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--muted-3);
    line-height: 1.45;
  }
  .save .foot {
    padding: 10px 0 0;
    margin-top: 12px;
  }
  .confirm {
    padding: 8px 10px;
  }
  .ct,
  .st {
    font-size: 13.5px;
    font-weight: 600;
  }
  .cs,
  .si {
    font-size: 12px;
    color: var(--muted);
    margin-top: 3px;
    line-height: 1.45;
  }
  .cbtns {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 12px;
  }
  .name {
    width: 100%;
    height: 34px;
    margin-top: 12px;
    padding: 0 12px;
    border-radius: 9px;
    border: 1px solid var(--line-3);
    background: var(--bg);
    color: var(--ink);
    font: inherit;
    font-size: 13px;
  }
  .name:focus {
    outline: none;
    border-color: var(--accent);
  }
  .dup {
    font-size: 12px;
    color: var(--red);
    margin-top: 5px;
  }
  .sm {
    height: 30px;
    padding: 0 13px;
    font-size: 12.5px;
  }
  .btn-danger {
    border: none;
    border-radius: 999px;
    background: var(--red);
    color: var(--bg);
    font-weight: 600;
    cursor: pointer;
  }
</style>
