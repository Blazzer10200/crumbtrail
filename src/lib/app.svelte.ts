import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  Cat,
  CleanDone,
  CleanRes,
  CleanSummary,
  Diff,
  Drive,
  Folder,
  Launcher,
  Preset,
  ScanError,
  ScanRes,
  SnapMeta,
  SpaceResult,
  SpaceView,
  TypeKey,
} from "./types";

// Clean choreography (ms). Real clean:result events are queued and replayed at
// this cadence so rows wash one after another even when the backend is instant.
const FIRST_WASH = 250;
const STAGGER = 380;
const SWEPT_AFTER = 560;
const SETTLE = 700;

const THEME_KEY = "crumbtrail-theme";
const LEGACY_THEME_KEY = "sweep-theme";
const WELCOMED_KEY = "crumbtrail-welcomed";

// Diffs below this are noise (matches the backend's 50 MB floor).
const MIN_DELTA = 50 * 1024 ** 2;

type Theme = "dark" | "light";
type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "ready" | "restarting" | "current" | "failed";

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null; // storage blocked (private mode / policy) — fall back to defaults
  }
}

function writeStorage(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch (e) {
    console.warn(`Couldn't persist ${key}:`, e);
  }
}

function ageDays(unix: number): number {
  return (Date.now() / 1000 - unix) / 86400;
}

class AppStore {
  tab = $state<"clean" | "space">("clean");
  admin = $state(false);
  theme = $state<Theme>("dark");
  version = $state("");
  welcomeOpen = $state(false);
  // One open popover/menu at a time: "more", "snap", "save", "preset:<id>".
  popover = $state<string | null>(null);

  update = $state.raw<Update | null>(null);
  updateStatus = $state<UpdateStatus>("idle");
  updateError = $state("");
  updateHidden = $state(false);
  dlDone = $state(0);
  dlTotal = $state(0);

  // ---- Clean ----
  cats = $state.raw<Cat[]>([]);
  sizes = $state.raw<Record<string, ScanRes>>({});
  checked = $state.raw<Record<string, boolean>>({});
  open = $state.raw<Record<string, boolean>>({ core: true, gaming: false, gpu: true, dev: false, adv: false });
  info = $state.raw<Record<string, boolean>>({});
  scanning = $state(false);
  cleaning = $state(false);
  retrying = $state(false);
  done = $state(false);
  sheetOpen = $state(false);
  washing = $state.raw<Record<string, boolean>>({});
  swept = $state.raw<Record<string, boolean>>({});
  cleanStep = $state(0);
  cleanTotal = $state(0);
  result = $state.raw<CleanSummary | null>(null);
  glow = $state(0);
  celebrateTick = $state(0);
  error = $state("");
  scanError = $state.raw<ScanError | null>(null);
  allTime = $state(0);
  adminDismissed = $state(false);
  adminCancelled = $state(false);

  presets = $state.raw<Preset[]>([]);
  presetNote = $state("");
  presetNoteBad = $state(false);
  private presetsLoadFailed = false;
  private noteTimer: ReturnType<typeof setTimeout> | undefined;

  // ---- Space (view-only) ----
  drives = $state.raw<Drive[]>([]);
  spaceScanning = $state(false);
  spaceProgress = $state.raw({ files: 0, bytes: 0 });
  spaceResult = $state.raw<SpaceResult | null>(null);
  spaceRoot = $state("");
  spaceTick = $state(0);
  view = $state<SpaceView>("changed");
  crumbs = $state.raw<Folder[]>([]);
  browseEntries = $state.raw<Folder[]>([]);
  rowsKey = $state(0);
  spaceError = $state("");
  diff = $state.raw<Diff | null>(null);
  diffId = $state("");
  diffLoading = $state(false);
  diffError = $state("");
  snapsC = $state.raw<SnapMeta[]>([]);
  launcherIcons = $state.raw<Record<string, string>>({});
  typeSel = $state<TypeKey | null>("video");
  gameSort = $state<"size" | "played">("size");

  busy = $derived(this.scanning || this.cleaning);
  selectableCats = $derived(this.cats.filter((c) => this.isSelectable(c)));
  selected = $derived(
    this.cats.filter(
      (c) => this.checked[c.id] && this.isSelectable(c) && (this.sizes[c.id]?.bytes ?? 0) > 0,
    ),
  );
  // Checked + selectable, regardless of size — what a preset captures.
  checkedIds = $derived(this.selectableCats.filter((c) => this.checked[c.id]).map((c) => c.id));
  selectedBytes = $derived(this.selected.reduce((a, c) => a + this.sizes[c.id].bytes, 0));
  selectedFiles = $derived(this.selected.reduce((a, c) => a + this.sizes[c.id].files, 0));
  scanStep = $derived(this.selectableCats.filter((c) => this.sizes[c.id]).length);
  lockedCats = $derived(this.admin ? [] : this.cats.filter((c) => c.needs_admin && c.available));
  heroTarget = $derived(
    this.done && this.result
      ? this.result.freed
      : this.selected.filter((c) => !this.swept[c.id]).reduce((a, c) => a + this.sizes[c.id].bytes, 0),
  );

  /** The snapshot What changed compares against (drive scans). */
  compareSnap = $derived(this.spaceResult?.snapshots.older.find((s) => s.id === this.diffId) ?? null);

  /** Clean-tab "What changed" one-liner. */
  teaser = $derived.by(() => {
    if (this.busy || this.scanError) return null;
    const r = this.spaceResult;
    if (r && r.is_drive && r.drive === "C:" && this.diff && this.compareSnap) {
      if (Math.abs(this.diff.net) < MIN_DELTA) return null;
      return { net: this.diff.net, since: this.compareSnap.taken_at, grew: this.diff.grew.slice(0, 2) };
    }
    const c = this.drives.find((d) => d.letter.toUpperCase() === "C:");
    const snap = this.snapsC[0];
    if (!c || !snap) return null;
    const net = c.total - c.free - snap.used_bytes;
    if (Math.abs(net) < MIN_DELTA) return null;
    return { net, since: snap.taken_at, grew: [] };
  });

  private timers: ReturnType<typeof setTimeout>[] = [];
  private cleanQueue: CleanRes[] = [];
  private cleanResults: CleanRes[] = [];
  private cleanDone: CleanDone | null = null;
  private pumping = false;
  private nextSlot = 0;
  private dlToken = 0;

  isSelectable(c: Cat): boolean {
    return c.available && !(c.needs_admin && !this.admin);
  }

  /** Wire up backend events, load initial data, kick off the first scan. Returns cleanup. */
  init(): () => void {
    this.loadTheme();
    if (readStorage(WELCOMED_KEY) !== "1") this.welcomeOpen = true;

    let disposed = false;
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      try {
        const ls = await Promise.all([
          listen<ScanRes>("scan:result", (e) => {
            this.sizes = { ...this.sizes, [e.payload.id]: e.payload };
          }),
          listen<ScanError>("scan:error", (e) => {
            this.scanError = e.payload;
          }),
          listen("scan:done", () => {
            this.scanning = false;
          }),
          listen<CleanRes>("clean:result", (e) => {
            this.cleanQueue.push(e.payload);
            this.pump();
          }),
          listen<CleanDone>("clean:done", (e) => {
            this.cleanDone = e.payload;
            this.pump();
          }),
          listen<{ files: number; bytes: number }>("space:progress", (e) => {
            this.spaceProgress = e.payload;
          }),
          listen<SpaceResult>("space:done", (e) => this.onSpaceDone(e.payload)),
        ]);
        if (disposed) return ls.forEach((u) => u());
        unlisteners.push(...ls);

        const [admin, cats, drives, version, allTime] = await Promise.all([
          invoke<boolean>("elevated"),
          invoke<Cat[]>("get_categories"),
          invoke<Drive[]>("drives"),
          getVersion(),
          invoke<number>("all_time_freed"),
        ]);
        this.admin = admin;
        this.cats = cats;
        this.drives = drives;
        this.version = version;
        this.allTime = allTime;
        this.checked = Object.fromEntries(
          cats.map((c) => [c.id, c.risk === "safe" && this.isSelectable(c)]),
        );

        void this.loadPresets();
        void this.loadSnapsC();
        void this.checkForUpdates(false);
        await this.startScan();
      } catch (e) {
        this.error = `Couldn't start: ${e}`;
      }
    })();

    return () => {
      disposed = true;
      unlisteners.forEach((u) => u());
      this.clearTimers();
    };
  }

  // ---- theme / welcome / tabs ----

  private loadTheme() {
    const stored = readStorage(THEME_KEY) ?? readStorage(LEGACY_THEME_KEY);
    this.applyTheme(stored === "light" ? "light" : "dark");
  }

  private applyTheme(t: Theme) {
    this.theme = t;
    document.documentElement.dataset.theme = t;
  }

  toggleTheme() {
    this.applyTheme(this.theme === "dark" ? "light" : "dark");
    writeStorage(THEME_KEY, this.theme);
  }

  openWelcome() {
    this.welcomeOpen = true;
  }

  closeWelcome() {
    this.welcomeOpen = false;
    writeStorage(WELCOMED_KEY, "1");
  }

  setTab(tab: "clean" | "space") {
    this.tab = tab;
    this.popover = null;
  }

  togglePopover(key: string) {
    this.popover = this.popover === key ? null : key;
  }

  // ---- updates ----

  async checkForUpdates(manual: boolean) {
    const s = this.updateStatus;
    if (s === "checking" || s === "downloading" || s === "restarting") return;
    // An update is already known: the chip just brings its banner back.
    if (this.update && (s === "available" || s === "ready")) {
      this.updateHidden = false;
      return;
    }
    this.updateStatus = "checking";
    this.updateHidden = false;
    this.updateError = "";
    try {
      const u = await check();
      this.update = u;
      this.updateStatus = u ? "available" : manual ? "current" : "idle";
    } catch (e) {
      console.error("Update check failed:", e);
      this.updateError = String(e);
      // The silent launch check stays quiet (offline is normal); a manual click shows it.
      this.updateStatus = manual ? "failed" : "idle";
    }
    if (this.updateStatus === "current") {
      setTimeout(() => {
        if (this.updateStatus === "current") this.updateStatus = "idle";
      }, 7000);
    }
  }

  dismissUpdate() {
    if (this.updateStatus === "current" || this.updateStatus === "failed") this.updateStatus = "idle";
    else this.updateHidden = true;
  }

  async downloadUpdate() {
    const u = this.update;
    if (!u || this.updateStatus !== "available") return;
    const token = ++this.dlToken;
    this.updateStatus = "downloading";
    this.updateError = "";
    this.dlDone = 0;
    this.dlTotal = 0;
    try {
      await u.download((ev) => {
        if (token !== this.dlToken) return;
        if (ev.event === "Started") this.dlTotal = ev.data.contentLength ?? 0;
        else if (ev.event === "Progress") this.dlDone += ev.data.chunkLength;
      });
      if (token === this.dlToken) this.updateStatus = "ready";
    } catch (e) {
      if (token !== this.dlToken) return;
      this.updateError = String(e);
      this.updateStatus = "available";
    }
  }

  async installUpdate() {
    const u = this.update;
    if (!u || this.updateStatus !== "ready" || this.busy) return;
    this.updateStatus = "restarting";
    this.updateHidden = false;
    try {
      await u.install();
      await relaunch();
    } catch (e) {
      this.updateError = String(e);
      this.updateStatus = "ready";
    }
  }

  async relaunchAdmin() {
    if (this.admin || this.busy) return;
    try {
      await invoke("relaunch_admin");
    } catch (e) {
      if (String(e).includes("cancelled")) {
        this.adminCancelled = true;
        this.adminDismissed = false;
      } else {
        this.error = `Couldn't restart as admin: ${e}`;
      }
    }
  }

  dismissAdmin() {
    this.adminDismissed = true;
    this.adminCancelled = false;
  }

  // ---- clean flow ----

  private later(fn: () => void, ms: number) {
    this.timers.push(setTimeout(fn, ms));
  }

  private clearTimers() {
    this.timers.forEach(clearTimeout);
    this.timers = [];
  }

  async startScan() {
    if (this.busy) return;
    this.clearTimers();
    this.sizes = {};
    this.washing = {};
    this.swept = {};
    this.result = null;
    this.done = false;
    this.glow = 0;
    this.error = "";
    this.scanError = null;
    this.scanning = true;
    try {
      await invoke("scan");
    } catch (e) {
      this.scanning = false;
      this.error = `Scan failed: ${e}`;
    }
  }

  toggle(c: Cat) {
    if (!this.isSelectable(c) || this.busy) return;
    this.checked = { ...this.checked, [c.id]: !this.checked[c.id] };
    this.done = false;
  }

  setGroup(rows: Cat[]) {
    if (this.busy) return;
    const selectable = rows.filter((c) => this.isSelectable(c));
    const anyOn = selectable.some((c) => this.checked[c.id]);
    const next = { ...this.checked };
    for (const c of selectable) next[c.id] = !anyOn;
    this.checked = next;
    this.done = false;
  }

  toggleOpen(key: string) {
    this.open = { ...this.open, [key]: !this.open[key] };
  }

  toggleInfo(id: string) {
    this.info = { ...this.info, [id]: !this.info[id] };
  }

  requestClean() {
    if (this.done) return void this.startScan();
    if (this.busy || this.selected.length === 0) return;
    this.sheetOpen = true;
  }

  closeSheet() {
    this.sheetOpen = false;
  }

  /** Re-run just the items that had files in use. The user already approved these in the review sheet. */
  retrySkipped() {
    const ids = this.result?.items.filter((i) => i.skipped > 0).map((i) => i.id) ?? [];
    if (ids.length) void this.doClean(ids);
  }

  async doClean(retryIds?: string[]) {
    const list = retryIds ? this.cats.filter((c) => retryIds.includes(c.id)) : this.selected;
    if (!list.length || this.busy) return;
    this.sheetOpen = false;
    this.error = "";
    this.retrying = !!retryIds;
    this.cleaning = true;
    this.cleanStep = 0;
    this.cleanTotal = list.length;
    this.cleanQueue = [];
    this.cleanResults = [];
    this.cleanDone = null;
    this.pumping = false;
    this.nextSlot = performance.now() + FIRST_WASH;
    try {
      await invoke("clean", { ids: list.map((c) => c.id) });
    } catch (e) {
      this.clearTimers();
      this.cleaning = false;
      this.retrying = false;
      this.error = `Clean failed: ${e}`;
    }
  }

  /** Replay queued clean results one row at a time; finish once the backend is done. */
  private pump() {
    if (this.pumping || !this.cleaning) return;
    const r = this.cleanQueue.shift();
    if (!r) {
      const done = this.cleanDone;
      if (done) this.later(() => this.finishClean(done), Math.max(0, this.nextSlot - performance.now()) + SETTLE);
      return;
    }
    this.pumping = true;
    this.later(
      () => {
        this.washing = { ...this.washing, [r.id]: true };
        this.cleanStep++;
        this.cleanResults.push(r);
        this.later(() => (this.swept = { ...this.swept, [r.id]: true }), SWEPT_AFTER);
        this.nextSlot = performance.now() + STAGGER;
        this.pumping = false;
        this.pump();
      },
      Math.max(0, this.nextSlot - performance.now()),
    );
  }

  private finishClean(payload: CleanDone) {
    const prev = this.retrying ? this.result : null;
    const byMod: Record<string, number> = { ...(prev?.byMod ?? {}) };
    const sizes = { ...this.sizes };
    const fresh = new Map(
      this.cleanResults.map((r) => {
        const cat = this.cats.find((c) => c.id === r.id);
        if (cat) byMod[cat.module] = (byMod[cat.module] ?? 0) + r.freed_bytes;
        const before = sizes[r.id];
        if (before) {
          sizes[r.id] = {
            ...before,
            bytes: Math.max(0, before.bytes - r.freed_bytes),
            files: Math.max(0, before.files - r.deleted),
          };
        }
        return [r.id, { id: r.id, name: cat?.name ?? r.id, freed: r.freed_bytes, skipped: r.skipped }];
      }),
    );
    // A retry folds into the existing card: recovered bytes add up, skipped counts are replaced.
    const items = prev
      ? prev.items.map((it) => {
          const r = fresh.get(it.id);
          return r ? { ...it, freed: it.freed + r.freed, skipped: r.skipped } : it;
        })
      : [...fresh.values()];
    this.sizes = sizes;
    const skipped = items.reduce((a, i) => a + i.skipped, 0);
    this.result = {
      freed: (prev?.freed ?? 0) + payload.total_bytes,
      byMod,
      items,
      skipped,
      logPath: payload.log_path,
    };
    if (payload.all_time !== null) this.allTime = payload.all_time;
    else {
      console.error("All-time total wasn't saved; see", payload.log_path);
      this.allTime += payload.total_bytes;
    }
    this.cleaning = false;
    this.retrying = false;
    this.done = true;
    this.glow = 0.16;
    // Confetti only for a clean sweep; a partial result stays calm (amber card).
    if (skipped === 0) this.celebrateTick++;
    this.later(() => (this.glow = 0.045), SETTLE);
    void this.refreshDrives();
  }

  private async refreshDrives() {
    try {
      this.drives = await invoke<Drive[]>("drives");
    } catch (e) {
      console.error("Couldn't refresh drives:", e);
    }
  }

  // ---- presets ----

  private note(text: string, bad = false) {
    clearTimeout(this.noteTimer);
    this.presetNote = text;
    this.presetNoteBad = bad;
    this.noteTimer = setTimeout(() => (this.presetNote = ""), bad ? 8000 : 4000);
  }

  private async loadPresets() {
    try {
      const v = await invoke<{ presets?: Preset[] }>("load_presets");
      const known = new Set(this.cats.map((c) => c.id));
      this.presets = (v.presets ?? []).map((p) => ({ ...p, ids: p.ids.filter((id) => known.has(id)) }));
    } catch (e) {
      this.presetsLoadFailed = true;
      this.note(`Couldn't load presets: ${e}`, true);
    }
  }

  private async persistPresets(next: Preset[]) {
    if (this.presetsLoadFailed) {
      return this.note("Presets file couldn't be read, so changes aren't saved. Fix or remove presets.json first.", true);
    }
    this.presets = next;
    try {
      await invoke("save_presets", { presets: { version: 1, presets: next } });
    } catch (e) {
      this.note(`Couldn't save presets: ${e}`, true);
    }
  }

  presetActive(p: Preset): boolean {
    const mine = this.selectableCats.filter((c) => p.ids.includes(c.id)).map((c) => c.id);
    return mine.length > 0 && mine.length === this.checkedIds.length && mine.every((id) => this.checked[id]);
  }

  presetBytes(p: Preset): number {
    return this.selectableCats.filter((c) => p.ids.includes(c.id)).reduce((a, c) => a + (this.sizes[c.id]?.bytes ?? 0), 0);
  }

  presetUnavailable(p: Preset): Cat[] {
    return this.cats.filter((c) => p.ids.includes(c.id) && !this.isSelectable(c));
  }

  applyPreset(p: Preset) {
    if (this.busy) return;
    const ids = new Set(p.ids);
    this.checked = Object.fromEntries(this.cats.map((c) => [c.id, ids.has(c.id) && this.isSelectable(c)]));
    this.done = false;
    const skipped = this.presetUnavailable(p);
    if (skipped.length) {
      const why = skipped.map((c) => `${c.name} (${c.available ? "needs admin" : "not found"})`).join(", ");
      this.note(`Applied “${p.name}” · skipped ${why}`);
    }
  }

  presetNameTaken(name: string, exceptId = ""): boolean {
    const n = name.trim().toLowerCase();
    return this.presets.some((p) => p.id !== exceptId && p.name.toLowerCase() === n);
  }

  savePreset(name: string) {
    const n = name.trim();
    if (!n || this.presetNameTaken(n) || !this.checkedIds.length) return;
    this.popover = null;
    void this.persistPresets([...this.presets, { id: `p${Date.now()}`, name: n, ids: [...this.checkedIds] }]);
  }

  renamePreset(id: string, name: string) {
    const n = name.trim();
    if (!n || this.presetNameTaken(n, id)) return; // reverts silently, per spec
    void this.persistPresets(this.presets.map((p) => (p.id === id ? { ...p, name: n } : p)));
  }

  updatePreset(id: string) {
    this.popover = null;
    void this.persistPresets(this.presets.map((p) => (p.id === id ? { ...p, ids: [...this.checkedIds] } : p)));
  }

  deletePreset(id: string) {
    this.popover = null;
    void this.persistPresets(this.presets.filter((p) => p.id !== id));
  }

  // ---- space ----

  private async loadSnapsC() {
    try {
      const s = await invoke<SnapMeta[]>("snapshots", { drive: "C:" });
      this.snapsC = [...s].sort((a, b) => b.taken_at - a.taken_at);
    } catch (e) {
      console.error("Couldn't list snapshots:", e);
    }
  }

  private onSpaceDone(r: SpaceResult) {
    this.spaceScanning = false;
    this.spaceResult = r;
    this.browseEntries = r.top;
    this.crumbs = [];
    this.view = "changed";
    this.diff = null;
    this.diffId = "";
    this.diffError = "";
    this.rowsKey++;
    this.spaceTick++;
    if (r.snapshots.error) this.spaceError = `Couldn't save a snapshot: ${r.snapshots.error}`;
    // Default comparison: the newest scan at least ~a week old, else the oldest one there is.
    const older = [...r.snapshots.older].sort((a, b) => b.taken_at - a.taken_at);
    const pick = older.find((s) => ageDays(s.taken_at) >= 6) ?? older[older.length - 1];
    if (pick) void this.compareWith(pick.id);
    void this.refreshDrives();
    if (r.drive === "C:") void this.loadSnapsC();
  }

  async compareWith(id: string) {
    this.diffId = id;
    this.popover = null;
    this.diffLoading = true;
    this.diffError = "";
    try {
      const d = await invoke<Diff>("space_diff", { id });
      if (this.diffId === id) this.diff = d;
    } catch (e) {
      if (this.diffId === id) this.diffError = `Couldn't compare with that scan: ${e}`;
    } finally {
      if (this.diffId === id) this.diffLoading = false;
    }
  }

  async scanRoot(root: string) {
    if (this.spaceScanning) return;
    this.spaceResult = null;
    this.spaceRoot = root;
    this.spaceProgress = { files: 0, bytes: 0 };
    this.spaceError = "";
    this.view = "changed";
    this.crumbs = [];
    this.browseEntries = [];
    this.diff = null;
    this.diffId = "";
    this.popover = null;
    this.spaceScanning = true;
    try {
      await invoke("space_scan", { root });
    } catch (e) {
      this.spaceScanning = false;
      this.spaceError = `Scan failed: ${e}`;
    }
  }

  scanDrive(letter: string) {
    return this.scanRoot(letter + "\\");
  }

  async scanFolder() {
    if (this.spaceScanning) return;
    try {
      const picked = await openDialog({ directory: true, title: "Scan a folder" });
      if (typeof picked === "string") await this.scanRoot(picked);
    } catch (e) {
      this.spaceError = `Couldn't open the folder picker: ${e}`;
    }
  }

  /** Teaser click: land on What changed for C:, scanning it first if needed. */
  openWhatChanged() {
    this.setTab("space");
    const r = this.spaceResult;
    if (r && r.is_drive && r.drive === "C:") this.setView("changed");
    else if (!this.spaceScanning) void this.scanDrive("C:");
  }

  setView(v: SpaceView) {
    this.view = v;
    this.popover = null;
    this.crumbs = [];
    this.browseEntries = this.spaceResult?.top ?? [];
    this.rowsKey++;
    if (v === "games" && !Object.keys(this.launcherIcons).length) void this.loadIcons();
  }

  private async loadIcons() {
    try {
      this.launcherIcons = await invoke<Record<string, string>>("launcher_icons");
    } catch (e) {
      console.error("Couldn't load launcher icons:", e); // letter chips stay as the fallback
    }
  }

  async openLauncher(launcher: Launcher) {
    try {
      await invoke("open_launcher", { launcher });
    } catch (e) {
      this.spaceError = `Couldn't open the launcher: ${e}`;
    }
  }

  async drillInto(f: Folder) {
    try {
      const entries = await invoke<Folder[]>("space_children", { dir: f.path });
      // No subfolders to show — open it in Explorer instead of a dead click.
      if (entries.length === 0) return void this.reveal(f.path);
      this.crumbs = [...this.crumbs, f];
      this.browseEntries = entries;
      this.rowsKey++;
    } catch (e) {
      this.spaceError = `Couldn't open ${f.name}: ${e}`;
    }
  }

  async jumpTo(index: number) {
    try {
      if (index < 0) {
        this.crumbs = [];
        this.browseEntries = this.spaceResult?.top ?? [];
      } else {
        const target = this.crumbs[index];
        this.browseEntries = await invoke<Folder[]>("space_children", { dir: target.path });
        this.crumbs = this.crumbs.slice(0, index + 1);
      }
      this.rowsKey++;
    } catch (e) {
      this.spaceError = `Couldn't open that folder: ${e}`;
    }
  }

  async reveal(path: string) {
    try {
      await invoke("reveal", { path });
    } catch (e) {
      const msg = `Couldn't open Explorer: ${e}`;
      if (this.tab === "space") this.spaceError = msg;
      else this.error = msg;
    }
  }
}

export const app = new AppStore();
