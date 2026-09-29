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
