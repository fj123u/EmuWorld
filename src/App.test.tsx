// End-to-end-ish tests of the whole UI against the fake backend in
// src/test/tauriMock.ts. They catch the regressions a monolithic App.tsx is
// prone to: a page that crashes on render, an effect that loops, duplicate
// React keys, a backend command called with the wrong arguments.
import { Component, type ReactNode } from "react";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import App, { CoverImage } from "./App";
import { mockBackend, type Handler } from "./test/tauriMock";

class Boundary extends Component<{ onError: (e: Error) => void; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: Error) {
    this.props.onError(error);
  }
  render() {
    return this.state.failed ? <div data-testid="crashed" /> : this.props.children;
  }
}

/** React problems that are bugs even when nothing visibly crashes. */
const REACT_BUG_PATTERNS = [
  /Encountered two children with the same key/,
  /Maximum update depth exceeded/,
  /Each child in a list should have a unique "key"/,
  /Cannot update a component .* while rendering a different component/,
];

let consoleErrors: string[] = [];
let renderErrors: Error[] = [];

beforeEach(() => {
  consoleErrors = [];
  renderErrors = [];
  localStorage.setItem("emuworld_onboarding_done", "true");
  localStorage.setItem("emuworld-locale", "fr");
  vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
    consoleErrors.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(renderErrors.map((e) => e.stack ?? String(e))).toEqual([]);
  const reactBugs = consoleErrors.filter((m) => REACT_BUG_PATTERNS.some((p) => p.test(m)));
  expect(reactBugs).toEqual([]);
});

async function renderApp(overrides: Record<string, Handler> = {}) {
  const backend = mockBackend(overrides);
  const utils = render(
    <Boundary onError={(e) => renderErrors.push(e)}>
      <App />
    </Boundary>,
  );
  // Startup is done once the library has been scanned.
  await waitFor(() => expect(backend.count("scan_roms")).toBeGreaterThan(0));
  return { backend, ...utils };
}

function sidebarButtons(container: HTMLElement) {
  const nav = container.querySelector(".sidebar__nav");
  if (!nav) throw new Error("sidebar not found");
  return [...nav.querySelectorAll("button")].filter((b) => b.textContent?.trim());
}

describe("App", () => {
  it("starts and loads the library from the backend", async () => {
    const { backend, container } = await renderApp();
    expect(screen.queryByTestId("crashed")).toBeNull();
    // The library scan uses the configured ROMs folder.
    const scan = backend.calls.find((c) => c.cmd === "scan_roms");
    expect(scan?.args).toMatchObject({ directory: "D:\\Roms" });
    expect(sidebarButtons(container).length).toBeGreaterThan(5);
  });

  it("opens every sidebar page without crashing", async () => {
    const { container, backend } = await renderApp();
    const labels = sidebarButtons(container).map((b) => b.textContent!.trim());
    // Guard against the test silently visiting nothing if the sidebar markup changes.
    expect(labels.length).toBeGreaterThanOrEqual(12);
    for (const label of labels) {
      const button = sidebarButtons(container).find((b) => b.textContent!.trim() === label)!;
      await act(async () => {
        fireEvent.click(button);
      });
      // Let effects (data loads) of the new page settle.
      await act(async () => {
        await new Promise((r) => setTimeout(r, 50));
      });
      expect(screen.queryByTestId("crashed"), `page "${label}" crashed`).toBeNull();
      const main = container.querySelector("main.main-content");
      expect(main?.textContent?.trim().length, `page "${label}" rendered nothing`).toBeGreaterThan(0);
    }
    // Commands answered with `null` because tauriMock.ts has no handler.
    // Not a failure, but a hint the mock should learn the real return shape.
    if (backend.unknown.size > 0) {
      process.stdout.write(`[tauriMock] unmocked commands: ${[...backend.unknown].join(", ")}\n`);
    }
  });

  it("renders an empty library without crashing", async () => {
    await renderApp({ scan_roms: () => [], get_playtime: () => ({ games: {}, emulators: {}, collections: [] }) });
    expect(screen.queryByTestId("crashed")).toBeNull();
  });

  it("shows hostile ROM names as text, never as HTML", async () => {
    const { container } = await renderApp();
    fireEvent.click(sidebarButtons(container).find((b) => b.textContent?.includes("Roms"))!);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });
    expect(container.querySelector("main b")).toBeNull();
    expect(container.querySelector("script")).toBeNull();
  });

  it("survives a backend that fails every command", async () => {
    const failing = mockBackend({
      get_config: () => { throw new Error("backend down"); },
      scan_roms: () => { throw new Error("backend down"); },
      get_playtime: () => { throw new Error("backend down"); },
    });
    render(
      <Boundary onError={(e) => renderErrors.push(e)}>
        <App />
      </Boundary>,
    );
    await waitFor(() => expect(failing.count("get_config")).toBeGreaterThan(0));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 100));
    });
    expect(screen.queryByTestId("crashed")).toBeNull();
  });
});

describe("Vimm store download", () => {
  const vimmGame = { id: "12345", name: "Zelda Test", console: "NES", size: "1 MB", region: "Europe" };

  async function openVimmResults(overrides: Record<string, Handler>) {
    const r = await renderApp({ search_vimm: () => [vimmGame], ...overrides });
    fireEvent.click(sidebarButtons(r.container).find((b) => b.textContent?.includes("Store"))!);
    const search = await screen.findByPlaceholderText(/Vimm/i);
    fireEvent.change(search, { target: { value: "zelda" } });
    const row = await screen.findByText("Zelda Test", {}, { timeout: 3000 });
    return { ...r, row: row.closest("div")!.parentElement! };
  }

  it("starts one download even on a fast double click", async () => {
    let finish: (v: string) => void = () => {};
    const { backend } = await openVimmResults({
      download_vimm_rom: () => new Promise<string>((r) => { finish = r; }),
    });
    const button = screen.getByRole("button", { name: /Télécharger/ });
    // Same tick, before any re-render: the old stale-closure guard let all
    // three through.
    fireEvent.click(button);
    fireEvent.click(button);
    fireEvent.click(button);
    await waitFor(() => expect(backend.count("download_vimm_rom")).toBe(1));
    expect(backend.calls.find((c) => c.cmd === "download_vimm_rom")?.args).toMatchObject({
      gameId: "12345",
      gameName: "Zelda Test",
    });
    await act(async () => finish("ok"));
  });

  it("shows extraction instead of freezing at 99%", async () => {
    let finish: (v: string) => void = () => {};
    await openVimmResults({
      download_vimm_rom: () => new Promise<string>((r) => { finish = r; }),
    });
    fireEvent.click(screen.getByRole("button", { name: /Télécharger/ }));

    await act(async () => {
      await emit("vimm-download-progress", { game_id: "12345", status: "downloading", progress: 99, downloaded_bytes: 99, total_bytes: 100, speed_bps: 10, eta: 1 });
    });
    expect(screen.getAllByText(/99%/).length).toBeGreaterThan(0);

    await act(async () => {
      await emit("vimm-download-progress", { game_id: "12345", status: "extracting", progress: 99 });
    });
    expect(screen.getAllByText(/Extraction/).length).toBeGreaterThan(0);

    await act(async () => finish("ok"));
    await waitFor(() => expect(screen.getByRole("button", { name: /Télécharger/ })).toBeTruthy());
  });

  it("reports a failed download and lets the user retry", async () => {
    let attempts = 0;
    const { backend } = await openVimmResults({
      download_vimm_rom: () => {
        attempts++;
        throw "Vimm download HTTP 503";
      },
    });
    fireEvent.click(screen.getByRole("button", { name: /Télécharger/ }));
    await screen.findByText(/HTTP 503/);
    fireEvent.click(await screen.findByRole("button", { name: /Télécharger/ }));
    await waitFor(() => expect(backend.count("download_vimm_rom")).toBe(2));
    expect(attempts).toBe(2);
  });
});

describe("CoverImage", () => {
  function loadWith(img: HTMLImageElement, width: number, height: number) {
    Object.defineProperty(img, "naturalWidth", { value: width, configurable: true });
    Object.defineProperty(img, "naturalHeight", { value: height, configurable: true });
    fireEvent.load(img);
  }

  it("shows square art (iiDB) whole over a blurred backdrop", () => {
    const { container } = render(<CoverImage src="data:image/webp;base64,AA" alt="Super Mario Bros. 3" />);
    loadWith(screen.getByAltText("Super Mario Bros. 3") as HTMLImageElement, 1024, 1024);
    expect(screen.getByAltText("Super Mario Bros. 3").className).toContain("cover--contain");
    expect(container.querySelector("img.cover-backdrop")).not.toBeNull();
  });

  it("leaves portrait box art as it was", () => {
    const { container } = render(<CoverImage src="data:image/webp;base64,AA" alt="Metroid Prime" />);
    loadWith(screen.getByAltText("Metroid Prime") as HTMLImageElement, 600, 800);
    expect(screen.getByAltText("Metroid Prime").className).not.toContain("cover--contain");
    expect(container.querySelector("img.cover-backdrop")).toBeNull();
  });
});

describe("toasts", () => {
  it("keeps two toasts fired in the same millisecond", async () => {
    vi.spyOn(Date, "now").mockReturnValue(1_800_000_000_000);
    await renderApp();
    await act(async () => {
      await emit("roms-detected", ["Game A.nes"]);
      await emit("roms-detected", ["Game B.nes"]);
    });
    expect(screen.getByText(/Game A\.nes/)).toBeTruthy();
    expect(screen.getByText(/Game B\.nes/)).toBeTruthy();
  });
});
