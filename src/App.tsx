import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow, WebviewWindow } from "@tauri-apps/api/window";
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
  X,
  ChevronRight,
  CheckCircle,
  AlertCircle,
  ExternalLink,
  HardDrive,
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
}

interface RomFile {
  name: string;
  path: string;
  console: string;
  emulator_id: string;
  extension: string;
}

interface AppConfig {
  roms_directory: string;
  emulators_directory: string;
}

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

type Page = "catalog" | "library" | "installed" | "settings";

/* ============================
   Console icon mapping
   ============================ */
const CONSOLE_ICONS: Record<string, string> = {
  "Game Boy Advance": "🟢",
  "Nintendo DS": "📱",
  "PlayStation 1": "⚪",
  "PlayStation 2": "🔵",
  "PlayStation Portable": "⬛",
  "Nintendo 64": "🟡",
  "Super Nintendo": "🟣",
  "GameCube / Wii": "🐬",
  "Multi-System": "🔄",
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
  });
  const [search, setSearch] = useState("");
  const [consoleFilter, setConsoleFilter] = useState<string | null>(null);
  const [installing, setInstalling] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

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
          setInstalling(null);
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
    setInstalling(id);
    showToast("Downloading emulator...", "info");
    try {
      await invoke("install_emulator", { emulatorId: id });
    } catch (err: any) {
      setInstalling(null);
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

  const handleLaunch = async (id: string, romPath?: string) => {
    try {
      await invoke("launch_emulator", {
        emulatorId: id,
        romPath: romPath || null,
      });
      showToast("Emulator launched!", "success");
    } catch (err: any) {
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

  const handleBrowseFolder = async (field: "roms_directory" | "emulators_directory") => {
    const selected = await open({ directory: true });
    if (selected) {
      const newConfig = { ...config, [field]: selected as string };
      handleSaveConfig(newConfig);
    }
  };

  // ---- Derived data ----
  const consoles = [...new Set(catalog.map((e) => e.console))];
  const installedCount = installed.length;

  const filteredCatalog = catalog.filter((emu) => {
    const matchesSearch =
      !search ||
      emu.name.toLowerCase().includes(search.toLowerCase()) ||
      emu.console.toLowerCase().includes(search.toLowerCase());
    const matchesConsole = !consoleFilter || emu.console === consoleFilter;
    return matchesSearch && matchesConsole;
  });

  const filteredRoms = roms.filter((rom) => {
    const matchesSearch =
      !search || rom.name.toLowerCase().includes(search.toLowerCase());
    const matchesConsole = !consoleFilter || rom.console === consoleFilter;
    return matchesSearch && matchesConsole;
  });

  // ---- Window controls ----
  const [appWindow, setAppWindow] = useState<WebviewWindow | null>(null);

  useEffect(() => {
    // Permet de tester l'UI dans un simple navigateur sans planter :
    // si on n'est pas dans Tauri, getCurrentWindow() lève une erreur.
    try {
      const win = getCurrentWindow();
      setAppWindow(win);
    } catch {
      setAppWindow(null);
    }
  }, []);

  const minimize = () => appWindow?.minimize();
  const maximize = () => appWindow?.toggleMaximize();
  const close = () => appWindow?.close();

  return (
    <>
      {/* ===== Custom Titlebar ===== */}
      <div className="titlebar">
        <div className="titlebar__logo">
          <div className="titlebar__logo-icon">🎮</div>
          <span>EmuWorld</span>
        </div>
        <div className="titlebar__controls">
          <button className="titlebar__btn" onClick={minimize}>
            <Minus size={14} />
          </button>
          <button className="titlebar__btn" onClick={maximize}>
            <Square size={12} />
          </button>
          <button className="titlebar__btn titlebar__btn--close" onClick={close}>
            <X size={14} />
          </button>
        </div>
      </div>

      {/* ===== Main Layout ===== */}
      <div className="app">
        {/* Sidebar */}
        <aside className="sidebar">
          <div className="sidebar__section">
            <div className="sidebar__label">Navigation</div>
            <button
              className={`sidebar__item ${page === "catalog" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("catalog"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><Grid3X3 size={16} /></span>
              Catalog
              <span className="sidebar__item-count">{catalog.length}</span>
            </button>
            <button
              className={`sidebar__item ${page === "library" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("library"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><Library size={16} /></span>
              Library
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
          </div>

          <div className="sidebar__divider" />

          <div className="sidebar__section">
            <div className="sidebar__label">Consoles</div>
            <button
              className={`sidebar__item ${!consoleFilter ? "sidebar__item--active" : ""}`}
              onClick={() => setConsoleFilter(null)}
            >
              <span className="sidebar__item-icon">🎮</span>
              All Consoles
            </button>
            {consoles.map((c) => (
              <button
                key={c}
                className={`sidebar__item ${consoleFilter === c ? "sidebar__item--active" : ""}`}
                onClick={() => {
                  setConsoleFilter(c);
                  if (page === "settings") setPage("catalog");
                }}
              >
                <span className="sidebar__item-icon">
                  {CONSOLE_ICONS[c] || "🎮"}
                </span>
                {c}
              </button>
            ))}
          </div>
        </aside>

        {/* Main Content */}
        <main className="main-content">
          <AnimatePresence mode="wait">
            <motion.div
              key={page}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
            >
              {/* Page Header */}
              <div className="main-content__header">
                <div>
                  <h1 className="main-content__title">
                    {page === "catalog" && "Emulator Catalog"}
                    {page === "library" && "Game Library"}
                    {page === "installed" && "Installed Emulators"}
                    {page === "settings" && "Settings"}
                  </h1>
                  <p className="main-content__subtitle">
                    {page === "catalog" &&
                      `${filteredCatalog.length} emulators available`}
                    {page === "library" &&
                      `${filteredRoms.length} games detected`}
                    {page === "installed" &&
                      `${installedCount} emulator${installedCount !== 1 ? "s" : ""} installed`}
                    {page === "settings" && "Configure your EmuWorld experience"}
                  </p>
                </div>
                <div className="main-content__actions">
                  {(page === "catalog" || page === "library") && (
                    <div className="search-bar">
                      <Search size={16} className="search-bar__icon" />
                      <input
                        className="search-bar__input"
                        placeholder="Search..."
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                      />
                    </div>
                  )}
                  {page === "library" && (
                    <button className="btn btn--ghost" onClick={() => loadData()}>
                      <RefreshCw size={14} />
                      Refresh
                    </button>
                  )}
                </div>
              </div>

              {/* ===== CATALOG PAGE ===== */}
              {page === "catalog" && (
                <div className="emu-grid">
                  {filteredCatalog.map((emu, index) => {
                    const isInstalled = installed.includes(emu.id);
                    const isInstalling = installing === emu.id;
                    return (
                      <motion.div
                        key={emu.id}
                        className="emu-card"
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: index * 0.05 }}
                      >
                        <div className="emu-card__header">
                          <div className="emu-card__icon">{emu.icon}</div>
                          <div className="emu-card__info">
                            <div className="emu-card__name">{emu.name}</div>
                            <div className="emu-card__console">{emu.console}</div>
                          </div>
                          <div
                            className={`emu-card__status ${isInstalled
                                ? "emu-card__status--installed"
                                : "emu-card__status--not-installed"
                              }`}
                          >
                            {isInstalled ? (
                              <>
                                <CheckCircle size={12} /> Installed
                              </>
                            ) : (
                              <>
                                <AlertCircle size={12} /> Not installed
                              </>
                            )}
                          </div>
                        </div>

                        <p className="emu-card__desc">{emu.description}</p>

                        <div className="emu-card__extensions">
                          {emu.supported_extensions.map((ext) => (
                            <span key={ext} className="emu-card__ext">
                              .{ext}
                            </span>
                          ))}
                        </div>

                        {isInstalling && (
                          <div className="progress-bar">
                            <div
                              className="progress-bar__fill"
                              style={{ width: "60%" }}
                            />
                          </div>
                        )}

                        <div className="emu-card__actions">
                          {isInstalled ? (
                            <>
                              <button
                                className="btn btn--success btn--sm"
                                onClick={() => handleLaunch(emu.id)}
                              >
                                <Play size={12} /> Launch
                              </button>
                              <button
                                className="btn btn--danger btn--sm"
                                onClick={() => handleUninstall(emu.id)}
                              >
                                <Trash2 size={12} /> Uninstall
                              </button>
                            </>
                          ) : (
                            <button
                              className={`btn btn--primary btn--sm ${isInstalling ? "btn--disabled" : ""
                                }`}
                              onClick={() => handleInstall(emu.id)}
                            >
                              {isInstalling ? (
                                <>
                                  <span className="spinner" /> Installing...
                                </>
                              ) : (
                                <>
                                  <Download size={12} /> Install
                                </>
                              )}
                            </button>
                          )}
                          <a
                            href={emu.website}
                            target="_blank"
                            rel="noreferrer"
                            className="btn btn--ghost btn--sm"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <ExternalLink size={12} /> Website
                          </a>
                        </div>
                      </motion.div>
                    );
                  })}
                </div>
              )}

              {/* ===== LIBRARY PAGE ===== */}
              {page === "library" && (
                <>
                  {roms.length === 0 ? (
                    <div className="empty-state">
                      <div className="empty-state__icon">📂</div>
                      <div className="empty-state__title">No games found</div>
                      <div className="empty-state__desc">
                        Set your ROMs directory in Settings, then click Refresh to
                        scan for games.
                      </div>
                      <button
                        className="btn btn--primary"
                        onClick={() => setPage("settings")}
                      >
                        <FolderOpen size={14} /> Go to Settings
                      </button>
                    </div>
                  ) : (
                    <div className="game-grid">
                      {filteredRoms.map((rom, index) => (
                        <motion.div
                          key={rom.path}
                          className="game-card"
                          initial={{ opacity: 0, scale: 0.95 }}
                          animate={{ opacity: 1, scale: 1 }}
                          transition={{ delay: index * 0.03 }}
                          onClick={() => handleLaunch(rom.emulator_id, rom.path)}
                        >
                          <div className="game-card__cover">
                            <div className="game-card__cover-text">
                              {rom.name.slice(0, 20)}
                            </div>
                            <div className="game-card__play-overlay">
                              <button className="game-card__play-btn">
                                <Play size={20} fill="white" />
                              </button>
                            </div>
                          </div>
                          <div className="game-card__info">
                            <div className="game-card__name">{rom.name}</div>
                            <div className="game-card__console">
                              {rom.console} &middot; .{rom.extension}
                            </div>
                          </div>
                        </motion.div>
                      ))}
                    </div>
                  )}
                </>
              )}

              {/* ===== INSTALLED PAGE ===== */}
              {page === "installed" && (
                <>
                  {installed.length === 0 ? (
                    <div className="empty-state">
                      <div className="empty-state__icon">📦</div>
                      <div className="empty-state__title">
                        No emulators installed
                      </div>
                      <div className="empty-state__desc">
                        Browse the catalog to download and install emulators.
                      </div>
                      <button
                        className="btn btn--primary"
                        onClick={() => setPage("catalog")}
                      >
                        <Grid3X3 size={14} /> Go to Catalog
                      </button>
                    </div>
                  ) : (
                    <div className="emu-grid">
                      {catalog
                        .filter((e) => installed.includes(e.id))
                        .map((emu, index) => (
                          <motion.div
                            key={emu.id}
                            className="emu-card"
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            transition={{ delay: index * 0.05 }}
                          >
                            <div className="emu-card__header">
                              <div className="emu-card__icon">{emu.icon}</div>
                              <div className="emu-card__info">
                                <div className="emu-card__name">{emu.name}</div>
                                <div className="emu-card__console">
                                  {emu.console}
                                </div>
                              </div>
                              <div className="emu-card__status emu-card__status--installed">
                                <CheckCircle size={12} /> Installed
                              </div>
                            </div>
                            <p className="emu-card__desc">{emu.description}</p>
                            <div className="emu-card__actions">
                              <button
                                className="btn btn--success btn--sm"
                                onClick={() => handleLaunch(emu.id)}
                              >
                                <Play size={12} /> Launch
                              </button>
                              <button
                                className="btn btn--danger btn--sm"
                                onClick={() => handleUninstall(emu.id)}
                              >
                                <Trash2 size={12} /> Uninstall
                              </button>
                            </div>
                          </motion.div>
                        ))}
                    </div>
                  )}
                </>
              )}

              {/* ===== SETTINGS PAGE ===== */}
              {page === "settings" && (
                <div className="settings">
                  <div className="settings__group">
                    <div className="settings__group-title">
                      <FolderOpen size={16} /> Directories
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">ROMs Folder</label>
                      <input
                        className="settings__field-input"
                        value={config.roms_directory}
                        onChange={(e) =>
                          setConfig({ ...config, roms_directory: e.target.value })
                        }
                        placeholder="Select ROMs directory..."
                      />
                      <button
                        className="btn btn--ghost btn--sm"
                        onClick={() => handleBrowseFolder("roms_directory")}
                      >
                        Browse
                      </button>
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">
                        Emulators Folder
                      </label>
                      <input
                        className="settings__field-input"
                        value={config.emulators_directory}
                        onChange={(e) =>
                          setConfig({
                            ...config,
                            emulators_directory: e.target.value,
                          })
                        }
                        placeholder="Select emulators directory..."
                      />
                      <button
                        className="btn btn--ghost btn--sm"
                        onClick={() => handleBrowseFolder("emulators_directory")}
                      >
                        Browse
                      </button>
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title">
                      <Gamepad2 size={16} /> About EmuWorld
                    </div>
                    <p
                      style={{
                        fontSize: 13,
                        color: "var(--text-secondary)",
                        lineHeight: 1.6,
                      }}
                    >
                      EmuWorld v0.1.0 — All-in-one emulator launcher.
                      <br />
                      Open source &middot; Built with Tauri + React
                    </p>
                  </div>

                  <button
                    className="btn btn--primary"
                    onClick={() => handleSaveConfig(config)}
                  >
                    Save Settings
                  </button>
                </div>
              )}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      {/* ===== Toast Notifications ===== */}
      <div className="toast-container">
        <AnimatePresence>
          {toasts.map((toast) => (
            <motion.div
              key={toast.id}
              className={`toast toast--${toast.type}`}
              initial={{ opacity: 0, x: 30, scale: 0.95 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 30, scale: 0.95 }}
              transition={{ duration: 0.2 }}
            >
              {toast.type === "success" && <CheckCircle size={16} color="var(--accent-green)" />}
              {toast.type === "error" && <AlertCircle size={16} color="var(--accent-red)" />}
              {toast.type === "info" && <ChevronRight size={16} color="var(--accent-primary)" />}
              {toast.message}
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </>
  );
}
