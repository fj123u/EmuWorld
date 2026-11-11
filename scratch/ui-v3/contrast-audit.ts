// Dev-only contrast audit for the v3 redesign (used by app-preview.tsx).
// window.__audit(): visible texts whose contrast with the background behind
// them is under 3:1 (texts on images or gradients are skipped).
// window.__auditAll(themes, pages): runs it on every page of every theme and
// stores the result in window.__auditResult (window.__auditDone = true).
type Rgba = { r: number; g: number; b: number; a: number };

function parse(c: string): Rgba | null {
  let m = c.match(/rgba?\(([^)]+)\)/);
  if (m) {
    const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  }
  m = c.match(/color\(srgb ([^)]+)\)/);
  if (m) {
    const p = m[1].split(/[ /]+/).filter(Boolean).map(Number);
    return { r: p[0] * 255, g: p[1] * 255, b: p[2] * 255, a: p.length > 3 ? p[3] : 1 };
  }
  return null;
}
const lum = ({ r, g, b }: Rgba) => {
  const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};
const blend = (t: Rgba, b: Rgba): Rgba => ({ r: t.r * t.a + b.r * (1 - t.a), g: t.g * t.a + b.g * (1 - t.a), b: t.b * t.a + b.b * (1 - t.a), a: 1 });

function bgOf(el: Element): Rgba | null {
  const layers: Rgba[] = [];
  for (let e: Element | null = el; e; e = e.parentElement) {
    const cs = getComputedStyle(e);
    if (cs.backgroundImage !== "none" && e !== document.body) return null;
    const c = parse(cs.backgroundColor);
    if (c && c.a > 0) { layers.push(c); if (c.a >= 1) break; }
  }
  let base = parse(getComputedStyle(document.body).backgroundColor)!;
  for (let i = layers.length - 1; i >= 0; i--) base = blend(layers[i], base);
  return base;
}

export function audit(): string[] {
  const out: string[] = [];
  const seen = new Set<Element>();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const txt = walker.currentNode.textContent?.trim() ?? "";
    const el = walker.currentNode.parentElement;
    if (txt.length < 2 || !el || seen.has(el)) continue;
    seen.add(el);
    const rect = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    if (!rect.width || !rect.height || cs.visibility === "hidden" || +cs.opacity === 0) continue;
    const fg = parse(cs.color);
    const bg = bgOf(el);
    if (!fg || !bg) continue;
    const f = blend(fg, bg);
    const ratio = (Math.max(lum(f), lum(bg)) + 0.05) / (Math.min(lum(f), lum(bg)) + 0.05);
    const cls = typeof el.className === "string" && el.className ? el.className.split(" ")[0] : el.tagName;
    if (ratio < 3) out.push(`${ratio.toFixed(2)} | ${cls} | ${txt.slice(0, 40)}`);
  }
  return out;
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function auditAll(themes: (string | null)[], pages: string[]) {
  const w = window as unknown as Record<string, unknown>;
  w.__auditDone = false;
  const result: Record<string, Record<string, string[]>> = {};
  for (const th of themes) {
    if (th) document.documentElement.setAttribute("data-theme", th);
    else document.documentElement.removeAttribute("data-theme");
    const res: Record<string, string[]> = {};
    for (const label of pages) {
      const btn = [...document.querySelectorAll<HTMLElement>("[data-nav-page], .sidebar__item, .v3-dock__item, .v3-tabs__item")]
        .find((b) => b.getAttribute("data-nav-page") === label || b.textContent?.trim().replace(/\d+$/, "").trim() === label);
      btn?.click();
      await wait(500);
      const r = audit();
      if (r.length) res[label] = r;
    }
    result[th ?? "light"] = res;
  }
  w.__auditResult = result;
  w.__auditDone = true;
}
