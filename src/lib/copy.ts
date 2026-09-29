import type { Cat, SpaceView } from "./types";

export const MODS = [
  { key: "core", label: "System & apps", color: "var(--accent)" },
  { key: "gaming", label: "Gaming", color: "var(--violet)" },
  { key: "dev", label: "Developer", color: "var(--sky)" },
] as const;

export type Group = {
  key: string;
  label: string;
  color: string;
  match: (c: Cat) => boolean;
};

// "More options" holds the risk: "care" items (Windows Update leftovers, Recycle Bin).
export const GROUPS: Group[] = [
  ...MODS.map((m) => ({ ...m, match: (c: Cat) => c.module === m.key && c.risk !== "care" })),
  { key: "adv", label: "More options", color: "var(--amber)", match: (c: Cat) => c.risk === "care" },
];

export type Risk = "safe" | "rebuild" | "perm";

export const RISK_LABEL: Record<Risk, string> = {
  safe: "Safe",
  rebuild: "Rebuilds itself",
  perm: "Permanent",
};

export function riskOf(id: string): Risk {
  if (id === "recycle_bin") return "perm";
  if (["user_temp", "system_temp", "crash_dumps"].includes(id)) return "safe";
  return "rebuild";
}

// ⓘ explainer copy: [what happens, where it lives].
export const INFO: Record<string, [string, string]> = {
  user_temp: [
    "Leftover files apps created and forgot to delete. Only files older than 48 hours are touched, so nothing running right now is affected.",
    "%TEMP%",
  ],
  system_temp: [
    "Windows' shared temp folder. Same 48-hour rule. Needs admin because it's a system folder.",
    "C:\\Windows\\Temp",
  ],
  windows_update: [
    "Installers for updates that are already installed. Your updates stay installed. Windows re-downloads these only if it ever needs them.",
    "C:\\Windows\\SoftwareDistribution\\Download",
  ],
  browser_cache: [
    "Saved copies of web pages that help them load faster. Logins, passwords, history and bookmarks are not touched.",
    "Chrome, Edge and Firefox profile caches",
  ],
  thumbnails: [
    "Small preview images Explorer keeps for folders. Windows re-creates them as you browse.",
    "%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer",
  ],
  crash_dumps: [
    "Reports saved when an app crashed. Only useful for debugging, safe to remove.",
    "%LOCALAPPDATA%\\CrashDumps · WER",
  ],
  recycle_bin: [
    "Permanently deletes everything in your Recycle Bin on every drive. You won't be able to restore these files afterwards.",
    "Recycle Bin (all drives)",
  ],
  dx_shader: [
    "Pre-built graphics data for games. Rebuilt automatically. The first launch of a game may take a few extra seconds.",
    "%LOCALAPPDATA%\\D3DSCache",
  ],
  nvidia_shader: [
    "NVIDIA's copy of compiled game graphics. Rebuilt in the background while you play.",
    "%LOCALAPPDATA%\\NVIDIA\\DXCache · GLCache",
  ],
  steam_shader: [
    "Per-game graphics caches from Steam. Steam rebuilds or re-downloads them when needed.",
    "steamapps\\shadercache",
  ],
  citizenfx_cache: [
    "Downloaded server content for FiveM / RedM. Re-downloads next time you join a server.",
    "%LOCALAPPDATA%\\FiveM\\FiveM.app\\data\\cache",
  ],
  npm_cache: [
    "Downloaded JavaScript packages. Your projects aren't touched. npm re-fetches packages when needed.",
    "%LOCALAPPDATA%\\npm-cache",
  ],
  pip_cache: [
    "Downloaded Python packages. Installed packages stay. pip re-fetches when needed.",
    "%LOCALAPPDATA%\\pip\\cache",
  ],
  cargo_cache: [
    "Downloaded Rust crate archives. Your projects aren't touched. Cargo re-fetches when needed.",
    "~\\.cargo\\registry\\cache",
  ],
};

export const VIEWS: { key: SpaceView; label: string; color: string; hint: string }[] = [
  {
    key: "hot",
    label: "Biggest folders",
    color: "var(--accent)",
    hint: "Folders where space piles up. Click one to open it in Explorer.",
  },
  {
    key: "big",
    label: "Largest files",
    color: "var(--violet)",
    hint: "The biggest single files. Anything tagged system is managed by Windows.",
  },
  {
    key: "browse",
    label: "Browse",
    color: "var(--sky)",
    hint: "Click a folder to look inside. Open shows it in Explorer.",
  },
];

// Windows-managed files that show up huge but aren't user-deletable.
export const SYSTEM_FILES = new Set(["pagefile.sys", "hiberfil.sys", "swapfile.sys", "dumpstack.log.tmp"]);
