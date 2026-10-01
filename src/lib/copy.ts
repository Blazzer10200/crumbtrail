import type { Cat, Launcher, SpaceView, TypeKey } from "./types";

export const MODS = [
  { key: "core", label: "System & apps", color: "var(--accent)" },
  { key: "gaming", label: "Gaming", color: "var(--violet)" },
  { key: "gpu", label: "Old GPU driver installers", color: "var(--rose)" },
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
  if (["user_temp", "system_temp", "crash_dumps", "nvidia_installers", "amd_installers"].includes(id)) return "safe";
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
  nvidia_installers: [
    "When you install an NVIDIA driver, the installer unpacks its setup files to a folder on your drive and leaves them there, often 1 to 3 GB for every version you've ever installed. Once the driver is installed they're just leftovers. Crumbtrail keeps the folder for the version you're running now. Your installed driver, control panel settings and game profiles are not touched.",
    "C:\\NVIDIA\\DisplayDriver\\<version> · C:\\NVIDIA\\GFExperience",
  ],
  amd_installers: [
    "The AMD Software installer unpacks its files to C:\\AMD and doesn't tidy up. Each driver you've installed can leave 1 to 2 GB behind, and nothing uses them after the install finishes. Your installed Radeon driver and Adrenalin settings are not touched. For a repair install, AMD's site has a fresh copy.",
    "C:\\AMD\\<package folders>",
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

type ViewMeta = { key: SpaceView; label: string; color: string; hint: string; w: number };

// w = switcher slot width (px), per the v6 handoff.
export const VIEWS: ViewMeta[] = [
  {
    key: "changed",
    label: "What changed",
    color: "var(--amber)",
    hint: "Folders compared with an earlier scan. View only: nothing here changes any files.",
    w: 128,
  },
  {
    key: "hot",
    label: "Biggest folders",
    color: "var(--accent)",
    hint: "Folders where space piles up. Click one to open it in Explorer.",
    w: 124,
  },
  {
    key: "big",
    label: "Largest files",
    color: "var(--violet)",
    hint: "The biggest single files. Anything tagged system is managed by Windows.",
    w: 108,
  },
  {
    key: "browse",
    label: "Browse",
    color: "var(--sky)",
    hint: "Click a folder to look inside. Open shows it in Explorer.",
    w: 76,
  },
];

export const MORE_W = 84;

// Views behind the "More ▾" slot (drive scans only).
export const MORE: (ViewMeta & { desc: string })[] = [
  {
    key: "games",
    label: "Games",
    color: "var(--violet)",
    desc: "Installed games, by size or last played",
    hint: "Installed games from Steam and Epic. To remove one, uninstall it from its launcher. Crumbtrail never deletes games.",
    w: 84,
  },
  {
    key: "installers",
    label: "Forgotten installers",
    color: "var(--amber)",
    desc: ".exe, .msi and .zip files over 30 days old",
    hint: "Setup files in Downloads untouched for over 30 days. Crumbtrail only lists them. Delete them yourself in Explorer if you're done with them.",
    w: 172,
  },
  {
    key: "types",
    label: "File types",
    color: "var(--sky)",
    desc: "Videos, games, archives and more at a glance",
    hint: "Everything on the drive grouped by type. Click a segment to list its biggest files.",
    w: 100,
  },
];

export const TYPES: { key: TypeKey; label: string; plural: string; color: string }[] = [
  { key: "video", label: "Videos", plural: "videos", color: "var(--violet)" },
  { key: "game", label: "Games", plural: "game files", color: "var(--accent)" },
  { key: "installer", label: "Installers", plural: "installers", color: "var(--amber)" },
  { key: "archive", label: "Archives", plural: "archives", color: "var(--sky)" },
  { key: "image", label: "Images", plural: "images", color: "var(--rose)" },
  { key: "doc", label: "Documents", plural: "documents", color: "var(--docs)" },
  { key: "other", label: "Other", plural: "other files", color: "var(--line-4)" },
];

export const LAUNCH: Record<Launcher, string> = { steam: "Steam", epic: "Epic" };

// Forgotten-installer badge colors by extension.
export const EXT_COLOR: Record<string, string> = { exe: "var(--sky)", msi: "var(--violet)", zip: "var(--amber)" };

// Windows-managed files that show up huge but aren't user-deletable.
export const SYSTEM_FILES = new Set(["pagefile.sys", "hiberfil.sys", "swapfile.sys", "dumpstack.log.tmp"]);
