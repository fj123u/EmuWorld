import { describe, expect, it, vi } from "vitest";
import {
  everyoneReady,
  findLobbyRom,
  isLobbyOpen,
  isValidHostCode,
  LOBBY_MAX_AGE_MS,
  parseNetplayCode,
  pickLobbyEmulator,
  shouldAutoJoin,
  startHostNetplay,
  type LobbyLike,
} from "./netplay";

const NOW = Date.parse("2026-11-06T20:00:00Z");
const lobby = (over: Partial<LobbyLike> = {}): LobbyLike => ({
  id: "lobby-1", host_id: "host", game_name: "Pokemon - Version Rubis (France)", game_console: "Game Boy Advance",
  status: "waiting", netplay_code: null, created_at: new Date(NOW - 60_000).toISOString(), ...over,
});
const relay = { ip: "europe-west1.relay.retroarch.com", port: 55435, session: "g3kjejYF3wcqOkMh" };

describe("which copy of the game a player uses", () => {
  const roms = [
    { name: "Pokemon - Version Rubis (Europe)", console: "Game Boy Advance" },
    { name: "Pokemon - Version Rubis (France)", console: "Game Boy Advance" },
    { name: "Pokemon - Version Rubis (France)", console: "NES" },
  ];

  it("prefers the same file, then the same title from another region", () => {
    expect(findLobbyRom(roms, lobby())).toBe(roms[1]);
    expect(findLobbyRom([roms[0]], lobby())).toBe(roms[0]);
  });

  it("never takes a game from another console or another title", () => {
    expect(findLobbyRom([roms[2]], lobby())).toBeUndefined();
    expect(findLobbyRom([{ name: "Pokemon - Version Saphir (France)", console: "Game Boy Advance" }], lobby())).toBeUndefined();
  });
});

describe("lobby state", () => {
  it("drops closed and abandoned lobbies", () => {
    expect(isLobbyOpen(lobby(), NOW)).toBe(true);
    expect(isLobbyOpen(lobby({ status: "closed" }), NOW)).toBe(false);
    expect(isLobbyOpen(lobby({ created_at: new Date(NOW - LOBBY_MAX_AGE_MS - 1).toISOString() }), NOW)).toBe(false);
    expect(isLobbyOpen(lobby({ created_at: "garbage" }), NOW)).toBe(false);
  });

  it("needs two ready players", () => {
    expect(everyoneReady([{ is_ready: true }])).toBe(false);
    expect(everyoneReady([{ is_ready: true }, { is_ready: false }])).toBe(false);
    expect(everyoneReady([{ is_ready: true }, { is_ready: true }])).toBe(true);
  });

  it("reads the published game, and ignores anything else", () => {
    expect(parseNetplayCode(JSON.stringify({ method: "retroarch", relay }))).toEqual({ method: "retroarch", relay });
    expect(parseNetplayCode("ABCD-1234")).toBeNull();
    expect(parseNetplayCode(null)).toBeNull();
  });

  it("makes each friend join once, only after the host published", () => {
    const published = lobby({ status: "playing", netplay_code: JSON.stringify({ method: "retroarch", relay }) });
    expect(shouldAutoJoin(published, "friend", new Set())).toBe(true);
    expect(shouldAutoJoin(published, "host", new Set()), "the host never joins its own game").toBe(false);
    expect(shouldAutoJoin(published, "friend", new Set(["lobby-1"])), "no second launch").toBe(false);
    expect(shouldAutoJoin(lobby({ status: "ready" }), "friend", new Set())).toBe(false);
    expect(shouldAutoJoin(lobby({ status: "playing" }), "friend", new Set()), "no session yet").toBe(false);
  });
});

describe("starting the host's game", () => {
  it("launches RetroArch, waits for its relay room, then publishes it", async () => {
    const order: string[] = [];
    const launch = vi.fn(async () => { order.push("launch"); return { method: "retroarch", nick: "EW-fj123u-3f2a91" }; });
    const findSession = vi.fn(async (nick: string) => { order.push(`find ${nick}`); return relay; });
    const publish = vi.fn(async () => { order.push("publish"); });
    const code = await startHostNetplay({ launch, findSession, publish });
    expect(order).toEqual(["launch", "find EW-fj123u-3f2a91", "publish"]);
    expect(publish).toHaveBeenCalledWith({ method: "retroarch", host_nick: "EW-fj123u-3f2a91", relay });
    expect(code.relay).toEqual(relay);
  });

  it("publishes right away for emulators without a relay", async () => {
    const findSession = vi.fn();
    const publish = vi.fn(async () => {});
    await startHostNetplay({ launch: async () => ({ method: "ppsspp", nick: null }), findSession, publish });
    expect(findSession).not.toHaveBeenCalled();
    expect(publish).toHaveBeenCalledWith({ method: "ppsspp" });
    // Ryubing: everyone joins the lobby's private LDN room, nothing to wait for.
    await startHostNetplay({ launch: async () => ({ method: "ryubing", nick: null }), findSession, publish });
    expect(publish).toHaveBeenLastCalledWith({ method: "ryubing" });
  });

  it("waits for the host's code with Dolphin and Azahar", async () => {
    const publish = vi.fn(async () => {});
    for (const method of ["dolphin", "azahar"]) {
      const code = await startHostNetplay({ launch: async () => ({ method, nick: null }), findSession: vi.fn(), publish });
      expect(code).toEqual({ method });
    }
    expect(publish).not.toHaveBeenCalled();
  });

  it("accepts only codes the emulators can use", () => {
    expect(isValidHostCode("dolphin", "Ab12Cd34")).toBe(true);
    expect(isValidHostCode("dolphin", "ab")).toBe(false);
    expect(isValidHostCode("dolphin", "Ab12 Cd34")).toBe(false);
    expect(isValidHostCode("azahar", "82.64.1.2:24872")).toBe(true);
    expect(isValidHostCode("azahar", "Salon de Léo")).toBe(true);
    expect(isValidHostCode("azahar", "x".repeat(101))).toBe(false);
    expect(isValidHostCode("azahar", "a\nb")).toBe(false);
    expect(isValidHostCode("retroarch", "anything")).toBe(false);
  });

  it("does not publish a game nobody can join", async () => {
    const publish = vi.fn(async () => {});
    await expect(
      startHostNetplay({
        launch: async () => ({ method: "retroarch", nick: "EW-a-b" }),
        findSession: async () => { throw new Error("La partie de l'hôte n'est pas apparue"); },
        publish,
      }),
    ).rejects.toThrow(/pas apparue/);
    expect(publish).not.toHaveBeenCalled();
  });
});

describe("pickLobbyEmulator", () => {
  const catalog = [
    { id: "duckstation", console: "PlayStation 1" },
    { id: "mesen", console: "NES" },
    { id: "retroarch-nes", console: "NES" },
    { id: "pcsx2", console: "PlayStation 2" },
  ];
  const online = ["mesen", "retroarch-nes"];

  it("offers no lobby for a console nothing plays online", () => {
    expect(pickLobbyEmulator(catalog, ["pcsx2"], online, "PlayStation 2")).toBeNull();
    expect(pickLobbyEmulator(catalog, ["duckstation"], online, "PlayStation 1")).toBeNull();
  });

  it("uses an installed emulator that plays online, never one that does not", () => {
    expect(pickLobbyEmulator(catalog, ["retroarch-nes"], online, "NES")).toEqual({ emulator: catalog[2] });
    expect(pickLobbyEmulator(catalog, ["mesen", "retroarch-nes"], online, "NES")).toEqual({ emulator: catalog[1] });
  });

  it("lists the emulators to install when none is installed", () => {
    expect(pickLobbyEmulator(catalog, ["pcsx2"], online, "NES")).toEqual({ missing: [catalog[1], catalog[2]] });
  });
});
