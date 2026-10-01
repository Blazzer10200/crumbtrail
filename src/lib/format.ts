export function fmt(b: number): string {
  if (b >= 1024 ** 3) return (b / 1024 ** 3).toFixed(2) + " GB";
  if (b >= 1024 ** 2) return (b / 1024 ** 2).toFixed(1) + " MB";
  if (b >= 1024) return (b / 1024).toFixed(0) + " KB";
  return Math.round(b) + " B";
}

// Space tab sizes can reach terabytes (drive totals).
export function fmtS(b: number): string {
  return b >= 1024 ** 4 ? (b / 1024 ** 4).toFixed(2) + " TB" : fmt(b);
}

// All-time stat: "84.3 GB" (always GB, 1 decimal).
export function gb1(b: number): string {
  return (b / 1024 ** 3).toFixed(1) + " GB";
}

// Signed size for diffs: "+7.90 GB" / "−1.20 GB".
export function signed(b: number): string {
  return (b < 0 ? "−" : "+") + fmtS(Math.abs(b));
}

const DAY = 86400;
const WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

function daysAgo(unix: number, now = Date.now() / 1000): number {
  const d = new Date(unix * 1000);
  const t = new Date(now * 1000);
  const a = Date.UTC(d.getFullYear(), d.getMonth(), d.getDate());
  const b = Date.UTC(t.getFullYear(), t.getMonth(), t.getDate());
  return Math.round((b - a) / (DAY * 1000));
}

// "today", "yesterday", "3 days ago", "2 weeks ago" (<60d), "4 months ago" (<365d), "2 years ago".
export function ago(unix: number): string {
  const d = daysAgo(unix);
  if (d <= 0) return "today";
  if (d === 1) return "yesterday";
  if (d < 14) return `${d} days ago`;
  if (d < 60) return `${Math.floor(d / 7)} weeks ago`;
  if (d < 365) return `${Math.floor(d / 30)} months ago`;
  const y = Math.floor(d / 365);
  return y === 1 ? "a year ago" : `${y} years ago`;
}

export function ageDays(days: number): string {
  if (days < 60) return `${Math.floor(days / 7)} weeks old`;
  if (days < 365) return `${Math.floor(days / 30)} months old`;
  const y = Math.floor(days / 365);
  return y === 1 ? "1 year old" : `${y} years old`;
}

// Snapshot label used after "since …": "yesterday", "Thursday", "last Tuesday", "Sep 15".
export function snapLabel(unix: number): string {
  const d = daysAgo(unix);
  const date = new Date(unix * 1000);
  if (d <= 0) return "earlier today";
  if (d === 1) return "yesterday";
  if (d < 7) return WEEKDAYS[date.getDay()];
  if (d < 14) return `last ${WEEKDAYS[date.getDay()]}`;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

// Snapshot picker title: "Last Tuesday", "Two weeks ago", "A month ago", "Sep 15".
export function snapTitle(unix: number): string {
  const d = daysAgo(unix);
  if (d < 14) {
    const s = snapLabel(unix);
    return s[0].toUpperCase() + s.slice(1);
  }
  if (d < 21) return "Two weeks ago";
  if (d < 28) return "Three weeks ago";
  if (d < 45) return "A month ago";
  return snapLabel(unix);
}

// "Tue, Sep 22"
export function shortDate(unix: number): string {
  return new Date(unix * 1000).toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
}

export function dayCount(unix: number): string {
  const d = daysAgo(unix);
  return d <= 0 ? "today" : d === 1 ? "1 day ago" : `${d} days ago`;
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

// "C:\Users\me\file.bin" -> "C:\Users\me" (drive roots keep their trailing slash).
export function parentPath(p: string): string {
  const parent = p.slice(0, p.lastIndexOf("\\") + 1);
  return parent.length <= 3 ? parent : parent.slice(0, -1);
}

export function baseName(p: string): string {
  return p.slice(p.lastIndexOf("\\") + 1);
}

// "C:\" stays as-is; "D:\Games\" -> "D:\Games".
export function rootLabel(root: string): string {
  return root.length <= 3 ? root : root.replace(/\\$/, "");
}
