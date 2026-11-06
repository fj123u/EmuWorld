// The app's own version comes from Tauri (tauri.conf.json), never from a
// constant in the code: the titlebar showed "v2.5.3" in 2.6.1.

/** Compares "2.6.1" and "2.10.0" numerically (text comparison says "2.10.0" < "2.9"). */
export function compareVersions(a: string, b: string): number {
  const pa = a.split(/[.-]/).map((n) => parseInt(n, 10) || 0);
  const pb = b.split(/[.-]/).map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

/** Whether an announcement targets this version (min/max inclusive, both optional). */
export function versionInRange(version: string, min?: string | null, max?: string | null): boolean {
  if (min && compareVersions(version, min) < 0) return false;
  if (max && compareVersions(version, max) > 0) return false;
  return true;
}
