// Global test setup: browser APIs jsdom lacks, a Supabase stub, and cleanup.
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";

// ---- Web Storage ------------------------------------------------------------
// Node 25+ has its own localStorage / sessionStorage globals; without
// --localstorage-file they are undefined and hide jsdom's, so every test
// touching storage failed on a recent Node. Use jsdom's (CI runs Node 20).
const dom = (globalThis as { jsdom?: { window: Window } }).jsdom;
for (const name of ["localStorage", "sessionStorage"] as const) {
  if (!globalThis[name] && dom) {
    Object.defineProperty(globalThis, name, { value: dom.window[name], configurable: true, writable: true });
  }
}

// ---- Browser APIs missing from jsdom -------------------------------------
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList;
}
for (const name of ["ResizeObserver", "IntersectionObserver"] as const) {
  if (!(name in window)) {
    (window as unknown as Record<string, unknown>)[name] = class {
      observe() {}
      unobserve() {}
      disconnect() {}
      takeRecords() { return []; }
    };
  }
}
Element.prototype.scrollTo = Element.prototype.scrollTo || function () {};
Element.prototype.scrollIntoView = Element.prototype.scrollIntoView || function () {};
window.scrollTo = () => {};
if (!navigator.getGamepads) {
  Object.defineProperty(navigator, "getGamepads", { value: () => [], configurable: true });
}
HTMLCanvasElement.prototype.getContext = (() => null) as unknown as typeof HTMLCanvasElement.prototype.getContext;

// ---- Supabase: no network in tests ----------------------------------------
// Any chain (supabase.from("x").select().eq()...) returns the same stub, and
// awaiting it yields an empty, error-free result — i.e. "signed out, no data".
function supabaseStub(): unknown {
  // `data: []` (not null) so `const { data: { session } } = await …` and
  // `data.length` / `data.map` all work like an empty response.
  const result = { data: [], error: null, count: 0 };
  const handler: ProxyHandler<() => void> = {
    get(_target, prop) {
      if (prop === "then") return (resolve: (v: unknown) => void) => resolve(result);
      return proxy;
    },
    apply() {
      return proxy;
    },
  };
  const proxy: unknown = new Proxy(function () {}, handler);
  return proxy;
}
vi.mock("../supabase", () => ({ supabase: supabaseStub() }));

afterEach(async () => {
  cleanup();
  // The app unsubscribes with `unlisten.then(fn => fn())`: let those
  // promises run while the event mock still exists, then remove it.
  await new Promise((r) => setTimeout(r, 0));
  clearMocks();
  localStorage.clear();
});
