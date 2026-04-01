import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { motion, AnimatePresence } from "framer-motion";
import { supabase, Profile } from "./supabase";
import type { User, Provider } from "@supabase/supabase-js";
import { openUrl } from "@tauri-apps/plugin-opener";
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
  User as UserIcon,
  LogOut,
  LogIn,
  Mail,
  Eye,
  EyeOff,
  Camera,
  ShieldCheck,
  Link2,
  ShoppingBag,
  Check,
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
interface RomStoreEntry {
  id: string;
  name: string;
  console: string;
  region: string;
  size: string;
  file_name: string;
  download_url: string;
  ia_id?: string;
}

type Page = "catalog" | "library" | "installed" | "settings" | "changelogs" | "account" | "store";

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
        const dataUrl: string = await invoke("fetch_boxart", { 
          gameName: rom.name, 
          console: rom.console 
        });
        setCover(dataUrl);
      } catch (e) {
        // No cover available, placeholder will show
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

const RomStoreCard = ({ rom, onDownload, downloading, downloaded, progress }: { 
  rom: RomStoreEntry, 
  onDownload: (rom: RomStoreEntry) => void,
  downloading: boolean,
  downloaded: boolean,
  progress?: number
}) => {
  const [cover, setCover] = useState<string | null>(null);
  const [loadingCover, setLoadingCover] = useState(false);

  useEffect(() => {
    const fetchCover = async () => {
      try {
        setLoadingCover(true);
        // We use the same fetch_boxart command as library games
        const dataUrl: string = await invoke("fetch_boxart", { 
          gameName: rom.name, 
          console: rom.console 
        });
        setCover(dataUrl);
      } catch (e) {
        // No cover found
      } finally {
        setLoadingCover(false);
      }
    };
    fetchCover();
  }, [rom.name, rom.console]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      className="store-card"
    >
      <div className="store-card__cover">
        {cover ? (
          <img src={cover} alt={rom.name} className="store-card__img" />
        ) : (
          <div className="store-card__placeholder">
            <span className="store-card__placeholder-icon">
              {loadingCover ? <RefreshCw className="animate-spin" size={24} /> : (CONSOLE_ICONS[rom.console] || "🎮")}
            </span>
            {!loadingCover && <div className="store-card__placeholder-text">{rom.console}</div>}
          </div>
        )}
        <div className="store-card__badge">{rom.region}</div>
      </div>
      <div className="store-card__info">
        <div className="store-card__name" title={rom.name}>{rom.name}</div>
        <div className="store-card__meta">
          <span>{rom.console}</span>
          <span className="store-card__size">{rom.size}</span>
        </div>
        <button 
          className={`btn btn--full ${downloaded ? 'btn--success' : downloading ? 'btn--loading' : 'btn--primary'}`}
          onClick={() => onDownload(rom)}
          disabled={downloading || downloaded}
        >
          {downloaded ? (
            <><Check size={14} /> Downloaded</>
          ) : downloading ? (
            <><RefreshCw size={14} className="animate-spin" /> {progress !== undefined ? `${progress}%` : 'Downloading...'}</>
          ) : (
            <><Download size={14} /> Download</>
          )}
        </button>
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

  // ---- ROM Store state ----
  const [storeRoms, setStoreRoms] = useState<RomStoreEntry[]>([]);
  const [storeSearch, setStoreSearch] = useState("");
  const [debouncedStoreSearch, setDebouncedStoreSearch] = useState("");
  const [storeConsoleFilter, setStoreConsoleFilter] = useState<string | null>(null);
  const [storeConsoles, setStoreConsoles] = useState<string[]>([]);
  const [downloading, setDownloading] = useState<string[]>([]);
  const [downloaded, setDownloaded] = useState<string[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [isSearchingStore, setIsSearchingStore] = useState(false);
  const [changelogs] = useState<ChangelogEntry[]>([
    { version: "0.3.6", date: "2026-03-25", changes: ["🎮 Manual Ryubing (Ryujinx) Installation from local zip", "Improved emulator discovery depth", "General stability fixes"] },
    { version: "0.3.5", date: "2026-03-20", changes: ["🗑️ Fixed uninstallation regression (Case-sensitivity fix)", "🖼️ Better Wii/Wii U cover matching (Region fallbacks)", "🔒 Added 'Access Denied' warning for running emulators"] },
    { version: "0.3.2", date: "2026-03-19", changes: ["🎮 Added Ryubing for Nintendo Switch emulation", "🖼️ Improved cover matching: Added GB/GBC fallbacks for GBA console (mGBA support)", "🎨 New custom EmuWorld app icon and branding"] },
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

  // ---- AUTH STATE ----
  const [user, setUser] = useState<User | null>(null);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [authLoading, setAuthLoading] = useState(false);
  const [showLoginModal, setShowLoginModal] = useState(false);
  const [loginEmail, setLoginEmail] = useState("");
  const [loginPassword, setLoginPassword] = useState("");
  const [loginPseudo, setLoginPseudo] = useState("");
  const [isSignUp, setIsSignUp] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  
  // Account settings
  const [showAccountModal, setShowAccountModal] = useState(false);
  const [newPseudo, setNewPseudo] = useState("");
  const [isUpdatingProfile, setIsUpdatingProfile] = useState(false);

  // Fetch profile from Supabase
  const fetchProfile = useCallback(async (userId: string) => {
    try {
      const { data, error } = await supabase
        .from('profiles')
        .select('*')
        .eq('id', userId)
        .single();
      if (error) {
        console.error("[EmuWorld] Error fetching profile:", error.message);
        return;
      }
      if (data) setProfile(data);
    } catch (e) {
      console.error("[EmuWorld] Unexpected fetch error:", e);
    }
  }, []);

  // Check session and setup real-time sync
  useEffect(() => {
    let profileChannel: any = null;

    const setupRealtime = (userId: string) => {
      if (profileChannel) supabase.removeChannel(profileChannel);
      profileChannel = supabase
        .channel(`public:profiles:id=eq.${userId}`)
        .on(
          'postgres_changes',
          { event: 'UPDATE', schema: 'public', table: 'profiles', filter: `id=eq.${userId}` },
          (payload) => {
            console.log("[EmuWorld] Realtime profile update:", payload.new);
            setProfile(payload.new as Profile);
          }
        )
        .subscribe();
    };

    supabase.auth.getSession().then(({ data: { session } }) => {
      if (session?.user) {
        setUser(session.user);
        fetchProfile(session.user.id);
        setupRealtime(session.user.id);
      }
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
      if (session?.user) {
        fetchProfile(session.user.id);
        setupRealtime(session.user.id);
      } else {
        setProfile(null);
        if (profileChannel) supabase.removeChannel(profileChannel);
      }
    });

    return () => {
      subscription.unsubscribe();
      if (profileChannel) supabase.removeChannel(profileChannel);
    };
  }, [fetchProfile]);

  // Listen for deep-link OAuth callbacks
  useEffect(() => {
    const unlistenPromise = listen<string>('oauth-callback', async (event) => {
      console.log('[Auth] OAuth callback received:', event.payload);
      try {
        const urlStr = event.payload;
        // Robust parsing of parameters from both query string (?) and hash fragment (#)
        // We replace "emuworld://" with "http://localhost/" so the URL constructor can parse it on all systems
        const urlObj = new URL(urlStr.replace('emuworld://', 'http://localhost/'));
        const params = new URLSearchParams(urlObj.search || urlObj.hash.substring(1));
        
        const accessToken = params.get('access_token');
        const refreshToken = params.get('refresh_token');
        const code = params.get('code');
        
        if (accessToken && refreshToken) {
          console.log('[Auth] Detected Implicit Flow (tokens)');
          const { error } = await supabase.auth.setSession({
            access_token: accessToken,
            refresh_token: refreshToken
          });
          if (error) throw error;
          showToast('Connected successfully! 🎉', 'success');
          setShowLoginModal(false);
        } else if (code) {
          console.log('[Auth] Detected PKCE Flow (code)');
          const { error } = await supabase.auth.exchangeCodeForSession(code);
          if (error) throw error;
          showToast('Connected successfully! 🎉', 'success');
          setShowLoginModal(false);
        } else {
          console.warn('[Auth] No session data found in callback URL');
        }
      } catch (e: any) {
        console.error('[Auth] Callback error:', e);
        showToast(`Auth error: ${e.message}`, 'error');
        setAuthError(e.message);
      }
    });

    return () => {
      unlistenPromise.then(fn => fn());
    };
  }, [showToast]);

  // Update profile
  const handleUpdateProfile = async () => {
    if (!user || !newPseudo.trim()) return;
    setIsUpdatingProfile(true);
    try {
      const { error } = await supabase
        .from('profiles')
        .upsert({ 
          id: user.id, 
          username: newPseudo,
          updated_at: new Date().toISOString()
        });
      
      if (error) throw error;
      await fetchProfile(user.id);
      showToast('Username updated! ✨', 'success');
    } catch (e: any) {
      showToast(`Update error: ${e.message}`, 'error');
    } finally {
      setIsUpdatingProfile(false);
    }
  };

  // Upload avatar
  const handleAvatarUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !user) return;

    setIsUpdatingProfile(true);
    try {
      const fileExt = file.name.split('.').pop();
      const fileName = `${user.id}-${Math.random()}.${fileExt}`;
      const filePath = `${fileName}`;

      // 1. Upload to Storage
      const { error: uploadError } = await supabase.storage
        .from('avatars')
        .upload(filePath, file);

      if (uploadError) throw uploadError;

      // 2. Get Public URL
      const { data } = supabase.storage
        .from('avatars')
        .getPublicUrl(filePath);

      // 3. Upsert Profile
      const { error: updateError } = await supabase
        .from('profiles')
        .upsert({ 
          id: user.id, 
          avatar_url: data.publicUrl,
          updated_at: new Date().toISOString()
        });

      if (updateError) throw updateError;
      
      // Force refresh with a slight delay and add cache buster to URL if needed
      await fetchProfile(user.id);
      showToast('Avatar updated! 📸', 'success');
    } catch (e: any) {
      console.error("[EmuWorld] Avatar upload error:", e);
      showToast(`Upload error: ${e.message}`, 'error');
    } finally {
      setIsUpdatingProfile(false);
    }
  };

  // Social login
  const handleSocialLogin = async (provider: Provider) => {
    setAuthLoading(true);
    setAuthError(null);
    try {
      const { data, error } = await supabase.auth.signInWithOAuth({
        provider,
        options: {
          redirectTo: 'emuworld://auth-callback',
          skipBrowserRedirect: true,
        },
      });
      if (error) throw error;
      if (data.url) {
        await openUrl(data.url);
      }
    } catch (e: any) {
      setAuthError(e.message);
    } finally {
      setAuthLoading(false);
    }
  };

  const handleLogout = async () => {
    try {
      await supabase.auth.signOut();
      setUser(null);
      setProfile(null);
      showToast('Signed out successfully', 'info');
    } catch (e: any) {
      showToast(`Logout error: ${e.message}`, 'error');
    }
  };

  // Email login/signup
  const handleEmailAuth = async () => {
    setAuthLoading(true);
    setAuthError(null);
    try {
      if (isSignUp) {
        if (loginPseudo.length < 3) {
          setAuthError('Username must be at least 3 characters');
          setAuthLoading(false);
          return;
        }
        const { data, error } = await supabase.auth.signUp({
          email: loginEmail,
          password: loginPassword,
        });
        if (error) throw error;
        // Insert profile with pseudo
        if (data.user) {
          await supabase.from('profiles').upsert({
            id: data.user.id,
            username: loginPseudo,
            updated_at: new Date().toISOString(),
          });
          showToast('Account created! Check your email to confirm.', 'success');
        }
      } else {
        const { error } = await supabase.auth.signInWithPassword({
          email: loginEmail,
          password: loginPassword,
        });
        if (error) throw error;
        showToast('Welcome back! 🎮', 'success');
      }
      setShowLoginModal(false);
    } catch (e: any) {
      setAuthError(e.message);
    } finally {
      setAuthLoading(false);
    }
  };

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

  // ---- ROM Store ----
  const loadStoreData = useCallback(async () => {
    try {
      const consoles = await invoke<string[]>("get_store_consoles");
      setStoreConsoles(consoles);
    } catch (e) {
      console.error("Failed to load store consoles:", e);
    }
  }, []);

  useEffect(() => {
    loadStoreData();
  }, [loadStoreData]);

  const searchStore = useCallback(async (query: string, consoleF: string | null) => {
    setIsSearchingStore(true);
    try {
      const results = await invoke<RomStoreEntry[]>("search_rom_store", { 
        query, 
        consoleFilter: consoleF 
      });
      setStoreRoms(results);
    } catch (e) {
      console.error("Store search failed:", e);
      showToast("Store search failed. Check your internet connection.", "error");
    } finally {
      setIsSearchingStore(false);
    }
  }, [showToast]);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedStoreSearch(storeSearch);
    }, 600);
    return () => clearTimeout(handler);
  }, [storeSearch]);

  useEffect(() => {
    searchStore(debouncedStoreSearch, storeConsoleFilter);
  }, [debouncedStoreSearch, storeConsoleFilter, searchStore]);

  const handleDownloadRom = async (rom: RomStoreEntry) => {
    if (downloading.includes(rom.id) || downloaded.includes(rom.id)) return;
    showToast(`Preparing download for ${rom.name}...`, "info");
    setDownloading(prev => [...prev, rom.id]);
    try {
      const result = await invoke<string>("download_rom", {
        downloadUrlArg: rom.download_url,
        console: rom.console,
        fileNameArg: rom.file_name,
        iaId: rom.ia_id || null,
      });
      setDownloaded(prev => [...prev, rom.id]);
      showToast(`${rom.name} downloaded! 🎮`, "success");
      loadData(); // Refresh the ROM library
    } catch (err: any) {
      showToast(`Download failed: ${err}`, "error");
    } finally {
      setDownloading(prev => prev.filter(id => id !== rom.id));
    }
  };

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

  // ---- Listen for ROM download progress events ----
  useEffect(() => {
    const unlisten = listen<{ file_name?: string; file_id?: string; status: string; progress: number }>(
      "rom-download-progress",
      (event) => {
        const { file_id, file_name, progress } = event.payload;
        const id = file_id || file_name;
        if (id) {
          setDownloadProgress(prev => ({ ...prev, [id]: progress }));
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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
              className={`sidebar__item ${page === "store" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("store"); setStoreConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><ShoppingBag size={16} /></span>
              Store
              <span className="sidebar__item-count">{storeRoms.length}</span>
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
            ) : page === "store" ? (
              <>
                <div className="sidebar__label">Store Systems</div>
                <button
                  className={`sidebar__item ${!storeConsoleFilter ? "sidebar__item--active" : ""}`}
                  onClick={() => setStoreConsoleFilter(null)}
                >
                  <span className="sidebar__item-icon">🏪</span>
                  All Systems
                </button>
                {storeConsoles.map((con) => (
                  <button
                    key={con}
                    className={`sidebar__item ${storeConsoleFilter === con ? "sidebar__item--active" : ""}`}
                    onClick={() => setStoreConsoleFilter(con)}
                  >
                    <span className="sidebar__item-icon">{CONSOLE_ICONS[con] || "🎮"}</span>
                    {con}
                  </button>
                ))}
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

          {/* User card at the bottom of the sidebar */}
          <div className="sidebar__user-card">
            {user ? (
              <div className="sidebar__user-info" onClick={() => { setNewPseudo(profile?.username || ""); setShowAccountModal(true); }}>
                <div className="sidebar__user-avatar">
                  {profile?.avatar_url ? (
                    <img src={`${profile.avatar_url}?t=${new Date().getTime()}`} alt="avatar" />
                  ) : (
                    <UserIcon size={20} />
                  )}
                </div>
                <div className="sidebar__user-details">
                  <div className="sidebar__user-name">{profile?.username || user.email?.split('@')[0] || 'User'}</div>
                  <div className="sidebar__user-email">{user.email}</div>
                </div>
                <button className="sidebar__user-settings" title="Settings">
                  <Settings size={14} />
                </button>
              </div>
            ) : (
              <button className="sidebar__login-btn" onClick={() => setShowLoginModal(true)}>
                <LogIn size={16} />
                <span>Login / Sign Up</span>
              </button>
            )}
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
                    {page === "store" && "ROM Store"}
                    {page === "settings" && "Settings"}
                  </h1>
                  <p className="main-content__subtitle">
                    {page === "catalog" && `${filteredCatalog.length} emulators available`}
                    {page === "store" && `${storeRoms.length} ROMs available`}
                    {page === "library" && `${filteredGames.length} games detected`}
                    {page === "installed" && `${installedCount} installed`}
                    {page === "settings" && "Configure your experience"}
                  </p>
                </div>
                <div className="main-content__actions">
                  {(page === "catalog" || page === "library" || page === "store") && (
                    <>
                      <div className="search-bar">
                        {page === "store" && storeSearch !== debouncedStoreSearch ? (
                          <RefreshCw size={16} className="search-bar__icon animate-spin" />
                        ) : (
                          <Search size={16} className="search-bar__icon" />
                        )}
                        <input
                          className="search-bar__input"
                          placeholder={page === "store" ? "Search in Store..." : "Search..."}
                          value={page === "store" ? storeSearch : search}
                          onChange={(e) => {
                            if (page === "store") setStoreSearch(e.target.value);
                            else setSearch(e.target.value);
                          }}
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

              {page === "store" && (
                <div className="store-page">
                  <div className="grid grid--fixed">
                    {isSearchingStore ? (
                      <div className="empty-state">
                        <RefreshCw size={48} className="animate-spin mb-4 text-primary" />
                        <h3 className="empty-state__title">Searching Archive.org...</h3>
                        <p className="empty-state__text">Fetching the best classics for you.</p>
                      </div>
                    ) : storeRoms.length > 0 ? (
                      storeRoms.map((rom) => (
                        <RomStoreCard
                          key={rom.id}
                          rom={rom}
                          onDownload={handleDownloadRom}
                          downloading={downloading.includes(rom.id)}
                          downloaded={downloaded.includes(rom.id)}
                          progress={downloadProgress[rom.id] || downloadProgress[rom.file_name]}
                        />
                      ))
                    ) : (
                      <div className="empty-state">
                        <div className="empty-state__icon">🕵️</div>
                        <h3 className="empty-state__title">No ROMs found</h3>
                        <p className="empty-state__text">Try adjusting your search or filters</p>
                      </div>
                    )}
                  </div>
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

      {/* ===== ACCOUNT MODAL (RESTRICTED) ===== */}
      <AnimatePresence>
        {showAccountModal && user && (
          <motion.div
            className="account-overlay"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setShowAccountModal(false)}
          >
            <motion.div
              className="account-modal"
              initial={{ opacity: 0, scale: 0.95, y: 10 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 10 }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="account-modal__header">
                <h2 className="account-modal__title">Account Settings</h2>
                <button className="account-modal__close" onClick={() => setShowAccountModal(false)}>
                  <X size={20} />
                </button>
              </div>

              <div className="account-modal__content">
                {/* Avatar Section */}
                <div className="account-modal__section">
                  <label className="account-modal__label">Profile Picture</label>
                  <div className="account-modal__avatar-container">
                    <div className="account-modal__avatar-wrapper">
                      {profile?.avatar_url ? (
                        <img src={`${profile.avatar_url}?t=${new Date().getTime()}`} alt="Avatar" className="account-modal__avatar" />
                      ) : (
                        <div className="account-modal__avatar-placeholder">
                          <UserIcon size={32} />
                        </div>
                      )}
                      <label className="account-modal__avatar-edit">
                        <Camera size={16} />
                        <input type="file" accept="image/*" onChange={handleAvatarUpload} hidden />
                      </label>
                    </div>
                    <div className="account-modal__avatar-info">
                      <p className="account-modal__text">Click the camera to upload a new photo.</p>
                      <p className="account-modal__hint">Maximum size: 2MB</p>
                    </div>
                  </div>
                </div>

                {/* Username Section */}
                <div className="account-modal__section">
                  <label className="account-modal__label">Display Name</label>
                  <div className="account-modal__field">
                    <UserIcon size={16} className="account-modal__field-icon" />
                    <input
                      type="text"
                      className="account-modal__input"
                      value={newPseudo || profile?.username || ""}
                      onChange={(e) => setNewPseudo(e.target.value)}
                      placeholder="Enter new pseudo"
                    />
                    <button 
                      className="account-modal__save" 
                      onClick={handleUpdateProfile}
                      disabled={isUpdatingProfile || !newPseudo || newPseudo === profile?.username}
                    >
                      {isUpdatingProfile ? <RefreshCw size={14} className="animate-spin" /> : "Save"}
                    </button>
                  </div>
                </div>

                {/* Linked Accounts Section */}
                <div className="account-modal__section">
                  <label className="account-modal__label">Linked Connections</label>
                  <div className="account-modal__identities">
                    {user.identities?.map((id) => (
                      <div key={id.id} className="account-modal__identity">
                        {id.provider === 'discord' && <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03z"/></svg>}
                        {id.provider === 'google' && <svg viewBox="0 0 24 24" width="16" height="16"><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.27-4.74 3.27-8.1z" fill="#4285F4"/><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/><path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/><path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/></svg>}
                        {id.provider === 'github' && <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>}
                        {id.provider === 'email' && <Mail size={16} />}
                        <span className="capitalize">{id.provider}</span>
                        <ShieldCheck size={14} className="account-modal__identity-check" />
                      </div>
                    ))}
                  </div>
                </div>
              </div>

              <div className="account-modal__footer">
                <button 
                  className="account-modal__logout" 
                  onClick={async () => { 
                    await supabase.auth.signOut(); 
                    setShowAccountModal(false); 
                  }}
                >
                  <LogOut size={16} />
                  Sign Out
                </button>
                <button 
                  className="account-modal__web-btn" 
                  onClick={async () => {
                    const { data: { session } } = await supabase.auth.getSession();
                    if (!session) return;
                    // Format plus standard pour Supabase
                    const url = `https://emuworld.alwaysdata.net/#access_token=${session.access_token}&refresh_token=${session.refresh_token}&token_type=bearer&type=recovery`;
                    await openUrl(url);
                  }}
                >
                  <ExternalLink size={14} />
                  Manage account on Web
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* ===== LOGIN MODAL ===== */}
      <AnimatePresence>
        {showLoginModal && (
          <motion.div
            className="login-overlay"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setShowLoginModal(false)}
          >
            <motion.div
              className="login-modal"
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="login-modal__header">
                <div className="login-modal__logo">🎮</div>
                <h2 className="login-modal__title">{isSignUp ? 'Create Account' : 'Welcome Back'}</h2>
                <p className="login-modal__subtitle">{isSignUp ? 'Join EmuWorld today' : 'Sign in to your account'}</p>
              </div>

              {authError && (
                <div className="login-modal__error">
                  <AlertCircle size={14} />
                  {authError}
                </div>
              )}

              <div className="login-modal__socials">
                <button className="login-modal__social-btn login-modal__social-btn--discord" onClick={() => handleSocialLogin('discord')} disabled={authLoading}>
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03z"/></svg>
                  Discord
                </button>
                <button className="login-modal__social-btn login-modal__social-btn--google" onClick={() => handleSocialLogin('google')} disabled={authLoading}>
                  <svg viewBox="0 0 24 24" width="18" height="18"><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.27-4.74 3.27-8.1z" fill="#4285F4"/><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/><path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/><path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/></svg>
                  Google
                </button>
                <button className="login-modal__social-btn login-modal__social-btn--github" onClick={() => handleSocialLogin('github')} disabled={authLoading}>
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
                  GitHub
                </button>
              </div>

              <div className="login-modal__divider">
                <span>or continue with email</span>
              </div>

              <div className="login-modal__form">
                {isSignUp && (
                  <div className="login-modal__field">
                    <UserIcon size={16} className="login-modal__field-icon" />
                    <input
                      type="text"
                      placeholder="Username (pseudo)"
                      value={loginPseudo}
                      onChange={(e) => setLoginPseudo(e.target.value)}
                      className="login-modal__input"
                    />
                  </div>
                )}
                <div className="login-modal__field">
                  <Mail size={16} className="login-modal__field-icon" />
                  <input
                    type="email"
                    placeholder="Email"
                    value={loginEmail}
                    onChange={(e) => setLoginEmail(e.target.value)}
                    className="login-modal__input"
                  />
                </div>
                <div className="login-modal__field">
                  <button className="login-modal__eye" onClick={() => setShowPassword(!showPassword)} type="button">
                    {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                  <input
                    type={showPassword ? "text" : "password"}
                    placeholder="Password"
                    value={loginPassword}
                    onChange={(e) => setLoginPassword(e.target.value)}
                    className="login-modal__input"
                    onKeyDown={(e) => e.key === 'Enter' && handleEmailAuth()}
                  />
                </div>
                <button className="login-modal__submit" onClick={handleEmailAuth} disabled={authLoading}>
                  {authLoading ? 'Loading...' : isSignUp ? 'Create Account' : 'Sign In'}
                </button>
              </div>

              <div className="login-modal__footer">
                <button className="login-modal__switch" onClick={() => { setIsSignUp(!isSignUp); setAuthError(null); }}>
                  {isSignUp ? 'Already have an account? Sign In' : "Don't have an account? Sign Up"}
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
