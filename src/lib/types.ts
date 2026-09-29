export type Cat = {
  id: string;
  name: string;
  description: string;
  module: "core" | "gaming" | "dev";
  risk: "safe" | "care";
  needs_admin: boolean;
  available: boolean;
};

export type ScanRes = { id: string; bytes: number; files: number };
export type CleanRes = { id: string; freed_bytes: number; deleted: number; skipped: number };
export type CleanDone = { total_bytes: number; log_path: string };

export type Drive = { letter: string; total: number; free: number };
export type Folder = { path: string; name: string; bytes: number };
export type SpaceResult = {
  root: string;
  files: number;
  bytes: number;
  hotspots: Folder[];
  top: Folder[];
  biggest: Folder[];
};

export type CleanItem = { id: string; name: string; freed: number; skipped: number };
export type CleanSummary = {
  freed: number;
  byMod: Record<string, number>;
  items: CleanItem[];
  skipped: number;
  logPath: string;
};

export type SpaceView = "hot" | "big" | "browse";
