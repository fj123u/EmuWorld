// Fake Rust backend for frontend tests.
//
// Every response mirrors the *real* shape returned by the matching
// `#[tauri::command]` in src-tauri/src/lib.rs. Returning `null` where Rust
// returns a Vec/struct makes the UI crash in ways the real app never does,
// so when you add a command that runs on startup or on a page, add it here.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

export type Handler = (args: Record<string, unknown> | undefined) => unknown;

const daysAgo = (d: number) => new Date(Date.now() - d * 86_400_000).toISOString();
const today = new Date();
const monthKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}`;
const dayKey = `${monthKey}-${String(today.getDate()).padStart(2, "0")}`;

export const sampleRoms = [
  { name: "Super Mario Bros. 3 (Europe)", path: "D:\\Roms\\NES\\smb3.nes", console: "NES", emulator_id: "mesen", extension: "nes" },
  { name: "Tetris", path: "D:\\Roms\\Game Boy\\tetris.gb", console: "Game Boy Advance", emulator_id: "mgba", extension: "gb" },
  { name: "Halo 2 (France) (En,Fr).xiso", path: "D:\\Roms\\Xbox\\halo2.xiso.iso", console: "Xbox", emulator_id: "xemu", extension: "iso" },
  // Hostile-looking names must render as plain text.
  { name: "ポケモン 🎮 <b>bold</b> & \"quotes\"", path: "D:\\Roms\\GBA\\jp.gba", console: "Game Boy Advance", emulator_id: "mgba", extension: "gba" },
];

export const sampleCatalog = [
  { id: "mesen", name: "Mesen", console: "NES", description: "NES emulator", download_url: "", executable_name: "Mesen.exe", supported_extensions: ["nes"], icon: "", website: "", archive_type: "zip", category: "Nintendo" },
  { id: "mgba", name: "mGBA", console: "Game Boy Advance", description: "GBA emulator", download_url: "", executable_name: "mGBA.exe", supported_extensions: ["gba", "gb", "gbc"], icon: "", website: "", archive_type: "zip", category: "Nintendo" },
  { id: "xemu", name: "xemu", console: "Xbox", description: "Xbox emulator", download_url: "", executable_name: "xemu.exe", supported_extensions: ["iso"], icon: "", website: "", archive_type: "zip", category: "Microsoft" },
];

export const samplePlaytime = {
  games: {
    "NES::Super Mario Bros. 3 (Europe)": {
      console: "NES", name: "Super Mario Bros. 3 (Europe)", seconds: 36_000, launches: 12,
      last_played: daysAgo(0), first_played: daysAgo(40), favorite: true, last_emulator_id: "mesen",
      rating: 5, notes: "Monde 8 <script>alert(1)</script>",
      monthly: { [monthKey]: { seconds: 5_400, launches: 2, longest_session: 3_600 } },
      active_days: [dayKey], longest_session: 7_200,
    },
    // A game that is no longer on disk, with a corrupt date: must not crash.
    "NES::Deleted Game": {
      console: "NES", name: "Deleted Game", seconds: 60, launches: 1,
      last_played: "not-a-date", first_played: null, favorite: false, rating: null, notes: null,
    },
  },
  emulators: { mesen: 36_000, mgba: 60 },
  collections: [{ name: "Backlog", games: ["NES::Super Mario Bros. 3 (Europe)", "NES::Missing"] }],
};

/** Responses keyed by command name, matching the Rust return types. */
export function defaultHandlers(): Record<string, Handler> {
  return {
    // Config & library
    get_config: () => ({ roms_directory: "D:\\Roms", emulators_directory: "C:\\Emu", covers_directory: "C:\\Covers", bandwidth_limit_kbps: 0 }),
    is_portable_mode: () => false,
    get_emulator_catalog: () => sampleCatalog,
    get_installed_emulators: () => ["mesen", "mgba"],
    scan_roms: () => sampleRoms,
    watch_roms_directory: () => null,
    check_roms_health: () => [],
    check_emulator_updates: () => [],
    fetch_boxart: () => null,
    get_cover_url: () => null,
    // Playtime & profile
    get_playtime: () => samplePlaytime,
    get_profile_stats: () => ({
      total_seconds: 36_060, total_launches: 13, games_played: 2, favorite_count: 1,
      most_played: null, favorite_game: null, top_games: [], top_emulator_id: "mesen",
      top_console: "NES", top_console_seconds: 36_060, first_played: null, streak_days: 1,
    }),
    get_current_playing: () => [null, null],
    set_current_playing: () => null,
    // Achievements
    get_achievements: () => [],
    get_achievement_rank: () => ({}),
    check_achievements: () => [],
    // Store
    get_store_consoles: () => [],
    get_featured_games: () => [],
    get_rgs_constructeurs: () => [],
    get_vimm_consoles: () => [{ id: "NES", name: "NES", target_console: "NES", manufacturer: "Nintendo" }],
    search_vimm: () => [],
    browse_vimm: () => [],
    // Integrations
    get_ra_credentials: () => ({ username: "", api_key: "" }),
    get_b2_config: () => ({}),
    scan_local_saves: () => [],
    list_cloud_backups: () => [],
    discord_set_idle: () => null,
    get_logs: () => [],
    get_log_file_path: () => "C:\\logs\\emuworld.log",
    get_logs_directory: () => "C:\\logs",
    get_all_screenshots: () => [],
    get_screenshots: () => [],
    // Save states manager (savestates.rs)
    list_save_states: () => [],
    rename_save_state: () => null,
    duplicate_save_state: () => "C:\\RetroArch\\states\\copy.state1",
    delete_save_state: () => null,
    log_event: () => null,
    // iiDB official API (iidb_api.rs)
    get_iidb_status: () => ({ configured: false, username: null, tier: null, requests_per_second: null }),
    set_iidb_api_key: () => ({ configured: true, username: "fj123u", tier: "standard", requests_per_second: 1 }),
    clear_iidb_api_key: () => null,
    iidb_cover_choices: () => [],
    set_cover_from_url: () => "data:image/webp;base64,NEW",
    toggle_favorite: () => true,
    set_cover_from_file: () => "data:image/webp;base64,FILE",
    "plugin:dialog|open": () => null,
    // Speedrun timer (speedrun.rs)
    get_speedrun: () => ({ splits: [], pb: null, best_segments: [], attempts: 0, completed: 0 }),
    save_speedrun: (args) => (args as { data: unknown }).data,
    open_speedrun_window: () => null,
    close_speedrun_window: () => null,
    // Netplay (netplay.rs)
    launch_netplay: () => ({ method: "retroarch", nick: "EW-test-lobby1" }),
    netplay_emulators: () => ["mesen"],
    find_netplay_session: () => ({ ip: "europe-west1.relay.retroarch.com", port: 55435, session: "g3kjejYF3wcqOkMh" }),
    read_save_state: () => new Uint8Array([0x52, 0x5a, 0x49, 0x50]).buffer, // tauri::ipc::Response → ArrayBuffer
    import_save_state: () => ({
      path: "C:\\RetroArch\\states\\received.state1", file_name: "received.state1", slot: 1, auto: false,
      folder: null, core: null, modified: null, size: 4, thumbnail: null, label: null,
    }),
    launch_emulator: () => "Launched",
    // Tauri plugins
    "plugin:app|version": () => "0.0.0-test",
    "plugin:updater|check": () => null,
    "plugin:window|is_fullscreen": () => false,
    "plugin:window|is_maximized": () => false,
  };
}

export interface BackendMock {
  /** Every command invoked, in order. */
  calls: { cmd: string; args: Record<string, unknown> | undefined }[];
  /** Commands invoked that have no handler (answered with null). */
  unknown: Set<string>;
  count(cmd: string): number;
}

/**
 * Installs the fake backend. `overrides` replace or add handlers;
 * events are mocked, so `emit()` from `@tauri-apps/api/event` reaches the
 * app's `listen()` callbacks.
 */
export function mockBackend(overrides: Record<string, Handler> = {}): BackendMock {
  const handlers = { ...defaultHandlers(), ...overrides };
  const state: BackendMock = {
    calls: [],
    unknown: new Set(),
    count: (cmd) => state.calls.filter((c) => c.cmd === cmd).length,
  };
  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      const args = payload as Record<string, unknown> | undefined;
      state.calls.push({ cmd, args });
      const handler = handlers[cmd];
      if (handler) return handler(args);
      if (!cmd.startsWith("plugin:")) state.unknown.add(cmd);
      return null;
    },
    { shouldMockEvents: true },
  );
  return state;
}
