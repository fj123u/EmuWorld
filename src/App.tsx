import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { motion, AnimatePresence } from "framer-motion";
import {
  Search,
  Settings,
  Download,
  Play,
  Trash2,
  Gamepad2,
  Library,
  Grid3X3,
  FolderOpen,
  RefreshCw,
  Minus,
  Square,
  Maximize2,
  Minimize2,
  X,
  ExternalLink,
  HardDrive,
  ChevronDown,
  ChevronRight,
  ChevronRight as ChevronIcon,
  CheckCircle,
  AlertCircle,
  FileText,
} from "lucide-react";

/* ============================
   Types
   ============================ */
interface EmulatorInfo {
  id: string;
  name: string;
  console: string;
  description: string;
  download_url: string;
  executable_name: string;
  supported_extensions: string[];
  icon: string;
  website: string;
  archive_type: string;
  category: string;
}

interface RomFile {
  name: string;
  path: string;
  console: string;
  extension: string;
  size: number;
  cover?: string;
}

interface AppConfig {
  roms_directory: string;
  emulators_directory: string;
  covers_directory: string;
}

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

interface ChangelogEntry {
  version: string;
  date: string;
  changes: string[];
}

type Page = "catalog" | "library" | "installed" | "settings" | "changelogs";

/* ============================
   Console icon mapping
   ============================ */
const CONSOLE_ICONS: Record<string, string> = {
  "NES": "🔴",
  "Super Nintendo": "🟣",
  "Nintendo 64": "🟡",
  "Game Boy Advance": "🟢",
  "Nintendo DS": "📱",
  "Nintendo Switch": "🎌",
  "GameCube / Wii": "🐬",
  "Wii U": "📽️",
  "Virtual Boy": "🕶️",
  "PlayStation 1": "⚪",
  "PlayStation 2": "🔵",
  "PlayStation 3": "💿",
  "PlayStation Portable": "⬛",
  "Dreamcast": "🌀",
  "Mega Drive": "🎮",
  "Master System": "🕹️",
  "Game Gear": "📟",
  "Saturn": "🪐",
  "Xbox": "❎",
  "DOS / Win 3.x": "💾",
  "Arcade": "🕹️",
  "Neo-Geo": "🅰️",
  "PC Engine": "🔶",
  "Atari 2600": "🟤",
  "WonderSwan": "🔲",
  "Multi-System": "🔄",
};

/* ============================
   Components
   ============================ */

const GameCard = ({ rom, onLaunch }: { rom: RomFile, onLaunch: (rom: RomFile) => void }) => {
  const [cover, setCover] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const fetchCover = async () => {
      try {
        setLoading(true);
        const path: string = await invoke("fetch_boxart", { 
          gameName: rom.name, 
          console: rom.console 
        });
        setCover(convertFileSrc(path));
      } catch (e) {
        // Fallback or silent fail
      } finally {
        setLoading(false);
      }
    };
    fetchCover();
  }, [rom.name, rom.console]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      whileHover={{ scale: 1.02, y: -4 }}
      whileTap={{ scale: 0.98 }}
      className="game-card"
      onClick={() => {
        console.log("GameCard click detected:", rom.name);
        onLaunch(rom);
      }}
    >
      <div className={`game-card__cover ${loading ? 'game-card__cover--loading' : ''}`}>
        {cover ? (
          <img src={cover} alt={rom.name} />
        ) : !loading ? (
          <div className="game-card__placeholder">
            <span className="game-card__placeholder-icon">🎮</span>
            <div className="game-card__placeholder-title">{rom.name}</div>
          </div>
        ) : null}
        <div className="game-card__overlay">
          <Play size={24} fill="currentColor" />
        </div>
      </div>
      <div className="game-card__info">
        <div className="game-card__name">{rom.name}</div>
        <div className="game-card__meta">{rom.console} • {rom.extension.toUpperCase()}</div>
      </div>
    </motion.div>
  );
};

/* ============================
   App Component
   ============================ */
export default function App() {
  const [page, setPage] = useState<Page>("catalog");
  const [catalog, setCatalog] = useState<EmulatorInfo[]>([]);
  const [installed, setInstalled] = useState<string[]>([]);
  const [roms, setRoms] = useState<RomFile[]>([]);
  const [config, setConfig] = useState<AppConfig>({
    roms_directory: "",
    emulators_directory: "",
    covers_directory: "",
  });
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [consoleFilter, setConsoleFilter] = useState<string | null>(null);
  const [expandedCategories, setExpandedCategories] = useState<string[]>([]);
  const [expandedSidebarConsoles, setExpandedSidebarConsoles] = useState<string[]>([]);
  const [expandedLibraryCategories, setExpandedLibraryCategories] = useState<string[]>(["NINTENDO", "SONY", "SEGA", "MICROSOFT"]);
  const [installing, setInstalling] = useState<string[]>([]);
  const [activeLibraryFilter, setActiveLibraryFilter] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [changelogs] = useState<ChangelogEntry[]>([
    { version: "0.3.0", date: "2026-03-19", changes: ["✨ Cover Art! Box art auto-downloaded from libretro-thumbnails CDN", "Per-console cover caching in Covers directory", "22 consoles supported for cover art", "Shimmer loading animation on game cards"] },
    { version: "0.2.9", date: "2026-03-19", changes: ["Renamed 'Library' to 'Roms' in the UI", "Stopped automatic ROM folder creation during emulator installation"] },
    { version: "0.2.3", date: "2026-03-19", changes: ["Fixed Close button (added window control permissions)", "Switched NES emulator to Nestopia UE", "Updated Xbox (xemu), PS3 (RPCS3), and Switch (Ryujinx) links to stable mirrors"] },
    { version: "0.2.0", date: "2026-03-19", changes: ["Added Changelogs tab", "Fixed Game Launch issues", "Restored Fullscreen permissions", "Implemented Smart Box Art fallbacks", "Unified Flat View for Catalog and Library"] },
    { version: "0.1.5", date: "2026-03-18", changes: ["Context-aware Sidebars", "Nested 3-level Hierarchy", "Flattened grids for cleaner UI"] },
    { version: "0.1.0", date: "2026-03-10", changes: ["Initial Beta Launch", "Support for 20+ retro consoles", "Automatic ROM scanning"] }
  ]);

  // ---- Toast helpers ----
  const showToast = useCallback(
    (message: string, type: Toast["type"] = "info") => {
      const id = Date.now();
      setToasts((prev) => [...prev, { id, message, type }]);
      setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 4000);
    },
    []
  );

  // ---- Data loading ----
  const loadData = useCallback(async () => {
    try {
      const [cat, inst, cfg] = await Promise.all([
        invoke<EmulatorInfo[]>("get_emulator_catalog"),
        invoke<string[]>("get_installed_emulators"),
        invoke<AppConfig>("get_config"),
      ]);
      setCatalog(cat);
      setInstalled(inst);
      setConfig(cfg);

      if (cfg.roms_directory) {
        try {
          const r = await invoke<RomFile[]>("scan_roms", {
            directory: cfg.roms_directory,
          });
          setRoms(r);
        } catch {
          // ROMs dir might not exist yet
        }
      }
    } catch (err) {
      console.error("Failed to load data:", err);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // ---- Listen for install progress events ----
  useEffect(() => {
    const unlisten = listen<{ emulator_id: string; status: string }>(
      "install-progress",
      (event) => {
        if (event.payload.status === "done") {
          setInstalling((prev) => prev.filter((id) => id !== event.payload.emulator_id));
          loadData();
          showToast("Emulator installed successfully!", "success");
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadData, showToast]);

  // ---- Actions ----
  const handleInstall = async (id: string) => {
    setInstalling((prev) => [...prev, id]);
    showToast("Downloading emulator...", "info");
    try {
      await invoke("install_emulator", { emulatorId: id });
    } catch (err: any) {
      setInstalling((prev) => prev.filter((i) => i !== id));
      showToast(`Install failed: ${err}`, "error");
    }
  };

  const handleUninstall = async (id: string) => {
    try {
      await invoke("uninstall_emulator", { emulatorId: id });
      showToast("Emulator uninstalled", "success");
      loadData();
    } catch (err: any) {
      showToast(`Uninstall failed: ${err}`, "error");
    }
  };

  const handleLaunch = async (rom: RomFile) => {
    try {
      console.log("handleLaunch triggered for ROM:", rom);
      const emulator = catalog.find(e => e.console === rom.console);
      if (!emulator) {
        console.error("Emulator discovery failure. ROM console:", rom.console, "Catalog consoles:", catalog.map(e => e.console));
        showToast(`No emulator found for ${rom.console}. Check if it's supported!`, "error");
        return;
      }
      console.log("Found Emulator:", emulator.name, "(ID:", emulator.id, ") for console:", rom.console);
      const res: string = await invoke("launch_emulator", {
        emulatorId: emulator.id,
        romPath: rom.path || null,
      });
      console.log("Backend Launch Success:", res);
      showToast(`Launching ${rom.name}...`, "success");
    } catch (err: any) {
      console.error("Launch Exception:", err);
      showToast(`Launch failed: ${err}`, "error");
    }
  };

  const handleSaveConfig = async (newConfig: AppConfig) => {
    try {
      await invoke("save_config", { config: newConfig });
      setConfig(newConfig);
      showToast("Settings saved!", "success");
      loadData();
    } catch (err: any) {
      showToast(`Save failed: ${err}`, "error");
    }
  };

  const handleBrowseFolder = async (field: keyof AppConfig) => {
    const selected = await open({ directory: true });
    if (selected) {
      const newConfig = { ...config, [field]: selected as string };
      handleSaveConfig(newConfig);
    }
  };

  const toggleCategory = (cat: string) => {
    setExpandedCategories(prev =>
      prev.includes(cat) ? prev.filter(c => c !== cat) : [...prev, cat]
    );
    setCategoryFilter(cat);
    setConsoleFilter(null);
    if (page !== "catalog" && page !== "library") setPage("catalog");
  };

  const toggleSidebarConsole = (con: string) => {
    setExpandedSidebarConsoles(prev =>
      prev.includes(con) ? prev.filter(c => c !== con) : [...prev, con]
    );
    setConsoleFilter(con);
    if (page !== "library") setPage("library");
  };

  const toggleLibraryCategory = (cat: string) => {
    setExpandedLibraryCategories(prev =>
      prev.includes(cat) ? prev.filter(c => c !== cat) : [...prev, cat]
    );
  };

  // ---- Derived data ----
  const consolesByCategory = catalog.reduce((acc, emu) => {
    if (!acc[emu.category]) acc[emu.category] = [];
    if (!acc[emu.category].includes(emu.console)) {
      acc[emu.category].push(emu.console);
    }
    return acc;
  }, {} as Record<string, string[]>);

  const installedCount = installed.length;

  const filteredCatalog = catalog.filter((emu) => {
    const matchesSearch =
      !search ||
      emu.name.toLowerCase().includes(search.toLowerCase()) ||
      emu.console.toLowerCase().includes(search.toLowerCase());
    const matchesCategory = !categoryFilter || emu.category === categoryFilter;
    const matchesConsole = !consoleFilter || emu.console === consoleFilter;
    return matchesSearch && matchesCategory && matchesConsole;
  });

  const filteredGames = roms.filter((g) => {
    const matchesSearch = !search || g.name.toLowerCase().includes(search.toLowerCase());
    const emu = catalog.find((e) => e.console === g.console);
    const matchesCategory = !categoryFilter || (emu && emu.category === categoryFilter);
    const matchesConsole = !consoleFilter || g.console === consoleFilter;
    return matchesSearch && matchesCategory && matchesConsole;
  });

  // ---- Window controls ----
  const [appWindow, setAppWindow] = useState<ReturnType<typeof getCurrentWindow> | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    try {
      const win = getCurrentWindow();
      setAppWindow(win);
      win.isFullscreen().then(setIsFullscreen).catch(() => {});
      win.isMaximized().then(setIsMaximized).catch(() => {});
    } catch {
      setAppWindow(null);
    }
  }, []);

  const minimize = () => appWindow?.minimize();
  const maximize = async () => {
    if (!appWindow) return;
    await appWindow.toggleMaximize();
    setIsMaximized(await appWindow.isMaximized());
  };
  const close = () => appWindow?.close();

  const toggleFullscreen = async () => {
    if (!appWindow) return;
    try {
      const next = !isFullscreen;
      await appWindow.setFullscreen(next);
      setIsFullscreen(next);
    } catch (err) {
      console.error("Fullscreen error:", err);
    }
  };

  return (
    <>
      <div className="titlebar" data-tauri-drag-region onDoubleClick={maximize}>
        <div className="titlebar__logo" data-tauri-drag-region>
          <div className="titlebar__logo-icon">🎮</div>
          <span data-tauri-drag-region>EmuWorld</span>
        </div>
        <div className="titlebar__controls">
          <button className="titlebar__btn" onClick={minimize} title="Réduire"><Minus size={14} /></button>
          <button className="titlebar__btn" onClick={maximize} title="Agrandir / Restaurer">
            {isMaximized ? <Minimize2 size={12} /> : <Square size={12} />}
          </button>
          <button className="titlebar__btn" onClick={toggleFullscreen} title={isFullscreen ? "Quitter le plein écran" : "Plein écran"}>
            {isFullscreen ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
          </button>
          <button className="titlebar__btn titlebar__btn--close" onClick={close} title="Fermer"><X size={14} /></button>
        </div>
      </div>

      <div className="app">
        <aside className="sidebar">
          <div className="sidebar__section">
            <div className="sidebar__label">Navigation</div>
            <button
              className={`sidebar__item ${page === "catalog" && !categoryFilter ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("catalog"); setConsoleFilter(null); setCategoryFilter(null); }}
            >
              <span className="sidebar__item-icon"><Grid3X3 size={16} /></span>
              Catalog
              <span className="sidebar__item-count">{catalog.length}</span>
            </button>
            <button
              className={`sidebar__item ${page === "library" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("library"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><Gamepad2 size={16} /></span>
              Roms
              <span className="sidebar__item-count">{roms.length}</span>
            </button>
            <button
              className={`sidebar__item ${page === "installed" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("installed"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><HardDrive size={16} /></span>
              Installed
              <span className="sidebar__item-count">{installedCount}</span>
            </button>
            <button
              className={`sidebar__item ${page === "settings" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("settings")}
            >
              <span className="sidebar__item-icon"><Settings size={16} /></span>
              Settings
            </button>
            <button
              className={`sidebar__item ${page === "changelogs" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("changelogs")}
            >
              <span className="sidebar__item-icon"><FileText size={16} /></span>
              Changelogs
            </button>
          </div>

          <div className="sidebar__divider" />

          <div className="sidebar__section">
            {page === "catalog" ? (
              <>
                <div className="sidebar__label">Emulators</div>
                <button
                  className={`sidebar__item ${!categoryFilter && !consoleFilter ? "sidebar__item--active" : ""}`}
                  onClick={() => { setCategoryFilter(null); setConsoleFilter(null); }}
                >
                  <span className="sidebar__item-icon">🕹️</span>
                  All Categories
                </button>
                {Object.entries(consolesByCategory).map(([category, categoryConsoles]) => {
                  const isCatExpanded = expandedCategories.includes(category);
                  const isCatActive = categoryFilter === category;
                  return (
                    <div key={category} className="sidebar__category">
                      <button
                        className={`sidebar__category-title ${isCatActive ? "sidebar__category-title--active" : ""}`}
                        onClick={() => toggleCategory(category)}
                      >
                        {isCatExpanded ? <ChevronDown size={10} /> : <ChevronIcon size={10} />}
                        {category}
                      </button>
                      <AnimatePresence>
                        {isCatExpanded && (
                          <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: "auto", opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            className="sidebar__category-items"
                          >
                            {categoryConsoles.sort().map((con) => (
                              <button
                                key={con}
                                className={`sidebar__item ${consoleFilter === con ? "sidebar__item--active" : ""}`}
                                onClick={() => setConsoleFilter(con)}
                              >
                                <span className="sidebar__item-icon">{CONSOLE_ICONS[con] || "🎮"}</span>
                                {con}
                              </button>
                            ))}
                          </motion.div>
                        )}
                      </AnimatePresence>
                    </div>
                  );
                })}
              </>
            ) : page === "library" ? (
              <>
                <div className="sidebar__label">Library</div>
                <button
                  className={`sidebar__item ${!categoryFilter && !consoleFilter ? "sidebar__item--active" : ""}`}
                  onClick={() => { setCategoryFilter(null); setConsoleFilter(null); }}
                >
                  <span className="sidebar__item-icon">📂</span>
                  All Games
                </button>
                {Object.entries(consolesByCategory).map(([category, categoryConsoles]) => {
                  const isCatExpanded = expandedCategories.includes(category);
                  const isCatActive = categoryFilter === category;
                  return (
                    <div key={category} className="sidebar__category">
                      <button
                        className={`sidebar__category-title ${isCatActive ? "sidebar__category-title--active" : ""}`}
                        onClick={() => toggleCategory(category)}
                      >
                        {isCatExpanded ? <ChevronDown size={10} /> : <ChevronIcon size={10} />}
                        {category}
                      </button>
                      <AnimatePresence>
                        {isCatExpanded && (
                          <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: "auto", opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            className="sidebar__category-items"
                          >
                            {categoryConsoles.sort().map((con) => {
                              const isConExpanded = expandedSidebarConsoles.includes(con);
                              const isConActive = consoleFilter === con;
                              const consoleGames = roms.filter(r => r.console === con);
                              
                              if (consoleGames.length === 0) return null;

                              return (
                                <div key={con} className="sidebar__console">
                                  <button
                                    className={`sidebar__item ${isConActive ? "sidebar__item--active" : ""}`}
                                    onClick={() => toggleSidebarConsole(con)}
                                  >
                                    <span className="sidebar__item-icon">{isConExpanded ? <ChevronDown size={10} /> : <ChevronIcon size={10} />}</span>
                                    <span className="sidebar__item-icon">{CONSOLE_ICONS[con] || "🎮"}</span>
                                    {con}
                                    <span className="sidebar__item-count">{consoleGames.length}</span>
                                  </button>
                                  
                                  <AnimatePresence>
                                    {isConExpanded && (
                                      <motion.div
                                        initial={{ height: 0, opacity: 0 }}
                                        animate={{ height: "auto", opacity: 1 }}
                                        exit={{ height: 0, opacity: 0 }}
                                        className="sidebar__game-list"
                                      >
                                        {consoleGames.sort((a,b) => a.name.localeCompare(b.name)).map(game => (
                                          <button 
                                            key={game.path} 
                                            className="sidebar__game-item"
                                            onClick={(e) => {
                                              e.stopPropagation();
                                              handleLaunch(game);
                                            }}
                                            title={game.name}
                                          >
                                            <Play size={10} className="sidebar__game-icon" />
                                            <span className="sidebar__game-name">{game.name}</span>
                                          </button>
                                        ))}
                                      </motion.div>
                                    )}
                                  </AnimatePresence>
                                </div>
                              );
                            })}
                          </motion.div>
                        )}
                      </AnimatePresence>
                    </div>
                  );
                })}
              </>
            ) : null}
          </div>
        </aside>

        <main className="main-content">
          <AnimatePresence mode="wait">
            <motion.div
              key={page}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
            >
              <div className="main-content__header">
                <div>
                  <h1 className="main-content__title">
                    {page === "catalog" && "Emulator Catalog"}
                    {page === "library" && "ROMs"}
                    {page === "installed" && "Installed Emulators"}
                    {page === "settings" && "Settings"}
                  </h1>
                  <p className="main-content__subtitle">
                    {page === "catalog" && `${filteredCatalog.length} emulators available`}
                    {page === "library" && `${filteredGames.length} games detected`}
                    {page === "installed" && `${installedCount} installed`}
                    {page === "settings" && "Configure your experience"}
                  </p>
                </div>
                <div className="main-content__actions">
                  {(page === "catalog" || page === "library") && (
                    <>
                      <div className="search-bar">
                        <Search size={16} className="search-bar__icon" />
                        <input
                          className="search-bar__input"
                          placeholder="Search..."
                          value={search}
                          onChange={(e) => setSearch(e.target.value)}
                        />
                      </div>
                    </>
                  )}
                  {page === "library" && (
                    <button className="btn btn--ghost" onClick={() => loadData()}>
                      <RefreshCw size={14} /> Refresh
                    </button>
                  )}
                  <button 
                    className="btn btn--ghost" 
                    onClick={toggleFullscreen}
                    title={isFullscreen ? "Exit Fullscreen" : "Enter Fullscreen"}
                  >
                    {isFullscreen ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
                  </button>
                </div>
              </div>

              {page === "catalog" && (
                <div className="catalog-content">
                  <div className="emu-grid">
                    {filteredCatalog.map((emu) => (
                      <motion.div key={emu.id} className="emu-card">
                        <div className="emu-card__header">
                          <div className="emu-card__icon">{emu.icon}</div>
                          <div className="emu-card__info">
                            <div className="emu-card__name">{emu.name}</div>
                            <div className="emu-card__console">{emu.console}</div>
                          </div>
                          {installed.includes(emu.id) && (
                            <div className="emu-card__status emu-card__status--installed">
                              <CheckCircle size={12} /> Installed
                            </div>
                          )}
                        </div>
                        <p className="emu-card__desc">{emu.description}</p>
                        <div className="emu-card__actions">
                          {installed.includes(emu.id) ? (
                            <button className="btn btn--success btn--sm" onClick={() => handleLaunch({ name: "", path: "", console: emu.console, extension: "", size: 0 })}><Play size={12} /> Launch</button>
                          ) : (
                            <button className="btn btn--primary btn--sm" onClick={() => handleInstall(emu.id)} disabled={installing.includes(emu.id)}>
                              {installing.includes(emu.id) ? <><span className="spinner" /> Installing...</> : <><Download size={12} /> Install</>}
                            </button>
                          )}
                          <a href={emu.website} target="_blank" rel="noreferrer" className="btn btn--ghost btn--sm"><ExternalLink size={12} /> Website</a>
                        </div>
                      </motion.div>
                    ))}
                  </div>
                  {filteredCatalog.length === 0 && (
                    <div className="empty-state">
                      <div className="empty-state__icon">🔍</div>
                      <div className="empty-state__title">No emulators found</div>
                    </div>
                  )}
                </div>
              )}

              {page === "library" && (
                <div className="library-content">
                  <div className="game-grid">
                    {filteredGames.map(rom => (
                      <GameCard key={rom.path} rom={rom} onLaunch={handleLaunch} />
                    ))}
                  </div>
                  {filteredGames.length === 0 && (
                    <div className="empty-state">
                      <div className="empty-state__icon">📂</div>
                      <div className="empty-state__title">No ROMs found</div>
                      <button className="btn btn--primary" onClick={() => setPage("settings")}><FolderOpen size={14} /> Go to Settings</button>
                    </div>
                  )}
                </div>
              )}

              {page === "installed" && (
                <div className="emu-grid">
                  {catalog.filter(e => installed.includes(e.id)).map(emu => (
                    <motion.div key={emu.id} className="emu-card">
                      <div className="emu-card__header">
                        <div className="emu-card__icon">{emu.icon}</div>
                        <div className="emu-card__info">
                          <div className="emu-card__name">{emu.name}</div>
                          <div className="emu-card__console">{emu.console}</div>
                        </div>
                      </div>
                      <div className="emu-card__actions">
                        <button className="btn btn--success btn--sm" onClick={() => handleLaunch({ name: "", path: "", console: emu.console, extension: "", size: 0 })}><Play size={12} /> Launch</button>
                        <button className="btn btn--danger btn--sm" onClick={() => handleUninstall(emu.id)}><Trash2 size={12} /> Uninstall</button>
                      </div>
                    </motion.div>
                  ))}
                </div>
              )}

              {page === "settings" && (
                <div className="settings">
                  <div className="settings__group">
                    <div className="settings__group-title"><FolderOpen size={16} /> Directories</div>
                    <div className="settings__field">
                      <label className="settings__field-label">ROMs Folder</label>
                      <input className="settings__field-input" value={config.roms_directory} readOnly />
                      <button className="btn btn--ghost btn--sm" onClick={() => handleBrowseFolder("roms_directory")}>Browse</button>
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">Emulators Folder</label>
                      <input className="settings__field-input" value={config.emulators_directory} readOnly />
                      <button className="btn btn--ghost btn--sm" onClick={() => handleBrowseFolder("emulators_directory")}>Browse</button>
                    </div>
                  </div>
                </div>
              )}

              {page === "changelogs" && (
                <div className="changelogs">
                  {changelogs.map((log: ChangelogEntry) => (
                    <div key={log.version} className="changelog-card">
                      <div className="changelog-card__header">
                        <h2 className="changelog-card__version">Version {log.version}</h2>
                        <span className="changelog-card__date">{log.date}</span>
                      </div>
                      <ul className="changelog-card__list">
                        {log.changes.map((change: string, i: number) => (
                          <li key={i} className="changelog-card__item">{change}</li>
                        ))}
                      </ul>
                    </div>
                  ))}
                </div>
              )}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      <div className="toasts">
        <AnimatePresence>
          {toasts.map((t) => (
            <motion.div key={t.id} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 20 }} className={`toast toast--${t.type}`}>
              {t.type === "success" && <CheckCircle size={14} />}
              {t.type === "error" && <AlertCircle size={14} />}
              {t.message}
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </>
  );
}
