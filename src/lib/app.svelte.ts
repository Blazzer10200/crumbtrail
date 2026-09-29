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
  Drive,
  Folder,
  ScanRes,
  SpaceResult,
  SpaceView,
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

type Theme = "dark" | "light";
type UpdateStatus = "idle" | "checking" | "current" | "failed" | "installing";

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

class AppStore {
  tab = $state<"clean" | "space">("clean");
  admin = $state(false);
  theme = $state<Theme>("dark");
  version = $state("");
  welcomeOpen = $state(false);

  update = $state.raw<Update | null>(null);
  updateStatus = $state<UpdateStatus>("idle");
  updateError = $state("");

  // ---- Clean ----
  cats = $state.raw<Cat[]>([]);
  sizes = $state.raw<Record<string, ScanRes>>({});
  checked = $state.raw<Record<string, boolean>>({});
  open = $state.raw<Record<string, boolean>>({ core: true, gaming: false, dev: false, adv: false });
  info = $state.raw<Record<string, boolean>>({});
  scanning = $state(false);
  cleaning = $state(false);
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

  // ---- Space (view-only) ----
  drives = $state.raw<Drive[]>([]);
  spaceScanning = $state(false);
  spaceProgress = $state.raw({ files: 0, bytes: 0 });
  spaceResult = $state.raw<SpaceResult | null>(null);
  spaceRoot = $state("");
  view = $state<SpaceView>("hot");
  crumbs = $state.raw<Folder[]>([]);
  browseEntries = $state.raw<Folder[]>([]);
  rowsKey = $state(0);
  spaceError = $state("");

  busy = $derived(this.scanning || this.cleaning);
  selectableCats = $derived(this.cats.filter((c) => this.isSelectable(c)));
  selected = $derived(
    this.cats.filter(
      (c) => this.checked[c.id] && this.isSelectable(c) && (this.sizes[c.id]?.bytes ?? 0) > 0,
    ),
  );
  selectedBytes = $derived(this.selected.reduce((a, c) => a + this.sizes[c.id].bytes, 0));
  selectedFiles = $derived(this.selected.reduce((a, c) => a + this.sizes[c.id].files, 0));
  scanStep = $derived(this.selectableCats.filter((c) => this.sizes[c.id]).length);
  heroTarget = $derived(
    this.done && this.result
      ? this.result.freed
      : this.selected.filter((c) => !this.swept[c.id]).reduce((a, c) => a + this.sizes[c.id].bytes, 0),
  );

  private timers: ReturnType<typeof setTimeout>[] = [];
  private cleanQueue: CleanRes[] = [];
  private cleanResults: CleanRes[] = [];
  private cleanDone: CleanDone | null = null;
  private pumping = false;
  private nextSlot = 0;

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
          listen<SpaceResult>("space:done", (e) => {
            this.spaceScanning = false;
            this.spaceResult = e.payload;
            this.browseEntries = e.payload.top;
            this.crumbs = [];
            this.rowsKey++;
          }),
        ]);
        if (disposed) return ls.forEach((u) => u());
        unlisteners.push(...ls);

        const [admin, cats, drives, version] = await Promise.all([
          invoke<boolean>("elevated"),
          invoke<Cat[]>("get_categories"),
          invoke<Drive[]>("drives"),
          getVersion(),
        ]);
        this.admin = admin;
        this.cats = cats;
        this.drives = drives;
        this.version = version;
        this.checked = Object.fromEntries(
          cats.map((c) => [c.id, c.risk === "safe" && this.isSelectable(c)]),
        );

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
  }

  // ---- updates ----

  async checkForUpdates(manual: boolean) {
    if (this.updateStatus === "checking" || this.updateStatus === "installing") return;
    this.updateStatus = "checking";
    try {
      const u = await check();
      this.update = u;
      this.updateStatus = !u && manual ? "current" : "idle";
    } catch (e) {
      console.error("Update check failed:", e);
      this.updateError = String(e);
      // The silent launch check stays quiet (offline is normal); a manual click shows it.
      this.updateStatus = manual ? "failed" : "idle";
    }
    if (this.updateStatus === "current" || this.updateStatus === "failed") {
      setTimeout(() => {
        if (this.updateStatus === "current" || this.updateStatus === "failed") this.updateStatus = "idle";
      }, 3000);
    }
  }

  async installUpdate() {
    if (!this.update || this.updateStatus === "installing") return;
    this.updateStatus = "installing";
    this.updateError = "";
    try {
      await this.update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      this.updateError = String(e);
      this.updateStatus = "idle";
    }
  }

  async relaunchAdmin() {
    if (this.admin || this.busy) return;
    try {
      await invoke("relaunch_admin");
    } catch (e) {
      this.error = `Couldn't restart as admin: ${e}`;
    }
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

  async doClean() {
    const list = this.selected;
    if (!list.length || this.busy) return;
    this.sheetOpen = false;
    this.error = "";
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
    const byMod: Record<string, number> = {};
    const sizes = { ...this.sizes };
    const items = this.cleanResults.map((r) => {
      const cat = this.cats.find((c) => c.id === r.id);
      if (cat) byMod[cat.module] = (byMod[cat.module] ?? 0) + r.freed_bytes;
      const before = sizes[r.id];
      if (before) {
        sizes[r.id] = {
          id: r.id,
          bytes: Math.max(0, before.bytes - r.freed_bytes),
          files: Math.max(0, before.files - r.deleted),
        };
      }
      return { id: r.id, name: cat?.name ?? r.id, freed: r.freed_bytes, skipped: r.skipped };
    });
    this.sizes = sizes;
    this.result = {
      freed: payload.total_bytes,
      byMod,
      items,
      skipped: items.reduce((a, i) => a + i.skipped, 0),
      logPath: payload.log_path,
    };
    this.cleaning = false;
    this.done = true;
    this.glow = 0.16;
    this.celebrateTick++;
    this.later(() => (this.glow = 0.045), SETTLE);
  }

  // ---- space ----

  async scanRoot(root: string) {
    if (this.spaceScanning) return;
    this.spaceResult = null;
    this.spaceRoot = root;
    this.spaceProgress = { files: 0, bytes: 0 };
    this.spaceError = "";
    this.view = "hot";
    this.crumbs = [];
    this.browseEntries = [];
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

  setView(v: SpaceView) {
    this.view = v;
    this.crumbs = [];
    this.browseEntries = this.spaceResult?.top ?? [];
    this.rowsKey++;
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
