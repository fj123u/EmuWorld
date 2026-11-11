// Opening a console used to ask the backend for every cover at once, again on
// every visit, and to re-render the whole app for each answer. These tests
// pin the three fixes: covers load when their card comes near the screen,
// answers (and "no cover") are kept in memory, and cover-search events only
// re-render the panel that shows them.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearCoverCache, useLazyCover } from "./coverCache";
import { clearBoxartLogs, pushBoxartLog, useBoxartLogs } from "./boxartLogs";
import { mockBackend } from "./test/tauriMock";

const COVER = "data:image/webp;base64,AAAA";

/** IntersectionObserver the test drives by hand. */
class ManualObserver {
  static instances: ManualObserver[] = [];
  targets: Element[] = [];
  constructor(private callback: IntersectionObserverCallback) {
    ManualObserver.instances.push(this);
  }
  observe(el: Element) { this.targets.push(el); }
  unobserve() {}
  disconnect() { this.targets = []; }
  takeRecords() { return []; }
  static scrollIntoView() {
    for (const o of [...ManualObserver.instances]) {
      if (o.targets.length === 0) continue;
      const entries = o.targets.map((target) => ({ isIntersecting: true, target }) as IntersectionObserverEntry);
      o.callback(entries, o as unknown as IntersectionObserver);
    }
  }
}

const originalObserver = window.IntersectionObserver;

beforeEach(() => {
  clearCoverCache();
  clearBoxartLogs();
  ManualObserver.instances = [];
  window.IntersectionObserver = ManualObserver as unknown as typeof IntersectionObserver;
});

afterEach(() => {
  window.IntersectionObserver = originalObserver;
});

function Card({ name = "Zelda" }: { name?: string }) {
  const { ref, cover, loading, retry } = useLazyCover({ name, console: "NES" });
  return (
    <div ref={ref} data-testid="card">
      {cover ? <img src={cover} alt={name} /> : <span>{loading ? "loading" : "placeholder"}</span>}
      <button onClick={retry}>retry</button>
    </div>
  );
}

describe("useLazyCover", () => {
  it("asks for the cover only once the card comes near the screen", async () => {
    const backend = mockBackend({ fetch_boxart: () => COVER });
    render(<Card />);
    await act(async () => {});
    expect(backend.count("fetch_boxart")).toBe(0);
    expect(screen.getByText("placeholder")).toBeTruthy();

    act(() => ManualObserver.scrollIntoView());
    await waitFor(() => expect(screen.getByRole("img").getAttribute("src")).toBe(COVER));
    expect(backend.count("fetch_boxart")).toBe(1);
  });

  it("shows a cover already loaded without asking the backend again", async () => {
    const backend = mockBackend({ fetch_boxart: () => COVER });
    const first = render(<Card />);
    act(() => ManualObserver.scrollIntoView());
    await waitFor(() => expect(screen.getByRole("img")).toBeTruthy());
    first.unmount();

    render(<Card />);
    // In memory: shown at once, before the card is even near the screen.
    expect(screen.getByRole("img").getAttribute("src")).toBe(COVER);
    act(() => ManualObserver.scrollIntoView());
    await act(async () => {});
    expect(backend.count("fetch_boxart")).toBe(1);
  });

  it("asks once for the same game shown by two cards", async () => {
    const backend = mockBackend({ fetch_boxart: () => COVER });
    render(<><Card /><Card /></>);
    act(() => ManualObserver.scrollIntoView());
    await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(2));
    expect(backend.count("fetch_boxart")).toBe(1);
  });

  it("remembers a game without cover, and searches again on retry", async () => {
    let available = false;
    const backend = mockBackend({
      fetch_boxart: () => {
        if (!available) throw new Error("No boxart found");
        return COVER;
      },
    });
    const first = render(<Card />);
    act(() => ManualObserver.scrollIntoView());
    await waitFor(() => expect(backend.count("fetch_boxart")).toBe(1));
    await waitFor(() => expect(screen.getByText("placeholder")).toBeTruthy());
    first.unmount();

    render(<Card />);
    act(() => ManualObserver.scrollIntoView());
    await act(async () => {});
    expect(backend.count("fetch_boxart")).toBe(1);

    available = true;
    fireEvent.click(screen.getByText("retry"));
    await waitFor(() => expect(screen.getByRole("img").getAttribute("src")).toBe(COVER));
    expect(backend.count("fetch_boxart")).toBe(2);
    expect(backend.calls.filter((c) => c.cmd === "fetch_boxart").pop()?.args).toMatchObject({ forceRefresh: true });
  });
});

describe("boxart logs", () => {
  it("re-render only the component that reads them, and keep the last 50", () => {
    let outsideRenders = 0;
    const Outside = () => { outsideRenders++; return null; };
    const Panel = () => <ul>{useBoxartLogs().map((l, i) => <li key={i}>{l.game}</li>)}</ul>;
    render(<><Outside /><Panel /></>);
    expect(outsideRenders).toBe(1);

    act(() => {
      for (let i = 0; i < 60; i++) pushBoxartLog({ game: `Game ${i}`, url: "Local Cache", status: "Match Found" });
    });
    expect(screen.getAllByRole("listitem")).toHaveLength(50);
    expect(screen.getByText("Game 59")).toBeTruthy();
    expect(screen.queryByText("Game 9")).toBeNull();
    expect(outsideRenders).toBe(1);
  });
});
