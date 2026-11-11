// Lobby → online game. The host starts first; for RetroArch, EmuWorld then
// finds the host's room on libretro's relay and publishes its session in the
// lobby (`netplay_code`). Friends start when the lobby turns "playing" and
// connect to that session. Backend side: src-tauri/src/netplay.rs.
import { normalizeGameTitle } from "./savestateSharing";

export interface RelaySession {
  ip: string;
  port: number;
  session: string;
}

/** What the host publishes in `lobbies.netplay_code` (JSON). */
export interface NetplayCode {
  /** "retroarch" (relay), "ppsspp", "dolphin", "azahar" or "ryubing" (LDN, private room of the lobby). */
  method: string;
  relay?: RelaySession | null;
  host_nick?: string | null;
  /** Dolphin NetPlay host code, or Azahar room (name or address), shared by the host. */
  host_code?: string | null;
}

/** Emulators where the host shares a code from the emulator before friends can join. */
export const MANUAL_CODE_METHODS = ["dolphin", "azahar"] as const;

export function needsHostCode(method: string): boolean {
  return (MANUAL_CODE_METHODS as readonly string[]).includes(method);
}

/** What the host may paste: a Dolphin traversal code, or an Azahar room name / address. */
export function isValidHostCode(method: string, code: string): boolean {
  const value = code.trim();
  if (method === "dolphin") return /^[A-Za-z0-9]{4,16}$/.test(value);
  if (method === "azahar") return value.length > 0 && value.length <= 100 && !/[\u0000-\u001f]/.test(value);
  return false;
}

export interface NetplayLaunch {
  method: string;
  nick: string | null;
}

export interface LobbyLike {
  id: string;
  host_id: string;
  game_name: string;
  game_console: string;
  status: string;
  netplay_code: string | null;
  created_at: string;
}

/** A lobby older than this is considered abandoned (host quit without closing it). */
export const LOBBY_MAX_AGE_MS = 6 * 60 * 60 * 1000;

export function parseNetplayCode(raw: string | null | undefined): NetplayCode | null {
  if (!raw) return null;
  try {
    const code = JSON.parse(raw) as NetplayCode;
    return typeof code?.method === "string" ? code : null;
  } catch {
    return null;
  }
}

/** Still worth showing: not closed and not abandoned. */
export function isLobbyOpen(lobby: Pick<LobbyLike, "status" | "created_at">, now: number = Date.now()): boolean {
  if (lobby.status === "closed") return false;
  const created = Date.parse(lobby.created_at);
  return !Number.isNaN(created) && now - created < LOBBY_MAX_AGE_MS;
}

/**
 * The player's copy of the lobby's game: the same file name if they have it,
 * otherwise the same title from another region/revision ("(USA)" vs "(Europe)").
 */
export function findLobbyRom<R extends { name: string; console: string }>(
  roms: R[],
  lobby: Pick<LobbyLike, "game_name" | "game_console">,
): R | undefined {
  const sameConsole = roms.filter((r) => r.console === lobby.game_console);
  const exact = sameConsole.find((r) => r.name === lobby.game_name);
  if (exact) return exact;
  const title = normalizeGameTitle(lobby.game_name);
  return sameConsole.find((r) => normalizeGameTitle(r.name) === title);
}

/**
 * Emulator a lobby uses for a console: the first installed one that plays
 * online, otherwise those to install (`missing`). `null`: nothing plays that
 * console online, so no lobby is offered for its games. `netplayIds` comes
 * from the backend (netplay_emulators).
 */
export type LobbyEmulator<E> = { emulator: E; missing?: undefined } | { emulator?: undefined; missing: E[] };

export function pickLobbyEmulator<E extends { id: string; console: string }>(
  catalog: readonly E[],
  installed: readonly string[],
  netplayIds: readonly string[],
  console: string,
): LobbyEmulator<E> | null {
  const online = catalog.filter((e) => e.console === console && netplayIds.includes(e.id));
  if (online.length === 0) return null;
  const emulator = online.find((e) => installed.includes(e.id));
  return emulator ? { emulator } : { missing: online };
}

export function everyoneReady(members: { is_ready: boolean }[]): boolean {
  return members.length >= 2 && members.every((m) => m.is_ready);
}

/** A friend joins once, when the host has published the game. */
export function shouldAutoJoin(lobby: LobbyLike, userId: string, alreadyJoined: ReadonlySet<string>): boolean {
  return (
    lobby.status === "playing" &&
    lobby.host_id !== userId &&
    !alreadyJoined.has(lobby.id) &&
    parseNetplayCode(lobby.netplay_code) !== null
  );
}

export interface HostNetplayDeps {
  launch: (role: "host") => Promise<NetplayLaunch>;
  findSession: (nick: string) => Promise<RelaySession>;
  publish: (code: NetplayCode) => Promise<void>;
}

/**
 * Host: start the emulator, wait for the relay room (RetroArch), then publish
 * it. Dolphin/Azahar are not published yet: the host first pastes the code
 * the emulator shows (see `needsHostCode`).
 */
export async function startHostNetplay(deps: HostNetplayDeps): Promise<NetplayCode> {
  const launch = await deps.launch("host");
  const code: NetplayCode = { method: launch.method };
  if (needsHostCode(launch.method)) return code;
  if (launch.method === "retroarch") {
    if (!launch.nick) throw new Error("RetroArch did not report its netplay nickname");
    code.host_nick = launch.nick;
    code.relay = await deps.findSession(launch.nick);
  }
  await deps.publish(code);
  return code;
}
