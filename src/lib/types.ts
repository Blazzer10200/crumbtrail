export type Cat = {
  id: string;
  name: string;
  description: string;
  module: "core" | "gaming" | "gpu" | "dev";
  risk: "safe" | "care";
  needs_admin: boolean;
  available: boolean;
};

export type ScanRes = { id: string; bytes: number; files: number; unreadable: number };
export type ScanError = { ids: string[]; message: string; log_path: string };
export type CleanRes = { id: string; freed_bytes: number; deleted: number; skipped: number };
export type CleanDone = { total_bytes: number; log_path: string; all_time: number | null };

export type Drive = { letter: string; total: number; free: number };
export type Folder = { path: string; name: string; bytes: number };

export type SnapMeta = { id: string; taken_at: number; used_bytes: number };
export type OlderSnap = SnapMeta & { net: number | null };
export type DiffRow = { path: string; name: string; now: number; then: number; delta: number; is_new: boolean };
export type Diff = { net: number; grew: DiffRow[]; shrank: DiffRow[] };

export type Launcher = "steam" | "epic";
export type Game = { name: string; launcher: Launcher; path: string; bytes: number; last_played: number | null };
export type Installer = { name: string; path: string; bytes: number; age_days: number };
export type Installers = { dir: string; old: Installer[]; newer: number };

export type TypeKey = "video" | "game" | "installer" | "archive" | "image" | "doc" | "other";

export type SpaceResult = {
  root: string;
  drive: string;
  is_drive: boolean;
  files: number;
  bytes: number;
  used_bytes: number | null;
  hotspots: Folder[];
  top: Folder[];
  biggest: Folder[];
  unreadable: { count: number; paths: string[] };
  types: { key: TypeKey; bytes: number }[];
  type_top: Record<TypeKey, Folder[]>;
  games: Game[];
  installers: Installers | null;
  snapshots: { saved: SnapMeta | null; older: OlderSnap[]; error: string | null };
};

export type CleanItem = { id: string; name: string; freed: number; skipped: number };
export type CleanSummary = {
  freed: number;
  byMod: Record<string, number>;
  items: CleanItem[];
  skipped: number;
  logPath: string;
};

export type Preset = { id: string; name: string; ids: string[] };

export type SpaceView = "changed" | "hot" | "big" | "browse" | "games" | "installers" | "types";
