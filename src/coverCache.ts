// Covers shown by the game cards. Loading one costs a disk read, a base64
// round-trip through the IPC bridge and an image decode, so:
// - a cover is only asked for once its card comes near the screen;
// - the answer is kept in memory, so going back to a console (or remounting a
//   card) does not ask the backend again;
// - a game with no cover anywhere is remembered too, instead of replaying the
//   whole network search every time its card is shown.
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { COVER_CHANGED_EVENT, type CoverChangedDetail } from "./CoverPicker";

/** Covers kept in memory (data URLs, ~70 KB each): oldest dropped first. */
const MAX_CACHED = 400;
/** How far outside the screen a card starts loading its cover. */
const NEAR_SCREEN = "600px";

const cache = new Map<string, string>();
const missing = new Set<string>();
const inflight = new Map<string, Promise<string>>();

const keyOf = (name: string, console: string) => `${console}::${name}`;

function remember(key: string, dataUrl: string) {
  cache.delete(key);
  cache.set(key, dataUrl);
  missing.delete(key);
  if (cache.size > MAX_CACHED) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
}

/** The cover already in memory for a game, if any. */
export function cachedCover(name: string, console: string): string | null {
  return cache.get(keyOf(name, console)) ?? null;
}

/** Forget every cover (the user emptied the cover cache, or a test starts). */
export function clearCoverCache() {
  cache.clear();
  missing.clear();
  inflight.clear();
}

/**
 * A game's cover as a data URL. Rejects when there is none.
 * `force` skips and replaces what is in memory (the card's "retry" button).
 */
export function loadCover(name: string, console: string, force = false): Promise<string> {
  const key = keyOf(name, console);
  if (force) {
    cache.delete(key);
    missing.delete(key);
  } else {
    const hit = cache.get(key);
    if (hit) return Promise.resolve(hit);
    if (missing.has(key)) return Promise.reject(new Error("no cover"));
    const pending = inflight.get(key);
    if (pending) return pending;
  }
  const request: Promise<string> = invoke<string | null>("fetch_boxart", { gameName: name, console, forceRefresh: force })
    .then((dataUrl) => {
      if (!dataUrl) throw new Error("no cover");
      remember(key, dataUrl);
      return dataUrl;
    })
    .catch((e) => {
      missing.add(key);
      throw e;
    })
    .finally(() => {
      if (inflight.get(key) === request) inflight.delete(key);
    });
  inflight.set(key, request);
  return request;
}

// A cover picked in "Choisir la cover" replaces the one in memory.
if (typeof window !== "undefined") {
  window.addEventListener(COVER_CHANGED_EVENT, (e) => {
    const d = (e as CustomEvent<CoverChangedDetail>).detail;
    if (d?.dataUrl) remember(keyOf(d.name, d.console), d.dataUrl);
  });
}

/**
 * Cover of a card, loaded when the card comes near the screen.
 * Put `ref` on the card's root element.
 */
export function useLazyCover(rom: { name: string; console: string }) {
  const [cover, setCover] = useState<string | null>(() => cachedCover(rom.name, rom.console));
  const [loading, setLoading] = useState(false);
  const [near, setNear] = useState(false);
  const node = useRef<Element | null>(null);
  const observer = useRef<IntersectionObserver | null>(null);

  const ref = useCallback((el: Element | null) => {
    observer.current?.disconnect();
    observer.current = null;
    node.current = el;
    if (!el) return;
    if (typeof IntersectionObserver === "undefined") {
      setNear(true);
      return;
    }
    observer.current = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setNear(true);
          observer.current?.disconnect();
        }
      },
      { rootMargin: NEAR_SCREEN },
    );
    observer.current.observe(el);
  }, []);

  const load = useCallback(
    (force = false) => {
      let alive = true;
      setLoading(true);
      loadCover(rom.name, rom.console, force)
        .then((dataUrl) => { if (alive) setCover(dataUrl); })
        .catch(() => { if (alive && force) setCover(null); })
        .finally(() => { if (alive) setLoading(false); });
      return () => { alive = false; };
    },
    [rom.name, rom.console],
  );

  // Another game in the same card (the list was filtered or sorted).
  useEffect(() => {
    setCover(cachedCover(rom.name, rom.console));
  }, [rom.name, rom.console]);

  useEffect(() => {
    if (!near || cachedCover(rom.name, rom.console)) return;
    return load();
  }, [near, load, rom.name, rom.console]);

  const retry = useCallback(() => { load(true); }, [load]);

  return { ref, cover, setCover, loading, retry };
}
