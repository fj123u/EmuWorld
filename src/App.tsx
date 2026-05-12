import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { motion, AnimatePresence } from "framer-motion";
import { supabase, Profile } from "./supabase";
import type { User, Provider } from "@supabase/supabase-js";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { useTranslation, type Locale } from "./i18n";
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
  Copy,
  Lock,
  Globe,
  Package,
  Activity,
  Trophy,
  Cloud,
  Upload,
  LayoutGrid,
  List,
  StickyNote,
  Palette,
  Users,
  UserPlus,
  UserCheck,
  UserX,
  MessageCircle,
  Send,
} from "lucide-react";

/* ============================
   Clock — titlebar date/time widget. Updates every 30 s so the minute
   stays in sync without spinning a per-second interval.
   ============================ */
function Clock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(id);
  }, []);
  const loc = (localStorage.getItem("emuworld-locale") || "fr") === "en" ? "en-GB" : "fr-FR";
  const time = now.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" });
  const date = now.toLocaleDateString(loc, { day: "2-digit", month: "short" }).replace(".", "");
  return (
    <div className="titlebar__clock" data-tauri-drag-region>
      <span className="titlebar__clock-time">{time}</span>
      <span className="titlebar__clock-sep">·</span>
      <span className="titlebar__clock-date">{date}</span>
    </div>
  );
}

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

interface DownloadStats {
  progress: number;
  downloaded_bytes: number;
  total_bytes: number;
  speed_bps: number;
  eta: number;
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

interface GameEntry {
  console: string;
  name: string;
  seconds: number;
  launches: number;
  last_played: string | null;
  first_played: string | null;
  favorite: boolean;
  last_emulator_id: string | null;
  rating?: number;
  notes?: string;
}

interface GameCollection {
  name: string;
  games: string[];
}

interface PlaytimeStore {
  games: Record<string, GameEntry>;
  emulators: Record<string, number>;
  collections: GameCollection[];
}

interface ProfileStats {
  total_seconds: number;
  total_launches: number;
  games_played: number;
  favorite_count: number;
  most_played: GameEntry | null;
  favorite_game: GameEntry | null;
  top_games: GameEntry[];
  top_emulator_id: string | null;
  top_console: string | null;
  top_console_seconds: number;
  first_played: string | null;
  streak_days: number;
}
interface AchievementItem {
  id: string;
  name: string;
  description: string;
  icon: string;
  unlocked: boolean;
  unlocked_at: string | null;
  hidden: boolean;
}

interface AchievementRank {
  count: number;
  total: number;
  rank: string;
  icon: string;
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
  thumbnail_url?: string;
}

interface RgsConstructeur {
  id: string;
  nom: string;
  icon: string;
}

interface RgsConsole {
  id: string;
  nom: string;
  image: string;
  constructeur_id: string;
  nb_liens: number;
}

interface RgsLien {
  id: string;
  url: string;
  nb_fichiers: string;
  taille: string;
  mot_de_passe: string | null;
  createur: string;
  informations: string | null;
  dossier: string | null;
  is_signaled: string;
  date_creation: string | null;
}

interface RgsFile {
  nom: string;
  taille: string;
  url: string;
}

interface RgsSearchResult {
  id: string;
  nom: string;
  type_result: string;
  constructeur_nom?: string;
  image?: string;
  lien_id?: string;
  url?: string;
}

interface RAGameAchievement {
  id: number;
  title: string;
  description: string;
  points: number;
  badge_name: string;
  date_earned: string | null;
  date_earned_hardcore: string | null;
}

interface RAGameInfo {
  game_id: number;
  title: string;
  console_name: string;
  image_icon: string;
  num_achievements: number;
  achievements: RAGameAchievement[];
  num_earned: number;
  num_earned_hardcore: number;
}

interface RACompletedGame {
  game_id: number;
  title: string;
  console_name: string;
  image_icon: string;
  max_possible: number;
  num_awarded: number;
  hardcore_mode: boolean;
}

interface VimmConsole {
  id: string;
  name: string;
  image: string;
  manufacturer: string;
  target_console: string;
}

interface VimmGame {
  id: string;
  name: string;
  region: string;
  version: string;
  languages: string;
  rating: string;
  size: string;
  console_name: string;
  box_url: string;
  page_url: string;
}

interface GamepadButtonMapping {
  action: string;
  label: string;
  buttonIndex: number;
}

interface GamepadConfig {
  selectedIndex: number;
  deadzone: number;
  mappings: GamepadButtonMapping[];
}

const DEFAULT_GAMEPAD_MAPPINGS: GamepadButtonMapping[] = [
  { action: "confirm", label: "A", buttonIndex: 0 },
  { action: "back", label: "B", buttonIndex: 1 },
  { action: "details", label: "X", buttonIndex: 2 },
  { action: "favorite", label: "Y", buttonIndex: 3 },
  { action: "prevPage", label: "LB", buttonIndex: 4 },
  { action: "nextPage", label: "RB", buttonIndex: 5 },
  { action: "search", label: "Select", buttonIndex: 8 },
  { action: "settings", label: "Start", buttonIndex: 9 },
];

const GAMEPAD_ACTIONS_FR: Record<string, string> = {
  confirm: "Confirmer / Lancer",
  back: "Retour",
  details: "Détails",
  favorite: "Favori",
  prevPage: "Page précédente",
  nextPage: "Page suivante",
  search: "Recherche",
  settings: "Controller",
};

const GAMEPAD_ACTIONS_EN: Record<string, string> = {
  confirm: "Confirm / Launch",
  back: "Back",
  details: "Details",
  favorite: "Favorite",
  prevPage: "Previous page",
  nextPage: "Next page",
  search: "Search",
  settings: "Controller",
};

type Page = "catalog" | "library" | "installed" | "settings" | "changelogs" | "account" | "store" | "controller" | "backup" | "leaderboard" | "stats" | "friends";

/* Brand logos: use one "emblematic" console logo per brand so everything comes
   from the same source (RetroArch monochrome pack) and looks visually consistent.
   Falls back to a curated emoji when no console logo exists. */
const BRAND_REPRESENTATIVE_CONSOLE: Record<string, string | null> = {
  "Nintendo": "Nintendo - Nintendo Entertainment System",
  "Sony": "Sony - PlayStation",
  "Sega": "Sega - Mega Drive - Genesis",
  "Microsoft": "Microsoft - Xbox",
  "Atari": "Atari - 2600",
  "NEC": "NEC - PC Engine - TurboGrafx 16",
  "SNK": null,           // RetroArch monochrome has no clean SNK logo
  "Panasonic": "The 3DO Company - 3DO",
  "Commodore": null,
  "Arcade": null,
  "Arcade & Retro": null,
  "Multi-System": null,
};

const BRAND_EMOJI_FALLBACK: Record<string, string> = {
  "Nintendo": "🍄",
  "Sony": "🎮",
  "Sega": "🦔",
  "Microsoft": "❎",
  "Atari": "🕹️",
  "NEC": "🔶",
  "Panasonic": "💿",
  "Commodore": "💾",
  "SNK": "🅰️",
  "Arcade": "🕹️",
  "Arcade & Retro": "🕹️",
  "Multi-System": "🔄",
};

const BrandLogo = ({ brand, size = 32 }: { brand: string; size?: number }) => {
  const slug = BRAND_REPRESENTATIVE_CONSOLE[brand];
  const emoji = BRAND_EMOJI_FALLBACK[brand] || "🎮";
  const [failed, setFailed] = useState(false);

  if (!slug || failed) {
    return (
      <span
        className="brand-logo brand-logo--emoji"
        style={{ fontSize: size * 0.75 }}
        aria-label={brand}
      >
        {emoji}
      </span>
    );
  }
  return (
    <img
      className="brand-logo"
      src={`https://raw.githubusercontent.com/libretro/retroarch-assets/master/xmb/monochrome/png/${encodeURIComponent(slug)}.png`}
      alt={brand}
      style={{ height: size, width: size * 1.6, objectFit: "contain" }}
      onError={() => setFailed(true)}
    />
  );
};

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

/* RetroArch monochrome asset names, indexed by the user-facing console label.
   Source: https://github.com/libretro/retroarch-assets/tree/master/xmb/monochrome/png */
const CONSOLE_RETROARCH_SLUG: Record<string, string> = {
  "NES": "Nintendo - Nintendo Entertainment System",
  "Nes": "Nintendo - Nintendo Entertainment System",
  "Famicom": "Nintendo - Nintendo Entertainment System",
  "Super Nintendo": "Nintendo - Super Nintendo Entertainment System",
  "Super Nes": "Nintendo - Super Nintendo Entertainment System",
  "SNES": "Nintendo - Super Nintendo Entertainment System",
  "Super Famicom": "Nintendo - Super Nintendo Entertainment System",
  "Nintendo 64": "Nintendo - Nintendo 64",
  "N64": "Nintendo - Nintendo 64",
  "Game Boy": "Nintendo - Game Boy",
  "Game Boy Color": "Nintendo - Game Boy Color",
  "GBC": "Nintendo - Game Boy Color",
  "Game Boy Advance": "Nintendo - Game Boy Advance",
  "GBA": "Nintendo - Game Boy Advance",
  "Nintendo DS": "Nintendo - Nintendo DS",
  "NDS": "Nintendo - Nintendo DS",
  "Nintendo 3DS": "Nintendo - Nintendo 3DS",
  "3DS": "Nintendo - Nintendo 3DS",
  "GameCube": "Nintendo - GameCube",
  "Gamecube": "Nintendo - GameCube",
  "GameCube / Wii": "Nintendo - GameCube",
  "GameCube - Wii": "Nintendo - GameCube",
  "Wii": "Nintendo - Wii",
  "Wii U": "Nintendo - Wii U",
  "WiiU": "Nintendo - Wii U",
  "Nintendo Switch": "Nintendo - Switch",
  "Switch": "Nintendo - Switch",
  "Virtual Boy": "Nintendo - Virtual Boy",
  "VirtualBoy": "Nintendo - Virtual Boy",

  "PlayStation 1": "Sony - PlayStation",
  "PS1": "Sony - PlayStation",
  "PlayStation": "Sony - PlayStation",
  "PlayStation 2": "Sony - PlayStation 2",
  "PS2": "Sony - PlayStation 2",
  "PlayStation 3": "Sony - PlayStation 3",
  "PS3": "Sony - PlayStation 3",
  "PSP": "Sony - PlayStation Portable",
  "PlayStation Portable": "Sony - PlayStation Portable",
  "PlayStation Vita": "Sony - PlayStation Vita",
  "PS Vita": "Sony - PlayStation Vita",

  "Mega Drive": "Sega - Mega Drive - Genesis",
  "Genesis": "Sega - Mega Drive - Genesis",
  "Master System": "Sega - Master System - Mark III",
  "Game Gear": "Sega - Game Gear",
  "Saturn": "Sega - Saturn",
  "Dreamcast": "Sega - Dreamcast",
  "Sega 32X": "Sega - 32X",
  "32X": "Sega - 32X",
  "Sega CD": "Sega - Mega-CD - Sega CD",

  "Xbox": "Microsoft - Xbox",
  "Xbox 360": "Microsoft - Xbox 360",

  "Atari 2600": "Atari - 2600",
  "Atari 5200": "Atari - 5200",
  "Atari 7800": "Atari - 7800",
  "Jaguar": "Atari - Jaguar",
  "Lynx": "Atari - Lynx",

  "TurboGrafx-16": "NEC - PC Engine - TurboGrafx 16",
  "TG16": "NEC - PC Engine - TurboGrafx 16",
  "TurboGrafx-CD": "NEC - PC Engine CD - TurboGrafx-CD",
  "TGCD": "NEC - PC Engine CD - TurboGrafx-CD",

  "CD-i": "Philips - CD-i",
};

const ConsoleLogo = ({ name, size = 48 }: { name: string; size?: number }) => {
  const slug = CONSOLE_RETROARCH_SLUG[name];
  const emoji = CONSOLE_ICONS[name] || "🎮";
  const [failed, setFailed] = useState(false);

  if (!slug || failed) {
    return (
      <span
        className="console-logo console-logo--emoji"
        style={{ fontSize: size, lineHeight: 1 }}
        aria-label={name}
      >
        {emoji}
      </span>
    );
  }
  return (
    <img
      className="console-logo"
      src={`https://raw.githubusercontent.com/libretro/retroarch-assets/master/xmb/monochrome/png/${encodeURIComponent(slug)}.png`}
      alt={name}
      style={{ width: size * 1.6, height: size, objectFit: "contain" }}
      onError={() => setFailed(true)}
    />
  );
};

/* Short human-readable playtime (e.g. "1h 30m", "24m", "45s"). */
const formatPlaytime = (seconds: number): string => {
  if (!seconds) return "";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
};

/* ============================
   Components
   ============================ */

const RA_SUPPORTED_CONSOLES = new Set([
  "NES", "Super Nintendo", "Nintendo 64", "Game Boy Advance", "Game Boy",
  "Game Boy Color", "Nintendo DS", "Virtual Boy", "PlayStation 1",
  "PlayStation 2", "PlayStation Portable", "Mega Drive", "Master System",
  "Game Gear", "Saturn", "Dreamcast", "Arcade", "Neo-Geo", "PC Engine",
  "Atari 2600", "WonderSwan",
]);

const GameCard = ({ rom, onLaunch, onDelete, entry, onToggleFavorite, onOpenRA, onHover, onRate, onNotes, onContextMenu }: {
  rom: RomFile,
  onLaunch: (rom: RomFile) => void,
  onDelete: (rom: RomFile) => void,
  entry?: GameEntry,
  onToggleFavorite?: (rom: RomFile) => void,
  onOpenRA?: (rom: RomFile) => void,
  onHover?: (coverUrl: string | null) => void,
  onRate?: (rom: RomFile, rating: number) => void,
  onNotes?: (rom: RomFile) => void,
  onContextMenu?: (rom: RomFile, x: number, y: number) => void,
}) => {
  const [cover, setCover] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchCover = useCallback(async (forceRefresh = false) => {
    try {
      setLoading(true);
      const dataUrl: string = await invoke("fetch_boxart", {
        gameName: rom.name,
        console: rom.console,
        forceRefresh,
      });
      setCover(dataUrl);
    } catch (e) {
      // No cover available, placeholder will show
    } finally {
      setLoading(false);
    }
  }, [rom.name, rom.console]);

  useEffect(() => {
    fetchCover();
  }, [fetchCover]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      whileHover={{ scale: 1.02, y: -4 }}
      whileTap={{ scale: 0.98 }}
      className="game-card"
      data-rom-path={rom.path}
      onMouseEnter={() => onHover?.(cover)}
      onMouseLeave={() => onHover?.(null)}
      onContextMenu={(e) => { e.preventDefault(); onContextMenu?.(rom, e.clientX, e.clientY); }}
    >
      <div className={`game-card__cover ${loading ? 'game-card__cover--loading' : ''}`} onClick={() => onLaunch(rom)}>
        {cover ? (
          <img src={cover} alt={rom.name} />
        ) : !loading ? (
          <div className="game-card__placeholder">
            <span className="game-card__placeholder-icon">🎮</span>
            <div className="game-card__placeholder-title">{rom.name}</div>
            <button
              className="game-card__retry"
              onClick={(e) => { e.stopPropagation(); fetchCover(true); }}
              title="Retry cover fetch (bypass cache)"
            >
              <RefreshCw size={12} /> Retry
            </button>
          </div>
        ) : null}
        <div className="game-card__overlay">
          <Play size={24} fill="currentColor" />
        </div>
      </div>

      {/* Favorite toggle */}
      {onToggleFavorite && (
        <button
          className={`game-card__fav ${entry?.favorite ? "game-card__fav--active" : ""}`}
          onClick={(e) => { e.stopPropagation(); onToggleFavorite(rom); }}
          title={entry?.favorite ? "Remove from favorites" : "Add to favorites"}
        >
          {entry?.favorite ? "★" : "☆"}
        </button>
      )}

      {/* Delete Button */}
      <button
        className="game-card__delete"
        onClick={(e) => {
          e.stopPropagation();
          if (confirm(`Delete ${rom.name}?`)) {
            onDelete(rom);
          }
        }}
        title="Delete ROM"
      >
        <Trash2 size={14} />
      </button>

      {/* Notes Button */}
      {onNotes && (
        <button
          className="game-card__notes"
          onClick={(e) => { e.stopPropagation(); onNotes(rom); }}
          title="Notes"
        >
          <StickyNote size={14} />
          {entry?.notes && <span className="game-card__notes-dot" />}
        </button>
      )}

      {/* RetroAchievements Button */}
      {onOpenRA && RA_SUPPORTED_CONSOLES.has(rom.console) && (
        <button
          className="game-card__ra"
          onClick={(e) => { e.stopPropagation(); onOpenRA(rom); }}
          title="RetroAchievements"
        >
          <Trophy size={14} />
        </button>
      )}

      {/* Playtime badge */}
      {entry && entry.seconds > 0 && (
        <div className="game-card__playtime" title={`${entry.launches} launch${entry.launches === 1 ? "" : "es"}`}>
          ⏱ {formatPlaytime(entry.seconds)}
        </div>
      )}

      <div className="game-card__info" onClick={() => onLaunch(rom)}>
        <div className="game-card__name">{rom.name}</div>
        <div className="game-card__rating" onClick={(e) => e.stopPropagation()}>
          {[1, 2, 3, 4, 5].map((star) => (
            <span
              key={star}
              className={`game-card__star ${(entry?.rating ?? 0) >= star ? "game-card__star--filled" : ""}`}
              onClick={() => onRate?.(rom, (entry?.rating ?? 0) === star ? 0 : star)}
            >★</span>
          ))}
        </div>
        <div className="game-card__meta">{rom.console} • {rom.extension.toUpperCase()}</div>
      </div>
    </motion.div>
  );
};

const BigPictureCard = ({ rom, focused, entry, onLaunch }: {
  rom: RomFile;
  focused: boolean;
  entry?: GameEntry;
  onLaunch: () => void;
}) => {
  const [cover, setCover] = useState<string | null>(null);

  useEffect(() => {
    invoke<string>("fetch_boxart", { gameName: rom.name, console: rom.console })
      .then(setCover)
      .catch(() => {});
  }, [rom.name, rom.console]);

  return (
    <motion.div
      className={`bp-card gamepad-nav-item ${focused ? "bp-card--focused" : ""}`}
      onClick={onLaunch}
      whileHover={{ scale: 1.04 }}
      whileTap={{ scale: 0.96 }}
    >
      <div className="bp-card__cover">
        {cover ? (
          <img src={cover} alt={rom.name} />
        ) : (
          <div className="bp-card__placeholder">
            <span>🎮</span>
          </div>
        )}
      </div>
      <div className="bp-card__info">
        <div className="bp-card__name">{rom.name}</div>
        {entry && entry.seconds > 0 && (
          <div className="bp-card__playtime">⏱ {formatPlaytime(entry.seconds)}</div>
        )}
      </div>
    </motion.div>
  );
};

const RomStoreCard = ({ rom, onDownload, downloading, downloaded, stats }: {
  rom: RomStoreEntry,
  onDownload: (rom: RomStoreEntry) => void,
  downloading: boolean,
  downloaded: boolean,
  stats?: DownloadStats
}) => {
  const [cover, setCover] = useState<string | null>(null);
  const [loadingCover, setLoadingCover] = useState(false);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const formatTime = (seconds: number) => {
    if (seconds <= 0) return 'calculating...';
    if (seconds < 60) return `${seconds}s left`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s left`;
  };

  useEffect(() => {
    const fetchCover = async () => {
      // If we already have a direct site thumbnail from IA, use it immediately
      if (rom.thumbnail_url) return;

      try {
        setLoadingCover(true);
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
  }, [rom.name, rom.console, rom.thumbnail_url]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      className="store-card"
    >
      <div className="store-card__cover">
        {cover || rom.thumbnail_url ? (
          <img src={cover || rom.thumbnail_url} alt={rom.name} className="store-card__img" />
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
            <div className="download-stats-container">
              <div className="download-stats-row">
                <RefreshCw size={12} className="animate-spin" />
                <span>{stats?.progress || 0}%</span>
                <span className="download-stats-divider">•</span>
                <span>{formatTime(stats?.eta || 0)}</span>
              </div>
              <div className="download-stats-subtext">
                {formatBytes(stats?.downloaded_bytes || 0)} / {formatBytes(stats?.total_bytes || 0)} 
                {stats?.speed_bps ? ` • ${formatBytes(stats.speed_bps)}/s` : ""}
              </div>
            </div>
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
  const [boxartLogs, setBoxartLogs] = useState<{ game: string; url: string; status: string; error?: string }[]>([]);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<{ game: string; url: string; status: string; error?: string }>(
      "boxart-log",
      (event) => {
        setBoxartLogs((prev) => [...prev, event.payload].slice(-50));
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [boxartLogs]);

  const { t, locale, setLocale } = useTranslation();

  const [page, setPage] = useState<Page>("catalog");
  const [catalog, setCatalog] = useState<EmulatorInfo[]>([]);
  const [installed, setInstalled] = useState<string[]>([]);
  const [roms, setRoms] = useState<RomFile[]>([]);
  // Playtime / profile
  const [playtime, setPlaytime] = useState<PlaytimeStore>({ games: {}, emulators: {}, collections: [] });
  const [profileStats, setProfileStats] = useState<ProfileStats | null>(null);
  const [achievements, setAchievements] = useState<AchievementItem[]>([]);
  const [achievementRank, setAchievementRank] = useState<AchievementRank>({ count: 0, total: 23, rank: "Bronze", icon: "🥉" });
  const [config, setConfig] = useState<AppConfig>({
    roms_directory: "",
    emulators_directory: "",
    covers_directory: "",
  });
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingStep, setOnboardingStep] = useState(0);
  const [tourStep, setTourStep] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [consoleFilter, setConsoleFilter] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  const [sortBy, setSortBy] = useState<"name" | "playtime" | "rating" | "last_played" | "launches">("name");
  const [filterMode, setFilterMode] = useState<"all" | "favorites" | "unplayed" | "rated">("all");
  const [collectionFilter, setCollectionFilter] = useState<string | null>(null);
  const [showCollectionModal, setShowCollectionModal] = useState(false);
  const [theme, setThemeState] = useState<string>(() => localStorage.getItem("emuworld_theme") || "default");
  const [accentHue, setAccentHueState] = useState<number | null>(() => {
    const saved = localStorage.getItem("emuworld_accent_hue");
    return saved ? parseInt(saved) : null;
  });
  const setTheme = (t: string) => {
    setThemeState(t);
    localStorage.setItem("emuworld_theme", t);
    if (t === "default") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  };
  const setAccentHue = (hue: number | null) => {
    setAccentHueState(hue);
    if (hue === null) {
      localStorage.removeItem("emuworld_accent_hue");
      document.documentElement.removeAttribute("data-accent-hue");
      document.documentElement.style.removeProperty("--custom-hue");
    } else {
      localStorage.setItem("emuworld_accent_hue", String(hue));
      document.documentElement.setAttribute("data-accent-hue", "");
      document.documentElement.style.setProperty("--custom-hue", String(hue));
    }
  };
  useEffect(() => {
    if (theme !== "default") document.documentElement.setAttribute("data-theme", theme);
    if (accentHue !== null) {
      document.documentElement.setAttribute("data-accent-hue", "");
      document.documentElement.style.setProperty("--custom-hue", String(accentHue));
    }
  }, []);

  const [bigPictureMode, setBigPictureMode] = useState(false);
  const [bpSelectedIndex, setBpSelectedIndex] = useState(0);
  const [bpConsoleFilter, setBpConsoleFilter] = useState<string | null>(null);
  const [newCollectionName, setNewCollectionName] = useState("");
  const [romContextMenu, setRomContextMenu] = useState<{ rom: RomFile; x: number; y: number } | null>(null);
  const [expandedCategories, setExpandedCategories] = useState<string[]>([]);
  const [expandedSidebarConsoles, setExpandedSidebarConsoles] = useState<string[]>([]);
  const [expandedLibraryCategories, setExpandedLibraryCategories] = useState<string[]>(["NINTENDO", "SONY", "SEGA", "MICROSOFT"]);
  const [installing, setInstalling] = useState<string[]>([]);
  const [activeLibraryFilter, setActiveLibraryFilter] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [sessionRecap, setSessionRecap] = useState<{
    gameName: string;
    console: string;
    sessionSeconds: number;
    totalSeconds: number;
    totalLaunches: number;
  } | null>(null);

  // ---- Launch splash ----
  const [launchSplash, setLaunchSplash] = useState<{ gameName: string; console: string } | null>(null);

  // ---- App update state ----
  // `null` = not checked yet / no update, object = newer version available
  const [updateAvailable, setUpdateAvailable] = useState<{ version: string } | null>(null);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "downloading" | "ready" | "error">("idle");
  const [updateProgress, setUpdateProgress] = useState<{ done: number; total: number | null }>({ done: 0, total: null });

  // ---- ROM Store state ----
  const [storeRoms, setStoreRoms] = useState<RomStoreEntry[]>([]);
  const [storeSearch, setStoreSearch] = useState("");
  const [debouncedStoreSearch, setDebouncedStoreSearch] = useState("");
  const [storeConsoleFilter, setStoreConsoleFilter] = useState<string | null>(null);
  const [storeConsoles, setStoreConsoles] = useState<string[]>([]);
  const [downloading, setDownloading] = useState<string[]>([]);
  const [downloaded, setDownloaded] = useState<string[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadStats>>({});
  const [downloadNames, setDownloadNames] = useState<Record<string, string>>({});
  const [isSearchingStore, setIsSearchingStore] = useState(false);

  // ---- RetroAchievements state ----
  const [raUsername, setRaUsername] = useState("");
  const [raApiKey, setRaApiKey] = useState("");
  const [raModalRom, setRaModalRom] = useState<RomFile | null>(null);
  const [raGameInfo, setRaGameInfo] = useState<RAGameInfo | null>(null);
  const [raLoading, setRaLoading] = useState(false);
  const [raCompletedGames, setRaCompletedGames] = useState<RACompletedGame[]>([]);
  const [raProfileLoading, setRaProfileLoading] = useState(false);
  const [raPassword, setRaPassword] = useState("");
  const [raLoginLoading, setRaLoginLoading] = useState(false);
  const [raToken, setRaToken] = useState("");
  const [raDownloadingCores, setRaDownloadingCores] = useState(false);

  // ---- Cloud Backup state ----
  const [b2KeyId, setB2KeyId] = useState("");
  const [b2AppKey, setB2AppKey] = useState("");
  const [b2BucketId, setB2BucketId] = useState("");
  const [b2BucketName, setB2BucketName] = useState("");
  const [localSaves, setLocalSaves] = useState<{ emulator: string; game_name: string; file_name: string; size: number; modified: string }[]>([]);
  const [cloudBackups, setCloudBackups] = useState<{ file_name: string; file_id: string; size: number; upload_timestamp: number }[]>([]);
  const [backupLoading, setBackupLoading] = useState(false);

  // ---- Leaderboard state ----
  interface LeaderboardEntry {
    user_id: string;
    username: string;
    avatar_url: string | null;
    week_seconds: number;
    week_games: number;
    total_seconds: number;
    total_launches: number;
  }
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [leaderboardLoading, setLeaderboardLoading] = useState(false);

  // ---- Friends system ----
  interface Friendship {
    id: string;
    requester_id: string;
    addressee_id: string;
    status: "pending" | "accepted" | "blocked";
    created_at: string;
    profile?: { username: string | null; avatar_url: string | null };
  }
  interface PresenceEntry {
    user_id: string;
    status: "online" | "playing" | "offline";
    current_game: string | null;
    current_console: string | null;
    updated_at: string;
  }
  const [friends, setFriends] = useState<Friendship[]>([]);
  const [pendingRequests, setPendingRequests] = useState<Friendship[]>([]);
  const [friendPresences, setFriendPresences] = useState<PresenceEntry[]>([]);
  const [friendSearch, setFriendSearch] = useState("");
  const [friendSearchResults, setFriendSearchResults] = useState<{ id: string; username: string; avatar_url: string | null }[]>([]);
  const [friendProfile, setFriendProfile] = useState<{
    id: string;
    username: string;
    avatar_url: string | null;
    totalSeconds: number;
    totalLaunches: number;
    gamesPlayed: number;
    topGames: { name: string; console: string; seconds: number }[];
    topConsoles: { name: string; seconds: number }[];
    achievements: number;
  } | null>(null);
  const [friendsLoading, setFriendsLoading] = useState(false);

  // ---- Chat state ----
  interface ChatMessage {
    id: string;
    sender_id: string;
    receiver_id: string;
    content: string;
    created_at: string;
    read_at: string | null;
  }
  const [chatOpen, setChatOpen] = useState<{ id: string; username: string; avatar_url: string | null } | null>(null);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [chatLoading, setChatLoading] = useState(false);
  const [unreadCounts, setUnreadCounts] = useState<Record<string, number>>({});
  const chatEndRef = useRef<HTMLDivElement>(null);

  // ---- Dynamic background ----
  const [bgCover, setBgCover] = useState<string | null>(null);

  // ---- Store state ----
  const [storeMode, setStoreMode] = useState<"rgs" | "archive" | "vimm">("vimm");

  // ---- Vimm's Lair state ----
  const [vimmConsoles, setVimmConsoles] = useState<VimmConsole[]>([]);
  const [selectedVimmConsole, setSelectedVimmConsole] = useState<VimmConsole | null>(null);
  const [selectedVimmManufacturer, setSelectedVimmManufacturer] = useState<string | null>(null);
  const [vimmGames, setVimmGames] = useState<VimmGame[]>([]);
  const [vimmLoading, setVimmLoading] = useState(false);
  const [vimmSearch, setVimmSearch] = useState("");
  const [rgsConstructeurs, setRgsConstructeurs] = useState<RgsConstructeur[]>([]);
  const [rgsConsoles, setRgsConsoles] = useState<RgsConsole[]>([]);
  const [rgsLiens, setRgsLiens] = useState<RgsLien[]>([]);
  const [rgsFolderFiles, setRgsFolderFiles] = useState<RgsFile[]>([]);
  const [rgsLoading, setRgsLoading] = useState(false);
  const [rgsSearchQuery, setRgsSearchQuery] = useState("");
  const [rgsSearchResults, setRgsSearchResults] = useState<RgsSearchResult[]>([]);
  const [isSearchingRgs, setIsSearchingRgs] = useState(false);
  const [selectedRgsLien, setSelectedRgsLien] = useState<RgsLien | null>(null);
  const [rgsFolderSearch, setRgsFolderSearch] = useState("");
  const [selectedConstructeur, setSelectedConstructeur] = useState<string | null>(null);
  const [selectedConstructeurName, setSelectedConstructeurName] = useState<string | null>(null);
  const [selectedRgsConsole, setSelectedRgsConsole] = useState<string | null>(null);
  const [selectedRgsConsoleName, setSelectedRgsConsoleName] = useState<string | null>(null);
  const [pendingImportConsole, setPendingImportConsole] = useState<string | null>(null);
  const [changelogs] = useState<ChangelogEntry[]>([
    { version: "1.8.0", date: "2026-05-11", changes: [
      "📦 Import/Export config: sauvegarder toute sa config + playtime en JSON pour restaurer sur un autre PC",
      "🔔 Notifications Windows natives: toast quand un achievement est débloqué, un download terminé, ou une session finie",
      "⌨️ Raccourcis clavier: Ctrl+Shift+L relance le dernier jeu, Ctrl+K focus la recherche",
    ] },
    { version: "1.7.0", date: "2026-05-11", changes: [
      "📊 Page Statistiques: temps total, launches, streak, top 8 jeux, top 6 consoles, heatmap 12 semaines",
      "📁 Collections custom: créer des playlists de jeux (RPGs, Backlog, etc.), ajouter/retirer via clic droit",
      "🖱️ Menu contextuel: clic droit sur un jeu → Jouer, Favori, Ajouter à une collection, Supprimer",
    ] },
    { version: "1.6.0", date: "2026-05-11", changes: [
      "📋 Vue liste/grille: basculer entre covers et tableau (nom, console, temps joué, note)",
      "🔀 Tri avancé: trier par A-Z, temps joué, note, dernier joué, nb de launches",
      "🔍 Filtres: afficher tous / favoris / non-joués / notés",
      "📝 Notes par jeu: champ texte libre pour codes, astuces, progression (icône StickyNote sur la card)",
      "⭐ Étoiles: noter chaque jeu de 1 à 5 étoiles, visible dans la grille et la liste",
      "☁️ Rating & notes synchronisés sur Supabase avec le reste du playtime"
    ] },
    { version: "1.5.0", date: "2026-05-10", changes: [
      "🎆 Fond d'écran dynamique: la cover du jeu survolé s'affiche en arrière-plan avec blur",
      "🚀 Animation de lancement: splash plein écran avec icône console + barre de progression",
      "🏆 Leaderboard in-app: classement hebdo par temps joué entre tous les utilisateurs",
      "📊 Session recap: popup détaillé à la fermeture d'un émulateur (durée, launches, total)",
      "🎮 Navigation manette étendue à toutes les pages (leaderboard, backup, settings, changelogs)",
      "☁️ Backup cloud: scan élargi (AppData/Roaming pour Ryujinx, Dolphin, Cemu, etc.)",
      "🗑️ Suppression de backups cloud depuis l'interface",
      "✨ Auto-update fonctionnel: bouton 'Mise à jour' dans la titlebar + install en 1 clic"
    ] },
    { version: "1.4.1", date: "2026-05-09", changes: [
      "🖼️ Fix covers Wii U: noms exacts libretro hardcodés pour les jeux populaires",
      "🖼️ Fix covers web panel: ajout suffixes région (USA/Europe/World) + inversion 'The'",
      "🖼️ Fix covers GBA: fallback vers GBC/GB pour les jeux Pokémon rétro",
      "🔧 Fix warning Rust unused_mut dans generate_search_candidates",
      "🌐 Covers GameTDB pour Wii/Wii U/DS/3DS sur le profil web"
    ] },
    { version: "1.4.0", date: "2026-05-05", changes: [
      "🎮 Navigation manette complète: D-pad/stick pour naviguer, A pour confirmer, B pour retour",
      "📋 Menu contextuel manette: A sur un jeu → Jouer/Favori/Supprimer, sur un émulateur → Installer-Lancer/Désinstaller/Site web",
      "⌨️ Clavier virtuel: appuyer A sur une barre de recherche ouvre un clavier navigable à la manette",
      "🔧 Remapping des touches manette avec détection anti-conflit",
      "🕹️ Détection native via Rust (gilrs) — compatible Xbox, PlayStation, Switch Pro Controller",
      "🛒 Store simplifié: Vimm's Lair + RetroGameSets, suppression de Myrient",
      "🐛 Fix lancement Wii U: Cemu reçoit maintenant le flag -g pour charger le jeu directement"
    ] },
    { version: "1.3.0", date: "2026-05-03", changes: [
      "🏆 Achievements: 33 succès (21 milestones + 12 cachés) avec détection en temps réel",
      "🎖️ Badge de rang à côté de la photo de profil (Bronze → Argent → Or → Platine → Diamant)",
      "☁️ Synchronisation cloud des achievements via Supabase",
      "🌐 Achievements visibles sur le profil web avec rareté % et indices pour les secrets",
      "🎮 Store: téléchargement de ROMs à l'unité via Vimm's Lair",
      "🦉 Succès cachés uniques: Oiseau de nuit, Speed Runner, Marathon, et plus"
    ] },
    { version: "1.2.0", date: "2026-04-28", changes: [
      "📊 Gaming Profile: total playtime, launches, games played and day streak now show in the Account panel",
      "🏆 Stats: most-played game, favorite, top emulator, top console and first-played date, with a top-5 podium",
      "⏱ Every session is tracked locally (stored in playtime.json) — the emulator exit time is the source of truth, no clock-in/out needed",
      "★ Favorite toggle on every game card + a subtle playtime badge once you've launched it",
      "🎨 Real console logos throughout the app (NES, SNES, Switch, PlayStation, Xbox, Genesis, Dreamcast…) from RetroArch's asset pack",
      "🗂 Drill-down navigation on both Roms and Console pages (Manufacturer → Console → content), mirroring the store"
    ] },
    { version: "1.1.1", date: "2026-04-27", changes: [
      "🔎 Store: replaced the A-Z alphabet with a real search bar — type a name, results appear instantly",
      "🎨 Cover art: Wii & Wii U now use the front-only box art (no more wrap-around jackets)",
      "🌐 Vimm search can be scoped to the current console or run globally when no console is picked"
    ] },
    { version: "1.1.0", date: "2026-04-27", changes: [
      "🎮 New Store: Vimm's Lair for individual game downloads",
      "🔀 Dual-source Store: toggle between Individual games (Vimm's Lair) and Complete packs (RetroGameSets)",
      "🖼️ Cover fix: Wii & Wii U covers now load correctly (GameTDB format per console, proper disc IDs)",
      "🧠 Smarter cover matching: composite titles (A & B), title-case fallback, franchise aliases",
      "🔁 Retry button on missing covers — force a fresh fetch without restarting the app",
      "🧹 Removed an unused_mut Rust warning"
    ] },
    { version: "1.0.0", date: "2026-04-20", changes: ["🚀 Automated RGS Imports: Automatic moving, unzipping, and cleanup", "📦 Switch Mastery: Full .xci/.nsp support with instant disk relocation", "🧹 Streamlined UI: Removed Archive.org to focus on RetroGameSets", "🛠️ Improved folder-view download triggers and file picker filters"] },
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

  // ---- Gamepad / Controller state ----
  const [gamepadConfig, setGamepadConfig] = useState<GamepadConfig>({
    selectedIndex: 0,
    deadzone: 0.15,
    mappings: [...DEFAULT_GAMEPAD_MAPPINGS],
  });
  const [gamepadActive, setGamepadActive] = useState(false);
  const [gamepadName, setGamepadName] = useState("");
  const [focusIndex, setFocusIndex] = useState(0);
  const [remappingAction, setRemappingAction] = useState<string | null>(null);
  const [gamepadContextMenu, setGamepadContextMenu] = useState<{ type: "rom"; romPath: string; x: number; y: number } | { type: "emu"; emuId: string; x: number; y: number } | null>(null);
  const [gamepadKeyboard, setGamepadKeyboard] = useState<{ inputEl: HTMLInputElement; keyIdx: number } | null>(null);
  const [gamepadTick, setGamepadTick] = useState(0);
  const lastGamepadButtonsRef = useRef<boolean[]>([]);
  const gamepadActiveRef = useRef(false);
  const lastAxisRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // Synchronize 'downloaded' state with locally scanned 'roms'
  useEffect(() => {
    if (roms.length === 0 && storeRoms.length === 0) return;

    // Efficiently identify which Store ROMs are present locally
    const downloadedIds = storeRoms
      .filter(sr => {
        // Standardize names for comparison
        const srNameLower = sr.name.toLowerCase();
        return roms.some(local => {
          const localNameLower = local.name.toLowerCase();
          const localPathLower = local.path.toLowerCase();
          const srFileLower = sr.file_name.toLowerCase();
          
          return localNameLower === srNameLower || 
                 (srFileLower && localPathLower.includes(srFileLower)) ||
                 (localNameLower.includes(srNameLower) && local.console === sr.console);
        });
      })
      .map(sr => sr.id);
    
    setDownloaded(downloadedIds);
  }, [roms, storeRoms]);

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
  // Cache-buster for the avatar <img>. Only changes when the user uploads a
  // new avatar — otherwise the browser can cache the URL across renders and
  // we stop re-downloading the same file from Supabase every frame.
  const [avatarCacheKey, setAvatarCacheKey] = useState<string>("");
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

  // Check session. Realtime postgres_changes were removed — the login-modal
  // banner ("cannot add postgres_changes callbacks after subscribe()") came
  // from Strict Mode re-mounting the effect and trying to attach a second
  // listener to the same channel. We already call fetchProfile() manually
  // after every mutation (username, avatar, public toggle), so the
  // realtime subscription added no real value.
  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      if (session?.user) {
        setUser(session.user);
        fetchProfile(session.user.id);
      }
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      const currentUser = session?.user ?? null;
      setUser(currentUser);
      if (currentUser) {
        fetchProfile(currentUser.id);
        const provider = currentUser.app_metadata?.provider;
        if (provider === "discord") triggerHiddenAchievement("login_discord");
        if (provider === "google") triggerHiddenAchievement("login_google");
      } else {
        setProfile(null);
      }
    });

    return () => {
      subscription.unsubscribe();
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

      // 1. Upload to Storage. We set a 1-year cacheControl because the file is
      // named with a random suffix — it's immutable, so browsers and CDN can
      // hold onto it indefinitely and we stop paying egress on every view.
      const { error: uploadError } = await supabase.storage
        .from('avatars')
        .upload(filePath, file, { cacheControl: '31536000', upsert: false });

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

      // Only bump the cache key on actual upload, so the <img> fetches the new
      // file exactly once and the browser can cache subsequent renders.
      setAvatarCacheKey(Date.now().toString());
      await fetchProfile(user.id);
      showToast('Avatar updated! 📸', 'success');
      triggerHiddenAchievement("change_avatar");
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
      // Redirect through a lightweight web page that forwards the tokens
      // back to the desktop app via the emuworld:// scheme. Going straight
      // to emuworld:// leaves the browser tab stuck on an unreachable URL,
      // so the bounce page shows a "you can close this tab" UI instead.
      const { data, error } = await supabase.auth.signInWithOAuth({
        provider,
        options: {
          redirectTo: 'https://emuworld.alwaysdata.net/auth-callback.html',
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
    loadData().then(() => {
      if (!localStorage.getItem("emuworld_onboarding_done")) {
        setShowOnboarding(true);
      }
    });
  }, [loadData]);

  useEffect(() => {
    isPermissionGranted().then(granted => {
      if (!granted) requestPermission();
    });
  }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+Shift+L → relaunch last played game
      if (e.ctrlKey && e.shiftKey && e.key === "L") {
        e.preventDefault();
        const allGames = Object.values(playtime.games || {});
        const lastGame = allGames
          .filter(g => g.last_played)
          .sort((a, b) => (b.last_played || "").localeCompare(a.last_played || ""))[0];
        if (lastGame) {
          const rom = roms.find(r => r.console === lastGame.console && r.name === lastGame.name);
          if (rom) handleLaunch(rom);
        }
      }
      // Ctrl+K → focus search bar
      if (e.ctrlKey && e.key === "k") {
        e.preventDefault();
        const input = document.querySelector<HTMLInputElement>(".search-bar input");
        if (input) input.focus();
      }
      // F11 → toggle fullscreen
      if (e.key === "F11") {
        e.preventDefault();
        const win = getCurrentWindow();
        win.isFullscreen().then(fs => {
          win.setFullscreen(!fs);
          setIsFullscreen(!fs);
        }).catch(() => {});
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [playtime, roms]);

  // ---- Load B2 config ----
  useEffect(() => {
    invoke<{ key_id: string; app_key: string; bucket_id: string; bucket_name: string }>("get_b2_config").then(cfg => {
      setB2KeyId(cfg.key_id || "");
      setB2AppKey(cfg.app_key || "");
      setB2BucketId(cfg.bucket_id || "");
      setB2BucketName(cfg.bucket_name || "");
    }).catch(() => {});
  }, []);

  // ---- Load RA credentials ----
  useEffect(() => {
    invoke<{ username: string; api_key: string; token: string }>("get_ra_credentials").then(creds => {
      setRaUsername(creds.username || "");
      setRaApiKey(creds.api_key || "");
      setRaToken(creds.token || "");
    }).catch(() => {});
  }, []);

  const handleSaveRaCredentials = useCallback(async () => {
    try {
      await invoke("save_ra_credentials", { username: raUsername, apiKey: raApiKey });
      showToast("RetroAchievements credentials saved!", "success");
    } catch (e: any) {
      showToast(`Failed to save: ${e}`, "error");
    }
  }, [raUsername, raApiKey, showToast]);

  const handleOpenRaModal = useCallback(async (rom: RomFile) => {
    setRaModalRom(rom);
    setRaGameInfo(null);
    setRaLoading(true);
    try {
      const info = await invoke<RAGameInfo>("get_ra_game_progress", { gameName: rom.name, console: rom.console });
      setRaGameInfo(info);
    } catch (e: any) {
      showToast(`RetroAchievements: ${e}`, "error");
      setRaModalRom(null);
    } finally {
      setRaLoading(false);
    }
  }, [showToast]);

  const handleRaLogin = useCallback(async () => {
    if (!raUsername || !raPassword) return;
    setRaLoginLoading(true);
    try {
      const token = await invoke<string>("ra_login", { username: raUsername, password: raPassword });
      setRaToken(token);
      setRaPassword("");
      showToast("Connecté à RetroAchievements ! Token obtenu.", "success");
    } catch (e: any) {
      showToast(`RA Login: ${e}`, "error");
    } finally {
      setRaLoginLoading(false);
    }
  }, [raUsername, raPassword, showToast]);

  const handleConfigureRaEmulators = useCallback(async () => {
    try {
      const configured = await invoke<string[]>("configure_ra_emulators");
      showToast(`RetroAchievements configuré dans : ${configured.join(", ")}`, "success");
    } catch (e: any) {
      showToast(`${e}`, "error");
    }
  }, [showToast]);

  const handleDownloadRaCores = useCallback(async () => {
    setRaDownloadingCores(true);
    try {
      const cores = await invoke<string[]>("download_ra_cores");
      showToast(`Cores installés : ${cores.join(", ")}`, "success");
    } catch (e: any) {
      showToast(`${e}`, "error");
    } finally {
      setRaDownloadingCores(false);
    }
  }, [showToast]);

  // ---- Leaderboard handler ----
  const loadLeaderboard = useCallback(async () => {
    setLeaderboardLoading(true);
    try {
      // Get all public profiles
      const { data: profiles } = await supabase
        .from("profiles")
        .select("id, username, avatar_url")
        .eq("public_profile", true);
      if (!profiles || profiles.length === 0) { setLeaderboard([]); return; }

      // Get all playtime_games rows for these users
      const userIds = profiles.map(p => p.id);
      const { data: games } = await supabase
        .from("playtime_games")
        .select("user_id, seconds, launches, last_played")
        .in("user_id", userIds);

      const now = Date.now();
      const weekAgo = now - 7 * 24 * 60 * 60 * 1000;

      const entries: LeaderboardEntry[] = profiles.map(p => {
        const userGames = (games || []).filter(g => g.user_id === p.id);
        const weekGames = userGames.filter(g => g.last_played && new Date(g.last_played).getTime() > weekAgo);
        const weekSeconds = weekGames.reduce((s, g) => s + (g.seconds || 0), 0);
        const weekGamesCount = weekGames.length;
        const totalSeconds = userGames.reduce((s, g) => s + (g.seconds || 0), 0);
        const totalLaunches = userGames.reduce((s, g) => s + (g.launches || 0), 0);
        return {
          user_id: p.id,
          username: p.username || "Anonymous",
          avatar_url: p.avatar_url,
          week_seconds: weekSeconds,
          week_games: weekGamesCount,
          total_seconds: totalSeconds,
          total_launches: totalLaunches,
        };
      });

      entries.sort((a, b) => b.week_seconds - a.week_seconds);
      setLeaderboard(entries);
    } catch (err) {
      console.error("[Leaderboard]", err);
    } finally {
      setLeaderboardLoading(false);
    }
  }, []);

  // ---- Friends handlers ----
  const loadFriends = useCallback(async () => {
    if (!user) return;
    setFriendsLoading(true);
    try {
      // Get all friendships involving this user
      const { data } = await supabase
        .from("friendships")
        .select("*")
        .or(`requester_id.eq.${user.id},addressee_id.eq.${user.id}`);

      if (!data) { setFriends([]); setPendingRequests([]); return; }

      // Collect all friend user IDs to fetch profiles
      const friendIds = data.map(f => f.requester_id === user.id ? f.addressee_id : f.requester_id);
      const { data: profiles } = await supabase
        .from("profiles")
        .select("id, username, avatar_url")
        .in("id", friendIds);

      const profileMap = new Map((profiles || []).map(p => [p.id, p]));

      const enriched: Friendship[] = data.map(f => {
        const otherId = f.requester_id === user.id ? f.addressee_id : f.requester_id;
        return { ...f, profile: profileMap.get(otherId) || { username: null, avatar_url: null } };
      });

      setFriends(enriched.filter(f => f.status === "accepted"));
      setPendingRequests(enriched.filter(f => f.status === "pending"));

      // Load presence for accepted friends
      const acceptedIds = enriched.filter(f => f.status === "accepted").map(f =>
        f.requester_id === user.id ? f.addressee_id : f.requester_id
      );
      if (acceptedIds.length > 0) {
        const { data: presences } = await supabase
          .from("presence")
          .select("*")
          .in("user_id", acceptedIds);
        setFriendPresences(presences || []);
      }
    } catch (err) {
      console.error("[Friends]", err);
    } finally {
      setFriendsLoading(false);
    }
  }, [user]);

  const searchFriends = useCallback(async (query: string) => {
    if (!user || query.trim().length < 2) { setFriendSearchResults([]); return; }
    const { data } = await supabase
      .from("profiles")
      .select("id, username, avatar_url")
      .ilike("username", `%${query}%`)
      .neq("id", user.id)
      .limit(10);
    setFriendSearchResults(data || []);
  }, [user]);

  const sendFriendRequest = useCallback(async (addresseeId: string) => {
    if (!user) return;
    const { error } = await supabase.from("friendships").insert({
      requester_id: user.id,
      addressee_id: addresseeId,
      status: "pending",
    });
    if (error) {
      if (error.code === "23505") showToast("Demande déjà envoyée", "info");
      else showToast(`Erreur: ${error.message}`, "error");
    } else {
      showToast("Demande d'ami envoyée !", "success");
      loadFriends();
    }
  }, [user, showToast, loadFriends]);

  const acceptFriendRequest = useCallback(async (friendshipId: string) => {
    await supabase.from("friendships").update({ status: "accepted" }).eq("id", friendshipId);
    showToast("Ami accepté !", "success");
    loadFriends();
  }, [showToast, loadFriends]);

  const declineFriendRequest = useCallback(async (friendshipId: string) => {
    await supabase.from("friendships").delete().eq("id", friendshipId);
    showToast("Demande refusée", "info");
    loadFriends();
  }, [showToast, loadFriends]);

  const removeFriend = useCallback(async (friendshipId: string) => {
    await supabase.from("friendships").delete().eq("id", friendshipId);
    showToast("Ami supprimé", "info");
    loadFriends();
  }, [showToast, loadFriends]);

  // Update own presence
  const viewFriendProfile = useCallback(async (friendId: string, username: string, avatar_url: string | null) => {
    try {
      // Fetch their playtime games
      const { data: games } = await supabase
        .from("playtime_games")
        .select("name, console, seconds, launches")
        .eq("user_id", friendId);

      // Fetch their achievements count
      const { count: achievementCount } = await supabase
        .from("user_achievements")
        .select("*", { count: "exact", head: true })
        .eq("user_id", friendId);

      const allGames = games || [];
      const totalSeconds = allGames.reduce((s, g) => s + (g.seconds || 0), 0);
      const totalLaunches = allGames.reduce((s, g) => s + (g.launches || 0), 0);
      const gamesPlayed = allGames.filter(g => g.seconds > 0 || g.launches > 0).length;

      const topGames = [...allGames]
        .sort((a, b) => (b.seconds || 0) - (a.seconds || 0))
        .slice(0, 5)
        .map(g => ({ name: g.name, console: g.console, seconds: g.seconds || 0 }));

      const consoleMap: Record<string, number> = {};
      allGames.forEach(g => { consoleMap[g.console] = (consoleMap[g.console] || 0) + (g.seconds || 0); });
      const topConsoles = Object.entries(consoleMap)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5)
        .map(([name, seconds]) => ({ name, seconds }));

      setFriendProfile({
        id: friendId,
        username,
        avatar_url,
        totalSeconds,
        totalLaunches,
        gamesPlayed,
        topGames,
        topConsoles,
        achievements: achievementCount || 0,
      });
    } catch (err) {
      console.error("[FriendProfile]", err);
    }
  }, []);

  // ---- Chat handlers ----
  const loadChatMessages = useCallback(async (friendId: string) => {
    if (!user) return;
    setChatLoading(true);
    const { data } = await supabase
      .from("messages")
      .select("*")
      .or(`and(sender_id.eq.${user.id},receiver_id.eq.${friendId}),and(sender_id.eq.${friendId},receiver_id.eq.${user.id})`)
      .order("created_at", { ascending: true })
      .limit(100);
    setChatMessages(data || []);
    setChatLoading(false);
    // Mark unread as read
    await supabase
      .from("messages")
      .update({ read_at: new Date().toISOString() })
      .eq("sender_id", friendId)
      .eq("receiver_id", user.id)
      .is("read_at", null);
    setUnreadCounts(prev => ({ ...prev, [friendId]: 0 }));
    setTimeout(() => chatEndRef.current?.scrollIntoView({ behavior: "smooth" }), 100);
  }, [user]);

  const openChat = useCallback((friendId: string, username: string, avatar_url: string | null) => {
    setChatOpen({ id: friendId, username, avatar_url });
    loadChatMessages(friendId);
  }, [loadChatMessages]);

  const sendMessage = useCallback(async () => {
    if (!user || !chatOpen || !chatInput.trim()) return;
    const content = chatInput.trim();
    setChatInput("");
    const { data } = await supabase.from("messages").insert({
      sender_id: user.id,
      receiver_id: chatOpen.id,
      content,
    }).select().single();
    if (data) {
      setChatMessages(prev => [...prev, data]);
      setTimeout(() => chatEndRef.current?.scrollIntoView({ behavior: "smooth" }), 50);
    }
  }, [user, chatOpen, chatInput]);

  const loadUnreadCounts = useCallback(async () => {
    if (!user) return;
    const { data } = await supabase
      .from("messages")
      .select("sender_id")
      .eq("receiver_id", user.id)
      .is("read_at", null);
    if (data) {
      const counts: Record<string, number> = {};
      data.forEach(m => { counts[m.sender_id] = (counts[m.sender_id] || 0) + 1; });
      setUnreadCounts(counts);
    }
  }, [user]);

  // Load unread counts when friends load
  useEffect(() => { if (user) loadUnreadCounts(); }, [user, loadUnreadCounts]);

  // Realtime chat subscription
  useEffect(() => {
    if (!user) return;
    const channel = supabase
      .channel("chat-realtime")
      .on("postgres_changes", {
        event: "INSERT",
        schema: "public",
        table: "messages",
        filter: `receiver_id=eq.${user.id}`,
      }, (payload) => {
        const msg = payload.new as ChatMessage;
        // If chat is open with this sender, add message and mark read
        if (chatOpen && msg.sender_id === chatOpen.id) {
          setChatMessages(prev => [...prev, msg]);
          setTimeout(() => chatEndRef.current?.scrollIntoView({ behavior: "smooth" }), 50);
          supabase.from("messages").update({ read_at: new Date().toISOString() }).eq("id", msg.id).then();
        } else {
          // Increment unread count
          setUnreadCounts(prev => ({ ...prev, [msg.sender_id]: (prev[msg.sender_id] || 0) + 1 }));
          sendNotification({ title: "EmuWorld — Nouveau message", body: `Message reçu` });
        }
      })
      .subscribe();
    return () => { supabase.removeChannel(channel); };
  }, [user, chatOpen]);

  const updatePresence = useCallback(async (status: "online" | "playing" | "offline", game?: string, console_name?: string) => {
    if (!user) return;
    await supabase.from("presence").upsert({
      user_id: user.id,
      status,
      current_game: game || null,
      current_console: console_name || null,
    });
  }, [user]);

  // Set online when logged in, offline on unmount
  useEffect(() => {
    if (user) {
      updatePresence("online");
      const interval = setInterval(() => updatePresence("online"), 60000);
      return () => { clearInterval(interval); updatePresence("offline"); };
    }
  }, [user, updatePresence]);

  // Load friends on login
  useEffect(() => { if (user) loadFriends(); }, [user, loadFriends]);

  // Realtime subscription for friend requests
  useEffect(() => {
    if (!user) return;
    const channel = supabase
      .channel("friendships-realtime")
      .on("postgres_changes", { event: "*", schema: "public", table: "friendships", filter: `addressee_id=eq.${user.id}` }, () => {
        loadFriends();
      })
      .on("postgres_changes", { event: "*", schema: "public", table: "presence" }, (payload) => {
        setFriendPresences(prev => {
          const updated = payload.new as PresenceEntry;
          const without = prev.filter(p => p.user_id !== updated.user_id);
          return [...without, updated];
        });
      })
      .subscribe();
    return () => { supabase.removeChannel(channel); };
  }, [user, loadFriends]);

  // ---- Cloud Backup handlers ----
  const handleSaveB2Config = useCallback(async () => {
    try {
      await invoke("save_b2_config", { keyId: b2KeyId, appKey: b2AppKey, bucketId: b2BucketId, bucketName: b2BucketName });
      showToast("Configuration B2 sauvegardée", "success");
      const saves = await invoke<typeof localSaves>("scan_local_saves");
      setLocalSaves(saves);
    } catch (e: any) {
      showToast(`${e}`, "error");
    }
  }, [b2KeyId, b2AppKey, b2BucketId, b2BucketName, showToast]);

  const handleBackupToCloud = useCallback(async () => {
    setBackupLoading(true);
    try {
      const result = await invoke<string>("backup_saves_to_cloud");
      showToast(result, "success");
    } catch (e: any) {
      showToast(`Backup failed: ${e}`, "error");
    } finally {
      setBackupLoading(false);
    }
  }, [showToast]);

  const handleListCloudBackups = useCallback(async () => {
    setBackupLoading(true);
    try {
      const files = await invoke<typeof cloudBackups>("list_cloud_backups");
      setCloudBackups(files);
    } catch (e: any) {
      showToast(`${e}`, "error");
    } finally {
      setBackupLoading(false);
    }
  }, [showToast]);

  const handleRestoreBackup = useCallback(async (fileId: string) => {
    setBackupLoading(true);
    try {
      const result = await invoke<string>("restore_cloud_backup", { fileId });
      showToast(result, "success");
    } catch (e: any) {
      showToast(`Restore failed: ${e}`, "error");
    } finally {
      setBackupLoading(false);
    }
  }, [showToast]);

  const handleDeleteBackup = useCallback(async (fileId: string, fileName: string) => {
    setBackupLoading(true);
    try {
      await invoke<string>("delete_cloud_backup", { fileId, fileName });
      setCloudBackups(prev => prev.filter(b => b.file_id !== fileId));
      showToast("Backup supprimé", "success");
    } catch (e: any) {
      showToast(`Suppression échouée: ${e}`, "error");
    } finally {
      setBackupLoading(false);
    }
  }, [showToast]);

  const handleLoadRaProfile = useCallback(async () => {
    setRaProfileLoading(true);
    try {
      const games = await invoke<RACompletedGame[]>("get_ra_completed_games");
      setRaCompletedGames(games);
    } catch (e: any) {
      showToast(`RetroAchievements: ${e}`, "error");
    } finally {
      setRaProfileLoading(false);
    }
  }, [showToast]);

  const handleOpenRaFromCompleted = useCallback(async (game: RACompletedGame) => {
    setRaGameInfo(null);
    setRaModalRom({ name: game.title, path: "", console: game.console_name, extension: "", size: 0 });
    setRaLoading(true);
    try {
      const info = await invoke<RAGameInfo>("get_ra_game_progress", { gameName: game.title, console: game.console_name });
      setRaGameInfo(info);
    } catch (e: any) {
      showToast(`RetroAchievements: ${e}`, "error");
      setRaModalRom(null);
    } finally {
      setRaLoading(false);
    }
  }, [showToast]);

  // ---- Playtime / profile stats ----
  const loadPlaytime = useCallback(async () => {
    try {
      const [pt, stats] = await Promise.all([
        invoke<PlaytimeStore>("get_playtime"),
        invoke<ProfileStats>("get_profile_stats"),
      ]);
      setPlaytime(pt);
      setProfileStats(stats);
    } catch (err) {
      console.error("Failed to load playtime:", err);
    }
  }, []);

  useEffect(() => { loadPlaytime(); }, [loadPlaytime]);

  const loadAchievements = useCallback(async () => {
    try {
      const [items, rank] = await Promise.all([
        invoke<AchievementItem[]>("get_achievements"),
        invoke<AchievementRank>("get_achievement_rank"),
      ]);
      setAchievements(items);
      setAchievementRank(rank);
    } catch (err) {
      console.error("Failed to load achievements:", err);
    }
  }, []);

  useEffect(() => { loadAchievements(); }, [loadAchievements]);

  const syncAchievementToCloud = useCallback(async (achievement: AchievementItem) => {
    if (!user) return;
    try {
      await supabase.from("user_achievements").upsert({
        user_id: user.id,
        achievement_id: achievement.id,
        unlocked_at: achievement.unlocked_at,
      }, { onConflict: "user_id,achievement_id" });
    } catch (err) {
      console.error("Achievement cloud sync failed:", err);
    }
  }, [user]);

  const syncAllAchievementsToCloud = useCallback(async () => {
    if (!user) return;
    const unlocked = achievements.filter(a => a.unlocked && a.unlocked_at);
    if (unlocked.length === 0) return;
    try {
      const rows = unlocked.map(a => ({
        user_id: user.id,
        achievement_id: a.id,
        unlocked_at: a.unlocked_at,
      }));
      await supabase.from("user_achievements").upsert(rows, { onConflict: "user_id,achievement_id" });
    } catch (err) {
      console.error("Bulk achievement sync failed:", err);
    }
  }, [user, achievements]);

  useEffect(() => {
    if (user && achievements.length > 0) {
      syncAllAchievementsToCloud();
    }
  }, [user, achievements.length, syncAllAchievementsToCloud]);

  const triggerHiddenAchievement = useCallback(async (id: string) => {
    try {
      const result = await invoke<AchievementItem | null>("unlock_achievement", { id });
      if (result) {
        showToast(`${result.icon} Achievement secret débloqué : ${result.name}`, "success");
        sendNotification({ title: "EmuWorld — Achievement !", body: `${result.icon} ${result.name}` });
        await loadAchievements();
        syncAchievementToCloud(result);
      }
    } catch (err) {
      console.error("Hidden achievement unlock failed:", err);
    }
  }, [showToast, loadAchievements, syncAchievementToCloud]);

  const checkAchievements = useCallback(async () => {
    try {
      const libraryCount = roms.length;
      const emulatorsInstalled = installed.length;
      const hasDownloaded = roms.length > 0;
      const newlyUnlocked = await invoke<AchievementItem[]>("check_achievements", {
        libraryCount,
        emulatorsInstalled,
        hasDownloaded,
      });
      if (newlyUnlocked.length > 0) {
        for (const a of newlyUnlocked) {
          showToast(`${a.icon} Achievement débloqué : ${a.name}`, "success");
          sendNotification({ title: "EmuWorld — Achievement !", body: `${a.icon} ${a.name}` });
          syncAchievementToCloud(a);
        }
        await loadAchievements();
      }
    } catch (err) {
      console.error("Achievement check failed:", err);
    }
  }, [roms.length, installed.length, showToast, loadAchievements, syncAchievementToCloud]);

  useEffect(() => { checkAchievements(); }, [checkAchievements]);

  // Load gamepad config from localStorage
  useEffect(() => {
    const saved = localStorage.getItem("emuworld_gamepad_config");
    if (saved) {
      try { setGamepadConfig(JSON.parse(saved)); } catch { /* ignore */ }
    }
  }, []);

  // Save gamepad config on change
  useEffect(() => {
    localStorage.setItem("emuworld_gamepad_config", JSON.stringify(gamepadConfig));
  }, [gamepadConfig]);

  // === Gamepad via Rust (gilrs) — receives state via Tauri event ===
  const gamepadConfigRef = useRef(gamepadConfig);
  gamepadConfigRef.current = gamepadConfig;
  const remappingActionRef = useRef(remappingAction);
  remappingActionRef.current = remappingAction;
  const pageRef = useRef(page);
  pageRef.current = page;
  const gamepadContextMenuRef = useRef(gamepadContextMenu);
  gamepadContextMenuRef.current = gamepadContextMenu;
  const bigPictureModeRef = useRef(bigPictureMode);
  bigPictureModeRef.current = bigPictureMode;
  const bpSelectedIndexRef = useRef(bpSelectedIndex);
  bpSelectedIndexRef.current = bpSelectedIndex;
  const bpConsoleFilterRef = useRef(bpConsoleFilter);
  bpConsoleFilterRef.current = bpConsoleFilter;
  const gamepadKeyboardRef = useRef(gamepadKeyboard);
  gamepadKeyboardRef.current = gamepadKeyboard;
  const remapReadyRef = useRef(false);
  const VIRTUAL_KB_KEYS = [
    "a","b","c","d","e","f","g","h","i","j",
    "k","l","m","n","o","p","q","r","s","t",
    "u","v","w","x","y","z","0","1","2","3",
    "4","5","6","7","8","9"," ","⌫","OK",
  ];
  const VIRTUAL_KB_COLS = 10;
  const gamepadStateRef = useRef<{ buttons: boolean[]; axes: number[] }>({ buttons: [], axes: [] });

  useEffect(() => {
    const pages: Page[] = ["catalog", "library", "installed", "store", "controller", "leaderboard", "friends", "stats", "settings", "changelogs"];
    let navCooldown = 0;

    const unlisten = listen<{ connected: boolean; name: string; buttons: boolean[]; axes: number[] }>("gamepad-state", (event) => {
      const { connected, name, buttons, axes } = event.payload;

      // Connection state
      if (connected && !gamepadActiveRef.current) {
        gamepadActiveRef.current = true;
        setGamepadActive(true);
        setGamepadName(name);
      } else if (!connected && gamepadActiveRef.current) {
        gamepadActiveRef.current = false;
        setGamepadActive(false);
        setGamepadName("");
        lastGamepadButtonsRef.current = [];
        gamepadStateRef.current = { buttons: [], axes: [] };
        return;
      }
      if (!connected) return;

      // Store raw state for live display
      gamepadStateRef.current = { buttons, axes };
      if (pageRef.current === "controller") {
        setGamepadTick(t => t + 1);
      }

      const config = gamepadConfigRef.current;
      const prev = lastGamepadButtonsRef.current;
      const justPressed = buttons.map((pressed, i) => pressed && !(prev[i] ?? false));

      // Remap mode — wait for all buttons released first, then capture next press
      if (remappingActionRef.current) {
        const anyPressed = buttons.some(b => b);
        if (!remapReadyRef.current) {
          if (!anyPressed) remapReadyRef.current = true;
        } else {
          for (let i = 0; i < buttons.length; i++) {
            if (justPressed[i]) {
              const action = remappingActionRef.current;
              setGamepadConfig(c => ({
                ...c,
                mappings: c.mappings.map(m =>
                  m.action === action ? { ...m, buttonIndex: i, label: `Button ${i}` } : m
                ),
              }));
              setRemappingAction(null);
              remapReadyRef.current = false;
              break;
            }
          }
        }
        lastGamepadButtonsRef.current = [...buttons];
        return;
      }

      // Navigation actions
      const getAction = (action: string) => {
        const mapping = config.mappings.find(m => m.action === action);
        return mapping ? justPressed[mapping.buttonIndex] ?? false : false;
      };

      // D-pad (rising edge from buttons 12-15)
      const dpadUp = justPressed[12] ?? false;
      const dpadDown = justPressed[13] ?? false;
      const dpadLeft = justPressed[14] ?? false;
      const dpadRight = justPressed[15] ?? false;

      // Stick navigation with cooldown
      const stickX = Math.abs(axes[0] ?? 0) > config.deadzone ? axes[0] : 0;
      const stickY = Math.abs(axes[1] ?? 0) > config.deadzone ? axes[1] : 0;

      let stickMoveDown = false, stickMoveUp = false, stickMoveRight = false, stickMoveLeft = false;
      if (navCooldown <= 0) {
        if (stickY > 0.4) { stickMoveDown = true; navCooldown = 8; }
        else if (stickY < -0.4) { stickMoveUp = true; navCooldown = 8; }
        if (stickX > 0.4) { stickMoveRight = true; navCooldown = 8; }
        else if (stickX < -0.4) { stickMoveLeft = true; navCooldown = 8; }
      } else if (Math.abs(stickX) > 0.4 || Math.abs(stickY) > 0.4) {
        navCooldown--;
      } else {
        navCooldown = 0;
      }

      const moveDown = dpadDown || stickMoveDown;
      const moveUp = dpadUp || stickMoveUp;
      const moveRight = dpadRight || stickMoveRight;
      const moveLeft = dpadLeft || stickMoveLeft;

      // Virtual keyboard navigation
      if (gamepadKeyboardRef.current) {
        const kb = gamepadKeyboardRef.current;
        let keyIdx = kb.keyIdx;
        const cols = VIRTUAL_KB_COLS;
        if (moveRight) keyIdx = Math.min(keyIdx + 1, VIRTUAL_KB_KEYS.length - 1);
        if (moveLeft) keyIdx = Math.max(keyIdx - 1, 0);
        if (moveDown) keyIdx = Math.min(keyIdx + cols, VIRTUAL_KB_KEYS.length - 1);
        if (moveUp) keyIdx = Math.max(keyIdx - cols, 0);
        if (keyIdx !== kb.keyIdx) {
          setGamepadKeyboard({ ...kb, keyIdx });
        }
        // A = type key
        if (getAction("confirm")) {
          const key = VIRTUAL_KB_KEYS[keyIdx];
          if (key === "⌫") {
            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')!.set!;
            nativeInputValueSetter.call(kb.inputEl, kb.inputEl.value.slice(0, -1));
            kb.inputEl.dispatchEvent(new Event('input', { bubbles: true }));
          } else if (key === "OK") {
            setGamepadKeyboard(null);
          } else {
            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')!.set!;
            nativeInputValueSetter.call(kb.inputEl, kb.inputEl.value + key);
            kb.inputEl.dispatchEvent(new Event('input', { bubbles: true }));
          }
        }
        // X = backspace shortcut
        if (justPressed[2]) {
          const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')!.set!;
          nativeInputValueSetter.call(kb.inputEl, kb.inputEl.value.slice(0, -1));
          kb.inputEl.dispatchEvent(new Event('input', { bubbles: true }));
        }
        // B = close keyboard
        if (getAction("back")) {
          setGamepadKeyboard(null);
        }
        lastGamepadButtonsRef.current = [...buttons];
        return;
      }

      // If context menu is open, navigate within it + handle A/B
      if (gamepadContextMenuRef.current) {
        if (moveDown || moveUp) {
          const menuBtns = document.querySelectorAll<HTMLElement>(".gamepad-context-menu__btn");
          if (menuBtns.length > 0) {
            const currentIdx = Array.from(menuBtns).findIndex(b => b.classList.contains("gamepad-ctx-focused"));
            let newIdx = currentIdx;
            if (moveDown) newIdx = Math.min(currentIdx + 1, menuBtns.length - 1);
            if (moveUp) newIdx = Math.max(currentIdx - 1, 0);
            menuBtns.forEach(b => b.classList.remove("gamepad-ctx-focused"));
            menuBtns[newIdx]?.classList.add("gamepad-ctx-focused");
          }
        }
        // A = confirm selection in menu
        if (getAction("confirm")) {
          const focusedBtn = document.querySelector<HTMLElement>(".gamepad-context-menu__btn.gamepad-ctx-focused");
          if (focusedBtn) focusedBtn.click();
          else {
            const firstBtn = document.querySelector<HTMLElement>(".gamepad-context-menu__btn");
            if (firstBtn) firstBtn.click();
          }
        }
        // B = close menu
        if (getAction("back")) {
          setGamepadContextMenu(null);
        }
        lastGamepadButtonsRef.current = [...buttons];
        return;
      }

      // Big Picture mode navigation
      if (bigPictureModeRef.current) {
        if (moveDown || moveUp || moveRight || moveLeft) {
          const items = document.querySelectorAll<HTMLElement>(".big-picture .gamepad-nav-item");
          if (items.length > 0) {
            const cols = Math.max(1, Math.round((items[0]?.parentElement?.clientWidth ?? 300) / Math.max(1, (items[0]?.clientWidth ?? 200) + 20)));
            let idx = bpSelectedIndexRef.current;
            if (moveRight) idx = Math.min(idx + 1, items.length - 1);
            if (moveLeft) idx = Math.max(idx - 1, 0);
            if (moveDown) idx = Math.min(idx + cols, items.length - 1);
            if (moveUp) idx = Math.max(idx - cols, 0);
            setBpSelectedIndex(idx);
            items.forEach(el => el.classList.remove("gamepad-focused"));
            items[idx]?.classList.add("gamepad-focused");
            items[idx]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
          }
        }
        if (getAction("confirm")) {
          const items = document.querySelectorAll<HTMLElement>(".big-picture .gamepad-nav-item");
          items[bpSelectedIndexRef.current]?.click();
        }
        if (getAction("back")) {
          if (bpConsoleFilterRef.current) { setBpConsoleFilter(null); setBpSelectedIndex(0); }
          else setBigPictureMode(false);
        }
        lastGamepadButtonsRef.current = [...buttons];
        return;
      }

      if (moveDown || moveUp || moveRight || moveLeft) {
        const SIDEBAR_SEL = ".sidebar__item";
        const CONTENT_SEL = ".main-content .game-card, .main-content .rgs-console-card, .main-content .emu-card, .main-content .vimm-game-row, .main-content .gamepad-nav-item, .main-content .friend-card, .main-content .btn, .main-content .settings__field-input, .main-content .leaderboard-entry, .main-content .changelog-card, .main-content .store-source-toggle__btn, .main-content .theme-picker__item";
        const sidebarItems = document.querySelectorAll<HTMLElement>(SIDEBAR_SEL);
        const contentItems = document.querySelectorAll<HTMLElement>(CONTENT_SEL);
        const allItems = [...sidebarItems, ...contentItems];

        if (allItems.length > 0) {
          let idx = focusIndexRef.current;
          const inSidebar = idx < sidebarItems.length;

          if (inSidebar) {
            // In sidebar: up/down moves within sidebar, right jumps to content
            if (moveDown) idx = Math.min(idx + 1, sidebarItems.length - 1);
            if (moveUp) idx = Math.max(idx - 1, 0);
            if (moveRight && contentItems.length > 0) idx = sidebarItems.length; // first content item
          } else {
            // In content: free movement with left/right + up/down by row
            const contentIdx = idx - sidebarItems.length;
            const currentContent = contentItems[contentIdx] ?? contentItems[0];
            const isLinear = currentContent?.classList.contains("gamepad-nav-item") || currentContent?.classList.contains("vimm-game-row") || currentContent?.classList.contains("changelog-card") || currentContent?.classList.contains("leaderboard-entry") || currentContent?.classList.contains("btn") || currentContent?.classList.contains("settings__field-input") || currentContent?.classList.contains("friend-card") || currentContent?.classList.contains("store-source-toggle__btn") || currentContent?.classList.contains("theme-picker__item");
            const cols = isLinear ? 1 : (currentContent ? Math.max(1, Math.round((currentContent.parentElement?.clientWidth ?? 300) / Math.max(1, currentContent.clientWidth + 16))) : 1);
            let newContentIdx = contentIdx;

            if (moveDown) newContentIdx = Math.min(contentIdx + cols, contentItems.length - 1);
            if (moveUp) newContentIdx = Math.max(contentIdx - cols, 0);
            if (moveRight) newContentIdx = Math.min(contentIdx + 1, contentItems.length - 1);
            if (moveLeft) {
              if (contentIdx === 0 || contentIdx % cols === 0) {
                // At left edge of content → go back to sidebar
                idx = Math.min(Math.floor(contentIdx / cols), sidebarItems.length - 1);
                focusIndexRef.current = idx;
                setFocusIndex(idx);
                lastGamepadButtonsRef.current = [...buttons];
                return;
              }
              newContentIdx = contentIdx - 1;
            }
            idx = sidebarItems.length + newContentIdx;
          }

          idx = Math.max(0, Math.min(idx, allItems.length - 1));
          focusIndexRef.current = idx;
          setFocusIndex(idx);
        }
      }

      // A = confirm / open context menu on game or emu cards
      if (getAction("confirm")) {
        if (gamepadContextMenuRef.current) {
          // If context menu open, click focused button inside it
          const menuBtns = document.querySelectorAll<HTMLElement>(".gamepad-context-menu__btn");
          const focusedBtn = document.querySelector<HTMLElement>(".gamepad-context-menu__btn.gamepad-ctx-focused");
          if (focusedBtn) focusedBtn.click();
          else if (menuBtns[0]) menuBtns[0].click();
        } else {
          const el = document.querySelector<HTMLElement>(".gamepad-focused");
          if (el) {
            // Game card → open context menu
            const romPath = el.getAttribute("data-rom-path");
            if (romPath && el.classList.contains("game-card")) {
              const rect = el.getBoundingClientRect();
              setGamepadContextMenu({ type: "rom", romPath, x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
            }
            // Emu card → open context menu
            else if (el.classList.contains("emu-card")) {
              const emuId = el.getAttribute("data-emu-id");
              if (emuId) {
                const rect = el.getBoundingClientRect();
                setGamepadContextMenu({ type: "emu", emuId, x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
              }
            }
            // Any input → open virtual keyboard
            else if (el.tagName === "INPUT") {
              setGamepadKeyboard({ inputEl: el as unknown as HTMLInputElement, keyIdx: 0 });
            }
            // Vimm row → click the Download button inside
            else if (el.classList.contains("vimm-game-row")) {
              const btn = el.querySelector<HTMLElement>("button");
              if (btn) btn.click();
            }
            // Everything else → normal click
            else {
              el.click();
            }
          }
        }
      }

      // B = back / close context menu
      if (getAction("back")) {
        if (gamepadContextMenuRef.current) {
          setGamepadContextMenu(null);
        } else if (document.querySelector(".account-overlay")) {
          const closeBtn = document.querySelector<HTMLElement>(".account-modal__close");
          if (closeBtn) closeBtn.click();
        } else {
          setConsoleFilter(prev => {
            if (prev) return null;
            setCategoryFilter(cf => {
              if (cf) return null;
              setPage(p => p !== "catalog" ? "catalog" : p);
              return cf;
            });
            return prev;
          });
        }
      }

      // Y = toggle favorite on focused game card
      if (getAction("favorite")) {
        const el = document.querySelector<HTMLElement>(".game-card.gamepad-focused");
        if (el) {
          const romPath = el.getAttribute("data-rom-path") ?? "";
          const rom = roms.find(r => r.path === romPath);
          if (rom) handleToggleFavorite(rom);
        }
      }

      // LB/RB = switch page
      if (getAction("prevPage")) {
        setPage(p => { const idx = pages.indexOf(p); return idx > 0 ? pages[idx - 1] : p; });
      }
      if (getAction("nextPage")) {
        setPage(p => { const idx = pages.indexOf(p); return idx < pages.length - 1 ? pages[idx + 1] : p; });
      }
      if (getAction("settings")) {
        setPage("controller");
      }

      lastGamepadButtonsRef.current = [...buttons];
    });

    return () => { unlisten.then(f => f()); };
  }, []);

  // Apply focus highlight — done directly in DOM to avoid re-render dependency issues
  const focusIndexRef = useRef(focusIndex);
  focusIndexRef.current = focusIndex;
  useEffect(() => {
    const applyFocus = () => {
      const sidebarItems = document.querySelectorAll<HTMLElement>(".sidebar__item");
      const contentItems = document.querySelectorAll<HTMLElement>(".main-content .game-card, .main-content .rgs-console-card, .main-content .emu-card, .main-content .vimm-game-row, .main-content .gamepad-nav-item, .main-content .friend-card, .main-content .btn, .main-content .settings__field-input, .main-content .leaderboard-entry, .main-content .changelog-card, .main-content .store-source-toggle__btn, .main-content .theme-picker__item");
      const allItems = [...sidebarItems, ...contentItems];
      allItems.forEach((el, i) => {
        if (i === focusIndexRef.current) {
          el.classList.add("gamepad-focused");
          el.scrollIntoView({ block: "nearest", behavior: "smooth" });
        } else {
          el.classList.remove("gamepad-focused");
        }
      });
    };
    const id = setInterval(applyFocus, 100);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (gamepadActive) {
      focusIndexRef.current = 0;
      setFocusIndex(0);
    }
  }, [page]);

  // Discord Rich Presence — idle pub on boot. Failures are expected when
  // Discord is not running; swallow them so the app doesn't spam toasts.
  useEffect(() => {
    invoke("discord_set_idle").catch(() => {});
  }, []);

  // ---- Check for updates on boot ----
  // Tauri updater polls our GitHub Releases endpoint (latest.json). If a
  // newer version is published, we just expose it via a banner — nothing
  // downloads until the user clicks "Install".
  useEffect(() => {
    (async () => {
      try {
        const update = await checkForUpdate();
        if (update) {
          console.log("[EmuWorld] update available:", update.version);
          setUpdateAvailable({ version: update.version });
        }
      } catch (err) {
        // Offline, endpoint down, or running in dev mode without a signed
        // build. Non-fatal — silently swallow.
        console.log("[EmuWorld] update check skipped:", err);
      }
    })();
  }, []);

  const handleInstallUpdate = useCallback(async () => {
    setUpdateStatus("downloading");
    try {
      const update = await checkForUpdate();
      if (!update) {
        setUpdateStatus("idle");
        setUpdateAvailable(null);
        return;
      }
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          setUpdateProgress({ done: 0, total: event.data.contentLength ?? null });
        } else if (event.event === "Progress") {
          setUpdateProgress((prev) => ({
            done: prev.done + event.data.chunkLength,
            total: prev.total,
          }));
        } else if (event.event === "Finished") {
          setUpdateStatus("ready");
        }
      });
      triggerHiddenAchievement("install_update");
      await relaunch();
    } catch (err) {
      console.error("[EmuWorld] update install failed:", err);
      setUpdateStatus("error");
    }
  }, [triggerHiddenAchievement]);

  // ---- Cloud sync (Supabase) ----
  // Debounced upsert of the local playtime store to Supabase. Only runs when
  // the user is signed in. No-op otherwise — stats still work fully offline.
  // Must be declared BEFORE any effect/callback that references
  // scheduleCloudSync so the closures get the up-to-date version.
  const cloudSyncTimer = useRef<number | null>(null);
  const isSyncingCloud = useRef(false);

  const syncPlaytimeToCloud = useCallback(async () => {
    if (!user || isSyncingCloud.current) return;
    isSyncingCloud.current = true;
    try {
      // Always pull the freshest store from Rust rather than relying on React state
      const pt = await invoke<PlaytimeStore>("get_playtime");
      console.log("[EmuWorld] syncPlaytimeToCloud · user", user.id, "games", Object.keys(pt.games).length);

      const gameRows = Object.values(pt.games).map((g) => ({
        user_id: user.id,
        console: g.console,
        name: g.name,
        seconds: g.seconds,
        launches: g.launches,
        last_played: g.last_played,
        first_played: g.first_played,
        favorite: g.favorite,
        last_emulator_id: g.last_emulator_id,
        rating: g.rating ?? null,
        notes: g.notes ?? null,
      }));
      const emuRows = Object.entries(pt.emulators).map(([id, seconds]) => ({
        user_id: user.id,
        emulator_id: id,
        seconds,
      }));

      if (gameRows.length > 0) {
        const { error } = await supabase
          .from("playtime_games")
          .upsert(gameRows, { onConflict: "user_id,console,name" });
        if (error) throw error;
      }
      if (emuRows.length > 0) {
        const { error } = await supabase
          .from("playtime_emulators")
          .upsert(emuRows, { onConflict: "user_id,emulator_id" });
        if (error) throw error;
      }
      console.log("[EmuWorld] sync ok — pushed", gameRows.length, "games,", emuRows.length, "emulators");
    } catch (err: any) {
      console.error("[EmuWorld] Cloud sync failed:", err?.message || err);
    } finally {
      isSyncingCloud.current = false;
    }
  }, [user]);

  // Coalesce many quick updates (ex: rapid favorite toggles) into a single network call.
  const scheduleCloudSync = useCallback(() => {
    if (cloudSyncTimer.current !== null) {
      window.clearTimeout(cloudSyncTimer.current);
    }
    cloudSyncTimer.current = window.setTimeout(() => {
      cloudSyncTimer.current = null;
      void syncPlaytimeToCloud();
    }, 2000);
  }, [syncPlaytimeToCloud]);

  // Refresh playtime whenever an emulator child process exits.
  // `scheduleCloudSync` has to be in the dep array — otherwise the closure
  // captures the version from before login (when user was null) and every
  // sync becomes a no-op because of the `if (!user) return;` early exit.
  useEffect(() => {
    const unlisten = listen<{ console: string; name: string; seconds: number }>(
      "game-closed",
      async (event) => {
        const sessionSecs = event.payload.seconds;
        if (sessionSecs >= 3) {
          const pt = await invoke<PlaytimeStore>("get_playtime").catch(() => null);
          const gameKey = `${event.payload.console}::${event.payload.name}`;
          const gameData = pt?.games?.[gameKey];
          const totalSecs = gameData?.seconds ?? sessionSecs;
          const totalLaunches = gameData?.launches ?? 1;
          const gameName = event.payload.name.replace(/\[.*?\]/g, '').replace(/\(.*?\)/g, '').replace(/\.[^.]+$/, '').trim();

          setSessionRecap({
            gameName,
            console: event.payload.console,
            sessionSeconds: sessionSecs,
            totalSeconds: totalSecs,
            totalLaunches,
          });
          // Native Windows notification
          const mins = Math.floor(sessionSecs / 60);
          sendNotification({ title: "EmuWorld — Session terminée", body: `${gameName} — ${mins > 0 ? `${mins} min` : `${sessionSecs}s`} jouées` });
          // Auto-dismiss after 8 seconds
          setTimeout(() => setSessionRecap(null), 8000);
          scheduleCloudSync();
        }
        loadPlaytime();
        updatePresence("online");
        invoke("discord_set_idle").catch(() => {});
        setTimeout(() => checkAchievements(), 500);
        if (sessionSecs > 0 && sessionSecs < 30) {
          triggerHiddenAchievement("speed_runner");
        }
        if (sessionSecs >= 14400) {
          triggerHiddenAchievement("marathon");
        }
        const hour = new Date().getHours();
        if (hour >= 2 && hour < 5) {
          triggerHiddenAchievement("night_owl");
        }
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [loadPlaytime, showToast, scheduleCloudSync, checkAchievements, triggerHiddenAchievement]);

  // Listen for background 7z extraction completion
  useEffect(() => {
    const unlisten = listen<{ file: string; status: string; message?: string }>(
      "import-extract-done",
      (event) => {
        if (event.payload.status === "success") {
          showToast(`Extraction terminée : ${event.payload.file.replace(".7z", "")}`, "success");
          loadData();
        } else {
          showToast(`Extraction échouée : ${event.payload.message || event.payload.file}`, "error");
        }
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [showToast, loadData]);

  const handleToggleFavorite = useCallback(async (rom: RomFile) => {
    try {
      const isFav = await invoke<boolean>("toggle_favorite", { console: rom.console, name: rom.name });
      loadPlaytime();
      scheduleCloudSync();
      if (isFav) {
        triggerHiddenAchievement("first_favorite");
        setTimeout(() => checkAchievements(), 300);
      }
    } catch (err: any) {
      showToast(`Favorite failed: ${err}`, "error");
    }
  }, [loadPlaytime, showToast, scheduleCloudSync, triggerHiddenAchievement, checkAchievements]);

  const handleSetRating = useCallback(async (rom: RomFile, rating: number) => {
    try {
      await invoke("set_game_rating", { console: rom.console, name: rom.name, rating });
      loadPlaytime();
      scheduleCloudSync();
    } catch (err: any) {
      showToast(`Rating failed: ${err}`, "error");
    }
  }, [loadPlaytime, showToast, scheduleCloudSync]);

  const [notesModal, setNotesModal] = useState<{ rom: RomFile; text: string } | null>(null);

  const handleOpenNotes = useCallback((rom: RomFile) => {
    const entry = playtime.games[`${rom.console}::${rom.name}`];
    setNotesModal({ rom, text: entry?.notes ?? "" });
  }, [playtime]);

  const handleSaveNotes = useCallback(async () => {
    if (!notesModal) return;
    try {
      await invoke("set_game_notes", { console: notesModal.rom.console, name: notesModal.rom.name, notes: notesModal.text });
      loadPlaytime();
      scheduleCloudSync();
      setNotesModal(null);
    } catch (err: any) {
      showToast(`Notes failed: ${err}`, "error");
    }
  }, [notesModal, loadPlaytime, showToast, scheduleCloudSync]);

  const handleCreateCollection = useCallback(async (name: string) => {
    try {
      await invoke("create_collection", { name });
      loadPlaytime();
      setNewCollectionName("");
    } catch (err: any) {
      showToast(err, "error");
    }
  }, [loadPlaytime, showToast]);

  const handleDeleteCollection = useCallback(async (name: string) => {
    try {
      await invoke("delete_collection", { name });
      loadPlaytime();
      if (collectionFilter === name) setCollectionFilter(null);
    } catch (err: any) {
      showToast(`Delete failed: ${err}`, "error");
    }
  }, [loadPlaytime, showToast, collectionFilter]);

  const handleAddToCollection = useCallback(async (collectionName: string, rom: RomFile) => {
    try {
      await invoke("add_to_collection", { collectionName, gameKey: `${rom.console}::${rom.name}` });
      loadPlaytime();
      showToast(`Ajouté à "${collectionName}"`, "success");
    } catch (err: any) {
      showToast(`Failed: ${err}`, "error");
    }
  }, [loadPlaytime, showToast]);

  const handleRemoveFromCollection = useCallback(async (collectionName: string, rom: RomFile) => {
    try {
      await invoke("remove_from_collection", { collectionName, gameKey: `${rom.console}::${rom.name}` });
      loadPlaytime();
    } catch (err: any) {
      showToast(`Failed: ${err}`, "error");
    }
  }, [loadPlaytime, showToast]);

  // Sign-in / sign-out transitions reset the local playtime file so two
  // users on the same machine never inherit each other's stats. On sign-in
  // we pull the cloud rows for this user and overwrite the local store with
  // them, then refresh UI state. On sign-out we wipe the file.
  const lastSyncedUserId = useRef<string | null>(null);
  useEffect(() => {
    const currentId = user?.id ?? null;
    if (currentId === lastSyncedUserId.current) return;
    lastSyncedUserId.current = currentId;

    (async () => {
      if (!currentId) {
        // Sign-out: drop local history so the next signed-in user gets a clean slate.
        await invoke("clear_playtime").catch(() => {});
        await loadPlaytime();
        return;
      }
      // Sign-in: fetch this user's rows and replace the local store with them.
      try {
        const [gamesRes, emusRes] = await Promise.all([
          supabase.from("playtime_games").select("*").eq("user_id", currentId),
          supabase.from("playtime_emulators").select("*").eq("user_id", currentId),
        ]);
        const cloud: PlaytimeStore = { games: {}, emulators: {}, collections: [] };
        for (const row of gamesRes.data || []) {
          cloud.games[`${row.console}::${row.name}`] = {
            console: row.console,
            name: row.name,
            seconds: row.seconds || 0,
            launches: row.launches || 0,
            last_played: row.last_played,
            first_played: row.first_played,
            favorite: !!row.favorite,
            last_emulator_id: row.last_emulator_id,
            rating: row.rating ?? undefined,
            notes: row.notes ?? undefined,
          };
        }
        for (const row of emusRes.data || []) {
          cloud.emulators[row.emulator_id] = row.seconds || 0;
        }
        await invoke("overwrite_playtime", { store: cloud });
        await loadPlaytime();
      } catch (err) {
        console.error("[EmuWorld] Cloud pull failed:", err);
      }
    })();
  }, [user, loadPlaytime]);

  const [isTogglingPublic, setIsTogglingPublic] = useState(false);
  const handleTogglePublicProfile = useCallback(async () => {
    if (!user) return;
    setIsTogglingPublic(true);
    const next = !profile?.public_profile;
    try {
      // upsert (not update) so the row is created on the fly if this user
      // has never had a `profiles` entry yet — otherwise `.update().eq()`
      // succeeds with 0 rows affected and nothing changes.
      const { error } = await supabase
        .from("profiles")
        .upsert(
          {
            id: user.id,
            username: profile?.username ?? null,
            avatar_url: profile?.avatar_url ?? null,
            public_profile: next,
            updated_at: new Date().toISOString(),
          },
          { onConflict: "id" }
        );
      if (error) throw error;
      await fetchProfile(user.id);
      showToast(next ? "Profile is now public 🌐" : "Profile is now private 🔒", "success");
      if (next) void syncPlaytimeToCloud();
    } catch (err: any) {
      console.error("[EmuWorld] togglePublic error:", err);
      showToast(`Update failed: ${err?.message || err}`, "error");
    } finally {
      setIsTogglingPublic(false);
    }
  }, [user, profile?.public_profile, profile?.username, profile?.avatar_url, fetchProfile, showToast, syncPlaytimeToCloud]);

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

  // ---- RetroGameSets ----
  const loadRgsConstructeurs = useCallback(async () => {
    try {
      const data = await invoke<RgsConstructeur[]>("get_rgs_constructeurs");
      setRgsConstructeurs(data);
    } catch (e) {
      console.error("Failed to load RGS constructeurs:", e);
    }
  }, []);

  useEffect(() => {
    loadRgsConstructeurs();
  }, [loadRgsConstructeurs]);

  const handleSelectConstructeur = useCallback(async (id: string, nom: string) => {
    setSelectedConstructeur(id);
    setSelectedConstructeurName(nom);
    setSelectedRgsConsole(null);
    setSelectedRgsConsoleName(null);
    setRgsLiens([]);
    setRgsFolderFiles([]);
    setSelectedRgsLien(null);
    setRgsLoading(true);
    try {
      const data = await invoke<RgsConsole[]>("get_rgs_consoles", { constructeurId: id });
      setRgsConsoles(data);
    } catch (e: any) {
      showToast(`Failed to load consoles: ${e}`, "error");
    } finally {
      setRgsLoading(false);
    }
  }, [showToast]);

  const handleOpenRgsLink = useCallback(async (urlOrLien: string | RgsLien) => {
    const isString = typeof urlOrLien === "string";
    let url = isString ? urlOrLien : urlOrLien.url;
    const lien = isString ? null : urlOrLien;

    // Add 1fichier affiliate tag
    if (url.includes("1fichier.com") && !url.includes("af=")) {
      url += (url.includes("?") ? "&" : "?") + "af=3186111";
    }
    
    // If it's a directory, scrape it instead of opening it
    if (url.includes("1fichier.com/dir/")) {
      setRgsLoading(true);
      if (lien) setSelectedRgsLien(lien);
      try {
        const files = await invoke<RgsFile[]>("scrape_1fichier_dir", { url });
        setRgsFolderFiles(files);
        showToast(`📁 Loaded ${files.length} files from folder`, "success");
      } catch (e: any) {
        showToast(`Failed to scrape folder: ${e}`, "error");
        // Fallback to opening in browser if scraping fails
        await openUrl(url).catch(() => window.open(url, "_blank"));
      } finally {
        setRgsLoading(false);
      }
      return;
    }

    // Individual file links: copy password and open in system browser
    if (lien && lien.mot_de_passe) {
      try {
        await navigator.clipboard.writeText(lien.mot_de_passe);
        showToast(`🔑 Password copied! Opening link...`, "success");
      } catch {
        showToast(`Password: ${lien.mot_de_passe}`, "info");
      }
    } else {
      showToast("Opening download link...", "info");
    }
    
    // Open in system browser to bypass anti-embedding protections
    try {
      await openUrl(url).catch(() => window.open(url, "_blank"));
    } catch (e) {
      console.error("Failed to open URL:", e);
      // Even if opening fails, we set the pending state so the user can import manually
    }
    
    // Set pending import state to help the user finalize
    const currentConsole = selectedRgsConsoleName || "Nintendo Switch";
    setPendingImportConsole(currentConsole);
    showToast(`Once downloaded, click 'Finalize' to move and unzip the game to your ${currentConsole} library!`, "info");
  }, [showToast, selectedRgsConsoleName]);

  const handleImportRom = useCallback(async (targetConsole: string) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'ROM Archives', extensions: ['zip', '7z', 'iso', 'bin', 'nes', 'sfc', 'n64', 'z64', 'rvz', 'wbfs', 'chd', 'xci', 'nsp', 'pbp', 'cso', 'rvz', 'wud', 'wux', 'rpx'] }]
      });

      if (!selected) return;
      
      setRgsLoading(true);
      const path = Array.isArray(selected) ? selected[0] : selected;
      
      const result = await invoke<string>("finalize_rgs_import", { 
        srcPath: path, 
        console: targetConsole 
      });

      showToast(result, "success");
      setPendingImportConsole(null);
      // Refresh library
      loadData();
    } catch (e: any) {
      showToast(`Import failed: ${e}`, "error");
    } finally {
      setRgsLoading(false);
    }
  }, [showToast, loadData]);

  const handleSelectRgsConsole = useCallback(async (id: string, nom: string) => {
    setSelectedRgsConsole(id);
    setSelectedRgsConsoleName(nom);
    setRgsFolderFiles([]);
    setSelectedRgsLien(null);
    setRgsLoading(true);
    try {
      const data = await invoke<RgsLien[]>("get_rgs_liens", { consoleId: id });
      setRgsLiens(data);
    } catch (e: any) {
      showToast(`Failed to load links: ${e}`, "error");
    } finally {
      setRgsLoading(false);
    }
  }, [showToast]);

  const handleSelectRgsSearchResult = useCallback(async (result: RgsSearchResult) => {
    if (result.type_result === 'console') {
      await handleSelectRgsConsole(result.id, result.nom);
      setRgsSearchQuery("");
      setRgsSearchResults([]);
    } else if (result.url) {
      // It's a direct link result
      const lienMock: RgsLien = {
        id: result.lien_id || result.id,
        url: result.url,
        nb_fichiers: "1",
        taille: "Unknown",
        mot_de_passe: null,
        createur: "RGS Search",
        informations: result.nom, 
        dossier: null,
        is_signaled: "0",
        date_creation: null
      };
      handleOpenRgsLink(lienMock);
    }
  }, [handleSelectRgsConsole, handleOpenRgsLink]);

  // Debounced Search Effect for RGS
  useEffect(() => {
    if (rgsSearchQuery.trim().length < 2) {
      setRgsSearchResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      setIsSearchingRgs(true);
      try {
        const results = await invoke<RgsSearchResult[]>("search_rgs", { query: rgsSearchQuery });
        setRgsSearchResults(results);
      } catch (e) {
        console.error("RGS Search failed:", e);
      } finally {
        setIsSearchingRgs(false);
      }
    }, 500);

    return () => clearTimeout(timer);
  }, [rgsSearchQuery]);

  const searchStore = useCallback(async (query: string, consoleF: string | null) => {
    setIsSearchingStore(true);
    try {
      if (!query && !consoleF) {
        // Load featured/popular games when store is first opened or search cleared
        const results = await invoke<RomStoreEntry[]>("get_featured_games");
        setStoreRoms(results);
      } else {
        const results = await invoke<RomStoreEntry[]>("search_rom_store", { 
          query, 
          consoleFilter: consoleF 
        });
        setStoreRoms(results);
      }
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
    showToast(`Téléchargement de ${rom.name}...`, "info");
    setDownloading(prev => [...prev, rom.id]);
    setDownloadNames(prev => ({ ...prev, [rom.id]: rom.name }));
    try {
      const result = await invoke<string>("download_rom", {
        downloadUrlArg: rom.download_url,
        console: rom.console,
        romName: rom.name,
        fileNameArg: rom.file_name,
        iaId: rom.ia_id || null,
        storeId: rom.id,
      });
      showToast(`${rom.name} téléchargé avec succès !`, "success");
      sendNotification({ title: "EmuWorld — Téléchargement terminé", body: rom.name });
      loadData();
      triggerHiddenAchievement("first_download");
    } catch (err: any) {
      showToast(`Échec du téléchargement : ${err}`, "error");
    } finally {
      setDownloading(prev => prev.filter(id => id !== rom.id));
      setDownloadNames(prev => { const n = { ...prev }; delete n[rom.id]; return n; });
      setDownloadProgress(prev => { const n = { ...prev }; delete n[rom.id]; return n; });
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

  useEffect(() => {
    const unlisten = listen<DownloadStats & { file_name?: string; file_id?: string; store_id?: string; status: string }>(
      "rom-download-progress",
      (event) => {
        const { store_id, file_id, file_name } = event.payload;
        const id = store_id || file_id || file_name;
        if (id) {
          setDownloadProgress(prev => {
            const current = prev[id] || { progress: 0, downloaded_bytes: 0, total_bytes: 0, speed_bps: 0, eta: 0 };
            return {
              ...prev,
              [id]: {
                ...current,
                ...event.payload
              }
            };
          });
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<{ game_id?: string; status: string; progress?: number; downloaded_bytes?: number; total_bytes?: number; speed_bps?: number; eta?: number }>(
      "vimm-download-progress",
      (event) => {
        const { game_id, status, progress, downloaded_bytes, total_bytes, speed_bps, eta } = event.payload;
        if (game_id && status === "downloading") {
          setDownloadProgress(prev => ({
            ...prev,
            [game_id]: {
              progress: progress || 0,
              downloaded_bytes: downloaded_bytes || 0,
              total_bytes: total_bytes || 0,
              speed_bps: speed_bps || 0,
              eta: eta || 0,
            }
          }));
        }
      }
    );
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // ---- Vimm loaders ----
  const loadVimmConsoles = useCallback(async () => {
    try {
      const data = await invoke<VimmConsole[]>("get_vimm_consoles");
      setVimmConsoles(data);
    } catch (e) {
      console.error("Failed to load Vimm consoles:", e);
    }
  }, []);

  useEffect(() => { loadVimmConsoles(); }, [loadVimmConsoles]);

  const handleSelectVimmConsole = useCallback((c: VimmConsole) => {
    setSelectedVimmConsole(c);
    setVimmGames([]);
    setVimmSearch("");
  }, []);

  const handleOpenVimmGame = useCallback(async (game: VimmGame) => {
    const targetConsole = selectedVimmConsole?.target_console || "Mixed";
    if (downloading.includes(game.id)) return;
    setDownloading(prev => [...prev, game.id]);
    setDownloadNames(prev => ({ ...prev, [game.id]: game.name }));
    showToast(`Téléchargement de ${game.name}...`, "info");
    try {
      await invoke<string>("download_vimm_rom", {
        gameId: game.id,
        gameName: game.name,
        console: targetConsole,
      });
      showToast(`${game.name} téléchargé avec succès !`, "success");
      sendNotification({ title: "EmuWorld — Téléchargement terminé", body: game.name });
      loadData();
      triggerHiddenAchievement("first_download");
    } catch (err: any) {
      showToast(`Échec : ${err}`, "error");
    } finally {
      setDownloading(prev => prev.filter(id => id !== game.id));
      setDownloadNames(prev => { const n = { ...prev }; delete n[game.id]; return n; });
      setDownloadProgress(prev => { const n = { ...prev }; delete n[game.id]; return n; });
    }
  }, [selectedVimmConsole, showToast, downloading, loadData, triggerHiddenAchievement]);

  // Debounced Vimm search — scoped to the currently selected console when available,
  // or global across all Vimm consoles otherwise.
  useEffect(() => {
    if (vimmSearch.trim().length < 2) {
      setVimmGames([]);
      setVimmLoading(false);
      return;
    }
    setVimmLoading(true);
    const timer = setTimeout(async () => {
      try {
        const data = await invoke<VimmGame[]>("search_vimm", {
          query: vimmSearch.trim(),
          consoleSlug: selectedVimmConsole?.id ?? null,
        });
        setVimmGames(data);
      } catch (e) {
        console.error("Vimm search failed:", e);
      } finally {
        setVimmLoading(false);
      }
    }, 400);
    return () => clearTimeout(timer);
  }, [vimmSearch, selectedVimmConsole]);

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

  const handleDeleteRom = async (rom: RomFile) => {
    try {
      await invoke("delete_rom", { path: rom.path });
      showToast(`Deleted ${rom.name}`, "success");
      loadData(); // This will trigger the reactive useEffect to update 'downloaded' state
    } catch (err: any) {
      showToast(`Delete failed: ${err}`, "error");
    }
  };

  const handleClearCache = async () => {
    if (!confirm("Are you sure you want to clear the cover cache? ALL covers will be re-downloaded next time they are viewed.")) return;
    try {
      await invoke("clear_cover_cache");
      showToast("Cover cache cleared! Re-scanning...", "success");
      await loadData();
      triggerHiddenAchievement("clear_covers");
    } catch (e: any) {
      showToast(`Error: ${e.message}`, "error");
    }
  };

  const handleExportConfig = async () => {
    try {
      const json: string = await invoke("export_config");
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `emuworld-backup-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      showToast("Export sauvegardé !", "success");
    } catch (e: any) {
      showToast(`Erreur export: ${e}`, "error");
    }
  };

  const handleImportConfig = async () => {
    const result = await open({ filters: [{ name: "JSON", extensions: ["json"] }], multiple: false });
    if (!result) return;
    try {
      const json: string = await invoke("read_text_file", { path: result as string });
      await invoke("import_config", { json });
      showToast("Import réussi ! Rechargement...", "success");
      await loadData();
    } catch (e: any) {
      showToast(`Erreur import: ${e}`, "error");
    }
  };

  const handleLaunch = async (rom: RomFile) => {
    try {
      console.log("handleLaunch triggered for ROM:", rom);
      const emulator = catalog.find(e => e.console === rom.console);
      if (!emulator) {
        showToast(`No emulator found for ${rom.console}. Check if it's supported!`, "error");
        return;
      }

      // Check if installed
      if (!installed.includes(emulator.id)) {
        showToast(`Please install the ${rom.console} emulator first!`, "info");
        setPage("catalog");
        setConsoleFilter(rom.console);
        setCategoryFilter(null); // Clear category filter to show the console
        return;
      }

      console.log("Found Emulator:", emulator.name, "(ID:", emulator.id, ") for console:", rom.console);
      const res: string = await invoke("launch_emulator", {
        emulatorId: emulator.id,
        romPath: rom.path || null,
        romName: rom.name || null,
        romConsole: rom.console || null,
      });
      console.log("Backend Launch Success:", res);
      updatePresence("playing", rom.name, rom.console);
      if (rom.name) {
        setLaunchSplash({ gameName: rom.name, console: rom.console });
        setTimeout(() => setLaunchSplash(null), 2500);
      }
      showToast(`Launching ${rom.name}...`, "success");
      // Announce to Discord — EmuWorld stays the big image so it reads as
      // "Playing <game> via EmuWorld" in the friend list.
      if (rom.name) {
        invoke("discord_set_playing", { gameName: rom.name }).catch(() => {});
      }
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
      if (field === "roms_directory") triggerHiddenAchievement("change_roms_dir");
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
    if (!matchesSearch || !matchesCategory || !matchesConsole) return false;
    const entry = playtime.games[`${g.console}::${g.name}`];
    if (filterMode === "favorites" && !entry?.favorite) return false;
    if (filterMode === "unplayed" && entry?.launches) return false;
    if (filterMode === "rated" && !entry?.rating) return false;
    if (collectionFilter) {
      const col = playtime.collections.find(c => c.name === collectionFilter);
      if (col && !col.games.includes(`${g.console}::${g.name}`)) return false;
    }
    return true;
  }).sort((a, b) => {
    const ea = playtime.games[`${a.console}::${a.name}`];
    const eb = playtime.games[`${b.console}::${b.name}`];
    switch (sortBy) {
      case "playtime": return (eb?.seconds ?? 0) - (ea?.seconds ?? 0);
      case "rating": return (eb?.rating ?? 0) - (ea?.rating ?? 0);
      case "last_played": return (eb?.last_played ?? "").localeCompare(ea?.last_played ?? "");
      case "launches": return (eb?.launches ?? 0) - (ea?.launches ?? 0);
      default: return a.name.localeCompare(b.name);
    }
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
      const current = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!current);
      setIsFullscreen(!current);
    } catch (err) {
      console.error("Fullscreen error:", err);
    }
  };

  const exitBigPicture = useCallback(() => {
    setBigPictureMode(false);
  }, []);

  useEffect(() => {
    if (bigPictureMode) {
      setBpSelectedIndex(0);
      setBpConsoleFilter(null);
    }
  }, [bigPictureMode]);

  const bpRoms = bpConsoleFilter ? roms.filter(r => r.console === bpConsoleFilter) : roms;
  const bpConsoles = [...new Set(roms.map(r => r.console))].sort();

  if (bigPictureMode) {
    return (
      <div className="big-picture">
        <div className="big-picture__header">
          <div className="big-picture__logo">🎮 EmuWorld</div>
          <Clock />
          <button className="big-picture__exit" onClick={exitBigPicture}>
            <X size={20} /> ESC
          </button>
        </div>

        {!bpConsoleFilter ? (
          <div className="big-picture__consoles">
            <h2 className="big-picture__section-title">{t("nav.library")}</h2>
            <div className="big-picture__console-grid">
              {bpConsoles.map((con, i) => {
                const count = roms.filter(r => r.console === con).length;
                return (
                  <motion.button
                    key={con}
                    className={`big-picture__console-card gamepad-nav-item ${bpSelectedIndex === i ? "big-picture__console-card--focused" : ""}`}
                    onClick={() => { setBpConsoleFilter(con); setBpSelectedIndex(0); }}
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                  >
                    <ConsoleLogo name={con} size={48} />
                    <span className="big-picture__console-name">{con}</span>
                    <span className="big-picture__console-count">{count} {count === 1 ? "jeu" : "jeux"}</span>
                  </motion.button>
                );
              })}
            </div>
          </div>
        ) : (
          <div className="big-picture__games">
            <div className="big-picture__breadcrumb">
              <button className="big-picture__back gamepad-nav-item" onClick={() => { setBpConsoleFilter(null); setBpSelectedIndex(0); }}>
                ← {t("nav.library")}
              </button>
              <span className="big-picture__current-console">
                <ConsoleLogo name={bpConsoleFilter} size={24} /> {bpConsoleFilter}
              </span>
              <span className="big-picture__game-count">{bpRoms.length} {bpRoms.length === 1 ? "jeu" : "jeux"}</span>
            </div>
            <div className="big-picture__game-grid">
              {bpRoms.map((rom, i) => (
                <BigPictureCard
                  key={rom.path}
                  rom={rom}
                  focused={bpSelectedIndex === i}
                  entry={playtime.games[`${rom.console}::${rom.name}`]}
                  onLaunch={() => handleLaunch(rom)}
                />
              ))}
            </div>
          </div>
        )}

        <div className="big-picture__footer">
          <span className="big-picture__hint">A — {t("gamepad.confirm")} &nbsp; B — {t("gamepad.back")} &nbsp; F11 / ESC — {t("bigPicture.exit")}</span>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="titlebar" data-tauri-drag-region onDoubleClick={maximize}>
        <div className="titlebar__logo" data-tauri-drag-region>
          <div className="titlebar__logo-icon">🎮</div>
          <span data-tauri-drag-region>EmuWorld</span>
        </div>
        <Clock />
        {updateAvailable && (
          <button
            className="titlebar__update"
            onClick={handleInstallUpdate}
            disabled={updateStatus === "downloading"}
            title={`Version ${updateAvailable.version} disponible — clique pour installer`}
          >
            {updateStatus === "downloading" ? (
              <>
                <span className="spinner" />
                {updateProgress.total
                  ? `${Math.round((updateProgress.done / updateProgress.total) * 100)}%`
                  : "Téléchargement…"}
              </>
            ) : updateStatus === "ready" ? (
              <>↻ Redémarrage…</>
            ) : updateStatus === "error" ? (
              <>⚠ Échec · réessayer</>
            ) : (
              <>✨ Mise à jour {updateAvailable.version}</>
            )}
          </button>
        )}
        <div className="titlebar__controls">
          <button className="titlebar__btn" onClick={minimize} title="Réduire"><Minus size={14} /></button>
          <button className="titlebar__btn" onClick={toggleFullscreen} title={isFullscreen ? "Quitter le plein écran" : "Plein écran"}>
            {isFullscreen ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
          </button>
          <button className="titlebar__btn titlebar__btn--close" onClick={close} title="Fermer"><X size={14} /></button>
        </div>
      </div>

      <div className="app">
        <aside className="sidebar">
          <div className="sidebar__nav">
          <div className="sidebar__section">
            <div className="sidebar__label">{t("nav.navigation")}</div>
            <button
              data-tour="emulators"
              className={`sidebar__item ${page === "catalog" && !categoryFilter ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("catalog"); setConsoleFilter(null); setCategoryFilter(null); }}
            >
              <span className="sidebar__item-icon"><Grid3X3 size={16} /></span>
              {t("nav.console")}
              <span className="sidebar__item-count">{catalog.length}</span>
            </button>
            <button
              data-tour="library"
              className={`sidebar__item ${page === "library" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("library"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><Gamepad2 size={16} /></span>
              {t("nav.roms")}
              <span className="sidebar__item-count">{roms.length}</span>
            </button>
            <button
              className={`sidebar__item ${page === "installed" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("installed"); setConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><HardDrive size={16} /></span>
              {t("nav.installed")}
              <span className="sidebar__item-count">{installedCount}</span>
            </button>
            <button
              data-tour="store"
              className={`sidebar__item ${page === "store" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("store"); setStoreConsoleFilter(null); }}
            >
              <span className="sidebar__item-icon"><ShoppingBag size={16} /></span>
              {t("nav.store")}
              <span className="sidebar__item-count">{storeRoms.length}</span>
            </button>
            <button
              className={`sidebar__item ${page === "controller" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("controller")}
            >
              <span className="sidebar__item-icon"><Gamepad2 size={16} /></span>
              {t("nav.controller")}
              {gamepadActive && <span className="sidebar__item-badge">●</span>}
            </button>
            <button
              className={`sidebar__item ${page === "leaderboard" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("leaderboard"); loadLeaderboard(); }}
            >
              <span className="sidebar__item-icon"><Trophy size={16} /></span>
              {t("nav.leaderboard")}
            </button>
            <button
              data-tour="friends"
              className={`sidebar__item ${page === "friends" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("friends"); loadFriends(); }}
            >
              <span className="sidebar__item-icon"><Users size={16} /></span>
              {t("nav.friends")}
              {pendingRequests.filter(f => f.addressee_id === user?.id).length > 0 && (
                <span className="sidebar__item-count" style={{ background: "var(--neon-red)", color: "white" }}>
                  {pendingRequests.filter(f => f.addressee_id === user?.id).length}
                </span>
              )}
            </button>
            <button
              className={`sidebar__item ${page === "stats" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("stats")}
            >
              <span className="sidebar__item-icon"><Activity size={16} /></span>
              {t("nav.stats")}
            </button>
            <button
              className={`sidebar__item ${page === "backup" ? "sidebar__item--active" : ""}`}
              onClick={() => { setPage("backup"); invoke<typeof localSaves>("scan_local_saves").then(setLocalSaves).catch(() => {}); }}
            >
              <span className="sidebar__item-icon"><Cloud size={16} /></span>
              {t("nav.backup")}
            </button>
            <button
              className={`sidebar__item ${page === "settings" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("settings")}
            >
              <span className="sidebar__item-icon"><Settings size={16} /></span>
              {t("nav.settings")}
            </button>
            <button
              className={`sidebar__item ${page === "changelogs" ? "sidebar__item--active" : ""}`}
              onClick={() => setPage("changelogs")}
            >
              <span className="sidebar__item-icon"><FileText size={16} /></span>
              {t("nav.changelogs")}
            </button>
          </div>

          <div className="sidebar__divider" />

          <div className="sidebar__section">
            {page === "catalog" ? (
              <>
                <div className="sidebar__label">{t("nav.consoles")}</div>
                <button
                  className={`sidebar__item ${!categoryFilter && !consoleFilter ? "sidebar__item--active" : ""}`}
                  onClick={() => { setCategoryFilter(null); setConsoleFilter(null); }}
                >
                  <span className="sidebar__item-icon">🕹️</span>
                  {t("nav.allConsoles")}
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
                                <span className="sidebar__item-icon"><ConsoleLogo name={con} size={14} /></span>
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
                <div className="sidebar__divider" />
                <div className="sidebar__label">{t("nav.manufacturers")}</div>
                {storeMode === "vimm"
                  ? (() => {
                      const manus = Array.from(new Set(vimmConsoles.map((c) => c.manufacturer)));
                      return manus.map((m) => (
                        <button
                          key={m}
                          className={`sidebar__item ${selectedVimmManufacturer === m ? "sidebar__item--active" : ""}`}
                          onClick={() =>
                            setSelectedVimmManufacturer(selectedVimmManufacturer === m ? null : m)
                          }
                        >
                          <span className="sidebar__item-icon"><BrandLogo brand={m} size={16} /></span>
                          {m}
                        </button>
                      ));
                    })()
                  : rgsConstructeurs.map((c) => (
                      <button
                        key={c.id}
                        className={`sidebar__item ${selectedConstructeur === c.id ? "sidebar__item--active" : ""}`}
                        onClick={() => handleSelectConstructeur(c.id, c.nom)}
                      >
                        <span className="sidebar__item-icon"><BrandLogo brand={c.nom} size={16} /></span>
                        {c.nom}
                      </button>
                    ))}
              </>
            ) : page === "library" ? (
              <>
                <div className="sidebar__label">{t("nav.library")}</div>
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
                                    <span className="sidebar__item-icon sidebar__item-icon--console"><ConsoleLogo name={con} size={14} /></span>
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

          </div>
          {/* User card at the bottom of the sidebar */}
          <div className="sidebar__user-card">
            {user ? (
              <div className="sidebar__user-info" onClick={() => { setNewPseudo(profile?.username || ""); setShowAccountModal(true); }}>
                <div className="sidebar__user-avatar">
                  {profile?.avatar_url ? (
                    <img src={`${profile.avatar_url}${avatarCacheKey ? `?v=${avatarCacheKey}` : ""}`} alt="avatar" />
                  ) : (
                    <UserIcon size={20} />
                  )}
                  <span className="sidebar__user-badge" title={`${achievementRank.rank} — ${achievementRank.count}/${achievementRank.total}`}>
                    {achievementRank.icon}
                  </span>
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
          <div className={`dynamic-bg ${bgCover ? "dynamic-bg--visible" : ""}`}>
            {bgCover && <img src={bgCover} alt="" />}
          </div>
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
                    {page === "catalog" && t("header.emulators")}
                    {page === "library" && t("header.library")}
                    {page === "installed" && t("header.installedEmulators")}
                    {page === "store" && (storeMode === "vimm" ? "Vimm's Lair" : "RetroGameSets")}
                    {page === "settings" && t("header.settings")}
                    {page === "leaderboard" && t("header.leaderboard")}
                    {page === "friends" && t("header.friends")}
                    {page === "stats" && t("header.stats")}
                    {page === "backup" && t("header.backup")}
                    {page === "controller" && t("header.controller")}
                  </h1>
                  <p className="main-content__subtitle">
                    {page === "catalog" && (
                      search.trim().length >= 2
                        ? `${filteredCatalog.length} result${filteredCatalog.length === 1 ? "" : "s"} for "${search}"`
                        : consoleFilter
                          ? `${filteredCatalog.length} emulator${filteredCatalog.length === 1 ? "" : "s"} for ${consoleFilter}`
                          : categoryFilter
                            ? `Pick a console in ${categoryFilter}`
                            : `${catalog.length} emulator${catalog.length === 1 ? "" : "s"} — pick a manufacturer`
                    )}
                    {page === "store" && storeMode === "vimm" && (
                      vimmSearch.trim().length >= 2
                        ? `${vimmGames.length} result${vimmGames.length === 1 ? "" : "s"}${selectedVimmConsole ? ` on ${selectedVimmConsole.name}` : ""}`
                        : selectedVimmConsole
                          ? `Search ${selectedVimmConsole.name} by name`
                          : "Search or pick a console"
                    )}
                    {page === "store" && storeMode === "rgs" && (selectedRgsConsoleName ? `${rgsLiens.length} packs for ${selectedRgsConsoleName}` : selectedConstructeurName ? `${rgsConsoles.length} consoles` : "Browse ROM collections")}
                    {page === "library" && (
                      consoleFilter
                        ? `${filteredGames.length} game${filteredGames.length === 1 ? "" : "s"} on ${consoleFilter}`
                        : categoryFilter
                          ? `Pick a console in ${categoryFilter}`
                          : `${roms.length} game${roms.length === 1 ? "" : "s"} — pick a manufacturer`
                    )}
                    {page === "installed" && `${installedCount} installed`}
                    {page === "settings" && t("subtitle.settings")}
                    {page === "leaderboard" && t("subtitle.leaderboard")}
                    {page === "friends" && `${friends.length} ${t("nav.friends").toLowerCase()}`}
                    {page === "stats" && t("subtitle.stats")}
                    {page === "backup" && t("subtitle.backup")}
                    {page === "controller" && (gamepadActive ? t("subtitle.controllerConnected") : t("subtitle.controllerNone"))}
                  </p>
                </div>
                <div className="main-content__actions">
                  {page === "store" && pendingImportConsole && (
                    <button className="btn btn--primary btn--glow gamepad-nav-item" onClick={() => handleImportRom(pendingImportConsole)}>
                      <CheckCircle size={14} /> Finalize {pendingImportConsole} Download
                    </button>
                  )}
                  {page === "library" && (
                    <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                      <button className={`btn btn--ghost btn--sm ${viewMode === "grid" ? "btn--active" : ""}`} onClick={() => setViewMode("grid")} title="Grid view">
                        <LayoutGrid size={14} />
                      </button>
                      <button className={`btn btn--ghost btn--sm ${viewMode === "list" ? "btn--active" : ""}`} onClick={() => setViewMode("list")} title="List view">
                        <List size={14} />
                      </button>
                      <select className="library-sort-select" value={sortBy} onChange={(e) => setSortBy(e.target.value as typeof sortBy)}>
                        <option value="name">A-Z</option>
                        <option value="playtime">Temps joué</option>
                        <option value="rating">Note</option>
                        <option value="last_played">Dernier joué</option>
                        <option value="launches">Nb launches</option>
                      </select>
                      <select className="library-sort-select" value={filterMode} onChange={(e) => setFilterMode(e.target.value as typeof filterMode)}>
                        <option value="all">Tous</option>
                        <option value="favorites">Favoris</option>
                        <option value="unplayed">Non joués</option>
                        <option value="rated">Notés</option>
                      </select>
                      <select className="library-sort-select" value={collectionFilter ?? ""} onChange={(e) => setCollectionFilter(e.target.value || null)}>
                        <option value="">Collections</option>
                        {playtime.collections.map(c => (
                          <option key={c.name} value={c.name}>{c.name} ({c.games.length})</option>
                        ))}
                      </select>
                      <button className="btn btn--ghost btn--sm" onClick={() => setShowCollectionModal(true)} title="Gérer les collections">
                        <Package size={14} />
                      </button>
                      <button className="btn btn--secondary gamepad-nav-item" onClick={() => handleImportRom("Mixed")}>
                        <Download size={14} /> Import ROM
                      </button>
                    </div>
                  )}
                  {(page === "catalog" || page === "library" || page === "store") && (
                    <>
                      <div className="search-bar">
                        {page === "store" && storeSearch !== debouncedStoreSearch ? (
                          <RefreshCw size={16} className="search-bar__icon animate-spin" />
                        ) : (
                          <Search size={16} className="search-bar__icon" />
                        )}
                        <input
                          className="search-bar__input gamepad-nav-item"
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

              {page === "catalog" && (() => {
                const renderEmuCard = (emu: EmulatorInfo) => (
                  <motion.div key={emu.id} className="emu-card" data-emu-id={emu.id}>
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
                );

                return (
                  <div className="catalog-content">
                    {/* Breadcrumb */}
                    <div className="rgs-breadcrumb">
                      <button
                        className={`rgs-breadcrumb__item ${!categoryFilter && !consoleFilter ? "rgs-breadcrumb__item--active" : ""}`}
                        onClick={() => { setCategoryFilter(null); setConsoleFilter(null); }}
                      >
                        <Globe size={14} /> All Manufacturers
                      </button>
                      {categoryFilter && (
                        <>
                          <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                          <button
                            className={`rgs-breadcrumb__item ${!consoleFilter ? "rgs-breadcrumb__item--active" : ""}`}
                            onClick={() => setConsoleFilter(null)}
                          >
                            {categoryFilter}
                          </button>
                        </>
                      )}
                      {consoleFilter && (
                        <>
                          <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                          <span className="rgs-breadcrumb__item rgs-breadcrumb__item--active">
                            {consoleFilter}
                          </span>
                        </>
                      )}
                    </div>

                    {/* ---- Continue playing (only on landing, no search / no drill-down) ---- */}
                    {!categoryFilter && !consoleFilter && search.trim().length < 2 && (() => {
                      const favEntry = Object.values(playtime.games).find((g) => g.favorite);
                      const favRom = favEntry ? roms.find((r) => r.console === favEntry.console && r.name === favEntry.name) : undefined;
                      const recent = Object.values(playtime.games)
                        .filter((g) => g.last_played)
                        .sort((a, b) => (b.last_played || "").localeCompare(a.last_played || ""))
                        .slice(0, favEntry ? 4 : 3)
                        .map((g) => ({ entry: g, rom: roms.find((r) => r.console === g.console && r.name === g.name) }))
                        .filter((x) => x.rom) as { entry: GameEntry; rom: RomFile }[];
                      const recentWithoutFav = recent.filter((x) => !(favEntry && x.entry.console === favEntry.console && x.entry.name === favEntry.name)).slice(0, 3);
                      if (!favRom && recentWithoutFav.length === 0) return null;
                      return (
                        <div className="continue-playing">
                          <div className="continue-playing__header">
                            <h2 className="continue-playing__title">Jump back in</h2>
                            <span className="continue-playing__sub">Your favorite and the 3 latest sessions</span>
                          </div>
                          <div className="continue-playing__row">
                            {favRom && (
                              <GameCard
                                key={`fav-${favRom.console}::${favRom.name}`}
                                rom={favRom}
                                onLaunch={handleLaunch}
                                onDelete={handleDeleteRom}
                                entry={playtime.games[`${favRom.console}::${favRom.name}`]}
                                onToggleFavorite={handleToggleFavorite}
                                onOpenRA={handleOpenRaModal}
                                onHover={setBgCover}
                                onRate={handleSetRating}
                                onNotes={handleOpenNotes}
                                onContextMenu={(r, x, y) => setRomContextMenu({ rom: r, x, y })}
                              />
                            )}
                            {recentWithoutFav.map(({ rom }) => (
                              <GameCard
                                key={`recent-${rom.console}::${rom.name}`}
                                rom={rom}
                                onLaunch={handleLaunch}
                                onDelete={handleDeleteRom}
                                entry={playtime.games[`${rom.console}::${rom.name}`]}
                                onToggleFavorite={handleToggleFavorite}
                                onOpenRA={handleOpenRaModal}
                                onHover={setBgCover}
                                onRate={handleSetRating}
                                onNotes={handleOpenNotes}
                                onContextMenu={(r, x, y) => setRomContextMenu({ rom: r, x, y })}
                              />
                            ))}
                          </div>
                        </div>
                      );
                    })()}

                    {search.trim().length >= 2 ? (
                      /* ---- Global search bypasses drill-down ---- */
                      <>
                        <div className="emu-grid">{filteredCatalog.map(renderEmuCard)}</div>
                        {filteredCatalog.length === 0 && (
                          <div className="empty-state">
                            <div className="empty-state__icon">🔍</div>
                            <div className="empty-state__title">No emulators match "{search}"</div>
                          </div>
                        )}
                      </>
                    ) : !categoryFilter ? (
                      /* ---- Manufacturer grid ---- */
                      <div className="rgs-console-grid">
                        {Object.entries(consolesByCategory).map(([category, cons]) => {
                          const emuCount = catalog.filter((e) => e.category === category).length;
                          return (
                            <motion.div
                              key={category}
                              className="rgs-console-card"
                              whileHover={{ scale: 1.03, y: -4 }}
                              whileTap={{ scale: 0.97 }}
                              onClick={() => setCategoryFilter(category)}
                            >
                              <div className="rgs-console-card__img">
                                <div className="vimm-console-card__fallback">
                                  <BrandLogo brand={category} size={56} />
                                </div>
                              </div>
                              <div className="rgs-console-card__info">
                                <div className="rgs-console-card__name">{category}</div>
                                <div className="rgs-console-card__meta">
                                  {cons.length} console{cons.length > 1 ? "s" : ""} • {emuCount} emulator{emuCount > 1 ? "s" : ""}
                                </div>
                              </div>
                            </motion.div>
                          );
                        })}
                      </div>
                    ) : !consoleFilter ? (
                      /* ---- Console grid for the selected manufacturer ---- */
                      <div className="rgs-console-grid">
                        {(consolesByCategory[categoryFilter] || []).sort().map((con) => {
                          const count = catalog.filter((e) => e.console === con).length;
                          return (
                            <motion.div
                              key={con}
                              className="rgs-console-card"
                              whileHover={{ scale: 1.03, y: -4 }}
                              whileTap={{ scale: 0.97 }}
                              onClick={() => setConsoleFilter(con)}
                            >
                              <div className="rgs-console-card__img">
                                <div className="vimm-console-card__fallback">
                                  <ConsoleLogo name={con} />
                                </div>
                              </div>
                              <div className="rgs-console-card__info">
                                <div className="rgs-console-card__name">{con}</div>
                                <div className="rgs-console-card__meta">
                                  {count} emulator{count > 1 ? "s" : ""}
                                </div>
                              </div>
                            </motion.div>
                          );
                        })}
                      </div>
                    ) : (
                      /* ---- Emulator grid for the selected console ---- */
                      <>
                        <div className="emu-grid">{filteredCatalog.map(renderEmuCard)}</div>
                        {filteredCatalog.length === 0 && (
                          <div className="empty-state">
                            <div className="empty-state__icon">🔍</div>
                            <div className="empty-state__title">No emulators for {consoleFilter}</div>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                );
              })()}

              {page === "store" && (
                <div className="store-source-toggle">
                  <button
                    className={`store-source-toggle__btn gamepad-nav-item ${storeMode === "vimm" ? "store-source-toggle__btn--active" : ""}`}
                    onClick={() => setStoreMode("vimm")}
                  >
                    🎮 Individual games (Vimm's Lair)
                  </button>
                  <button
                    className={`store-source-toggle__btn gamepad-nav-item ${storeMode === "rgs" ? "store-source-toggle__btn--active" : ""}`}
                    onClick={() => setStoreMode("rgs")}
                  >
                    📦 Complete packs (RetroGameSets)
                  </button>
                </div>
              )}

              {page === "store" && storeMode === "vimm" && (
                <div className="rgs-page">
                  {/* Breadcrumb */}
                  <div className="rgs-breadcrumb">
                    <button
                      className={`rgs-breadcrumb__item ${!selectedVimmConsole ? 'rgs-breadcrumb__item--active' : ''}`}
                      onClick={() => {
                        setSelectedVimmConsole(null);
                        setVimmGames([]);
                        setVimmSearch("");
                      }}
                    >
                      <Globe size={14} /> All Consoles
                    </button>
                    {selectedVimmConsole && (
                      <>
                        <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                        <span className="rgs-breadcrumb__item rgs-breadcrumb__item--active">
                          {selectedVimmConsole.name}
                        </span>
                      </>
                    )}
                  </div>

                  {/* Search bar — always visible, scoped to the current console if one is picked */}
                  <div className="rgs-search-header">
                    <div className="search-bar search-bar--glow">
                      <Search size={18} className="search-bar__icon" />
                      <input
                        type="text"
                        className="search-bar__input gamepad-nav-item"
                        placeholder={selectedVimmConsole
                          ? `Search games on ${selectedVimmConsole.name}...`
                          : "Search Vimm's Lair (type at least 2 letters)"}
                        value={vimmSearch}
                        onChange={(e) => setVimmSearch(e.target.value)}
                        autoFocus
                      />
                      {vimmLoading && <RefreshCw size={14} className="animate-spin text-cyan" />}
                    </div>
                  </div>

                  {vimmSearch.trim().length >= 2 ? (
                    /* ---- Search results: text list (no covers) ---- */
                    vimmLoading ? (
                      <div className="empty-state">
                        <RefreshCw size={48} className="animate-spin" />
                        <h3 className="empty-state__title">Searching Vimm's Lair...</h3>
                      </div>
                    ) : vimmGames.length > 0 ? (
                      <div className="rgs-folder-view">
                        <div className="rgs-folder-header">
                          <div className="rgs-folder-count">{vimmGames.length} results</div>
                        </div>
                        <div className="rgs-files-grid">
                          {vimmGames.map((game, idx) => (
                            <motion.div
                              key={game.id}
                              className="rgs-file-row vimm-game-row"
                              initial={{ opacity: 0, x: -10 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{ delay: Math.min(idx * 0.01, 0.5) }}
                            >
                              <div className="rgs-file-name" title={game.name}>{game.name}</div>
                              <div className="rgs-file-meta">
                                {game.console_name && <span className="vimm-tag vimm-tag--console">{game.console_name}</span>}
                                {game.region && <span className="vimm-tag vimm-tag--region">{game.region}</span>}
                                {game.version && <span className="vimm-tag vimm-tag--version">v{game.version}</span>}
                                {game.size && <span className="vimm-tag vimm-tag--size">{game.size}</span>}
                              </div>
                              <button
                                className={`btn btn--sm gamepad-nav-item ${downloading.includes(game.id) ? 'btn--loading' : 'btn--primary'}`}
                                onClick={() => handleOpenVimmGame(game)}
                                disabled={downloading.includes(game.id)}
                              >
                                {downloading.includes(game.id) ? (
                                  <><RefreshCw size={12} className="animate-spin" /> {downloadProgress[game.id]?.progress || 0}%</>
                                ) : (
                                  <><Download size={12} /> Download</>
                                )}
                              </button>
                            </motion.div>
                          ))}
                        </div>
                      </div>
                    ) : (
                      <div className="empty-state">
                        <div className="empty-state__icon">🔍</div>
                        <div className="empty-state__title">No games found</div>
                        <p className="empty-state__text">
                          No match for "{vimmSearch}"{selectedVimmConsole ? ` on ${selectedVimmConsole.name}` : ""}.
                        </p>
                      </div>
                    )
                  ) : !selectedVimmConsole ? (
                    /* ---- Console grid (empty query, no console selected) ---- */
                    <div className="rgs-console-grid">
                      {vimmConsoles
                        .filter((c) => !selectedVimmManufacturer || c.manufacturer === selectedVimmManufacturer)
                        .map((c) => (
                        <motion.div
                          key={c.id}
                          className="rgs-console-card"
                          whileHover={{ scale: 1.03, y: -4 }}
                          whileTap={{ scale: 0.97 }}
                          onClick={() => handleSelectVimmConsole(c)}
                        >
                          <div className="rgs-console-card__img">
                            <div className="vimm-console-card__fallback">
                              <ConsoleLogo name={c.target_console} />
                            </div>
                          </div>
                          <div className="rgs-console-card__info">
                            <div className="rgs-console-card__name">{c.name}</div>
                            <div className="rgs-console-card__meta">{c.manufacturer}</div>
                          </div>
                        </motion.div>
                      ))}
                    </div>
                  ) : (
                    /* ---- Console selected but no query yet: hint the user ---- */
                    <div className="empty-state">
                      <div className="empty-state__icon">⌨️</div>
                      <div className="empty-state__title">Type to search {selectedVimmConsole.name}</div>
                      <p className="empty-state__text">
                        Results will appear as you type. Clear the search to pick another console.
                      </p>
                    </div>
                  )}
                </div>
              )}

              {page === "store" && storeMode === "archive" && (
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
                          stats={downloadProgress[rom.id] || downloadProgress[rom.file_name]}
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

              {page === "store" && storeMode === "rgs" && (
                <div className="rgs-page">
                  {/* Breadcrumb */}
                  <div className="rgs-breadcrumb">
                    <button 
                      className={`rgs-breadcrumb__item ${!selectedConstructeur ? 'rgs-breadcrumb__item--active' : ''}`}
                      onClick={() => { setSelectedConstructeur(null); setSelectedConstructeurName(null); setRgsConsoles([]); setRgsLiens([]); setSelectedRgsConsole(null); }}
                    >
                      <Globe size={14} /> All Manufacturers
                    </button>
                    {selectedConstructeurName && (
                      <>
                        <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                        <button 
                          className={`rgs-breadcrumb__item ${!selectedRgsConsole ? 'rgs-breadcrumb__item--active' : ''}`}
                          onClick={() => { setSelectedRgsConsole(null); setSelectedRgsConsoleName(null); setRgsLiens([]); }}
                        >
                          {selectedConstructeurName}
                        </button>
                      </>
                    )}
                    {selectedRgsConsoleName && (
                      <>
                        <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                        <button 
                          className={`rgs-breadcrumb__item ${!selectedRgsLien ? 'rgs-breadcrumb__item--active' : ''}`}
                          onClick={() => { setRgsFolderFiles([]); setSelectedRgsLien(null); }}
                        >
                          {selectedRgsConsoleName}
                        </button>
                      </>
                    )}
                    {selectedRgsLien && (
                      <>
                        <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                        <span className="rgs-breadcrumb__item rgs-breadcrumb__item--active">
                          Pack: {new URL(selectedRgsLien.url).hostname.replace('www.', '')}
                        </span>
                      </>
                    )}
                  </div>

                  {/* RGS Search Header */}
                  {!selectedRgsLien && rgsFolderFiles.length === 0 && (
                    <div className="rgs-search-header">
                      <div className="search-bar search-bar--glow">
                        <Search size={18} className="search-bar__icon" />
                        <input 
                          type="text" 
                          className="search-bar__input gamepad-nav-item" 
                          placeholder="Search RetroGameSets (e.g. Mario, Zelda, PS1...)" 
                          value={rgsSearchQuery}
                          onChange={(e) => setRgsSearchQuery(e.target.value)}
                        />
                        {isSearchingRgs && <RefreshCw size={14} className="animate-spin text-cyan" />}
                      </div>
                    </div>
                  )}

                  {rgsLoading ? (
                    <div className="empty-state">
                      <RefreshCw size={48} className="animate-spin" />
                      <h3 className="empty-state__title">Loading from RetroGameSets...</h3>
                    </div>
                  ) : rgsSearchResults.length > 0 ? (
                    /* ---- Search Results Grid ---- */
                    <div className="rgs-search-results">
                      <div className="rgs-results-header">Search Results for "{rgsSearchQuery}"</div>
                      <div className="rgs-console-grid">
                        {rgsSearchResults.map((result) => (
                          <motion.div
                            key={result.id}
                            className="rgs-console-card rgs-console-card--search"
                            whileHover={{ scale: 1.03, y: -4 }}
                            whileTap={{ scale: 0.97 }}
                            onClick={() => handleSelectRgsSearchResult(result)}
                          >
                            <div className="rgs-console-card__img">
                              <img 
                                src={result.image === '404.png' ? 'https://www.retrogamesets.fr/assets/images/consoles/default.png' : `https://www.retrogamesets.fr/assets/images/consoles/${result.image}`} 
                                alt={result.nom}
                                onError={(e) => { (e.target as HTMLImageElement).src = 'https://www.retrogamesets.fr/assets/images/consoles/default.png'; }}
                              />
                            </div>
                            <div className="rgs-console-card__info">
                              <div className="rgs-console-card__type">{result.type_result}</div>
                              <div className="rgs-console-card__name">{result.nom}</div>
                              {result.constructeur_nom && (
                                <div className="rgs-console-card__count">
                                  <Globe size={12} /> {result.constructeur_nom}
                                </div>
                              )}
                            </div>
                          </motion.div>
                        ))}
                      </div>
                    </div>
                  ) : !selectedConstructeur ? (
                    /* ---- Constructeur Grid ---- */
                    <div className="rgs-constructeur-grid">
                      {rgsConstructeurs.map((c) => (
                        <motion.div
                          key={c.id}
                          className="rgs-constructeur-card"
                          whileHover={{ scale: 1.03, y: -4 }}
                          whileTap={{ scale: 0.97 }}
                          onClick={() => handleSelectConstructeur(c.id, c.nom)}
                        >
                          <div className="rgs-constructeur-card__icon"><BrandLogo brand={c.nom} size={48} /></div>
                          <div className="rgs-constructeur-card__name">{c.nom}</div>
                        </motion.div>
                      ))}
                    </div>
                  ) : !selectedRgsConsole ? (
                    /* ---- Console Grid ---- */
                    <div className="rgs-console-grid">
                      {rgsConsoles.map((c) => (
                        <motion.div
                          key={c.id}
                          className="rgs-console-card"
                          whileHover={{ scale: 1.03, y: -4 }}
                          whileTap={{ scale: 0.97 }}
                          onClick={() => handleSelectRgsConsole(c.id, c.nom)}
                        >
                          <div className="rgs-console-card__img">
                            <div className="vimm-console-card__fallback">
                              <ConsoleLogo name={c.nom} />
                            </div>
                          </div>
                          <div className="rgs-console-card__info">
                            <div className="rgs-console-card__name">{c.nom}</div>
                            <div className="rgs-console-card__count">
                              <Package size={12} /> {c.nb_liens} pack{c.nb_liens > 1 ? 's' : ''}
                            </div>
                          </div>
                        </motion.div>
                      ))}
                      {rgsConsoles.length === 0 && (
                        <div className="empty-state">
                          <div className="empty-state__icon">📦</div>
                          <h3 className="empty-state__title">No consoles with downloads</h3>
                        </div>
                      )}
                    </div>
                  ) : rgsFolderFiles.length > 0 ? (
                    /* ---- Folder Files View ---- */
                    <div className="rgs-folder-view">
                      <div className="rgs-folder-header">
                        <div className="search-bar">
                          <Search size={16} className="search-bar__icon" />
                          <input 
                            type="text" 
                            className="search-bar__input gamepad-nav-item" 
                            placeholder="Search in folder..." 
                            value={rgsFolderSearch}
                            onChange={(e) => setRgsFolderSearch(e.target.value)}
                          />
                        </div>
                        <div className="rgs-folder-count">{rgsFolderFiles.length} files</div>
                      </div>

                      <div className="rgs-files-grid">
                        {rgsFolderFiles
                          .filter(f => f.nom.toLowerCase().includes(rgsFolderSearch.toLowerCase()))
                          .map((file, idx) => (
                            <motion.div 
                              key={idx}
                              className="rgs-file-row"
                              initial={{ opacity: 0, x: -10 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{ delay: Math.min(idx * 0.01, 0.5) }}
                            >
                              <div className="rgs-file-name">{file.nom}</div>
                              <div className="rgs-file-size">{file.taille}</div>
                              <button 
                                className="btn btn--primary btn--sm"
                                onClick={() => handleOpenRgsLink(file.url)}
                              >
                                Download
                              </button>
                            </motion.div>
                          ))}
                      </div>
                    </div>
                  ) : (
                    /* ---- Links List ---- */
                    <div className="rgs-liens-list">
                      {rgsLiens.map((lien) => {
                        const hostname = (() => { try { return new URL(lien.url).hostname.replace('www.', ''); } catch { return 'link'; } })();
                        const is1fichier = lien.url.includes('1fichier.com');
                        const isTorrent = lien.url.endsWith('.torrent');
                        
                        return (
                          <motion.div
                            key={lien.id}
                            className="rgs-lien-card"
                            initial={{ opacity: 0, y: 10 }}
                            animate={{ opacity: 1, y: 0 }}
                            onClick={() => handleOpenRgsLink(lien)}
                          >
                            <div className="rgs-lien-card__header">
                              <div className="rgs-lien-card__source">
                                {is1fichier ? '☁️' : isTorrent ? '🧲' : '🌐'} {hostname}
                              </div>
                              <div className="rgs-lien-card__creator">by {lien.createur}</div>
                            </div>
                            
                            <div className="rgs-lien-card__stats">
                              <div className="rgs-lien-card__stat">
                                <Gamepad2 size={14} />
                                <span>{lien.nb_fichiers !== "0" ? `${lien.nb_fichiers} games` : 'Pack'}</span>
                              </div>
                              <div className="rgs-lien-card__stat">
                                <HardDrive size={14} />
                                <span>{lien.taille}</span>
                              </div>
                              {lien.informations && (
                                <div className="rgs-lien-card__stat rgs-lien-card__stat--info">
                                  <FileText size={14} />
                                  <span>{lien.informations}</span>
                                </div>
                              )}
                              {lien.dossier && (
                                <div className="rgs-lien-card__stat">
                                  <FolderOpen size={14} />
                                  <span>/roms/{lien.dossier}</span>
                                </div>
                              )}
                            </div>
                            
                            {lien.mot_de_passe && (
                              <div className="rgs-lien-card__password">
                                <Lock size={14} />
                                <span>Password required</span>
                                <button 
                                  className="rgs-lien-card__copy-pwd"
                                  onClick={async (e) => {
                                    e.stopPropagation();
                                    try {
                                      await navigator.clipboard.writeText(lien.mot_de_passe!);
                                      showToast('🔑 Password copied!', 'success');
                                    } catch {
                                      showToast(`Password: ${lien.mot_de_passe}`, 'info');
                                    }
                                  }}
                                >
                                  <Copy size={12} /> Copy
                                </button>
                              </div>
                            )}
                            
                            <button className="btn btn--ghost btn--full rgs-lien-card__dl-btn">
                              {lien.url.includes('/dir/') ? 'Open Folder' : 'Download File'}
                            </button>
                          </motion.div>
                        );
                      })}
                      {rgsLiens.length === 0 && (
                        <div className="empty-state">
                          <div className="empty-state__icon">📦</div>
                          <h3 className="empty-state__title">No downloads available</h3>
                          <p className="empty-state__text">This console has no download links yet</p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              )}

              {page === "library" && (() => {
                // Helper: only offer a manufacturer/console if the user actually owns ROMs for it.
                const ownedConsoles = new Set(roms.map((r) => r.console));
                const manufacturersWithRoms = Object.entries(consolesByCategory)
                  .map(([cat, cons]) => {
                    const ownedInCat = cons.filter((c) => ownedConsoles.has(c));
                    const romCount = roms.filter((r) => ownedInCat.includes(r.console)).length;
                    return { category: cat, consoles: ownedInCat, romCount };
                  })
                  .filter((m) => m.consoles.length > 0);

                return (
                  <div className="library-content">
                    {/* Breadcrumb */}
                    <div className="rgs-breadcrumb">
                      <button
                        className={`rgs-breadcrumb__item ${!categoryFilter && !consoleFilter ? "rgs-breadcrumb__item--active" : ""}`}
                        onClick={() => { setCategoryFilter(null); setConsoleFilter(null); }}
                      >
                        <Globe size={14} /> All Manufacturers
                      </button>
                      {categoryFilter && (
                        <>
                          <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                          <button
                            className={`rgs-breadcrumb__item ${!consoleFilter ? "rgs-breadcrumb__item--active" : ""}`}
                            onClick={() => setConsoleFilter(null)}
                          >
                            {categoryFilter}
                          </button>
                        </>
                      )}
                      {consoleFilter && (
                        <>
                          <ChevronRight size={14} className="rgs-breadcrumb__sep" />
                          <span className="rgs-breadcrumb__item rgs-breadcrumb__item--active">
                            {consoleFilter}
                          </span>
                        </>
                      )}
                    </div>

                    {roms.length === 0 ? (
                      <div className="empty-state">
                        <div className="empty-state__icon">📂</div>
                        <div className="empty-state__title">No ROMs found</div>
                        <button className="btn btn--primary" onClick={() => setPage("settings")}><FolderOpen size={14} /> Go to Settings</button>
                      </div>
                    ) : search.trim().length >= 2 || sortBy !== "name" || filterMode !== "all" || collectionFilter ? (
                      /* ---- Search/sort/filter bypasses the drill-down ---- */
                      <>
                        {viewMode === "grid" ? (
                          <div className="game-grid">
                            {filteredGames.map(rom => (
                              <GameCard
                                key={rom.path}
                                rom={rom}
                                onLaunch={handleLaunch}
                                onDelete={handleDeleteRom}
                                entry={playtime.games[`${rom.console}::${rom.name}`]}
                                onToggleFavorite={handleToggleFavorite}
                                onOpenRA={handleOpenRaModal}
                                onHover={setBgCover}
                                onRate={handleSetRating}
                                onNotes={handleOpenNotes}
                                onContextMenu={(r, x, y) => setRomContextMenu({ rom: r, x, y })}
                              />
                            ))}
                          </div>
                        ) : (
                          <div className="game-list">
                            <div className="game-list__header">
                              <span className="game-list__col game-list__col--name">Nom</span>
                              <span className="game-list__col game-list__col--console">Console</span>
                              <span className="game-list__col game-list__col--time">Temps</span>
                              <span className="game-list__col game-list__col--rating">Note</span>
                              <span className="game-list__col game-list__col--actions"></span>
                            </div>
                            {filteredGames.map(rom => {
                              const entry = playtime.games[`${rom.console}::${rom.name}`];
                              return (
                                <div key={rom.path} className="game-list__row gamepad-nav-item" onClick={() => handleLaunch(rom)}>
                                  <span className="game-list__col game-list__col--name">{rom.name}</span>
                                  <span className="game-list__col game-list__col--console">{rom.console}</span>
                                  <span className="game-list__col game-list__col--time">{entry ? formatPlaytime(entry.seconds) : "—"}</span>
                                  <span className="game-list__col game-list__col--rating">{"★".repeat(entry?.rating ?? 0)}{"☆".repeat(5 - (entry?.rating ?? 0))}</span>
                                  <span className="game-list__col game-list__col--actions">
                                    <button className="btn btn--ghost btn--sm" onClick={(e) => { e.stopPropagation(); handleDeleteRom(rom); }}><Trash2 size={12} /></button>
                                  </span>
                                </div>
                              );
                            })}
                          </div>
                        )}
                        {filteredGames.length === 0 && (
                          <div className="empty-state">
                            <div className="empty-state__icon">🔍</div>
                            <div className="empty-state__title">No matches for "{search}"</div>
                          </div>
                        )}
                      </>
                    ) : !categoryFilter ? (
                      /* ---- Manufacturer grid ---- */
                      <div className="rgs-console-grid">
                        {manufacturersWithRoms.map(({ category, consoles, romCount }) => (
                          <motion.div
                            key={category}
                            className="rgs-console-card"
                            whileHover={{ scale: 1.03, y: -4 }}
                            whileTap={{ scale: 0.97 }}
                            onClick={() => setCategoryFilter(category)}
                          >
                            <div className="rgs-console-card__img">
                              <div className="vimm-console-card__fallback">
                                <BrandLogo brand={category} size={56} />
                              </div>
                            </div>
                            <div className="rgs-console-card__info">
                              <div className="rgs-console-card__name">{category}</div>
                              <div className="rgs-console-card__meta">
                                {consoles.length} console{consoles.length > 1 ? "s" : ""} • {romCount} game{romCount > 1 ? "s" : ""}
                              </div>
                            </div>
                          </motion.div>
                        ))}
                      </div>
                    ) : !consoleFilter ? (
                      /* ---- Console grid for the selected manufacturer ---- */
                      <div className="rgs-console-grid">
                        {(consolesByCategory[categoryFilter] || [])
                          .filter((con) => ownedConsoles.has(con))
                          .sort()
                          .map((con) => {
                            const count = roms.filter((r) => r.console === con).length;
                            return (
                              <motion.div
                                key={con}
                                className="rgs-console-card"
                                whileHover={{ scale: 1.03, y: -4 }}
                                whileTap={{ scale: 0.97 }}
                                onClick={() => setConsoleFilter(con)}
                              >
                                <div className="rgs-console-card__img">
                                  <div className="vimm-console-card__fallback">
                                    <ConsoleLogo name={con} />
                                  </div>
                                </div>
                                <div className="rgs-console-card__info">
                                  <div className="rgs-console-card__name">{con}</div>
                                  <div className="rgs-console-card__meta">
                                    {count} game{count > 1 ? "s" : ""}
                                  </div>
                                </div>
                              </motion.div>
                            );
                          })}
                      </div>
                    ) : (
                      /* ---- Game grid/list for the selected console ---- */
                      <>
                        {viewMode === "grid" ? (
                          <div className="game-grid" data-tour="play">
                            {filteredGames.map(rom => (
                              <GameCard
                                key={rom.path}
                                rom={rom}
                                onLaunch={handleLaunch}
                                onDelete={handleDeleteRom}
                                entry={playtime.games[`${rom.console}::${rom.name}`]}
                                onToggleFavorite={handleToggleFavorite}
                                onOpenRA={handleOpenRaModal}
                                onHover={setBgCover}
                                onRate={handleSetRating}
                                onNotes={handleOpenNotes}
                                onContextMenu={(r, x, y) => setRomContextMenu({ rom: r, x, y })}
                              />
                            ))}
                          </div>
                        ) : (
                          <div className="game-list">
                            <div className="game-list__header">
                              <span className="game-list__col game-list__col--name">Nom</span>
                              <span className="game-list__col game-list__col--console">Console</span>
                              <span className="game-list__col game-list__col--time">Temps</span>
                              <span className="game-list__col game-list__col--rating">Note</span>
                              <span className="game-list__col game-list__col--actions"></span>
                            </div>
                            {filteredGames.map(rom => {
                              const entry = playtime.games[`${rom.console}::${rom.name}`];
                              return (
                                <div key={rom.path} className="game-list__row gamepad-nav-item" onClick={() => handleLaunch(rom)}>
                                  <span className="game-list__col game-list__col--name">{rom.name}</span>
                                  <span className="game-list__col game-list__col--console">{rom.console}</span>
                                  <span className="game-list__col game-list__col--time">{entry ? formatPlaytime(entry.seconds) : "—"}</span>
                                  <span className="game-list__col game-list__col--rating">{"★".repeat(entry?.rating ?? 0)}{"☆".repeat(5 - (entry?.rating ?? 0))}</span>
                                  <span className="game-list__col game-list__col--actions">
                                    <button className="btn btn--ghost btn--sm" onClick={(e) => { e.stopPropagation(); handleDeleteRom(rom); }}><Trash2 size={12} /></button>
                                  </span>
                                </div>
                              );
                            })}
                          </div>
                        )}
                        {filteredGames.length === 0 && (
                          <div className="empty-state">
                            <div className="empty-state__icon">🔍</div>
                            <div className="empty-state__title">No matches</div>
                            <p className="empty-state__text">No ROMs match the current search on {consoleFilter}.</p>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                );
              })()}

              {page === "installed" && (
                <div className="emu-grid">
                  {catalog.filter(e => installed.includes(e.id)).map(emu => (
                    <motion.div key={emu.id} className="emu-card" data-emu-id={emu.id}>
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
                    <div className="settings__group-title"><Globe size={16} /> {t("settings.language")}</div>
                    <p className="settings__field-desc" style={{ marginBottom: 12 }}>
                      {t("settings.languageDesc")}
                    </p>
                    <div className="settings__field" style={{ gap: 8 }}>
                      <button
                        className={`btn btn--sm ${locale === "fr" ? "btn--primary" : "btn--ghost"}`}
                        onClick={() => setLocale("fr")}
                      >
                        🇫🇷 Français
                      </button>
                      <button
                        className={`btn btn--sm ${locale === "en" ? "btn--primary" : "btn--ghost"}`}
                        onClick={() => setLocale("en")}
                      >
                        🇬🇧 English
                      </button>
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><Palette size={16} /> {t("settings.theme")}</div>
                    <div className="theme-picker">
                      {[
                        { id: "default", name: "Discord Dark", color: "#5865F2" },
                        { id: "midnight", name: "Midnight Blue", color: "#3b82f6" },
                        { id: "oled", name: "OLED Black", color: "#ffffff" },
                        { id: "retro", name: "Retro Green", color: "#22c55e" },
                        { id: "sakura", name: "Sakura", color: "#ec4899" },
                        { id: "sunset", name: "Sunset", color: "#f59e0b" },
                      ].map(th => (
                        <button
                          key={th.id}
                          className={`theme-picker__item ${theme === th.id ? "theme-picker__item--active" : ""}`}
                          onClick={() => { setTheme(th.id); setAccentHue(null); }}
                        >
                          <span className="theme-picker__swatch" style={{ background: th.color }} />
                          <span className="theme-picker__name">{th.name}</span>
                        </button>
                      ))}
                    </div>
                    <div className="settings__field" style={{ marginTop: 12 }}>
                      <label className="settings__field-label">{t("settings.customAccent")}</label>
                      <input
                        type="range"
                        min="0"
                        max="360"
                        value={accentHue ?? 240}
                        onChange={(e) => setAccentHue(parseInt(e.target.value))}
                        className="theme-hue-slider"
                        style={{ background: `linear-gradient(to right, hsl(0,80%,65%), hsl(60,80%,65%), hsl(120,80%,65%), hsl(180,80%,65%), hsl(240,80%,65%), hsl(300,80%,65%), hsl(360,80%,65%))` }}
                      />
                      {accentHue !== null && (
                        <button className="btn btn--ghost btn--sm" onClick={() => setAccentHue(null)}>{t("settings.reset")}</button>
                      )}
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><FolderOpen size={16} /> {t("settings.directories")}</div>
                    <div className="settings__field">
                      <label className="settings__field-label">{t("settings.romsFolder")}</label>
                      <input className="settings__field-input" value={config.roms_directory} readOnly />
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => handleBrowseFolder("roms_directory")}>{t("settings.browse")}</button>
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">{t("settings.emulatorsFolder")}</label>
                      <input className="settings__field-input" value={config.emulators_directory} readOnly />
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => handleBrowseFolder("emulators_directory")}>{t("settings.browse")}</button>
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><RefreshCw size={16} /> {t("settings.maintenance")}</div>
                    <div className="settings__field">
                      <div className="settings__field-info">
                        <label className="settings__field-label">{t("settings.coverCache")}</label>
                        <p className="settings__field-desc">{t("settings.coverCacheDesc")}</p>
                      </div>
                      <button className="btn btn--danger btn--sm gamepad-nav-item" onClick={handleClearCache}>
                        <X size={14} /> {t("settings.clearCache")}
                      </button>
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><Package size={16} /> {t("settings.importExport")}</div>
                    <p className="settings__field-desc" style={{ marginBottom: 12 }}>
                      {t("settings.importExportDesc")}
                    </p>
                    <div className="settings__field" style={{ gap: 8 }}>
                      <button className="btn btn--primary btn--sm gamepad-nav-item" onClick={handleExportConfig}>
                        <Upload size={14} /> {t("settings.export")}
                      </button>
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={handleImportConfig}>
                        <Download size={14} /> {t("settings.import")}
                      </button>
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><Activity size={16} /> {t("settings.boxartLogs")}</div>
                    <div className="settings__logs">
                      {boxartLogs.length === 0 ? (
                        <p className="settings__field-desc">{t("settings.noLogs")}</p>
                      ) : (
                        <div className="logs-container">
                          {boxartLogs.map((log, i) => (
                            <div key={i} className="log-item">
                              <span className="log-game">[{log.game}]</span>
                              <span className="log-url">{log.url}</span>
                              <span className={`log-status log-status--${log.status.toLowerCase().includes("success") ? "success" : "info"}`}>
                                {log.status}
                              </span>
                            </div>
                          ))}
                          <div ref={logsEndRef} />
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="settings__group">
                    <div className="settings__group-title"><Trophy size={16} /> {t("settings.retroachievements")}</div>
                    <p className="settings__field-desc" style={{ marginBottom: 12 }}>
                      {t("settings.raDesc")}
                    </p>
                    <div className="settings__field">
                      <label className="settings__field-label">{t("settings.raUsername")}</label>
                      <input
                        className="settings__field-input"
                        value={raUsername}
                        onChange={(e) => setRaUsername(e.target.value)}
                        placeholder={t("settings.raUsernamePlaceholder")}
                      />
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">{t("settings.raApiKey")}</label>
                      <input
                        className="settings__field-input"
                        type="password"
                        value={raApiKey}
                        onChange={(e) => setRaApiKey(e.target.value)}
                        placeholder={t("settings.raApiKeyPlaceholder")}
                      />
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">{t("settings.raPassword")}</label>
                      <input
                        className="settings__field-input"
                        type="password"
                        value={raPassword}
                        onChange={(e) => setRaPassword(e.target.value)}
                        placeholder={t("settings.raPasswordPlaceholder")}
                      />
                    </div>
                    <div className="settings__field" style={{ justifyContent: "flex-end", gap: 8, flexWrap: "wrap" }}>
                      <button
                        className="btn btn--primary btn--sm"
                        onClick={handleSaveRaCredentials}
                        disabled={!raUsername || !raApiKey}
                      >
                        <Check size={14} /> {t("settings.raSaveApi")}
                      </button>
                      <button
                        className="btn btn--primary btn--sm"
                        onClick={handleRaLogin}
                        disabled={!raUsername || !raPassword || raLoginLoading}
                        style={{ background: "linear-gradient(180deg, #f59e0b 0%, #d97706 100%)" }}
                      >
                        {raLoginLoading ? <RefreshCw size={14} className="animate-spin" /> : <Lock size={14} />}
                        {raLoginLoading ? ` ${t("settings.raConnecting")}` : ` ${t("settings.raLogin")}`}
                      </button>
                      {raUsername && (
                        <button
                          className="btn btn--ghost btn--sm"
                          onClick={() => openUrl(`https://retroachievements.org/user/${raUsername}`)}
                        >
                          <ExternalLink size={14} /> {t("settings.profile")}
                        </button>
                      )}
                    </div>
                    {raToken && (
                      <div className="settings__field" style={{ marginTop: 8, flexDirection: "column", alignItems: "flex-start", gap: 8 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "0.8rem", color: "var(--neon-green)" }}>
                          <CheckCircle size={14} /> {t("settings.raTokenActive")}
                        </div>
                        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                          <button
                            className="btn btn--primary btn--sm"
                            onClick={handleConfigureRaEmulators}
                            style={{ background: "linear-gradient(180deg, #22c55e 0%, #16a34a 100%)" }}
                          >
                            <Gamepad2 size={14} /> {t("settings.raConfigureEmulators")}
                          </button>
                          <button
                            className="btn btn--primary btn--sm"
                            onClick={handleDownloadRaCores}
                            disabled={raDownloadingCores}
                            style={{ background: "linear-gradient(180deg, #6366f1 0%, #4f46e5 100%)" }}
                          >
                            {raDownloadingCores ? <RefreshCw size={14} className="animate-spin" /> : <Download size={14} />}
                            {raDownloadingCores ? ` ${t("settings.raDownloading")}` : ` ${t("settings.raInstallCores")}`}
                          </button>
                        </div>
                        <p className="settings__field-desc" style={{ margin: 0 }}>
                          {t("settings.raEmuDesc")}
                        </p>
                      </div>
                    )}
                  </div>

                  {raUsername && raApiKey && (
                    <div className="settings__group">
                      <div className="settings__group-title" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                        <span><Trophy size={16} /> {t("settings.raCompletedGames")}</span>
                        <button className="btn btn--ghost btn--sm" onClick={handleLoadRaProfile} disabled={raProfileLoading}>
                          <RefreshCw size={12} className={raProfileLoading ? "animate-spin" : ""} /> {raProfileLoading ? t("settings.raLoading") : t("settings.raLoad")}
                        </button>
                      </div>
                      {raCompletedGames.length > 0 ? (
                        <div className="ra-completed-grid">
                          {raCompletedGames
                            .filter(g => g.num_awarded >= g.max_possible && g.max_possible > 0)
                            .map(game => (
                            <div
                              key={`${game.game_id}-${game.hardcore_mode}`}
                              className="ra-completed-card"
                              onClick={() => handleOpenRaFromCompleted(game)}
                            >
                              <img
                                src={`https://retroachievements.org${game.image_icon}`}
                                alt={game.title}
                                className="ra-completed-card__icon"
                              />
                              <div className="ra-completed-card__info">
                                <div className="ra-completed-card__title">{game.title}</div>
                                <div className="ra-completed-card__meta">
                                  {game.console_name} {game.hardcore_mode && <span className="ra-completed-card__hc">HC</span>}
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      ) : !raProfileLoading ? (
                        <p className="settings__field-desc">{t("settings.raLoadPrompt")}</p>
                      ) : null}
                    </div>
                  )}
                </div>
              )}

              {page === "leaderboard" && (
                <div className="settings-content">
                  <div className="settings__group">
                    <div className="settings__group-title" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <span><Trophy size={16} /> {t("leaderboard.weeklyRanking")}</span>
                      <button className="btn btn--ghost btn--sm" onClick={loadLeaderboard} disabled={leaderboardLoading}>
                        <RefreshCw size={14} className={leaderboardLoading ? "animate-spin" : ""} /> {t("leaderboard.refresh")}
                      </button>
                    </div>
                    {leaderboardLoading && <p style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>{t("leaderboard.loading")}</p>}
                    {!leaderboardLoading && leaderboard.length === 0 && (
                      <p style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>Aucun joueur avec un profil public trouvé. Active ton profil public dans les paramètres du compte.</p>
                    )}
                    {leaderboard.length > 0 && (
                      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                        {leaderboard.map((entry, i) => {
                          const medal = i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : `${i + 1}.`;
                          const fmtTime = (s: number) => {
                            if (s >= 3600) return `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, '0')}`;
                            if (s >= 60) return `${Math.floor(s / 60)} min`;
                            return `${s}s`;
                          };
                          const isMe = entry.user_id === user?.id;
                          return (
                            <div key={entry.user_id} className="leaderboard-entry" style={{
                              display: "flex", alignItems: "center", gap: 12, padding: "10px 12px",
                              borderRadius: 8, background: isMe ? "rgba(99, 102, 241, 0.15)" : "rgba(255,255,255,0.03)",
                              border: isMe ? "1px solid rgba(99, 102, 241, 0.3)" : "1px solid transparent",
                            }}>
                              <span style={{ fontSize: "1.2rem", width: 32, textAlign: "center" }}>{medal}</span>
                              {entry.avatar_url ? (
                                <img src={entry.avatar_url} alt="" style={{ width: 32, height: 32, borderRadius: "50%", objectFit: "cover" }} />
                              ) : (
                                <div style={{ width: 32, height: 32, borderRadius: "50%", background: "var(--border)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "0.7rem", fontWeight: 700 }}>
                                  {entry.username.slice(0, 2).toUpperCase()}
                                </div>
                              )}
                              <div style={{ flex: 1 }}>
                                <div style={{ fontWeight: 600, fontSize: "0.85rem" }}>{entry.username} {isMe && <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>(toi)</span>}</div>
                                <div style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
                                  {entry.week_games} jeu{entry.week_games > 1 ? "x" : ""} cette semaine · {entry.total_launches} launches total
                                </div>
                              </div>
                              <div style={{ textAlign: "right" }}>
                                <div style={{ fontWeight: 700, fontSize: "0.9rem", color: "var(--accent)" }}>{fmtTime(entry.week_seconds)}</div>
                                <div style={{ fontSize: "0.68rem", color: "var(--text-muted)" }}>total {fmtTime(entry.total_seconds)}</div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {page === "friends" && (
                <div className="settings-content">
                  {!user ? (
                    <div className="settings__group">
                      <p className="settings__field-desc">Connecte-toi pour ajouter des amis et voir leur activité.</p>
                    </div>
                  ) : (
                    <>
                      {/* Search for users */}
                      <div className="settings__group">
                        <div className="settings__group-title"><UserPlus size={16} /> Ajouter un ami</div>
                        <div className="settings__field">
                          <input
                            className="settings__field-input"
                            placeholder="Rechercher un pseudo..."
                            value={friendSearch}
                            onChange={(e) => { setFriendSearch(e.target.value); searchFriends(e.target.value); }}
                          />
                        </div>
                        {friendSearchResults.length > 0 && (
                          <div className="friends-search-results">
                            {friendSearchResults.map(u => {
                              const alreadyFriend = friends.some(f =>
                                f.requester_id === u.id || f.addressee_id === u.id
                              );
                              const alreadyPending = pendingRequests.some(f =>
                                f.requester_id === u.id || f.addressee_id === u.id
                              );
                              return (
                                <div key={u.id} className="friend-card">
                                  <div className="friend-card__avatar">
                                    {u.avatar_url ? (
                                      <img src={u.avatar_url} alt="" />
                                    ) : (
                                      <div className="friend-card__avatar-placeholder">
                                        {(u.username || "?").slice(0, 2).toUpperCase()}
                                      </div>
                                    )}
                                  </div>
                                  <span className="friend-card__name">{u.username}</span>
                                  {alreadyFriend ? (
                                    <span className="friend-card__badge friend-card__badge--accepted"><UserCheck size={12} /> Ami</span>
                                  ) : alreadyPending ? (
                                    <span className="friend-card__badge friend-card__badge--pending">En attente</span>
                                  ) : (
                                    <button className="btn btn--primary btn--sm" onClick={() => sendFriendRequest(u.id)}>
                                      <UserPlus size={12} /> Ajouter
                                    </button>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        )}
                      </div>

                      {/* Pending requests */}
                      {pendingRequests.filter(f => f.addressee_id === user.id).length > 0 && (
                        <div className="settings__group">
                          <div className="settings__group-title"><UserPlus size={16} /> Demandes reçues</div>
                          {pendingRequests.filter(f => f.addressee_id === user.id).map(req => (
                            <div key={req.id} className="friend-card">
                              <div className="friend-card__avatar">
                                {req.profile?.avatar_url ? (
                                  <img src={req.profile.avatar_url} alt="" />
                                ) : (
                                  <div className="friend-card__avatar-placeholder">
                                    {(req.profile?.username || "?").slice(0, 2).toUpperCase()}
                                  </div>
                                )}
                              </div>
                              <span className="friend-card__name">{req.profile?.username || "Anonyme"}</span>
                              <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
                                <button className="btn btn--primary btn--sm" onClick={() => acceptFriendRequest(req.id)}>
                                  <UserCheck size={12} /> Accepter
                                </button>
                                <button className="btn btn--danger btn--sm" onClick={() => declineFriendRequest(req.id)}>
                                  <UserX size={12} />
                                </button>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}

                      {/* Sent pending */}
                      {pendingRequests.filter(f => f.requester_id === user.id).length > 0 && (
                        <div className="settings__group">
                          <div className="settings__group-title"><UserPlus size={16} /> Demandes envoyées</div>
                          {pendingRequests.filter(f => f.requester_id === user.id).map(req => (
                            <div key={req.id} className="friend-card">
                              <div className="friend-card__avatar">
                                {req.profile?.avatar_url ? (
                                  <img src={req.profile.avatar_url} alt="" />
                                ) : (
                                  <div className="friend-card__avatar-placeholder">
                                    {(req.profile?.username || "?").slice(0, 2).toUpperCase()}
                                  </div>
                                )}
                              </div>
                              <span className="friend-card__name">{req.profile?.username || "Anonyme"}</span>
                              <span className="friend-card__badge friend-card__badge--pending">En attente...</span>
                              <button className="btn btn--ghost btn--sm" onClick={() => declineFriendRequest(req.id)} style={{ marginLeft: "auto" }}>
                                <X size={12} /> Annuler
                              </button>
                            </div>
                          ))}
                        </div>
                      )}

                      {/* Friends list */}
                      <div className="settings__group">
                        <div className="settings__group-title" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                          <span><Users size={16} /> Mes amis ({friends.length})</span>
                          <button className="btn btn--ghost btn--sm" onClick={loadFriends} disabled={friendsLoading}>
                            <RefreshCw size={12} className={friendsLoading ? "animate-spin" : ""} /> Rafraîchir
                          </button>
                        </div>
                        {friends.length === 0 && !friendsLoading && (
                          <p className="settings__field-desc">Aucun ami pour l'instant. Cherche un pseudo ci-dessus pour en ajouter !</p>
                        )}
                        {friends.map(f => {
                          const otherId = f.requester_id === user.id ? f.addressee_id : f.requester_id;
                          const presence = friendPresences.find(p => p.user_id === otherId);
                          const isOnline = presence && presence.status !== "offline" &&
                            (Date.now() - new Date(presence.updated_at).getTime()) < 120000;
                          const isPlaying = presence?.status === "playing";
                          return (
                            <div key={f.id} className="friend-card friend-card--clickable" onClick={() => viewFriendProfile(otherId, f.profile?.username || "Anonyme", f.profile?.avatar_url || null)}>
                              <div className="friend-card__avatar">
                                {f.profile?.avatar_url ? (
                                  <img src={f.profile.avatar_url} alt="" />
                                ) : (
                                  <div className="friend-card__avatar-placeholder">
                                    {(f.profile?.username || "?").slice(0, 2).toUpperCase()}
                                  </div>
                                )}
                                <span className={`friend-card__status ${isOnline ? (isPlaying ? "friend-card__status--playing" : "friend-card__status--online") : "friend-card__status--offline"}`} />
                              </div>
                              <div className="friend-card__info">
                                <span className="friend-card__name">{f.profile?.username || "Anonyme"}</span>
                                <span className="friend-card__activity">
                                  {isPlaying ? `Joue à ${presence.current_game}` : isOnline ? "En ligne" : "Hors ligne"}
                                </span>
                              </div>
                              <div style={{ display: "flex", gap: 4, marginLeft: "auto", alignItems: "center" }}>
                                {unreadCounts[otherId] > 0 && (
                                  <span className="friend-card__unread">{unreadCounts[otherId]}</span>
                                )}
                                <button className="btn btn--ghost btn--sm" onClick={(e) => { e.stopPropagation(); openChat(otherId, f.profile?.username || "Anonyme", f.profile?.avatar_url || null); }}>
                                  <MessageCircle size={12} />
                                </button>
                                <button className="btn btn--ghost btn--sm" onClick={(e) => { e.stopPropagation(); removeFriend(f.id); }}>
                                  <UserX size={12} />
                                </button>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </>
                  )}

                  {/* Chat Panel */}
                  {chatOpen && (
                    <div className="chat-overlay" onClick={() => setChatOpen(null)}>
                      <div className="chat-panel" onClick={(e) => e.stopPropagation()}>
                        <div className="chat-panel__header">
                          <div className="friend-card__avatar">
                            {chatOpen.avatar_url ? (
                              <img src={chatOpen.avatar_url} alt="" />
                            ) : (
                              <div className="friend-card__avatar-placeholder">
                                {chatOpen.username.slice(0, 2).toUpperCase()}
                              </div>
                            )}
                          </div>
                          <span className="chat-panel__name">{chatOpen.username}</span>
                          <button className="friend-profile-modal__close" onClick={() => setChatOpen(null)}>
                            <X size={14} />
                          </button>
                        </div>
                        <div className="chat-panel__messages">
                          {chatLoading && <p style={{ textAlign: "center", color: "var(--text-muted)", fontSize: "0.75rem" }}>Chargement...</p>}
                          {!chatLoading && chatMessages.length === 0 && (
                            <p style={{ textAlign: "center", color: "var(--text-muted)", fontSize: "0.75rem", marginTop: 40 }}>Aucun message. Dis bonjour !</p>
                          )}
                          {chatMessages.map(msg => (
                            <div key={msg.id} className={`chat-bubble ${msg.sender_id === user!.id ? "chat-bubble--mine" : "chat-bubble--theirs"}`}>
                              <p className="chat-bubble__text">{msg.content}</p>
                              <span className="chat-bubble__time">
                                {new Date(msg.created_at).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}
                              </span>
                            </div>
                          ))}
                          <div ref={chatEndRef} />
                        </div>
                        <div className="chat-panel__input">
                          <input
                            className="chat-panel__input-field"
                            placeholder="Écrire un message..."
                            value={chatInput}
                            onChange={(e) => setChatInput(e.target.value)}
                            onKeyDown={(e) => { if (e.key === "Enter") sendMessage(); }}
                            autoFocus
                          />
                          <button className="chat-panel__send" onClick={sendMessage} disabled={!chatInput.trim()}>
                            <Send size={16} />
                          </button>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Friend Profile Modal */}
                  {friendProfile && (
                    <div className="friend-profile-overlay" onClick={() => setFriendProfile(null)}>
                      <div className="friend-profile-modal" onClick={(e) => e.stopPropagation()}>
                        <button className="friend-profile-modal__close" onClick={() => setFriendProfile(null)}>
                          <X size={16} />
                        </button>
                        <div className="friend-profile-modal__header">
                          <div className="friend-card__avatar" style={{ transform: "scale(1.5)", marginRight: 8 }}>
                            {friendProfile.avatar_url ? (
                              <img src={friendProfile.avatar_url} alt="" />
                            ) : (
                              <div className="friend-card__avatar-placeholder">
                                {friendProfile.username.slice(0, 2).toUpperCase()}
                              </div>
                            )}
                          </div>
                          <div>
                            <h2 className="friend-profile-modal__name">{friendProfile.username}</h2>
                            <p className="friend-profile-modal__sub">{friendProfile.achievements} achievements</p>
                          </div>
                        </div>

                        <div className="friend-profile-modal__stats">
                          <div className="friend-profile-modal__stat">
                            <span className="friend-profile-modal__stat-value">
                              {friendProfile.totalSeconds >= 3600
                                ? `${Math.floor(friendProfile.totalSeconds / 3600)}h${String(Math.floor((friendProfile.totalSeconds % 3600) / 60)).padStart(2, '0')}`
                                : friendProfile.totalSeconds >= 60
                                  ? `${Math.floor(friendProfile.totalSeconds / 60)} min`
                                  : `${friendProfile.totalSeconds}s`}
                            </span>
                            <span className="friend-profile-modal__stat-label">Temps total</span>
                          </div>
                          <div className="friend-profile-modal__stat">
                            <span className="friend-profile-modal__stat-value">{friendProfile.totalLaunches}</span>
                            <span className="friend-profile-modal__stat-label">Launches</span>
                          </div>
                          <div className="friend-profile-modal__stat">
                            <span className="friend-profile-modal__stat-value">{friendProfile.gamesPlayed}</span>
                            <span className="friend-profile-modal__stat-label">Jeux joués</span>
                          </div>
                        </div>

                        {friendProfile.topGames.length > 0 && (
                          <div className="friend-profile-modal__section">
                            <h3 className="friend-profile-modal__section-title">Top jeux</h3>
                            {friendProfile.topGames.map((g, i) => (
                              <div key={`${g.console}::${g.name}`} className="friend-profile-modal__game">
                                <span className="friend-profile-modal__game-rank">{i + 1}.</span>
                                <span className="friend-profile-modal__game-name">{g.name}</span>
                                <span className="friend-profile-modal__game-time">
                                  {g.seconds >= 3600 ? `${Math.floor(g.seconds / 3600)}h${String(Math.floor((g.seconds % 3600) / 60)).padStart(2, '0')}` : g.seconds >= 60 ? `${Math.floor(g.seconds / 60)} min` : `${g.seconds}s`}
                                </span>
                              </div>
                            ))}
                          </div>
                        )}

                        {friendProfile.topConsoles.length > 0 && (
                          <div className="friend-profile-modal__section">
                            <h3 className="friend-profile-modal__section-title">Top consoles</h3>
                            {friendProfile.topConsoles.map(c => (
                              <div key={c.name} className="friend-profile-modal__game">
                                <span className="friend-profile-modal__game-name">{c.name}</span>
                                <span className="friend-profile-modal__game-time">
                                  {c.seconds >= 3600 ? `${Math.floor(c.seconds / 3600)}h${String(Math.floor((c.seconds % 3600) / 60)).padStart(2, '0')}` : c.seconds >= 60 ? `${Math.floor(c.seconds / 60)} min` : `${c.seconds}s`}
                                </span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {page === "stats" && (() => {
                const fmtTime = (s: number) => {
                  if (s >= 3600) return `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, '0')}`;
                  if (s >= 60) return `${Math.floor(s / 60)} min`;
                  return `${s}s`;
                };
                const games = Object.values(playtime.games);
                const totalSeconds = games.reduce((acc, g) => acc + g.seconds, 0);
                const totalLaunches = games.reduce((acc, g) => acc + g.launches, 0);
                const gamesPlayed = games.filter(g => g.launches > 0).length;
                const favCount = games.filter(g => g.favorite).length;
                const topGames = [...games].filter(g => g.seconds > 0).sort((a, b) => b.seconds - a.seconds).slice(0, 8);
                const maxSeconds = topGames[0]?.seconds ?? 1;

                const consoleMap: Record<string, number> = {};
                games.forEach(g => { consoleMap[g.console] = (consoleMap[g.console] || 0) + g.seconds; });
                const topConsoles = Object.entries(consoleMap).sort((a, b) => b[1] - a[1]).slice(0, 6);
                const maxConsoleSec = topConsoles[0]?.[1] ?? 1;

                // Heatmap: last 12 weeks
                const today = new Date();
                const heatmapDays: { date: string; count: number }[] = [];
                for (let i = 83; i >= 0; i--) {
                  const d = new Date(today);
                  d.setDate(d.getDate() - i);
                  const ds = d.toISOString().slice(0, 10);
                  const count = games.filter(g => g.last_played?.startsWith(ds)).length;
                  heatmapDays.push({ date: ds, count });
                }

                // Streak
                let streak = 0;
                for (let i = 0; i < 365; i++) {
                  const d = new Date(today);
                  d.setDate(d.getDate() - i);
                  const ds = d.toISOString().slice(0, 10);
                  if (games.some(g => g.last_played?.startsWith(ds))) streak++;
                  else break;
                }

                return (
                  <div className="stats-page">
                    {/* Big numbers */}
                    <div className="stats-cards">
                      <div className="stats-card">
                        <div className="stats-card__value">{fmtTime(totalSeconds)}</div>
                        <div className="stats-card__label">Temps total</div>
                      </div>
                      <div className="stats-card">
                        <div className="stats-card__value">{totalLaunches}</div>
                        <div className="stats-card__label">Launches</div>
                      </div>
                      <div className="stats-card">
                        <div className="stats-card__value">{gamesPlayed}</div>
                        <div className="stats-card__label">Jeux joués</div>
                      </div>
                      <div className="stats-card">
                        <div className="stats-card__value">{favCount}</div>
                        <div className="stats-card__label">Favoris</div>
                      </div>
                      <div className="stats-card">
                        <div className="stats-card__value">{streak}j</div>
                        <div className="stats-card__label">Streak</div>
                      </div>
                    </div>

                    {/* Top games bar chart */}
                    <div className="stats-section">
                      <h3 className="stats-section__title">Top jeux</h3>
                      <div className="stats-bars">
                        {topGames.map((g, i) => (
                          <div key={`${g.console}::${g.name}`} className="stats-bar-row">
                            <span className="stats-bar-row__rank">{i + 1}.</span>
                            <span className="stats-bar-row__name">{g.name}</span>
                            <div className="stats-bar-row__bar">
                              <div className="stats-bar-row__fill" style={{ width: `${(g.seconds / maxSeconds) * 100}%` }} />
                            </div>
                            <span className="stats-bar-row__time">{fmtTime(g.seconds)}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Top consoles */}
                    <div className="stats-section">
                      <h3 className="stats-section__title">Top consoles</h3>
                      <div className="stats-bars">
                        {topConsoles.map(([con, secs]) => (
                          <div key={con} className="stats-bar-row">
                            <span className="stats-bar-row__name">{con}</span>
                            <div className="stats-bar-row__bar">
                              <div className="stats-bar-row__fill stats-bar-row__fill--console" style={{ width: `${(secs / maxConsoleSec) * 100}%` }} />
                            </div>
                            <span className="stats-bar-row__time">{fmtTime(secs)}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Heatmap */}
                    <div className="stats-section">
                      <h3 className="stats-section__title">Activité (12 semaines)</h3>
                      <div className="stats-heatmap">
                        {heatmapDays.map(d => (
                          <div
                            key={d.date}
                            className={`stats-heatmap__cell stats-heatmap__cell--${Math.min(d.count, 4)}`}
                            title={`${d.date}: ${d.count} jeu${d.count > 1 ? "x" : ""}`}
                          />
                        ))}
                      </div>
                    </div>
                  </div>
                );
              })()}

              {page === "backup" && (
                <div className="settings-content">
                  <div className="settings__group">
                    <div className="settings__group-title"><Cloud size={16} /> Backblaze B2 — Cloud Backup</div>
                    <p className="settings__field-desc" style={{ marginBottom: 12 }}>
                      Sauvegardez vos saves sur le cloud (Backblaze B2 — 10GB gratuit). Vos sauvegardes de tous les émulateurs sont zipées et uploadées.
                    </p>
                    <div className="settings__field">
                      <label className="settings__field-label">Key ID</label>
                      <input className="settings__field-input" value={b2KeyId} onChange={(e) => setB2KeyId(e.target.value)} placeholder="Application Key ID" />
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">Application Key</label>
                      <input className="settings__field-input" type="password" value={b2AppKey} onChange={(e) => setB2AppKey(e.target.value)} placeholder="Application Key" />
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">Bucket ID</label>
                      <input className="settings__field-input" value={b2BucketId} onChange={(e) => setB2BucketId(e.target.value)} placeholder="Bucket ID" />
                    </div>
                    <div className="settings__field">
                      <label className="settings__field-label">Bucket Name</label>
                      <input className="settings__field-input" value={b2BucketName} onChange={(e) => setB2BucketName(e.target.value)} placeholder="Bucket Name" />
                    </div>
                    <div className="settings__field" style={{ justifyContent: "flex-end", gap: 8 }}>
                      <button className="btn btn--primary btn--sm" onClick={handleSaveB2Config} disabled={!b2KeyId || !b2AppKey || !b2BucketId}>
                        <Check size={14} /> Sauvegarder
                      </button>
                    </div>
                  </div>

                  {b2KeyId && b2AppKey && (
                    <div className="settings__group">
                      <div className="settings__group-title" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span><HardDrive size={16} /> Saves locales</span>
                        <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{localSaves.length} fichiers</span>
                      </div>
                      {localSaves.length > 0 && (() => {
                        const grouped = localSaves.reduce((acc, s) => {
                          const key = s.emulator;
                          if (!acc[key]) acc[key] = { count: 0, size: 0 };
                          acc[key].count++;
                          acc[key].size += s.size;
                          return acc;
                        }, {} as Record<string, { count: number; size: number }>);
                        return (
                        <div style={{ maxHeight: 200, overflowY: "auto", marginBottom: 12 }}>
                          {Object.entries(grouped).map(([emu, info]) => (
                            <div key={emu} style={{ display: "flex", justifyContent: "space-between", padding: "6px 0", fontSize: "0.8rem", borderBottom: "1px solid var(--border)" }}>
                              <span style={{ color: "var(--text-secondary)", fontWeight: 600 }}>{emu}</span>
                              <span style={{ color: "var(--text-muted)" }}>{info.count} fichiers — {(info.size / 1024 / 1024).toFixed(1)} MB</span>
                            </div>
                          ))}
                        </div>
                        );
                      })()}
                      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                        <button className="btn btn--primary btn--sm" onClick={handleBackupToCloud} disabled={backupLoading} style={{ background: "linear-gradient(180deg, #22c55e 0%, #16a34a 100%)" }}>
                          {backupLoading ? <RefreshCw size={14} className="animate-spin" /> : <Upload size={14} />}
                          {backupLoading ? " Backup..." : " Backup vers le cloud"}
                        </button>
                        <button className="btn btn--ghost btn--sm" onClick={handleListCloudBackups} disabled={backupLoading}>
                          <RefreshCw size={14} /> Voir les backups cloud
                        </button>
                      </div>
                    </div>
                  )}

                  {cloudBackups.length > 0 && (
                    <div className="settings__group">
                      <div className="settings__group-title"><Cloud size={16} /> Backups disponibles</div>
                      {cloudBackups.map((b, i) => (
                        <div key={i} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid var(--border)" }}>
                          <div>
                            <div style={{ fontSize: "0.8rem", fontWeight: 600 }}>{b.file_name.replace("emuworld/", "")}</div>
                            <div style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>{(b.size / 1024 / 1024).toFixed(1)} MB — {new Date(b.upload_timestamp).toLocaleDateString()}</div>
                          </div>
                          <div style={{ display: "flex", gap: "4px" }}>
                            <button className="btn btn--ghost btn--sm" onClick={() => handleRestoreBackup(b.file_id)} disabled={backupLoading}>
                              <Download size={14} /> Restore
                            </button>
                            <button className="btn btn--danger btn--sm" onClick={() => handleDeleteBackup(b.file_id, b.file_name)} disabled={backupLoading}>
                              <Trash2 size={14} />
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {page === "controller" && (
                <div className="controller-page">
                  {/* Controller selection */}
                  <div className="settings__group">
                    <div className="settings__group-title"><Gamepad2 size={16} /> Manette active</div>
                    {!gamepadActive ? (
                      <div className="controller-empty">
                        <div className="controller-empty__icon">🎮</div>
                        <p className="controller-empty__text">Connectez une manette pour commencer</p>
                        <p className="controller-empty__hint">Compatible Xbox, PlayStation, Switch Pro Controller et plus</p>
                      </div>
                    ) : (
                      <div className="controller-select">
                        <button className="controller-select__item controller-select__item--active">
                          <span className="controller-select__icon">🎮</span>
                          <span className="controller-select__name">{gamepadName}</span>
                          <span className="controller-select__index">Player 1</span>
                          <Check size={16} className="controller-select__check" />
                        </button>
                      </div>
                    )}
                  </div>

                  {/* Visual controller display */}
                  {gamepadActive && (() => {
                    void gamepadTick;
                    const gpState = gamepadStateRef.current;
                    return (
                      <div className="settings__group">
                        <div className="settings__group-title"><Activity size={16} /> État en direct</div>
                        <div className="controller-live">
                          <div className="controller-live__sticks">
                            <div className="controller-live__stick">
                              <div className="controller-live__stick-label">Left Stick</div>
                              <div className="controller-live__stick-ring">
                                <div
                                  className="controller-live__stick-dot"
                                  style={{
                                    transform: `translate(${(gpState.axes[0] ?? 0) * 20}px, ${(gpState.axes[1] ?? 0) * 20}px)`
                                  }}
                                />
                              </div>
                            </div>
                            <div className="controller-live__stick">
                              <div className="controller-live__stick-label">Right Stick</div>
                              <div className="controller-live__stick-ring">
                                <div
                                  className="controller-live__stick-dot"
                                  style={{
                                    transform: `translate(${(gpState.axes[2] ?? 0) * 20}px, ${(gpState.axes[3] ?? 0) * 20}px)`
                                  }}
                                />
                              </div>
                            </div>
                          </div>
                          <div className="controller-live__buttons">
                            {gpState.buttons.slice(0, 17).map((pressed, i) => (
                              <div key={i} className={`controller-live__btn ${pressed ? "controller-live__btn--pressed" : ""}`}>
                                {i}
                              </div>
                            ))}
                          </div>
                        </div>
                      </div>
                    );
                  })()}

                  {/* Button mapping */}
                  {gamepadActive && (
                    <div className="settings__group">
                      <div className="settings__group-title"><Settings size={16} /> Mapping des touches</div>
                      <p className="settings__field-desc">Cliquez sur un bouton puis appuyez sur la touche de votre manette pour reassigner.</p>
                      <div className="controller-mapping">
                        {gamepadConfig.mappings.map(mapping => (
                          <div key={mapping.action} className="controller-mapping__row">
                            <span className="controller-mapping__action">{(locale === "fr" ? GAMEPAD_ACTIONS_FR : GAMEPAD_ACTIONS_EN)[mapping.action]}</span>
                            <button
                              className={`controller-mapping__btn gamepad-nav-item ${remappingAction === mapping.action ? "controller-mapping__btn--listening" : ""}`}
                              onClick={() => setRemappingAction(remappingAction === mapping.action ? null : mapping.action)}
                            >
                              {remappingAction === mapping.action ? "Appuyez..." : `Button ${mapping.buttonIndex}`}
                            </button>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Deadzone */}
                  {gamepadActive && (
                    <div className="settings__group">
                      <div className="settings__group-title"><Activity size={16} /> Deadzone</div>
                      <div className="controller-deadzone">
                        <input
                          type="range"
                          min="0.05"
                          max="0.5"
                          step="0.01"
                          value={gamepadConfig.deadzone}
                          onChange={e => setGamepadConfig(prev => ({ ...prev, deadzone: parseFloat(e.target.value) }))}
                          className="controller-deadzone__slider"
                        />
                        <span className="controller-deadzone__value">{gamepadConfig.deadzone.toFixed(2)}</span>
                      </div>
                    </div>
                  )}

                  {/* Reset */}
                  {gamepadActive && (
                    <div className="settings__group">
                      <button
                        className="btn btn--danger btn--sm gamepad-nav-item"
                        onClick={() => setGamepadConfig({ selectedIndex: gamepadConfig.selectedIndex, deadzone: 0.15, mappings: [...DEFAULT_GAMEPAD_MAPPINGS] })}
                      >
                        <RefreshCw size={14} /> Réinitialiser le mapping
                      </button>
                    </div>
                  )}
                </div>
              )}

              {page === "changelogs" && (
                <div className="changelogs">
                  {changelogs.map((log: ChangelogEntry) => (
                    <div key={log.version} className="changelog-card gamepad-nav-item">
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

      {/* Gamepad context menu */}
      <AnimatePresence>
        {gamepadContextMenu && (() => {
          if (gamepadContextMenu.type === "rom") {
            const rom = roms.find(r => r.path === gamepadContextMenu.romPath);
            if (!rom) return null;
            const isFav = playtime.games[rom.name]?.favorite ?? false;
            return (
              <motion.div
                className="gamepad-context-overlay"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                onClick={() => setGamepadContextMenu(null)}
              >
                <motion.div
                  className="gamepad-context-menu"
                  initial={{ opacity: 0, scale: 0.9 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.9 }}
                  style={{ left: gamepadContextMenu.x, top: gamepadContextMenu.y }}
                  onClick={e => e.stopPropagation()}
                >
                  <div className="gamepad-context-menu__title">{rom.name}</div>
                  <button
                    className="gamepad-context-menu__btn gamepad-context-menu__btn--play gamepad-ctx-focused"
                    onClick={() => { setGamepadContextMenu(null); handleLaunch(rom); }}
                  >
                    <Play size={14} /> Jouer
                  </button>
                  <button
                    className="gamepad-context-menu__btn"
                    onClick={() => { setGamepadContextMenu(null); handleToggleFavorite(rom); }}
                  >
                    {isFav ? "★ Retirer des favoris" : "☆ Mettre en favori"}
                  </button>
                  {playtime.collections.length > 0 && playtime.collections.map(col => (
                    <button
                      key={col.name}
                      className="gamepad-context-menu__btn"
                      onClick={() => { setGamepadContextMenu(null); handleAddToCollection(col.name, rom); }}
                    >
                      <Package size={14} /> + {col.name}
                    </button>
                  ))}
                  <button
                    className="gamepad-context-menu__btn gamepad-context-menu__btn--danger"
                    onClick={() => { setGamepadContextMenu(null); handleDeleteRom(rom); }}
                  >
                    <Trash2 size={14} /> Supprimer
                  </button>
                </motion.div>
              </motion.div>
            );
          } else {
            const emu = catalog.find(e => e.id === gamepadContextMenu.emuId);
            if (!emu) return null;
            const isInstalled = installed.includes(emu.id);
            return (
              <motion.div
                className="gamepad-context-overlay"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                onClick={() => setGamepadContextMenu(null)}
              >
                <motion.div
                  className="gamepad-context-menu"
                  initial={{ opacity: 0, scale: 0.9 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.9 }}
                  style={{ left: gamepadContextMenu.x, top: gamepadContextMenu.y }}
                  onClick={e => e.stopPropagation()}
                >
                  <div className="gamepad-context-menu__title">{emu.name}</div>
                  {isInstalled ? (
                    <button
                      className="gamepad-context-menu__btn gamepad-context-menu__btn--play gamepad-ctx-focused"
                      onClick={() => { setGamepadContextMenu(null); handleLaunch({ name: "", path: "", console: emu.console, extension: "", size: 0 }); }}
                    >
                      <Play size={14} /> Lancer
                    </button>
                  ) : (
                    <button
                      className="gamepad-context-menu__btn gamepad-context-menu__btn--play gamepad-ctx-focused"
                      onClick={() => { setGamepadContextMenu(null); handleInstall(emu.id); }}
                    >
                      <Download size={14} /> Installer
                    </button>
                  )}
                  {isInstalled && (
                    <button
                      className="gamepad-context-menu__btn gamepad-context-menu__btn--danger"
                      onClick={() => { setGamepadContextMenu(null); handleUninstall(emu.id); }}
                    >
                      <Trash2 size={14} /> Désinstaller
                    </button>
                  )}
                  <button
                    className="gamepad-context-menu__btn"
                    onClick={() => { setGamepadContextMenu(null); window.open(emu.website, "_blank"); }}
                  >
                    <ExternalLink size={14} /> Site web
                  </button>
                </motion.div>
              </motion.div>
            );
          }
        })()}
      </AnimatePresence>

      {/* Virtual keyboard for gamepad */}
      <AnimatePresence>
        {gamepadKeyboard && (
          <motion.div
            className="gamepad-keyboard-overlay"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setGamepadKeyboard(null)}
          >
            <motion.div
              className="gamepad-keyboard"
              initial={{ opacity: 0, y: 40 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 40 }}
              onClick={e => e.stopPropagation()}
            >
              <div className="gamepad-keyboard__preview">
                {gamepadKeyboard.inputEl.value}<span className="gamepad-keyboard__cursor">|</span>
              </div>
              <div className="gamepad-keyboard__grid">
                {VIRTUAL_KB_KEYS.map((key, i) => (
                  <button
                    key={i}
                    className={`gamepad-keyboard__key ${i === gamepadKeyboard.keyIdx ? "gamepad-keyboard__key--focused" : ""} ${key === "⌫" ? "gamepad-keyboard__key--backspace" : ""} ${key === "OK" ? "gamepad-keyboard__key--ok" : ""} ${key === " " ? "gamepad-keyboard__key--space" : ""}`}
                    onClick={() => {
                      const kb = gamepadKeyboard;
                      const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')!.set!;
                      if (key === "⌫") {
                        nativeInputValueSetter.call(kb.inputEl, kb.inputEl.value.slice(0, -1));
                      } else if (key === "OK") {
                        setGamepadKeyboard(null);
                        return;
                      } else {
                        nativeInputValueSetter.call(kb.inputEl, kb.inputEl.value + key);
                      }
                      kb.inputEl.dispatchEvent(new Event('input', { bubbles: true }));
                    }}
                  >
                    {key === " " ? "␣" : key}
                  </button>
                ))}
              </div>
              <div className="gamepad-keyboard__hints">
                <span>A = taper</span>
                <span>X = effacer</span>
                <span>B = fermer</span>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {downloading.length > 0 && (
          <motion.div
            className="download-banner"
            initial={{ opacity: 0, y: 40 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 40 }}
          >
            <div className="download-banner__icon">
              <Download size={16} className="download-banner__pulse" />
            </div>
            <div className="download-banner__content">
              {downloading.map(id => {
                const name = downloadNames[id] || id;
                const stats = downloadProgress[id];
                const progress = stats?.progress || 0;
                const speed = stats?.speed_bps || 0;
                const eta = stats?.eta || 0;
                const formatBytes = (bytes: number) => {
                  if (bytes === 0) return '0 B';
                  const k = 1024;
                  const sizes = ['B', 'KB', 'MB', 'GB'];
                  const i = Math.floor(Math.log(bytes) / Math.log(k));
                  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
                };
                const formatEta = (s: number) => {
                  if (s <= 0) return '';
                  if (s < 60) return `${s}s`;
                  return `${Math.floor(s / 60)}m${s % 60}s`;
                };
                return (
                  <div key={id} className="download-banner__item">
                    <div className="download-banner__info">
                      <span className="download-banner__name">{name}</span>
                      <span className="download-banner__stats">
                        {progress}%{speed > 0 && ` · ${formatBytes(speed)}/s`}{eta > 0 && ` · ${formatEta(eta)}`}
                      </span>
                    </div>
                    <div className="download-banner__bar">
                      <div className="download-banner__bar-fill" style={{ width: `${progress}%` }} />
                    </div>
                  </div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Right-click context menu on game cards */}
      {romContextMenu && (
        <div className="rom-context-overlay" onClick={() => setRomContextMenu(null)}>
          <div className="rom-context-menu" style={{ left: romContextMenu.x, top: romContextMenu.y }} onClick={(e) => e.stopPropagation()}>
            <button className="rom-context-menu__btn" onClick={() => { handleLaunch(romContextMenu.rom); setRomContextMenu(null); }}>
              <Play size={14} /> Jouer
            </button>
            <button className="rom-context-menu__btn" onClick={() => { handleToggleFavorite(romContextMenu.rom); setRomContextMenu(null); }}>
              {playtime.games[`${romContextMenu.rom.console}::${romContextMenu.rom.name}`]?.favorite ? "★ Retirer des favoris" : "☆ Mettre en favori"}
            </button>
            <button className="rom-context-menu__btn" onClick={() => { handleOpenNotes(romContextMenu.rom); setRomContextMenu(null); }}>
              <StickyNote size={14} /> Notes
            </button>
            <div className="rom-context-menu__sep" />
            {playtime.collections.map(col => {
              const inCol = col.games.includes(`${romContextMenu.rom.console}::${romContextMenu.rom.name}`);
              return (
                <button key={col.name} className="rom-context-menu__btn" onClick={() => {
                  if (inCol) handleRemoveFromCollection(col.name, romContextMenu.rom);
                  else handleAddToCollection(col.name, romContextMenu.rom);
                  setRomContextMenu(null);
                }}>
                  <Package size={14} /> {inCol ? `✓ ${col.name}` : `+ ${col.name}`}
                </button>
              );
            })}
            <button className="rom-context-menu__btn" onClick={() => { setRomContextMenu(null); setShowCollectionModal(true); }}>
              <Package size={14} /> Nouvelle collection...
            </button>
            <div className="rom-context-menu__sep" />
            <button className="rom-context-menu__btn rom-context-menu__btn--danger" onClick={() => { handleDeleteRom(romContextMenu.rom); setRomContextMenu(null); }}>
              <Trash2 size={14} /> Supprimer
            </button>
          </div>
        </div>
      )}

      {/* Collections Modal */}
      <AnimatePresence>
        {showCollectionModal && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="notes-modal-overlay"
            onClick={() => setShowCollectionModal(false)}
          >
            <motion.div
              initial={{ scale: 0.9, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="notes-modal"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="notes-modal__header">
                <Package size={16} />
                <span>Collections</span>
                <button className="btn btn--ghost btn--sm" onClick={() => setShowCollectionModal(false)}><X size={14} /></button>
              </div>
              <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                <input
                  className="settings__field-input"
                  value={newCollectionName}
                  onChange={(e) => setNewCollectionName(e.target.value)}
                  placeholder="Nouvelle collection..."
                  onKeyDown={(e) => { if (e.key === "Enter" && newCollectionName.trim()) handleCreateCollection(newCollectionName.trim()); }}
                  style={{ flex: 1 }}
                />
                <button className="btn btn--primary btn--sm" onClick={() => { if (newCollectionName.trim()) handleCreateCollection(newCollectionName.trim()); }} disabled={!newCollectionName.trim()}>
                  <Check size={14} /> Créer
                </button>
              </div>
              {playtime.collections.length === 0 ? (
                <p style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>Aucune collection. Crée-en une pour organiser tes jeux !</p>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 250, overflowY: "auto" }}>
                  {playtime.collections.map(col => (
                    <div key={col.name} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 10px", borderRadius: 6, background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)" }}>
                      <div>
                        <div style={{ fontWeight: 600, fontSize: "0.85rem" }}>{col.name}</div>
                        <div style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>{col.games.length} jeu{col.games.length > 1 ? "x" : ""}</div>
                      </div>
                      <button className="btn btn--danger btn--sm" onClick={() => handleDeleteCollection(col.name)}><Trash2 size={12} /></button>
                    </div>
                  ))}
                </div>
              )}
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Notes Modal */}
      <AnimatePresence>
        {notesModal && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="notes-modal-overlay"
            onClick={() => setNotesModal(null)}
          >
            <motion.div
              initial={{ scale: 0.9, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="notes-modal"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="notes-modal__header">
                <StickyNote size={16} />
                <span>Notes — {notesModal.rom.name}</span>
                <button className="btn btn--ghost btn--sm" onClick={() => setNotesModal(null)}><X size={14} /></button>
              </div>
              <textarea
                className="notes-modal__textarea"
                value={notesModal.text}
                onChange={(e) => setNotesModal({ ...notesModal, text: e.target.value })}
                placeholder="Codes, astuces, progression..."
                autoFocus
              />
              <div className="notes-modal__actions">
                <button className="btn btn--ghost btn--sm" onClick={() => setNotesModal(null)}>Annuler</button>
                <button className="btn btn--primary btn--sm" onClick={handleSaveNotes}><Check size={14} /> Sauvegarder</button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Launch Splash */}
      <AnimatePresence>
        {launchSplash && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.3 }}
            className="launch-splash"
          >
            <motion.div
              initial={{ scale: 0.5, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 1.2, opacity: 0 }}
              transition={{ type: "spring", stiffness: 200, damping: 20 }}
              className="launch-splash__content"
            >
              <div className="launch-splash__console-icon">
                {launchSplash.console.includes("Switch") ? "🎮" :
                 launchSplash.console.includes("Wii U") ? "🎮" :
                 launchSplash.console.includes("Wii") ? "🕹️" :
                 launchSplash.console.includes("GameCube") ? "🟣" :
                 launchSplash.console.includes("Nintendo 64") ? "🔴" :
                 launchSplash.console.includes("SNES") || launchSplash.console.includes("Super Nintendo") ? "🟪" :
                 launchSplash.console.includes("NES") ? "⬜" :
                 launchSplash.console.includes("Game Boy") ? "🟩" :
                 launchSplash.console.includes("DS") || launchSplash.console.includes("3DS") ? "📱" :
                 launchSplash.console.includes("PlayStation") || launchSplash.console.includes("PS") ? "🔵" :
                 launchSplash.console.includes("PSP") ? "⚫" :
                 launchSplash.console.includes("Xbox") ? "🟢" :
                 launchSplash.console.includes("Dreamcast") ? "🌀" :
                 launchSplash.console.includes("Sega") || launchSplash.console.includes("Mega Drive") ? "🔷" :
                 "🎮"}
              </div>
              <div className="launch-splash__game">{launchSplash.gameName}</div>
              <div className="launch-splash__console-name">{launchSplash.console}</div>
              <motion.div
                className="launch-splash__bar"
                initial={{ scaleX: 0 }}
                animate={{ scaleX: 1 }}
                transition={{ duration: 2.2, ease: "linear" }}
              />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Session Recap Popup */}
      <AnimatePresence>
        {sessionRecap && (
          <motion.div
            initial={{ opacity: 0, y: 40, scale: 0.9 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 20, scale: 0.95 }}
            className="session-recap"
            onClick={() => setSessionRecap(null)}
          >
            <div className="session-recap__header">
              <Gamepad2 size={20} />
              <span>Session terminée</span>
            </div>
            <div className="session-recap__game">{sessionRecap.gameName}</div>
            <div className="session-recap__console">{sessionRecap.console}</div>
            <div className="session-recap__stats">
              <div className="session-recap__stat">
                <span className="session-recap__stat-value">
                  {sessionRecap.sessionSeconds >= 3600
                    ? `${Math.floor(sessionRecap.sessionSeconds / 3600)}h${String(Math.floor((sessionRecap.sessionSeconds % 3600) / 60)).padStart(2, '0')}`
                    : sessionRecap.sessionSeconds >= 60
                    ? `${Math.floor(sessionRecap.sessionSeconds / 60)} min`
                    : `${sessionRecap.sessionSeconds}s`}
                </span>
                <span className="session-recap__stat-label">cette session</span>
              </div>
              <div className="session-recap__stat">
                <span className="session-recap__stat-value">{sessionRecap.totalLaunches}</span>
                <span className="session-recap__stat-label">launch{sessionRecap.totalLaunches > 1 ? "es" : ""}</span>
              </div>
              <div className="session-recap__stat">
                <span className="session-recap__stat-value">
                  {sessionRecap.totalSeconds >= 3600
                    ? `${Math.floor(sessionRecap.totalSeconds / 3600)}h${String(Math.floor((sessionRecap.totalSeconds % 3600) / 60)).padStart(2, '0')}`
                    : `${Math.floor(sessionRecap.totalSeconds / 60)} min`}
                </span>
                <span className="session-recap__stat-label">total</span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

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
                        <img src={`${profile.avatar_url}${avatarCacheKey ? `?v=${avatarCacheKey}` : ""}`} alt="Avatar" className="account-modal__avatar" />
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

                {/* Gaming Profile Section — kept minimal here; full stats live on the web profile */}
                {profileStats && (profileStats.total_seconds > 0 || profileStats.favorite_count > 0) && (
                  <div className="account-modal__section">
                    <label className="account-modal__label">Gaming Profile</label>
                    <div className="gaming-profile">
                      <div className="gaming-profile__tiles">
                        <div className="gaming-profile__tile">
                          <div className="gaming-profile__tile-value">{formatPlaytime(profileStats.total_seconds) || "0m"}</div>
                          <div className="gaming-profile__tile-label">Total playtime</div>
                        </div>
                        <div className="gaming-profile__tile">
                          <div className="gaming-profile__tile-value">{profileStats.games_played}</div>
                          <div className="gaming-profile__tile-label">Games played</div>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {/* Achievements Section */}
                <div className="account-modal__section">
                  <label className="account-modal__label">
                    Achievements {achievementRank.icon} {achievementRank.rank} — {achievementRank.count}/{achievementRank.total}
                  </label>
                  <div className="achievements-grid">
                    {achievements.map((a) => (
                      <div
                        key={a.id}
                        className={`achievement-badge ${a.unlocked ? "achievement-badge--unlocked" : ""} ${a.hidden && !a.unlocked ? "achievement-badge--hidden" : ""}`}
                        title={a.hidden && !a.unlocked ? "Achievement secret — ???" : a.unlocked ? `${a.name} — ${a.description}\nDébloqué le ${a.unlocked_at ? new Date(a.unlocked_at).toLocaleDateString("fr-FR") : ""}` : `${a.name} — ${a.description}`}
                      >
                        <span className="achievement-badge__icon">{a.hidden && !a.unlocked ? "❓" : a.icon}</span>
                        <span className="achievement-badge__name">{a.hidden && !a.unlocked ? "???" : a.name}</span>
                      </div>
                    ))}
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

                {/* Cloud & public profile */}
                <div className="account-modal__section">
                  <label className="account-modal__label">Public Profile</label>
                  <div className="public-profile-toggle">
                    <div className="public-profile-toggle__info">
                      <div className="public-profile-toggle__title">
                        {profile?.public_profile ? "🌐 Public" : "🔒 Private"}
                      </div>
                      <div className="public-profile-toggle__desc">
                        {profile?.public_profile
                          ? "Your playtime stats and favorites are visible to anyone with your profile link."
                          : "Only you can see your stats. Flip this on to get a shareable profile URL."}
                      </div>
                    </div>
                    <button
                      className={`public-profile-toggle__switch ${profile?.public_profile ? "public-profile-toggle__switch--on" : ""}`}
                      onClick={handleTogglePublicProfile}
                      disabled={isTogglingPublic}
                      title="Toggle public profile"
                    >
                      <span className="public-profile-toggle__knob" />
                    </button>
                  </div>
                  {profile?.public_profile && profile.username && (
                    <button
                      className="btn btn--ghost btn--sm public-profile-toggle__view"
                      onClick={async () => {
                        // Hash route (#/u/...) — alwaysdata only serves index.html,
                        // so path-based /u/<pseudo> returns a 404.
                        const url = `https://emuworld.alwaysdata.net/#/u/${encodeURIComponent(profile.username!)}`;
                        await openUrl(url).catch(() => window.open(url, "_blank"));
                      }}
                    >
                      <ExternalLink size={12} /> View on web
                    </button>
                  )}
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
                    // Token handoff lives on the `#` fragment; the SPA router reads it once
                    // via `consumeTokenHandoff` and then navigates to whichever route follows.
                    // `/account` is the web equivalent of the in-app account modal —
                    // the SPA lands you on your own profile and auto-opens the edit modal.
                    const target = "/account";
                    // Timestamp in the query so Firefox can't silently focus an already-open
                    // EmuWorld tab (which still has `#/u/...` from a previous handoff) and
                    // skip navigating to the new URL — otherwise the tokens in the fragment
                    // never get loaded.
                    const url = `https://emuworld.alwaysdata.net/?h=${Date.now()}#access_token=${session.access_token}&refresh_token=${session.refresh_token}&token_type=bearer&next=${encodeURIComponent(target)}`;
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

      {/* ===== RETROACHIEVEMENTS MODAL ===== */}
      <AnimatePresence>
        {raModalRom && (
          <motion.div
            className="ra-overlay"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => { setRaModalRom(null); setRaGameInfo(null); }}
          >
            <motion.div
              className="ra-modal"
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              onClick={(e) => e.stopPropagation()}
            >
              <button className="ra-modal__close" onClick={() => { setRaModalRom(null); setRaGameInfo(null); }}>
                <X size={18} />
              </button>

              {raLoading ? (
                <div className="ra-modal__loading">
                  <RefreshCw size={32} className="animate-spin" />
                  <p>Recherche sur RetroAchievements...</p>
                </div>
              ) : raGameInfo ? (
                <>
                  <div className="ra-modal__header">
                    {raGameInfo.image_icon && (
                      <img
                        src={`https://retroachievements.org${raGameInfo.image_icon}`}
                        alt={raGameInfo.title}
                        className="ra-modal__game-icon"
                      />
                    )}
                    <div className="ra-modal__game-info">
                      <h2 className="ra-modal__title">{raGameInfo.title}</h2>
                      <span className="ra-modal__console">{raGameInfo.console_name}</span>
                    </div>
                  </div>

                  <div className="ra-modal__progress">
                    <div className="ra-modal__progress-bar">
                      <div
                        className="ra-modal__progress-fill"
                        style={{ width: `${raGameInfo.num_achievements > 0 ? (raGameInfo.num_earned / raGameInfo.num_achievements) * 100 : 0}%` }}
                      />
                    </div>
                    <div className="ra-modal__progress-text">
                      <span>{raGameInfo.num_earned} / {raGameInfo.num_achievements} débloqués</span>
                      <span className="ra-modal__points">
                        {raGameInfo.achievements.filter(a => a.date_earned).reduce((s, a) => s + a.points, 0)} / {raGameInfo.achievements.reduce((s, a) => s + a.points, 0)} pts
                      </span>
                    </div>
                  </div>

                  <div className="ra-modal__achievements">
                    {raGameInfo.achievements.map(ach => (
                      <div key={ach.id} className={`ra-achievement ${ach.date_earned ? "ra-achievement--unlocked" : ""}`}>
                        <img
                          src={`https://retroachievements.org/Badge/${ach.badge_name}.png`}
                          alt={ach.title}
                          className={`ra-achievement__badge ${!ach.date_earned ? "ra-achievement__badge--locked" : ""}`}
                        />
                        <div className="ra-achievement__info">
                          <div className="ra-achievement__title">
                            {ach.title}
                            <span className="ra-achievement__points">{ach.points} pts</span>
                          </div>
                          <div className="ra-achievement__desc">{ach.description}</div>
                          {ach.date_earned && (
                            <div className="ra-achievement__date">
                              {ach.date_earned_hardcore ? "🏆" : "✓"} {ach.date_earned}
                            </div>
                          )}
                        </div>
                      </div>
                    ))}
                    {raGameInfo.achievements.length === 0 && (
                      <div className="ra-modal__empty">Aucun achievement trouvé pour ce jeu.</div>
                    )}
                  </div>
                </>
              ) : (
                <div className="ra-modal__empty">
                  <p>Impossible de trouver ce jeu sur RetroAchievements.</p>
                  <p className="ra-modal__hint">Vérifiez que vos identifiants sont configurés dans les Settings.</p>
                </div>
              )}
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* ===== ONBOARDING WIZARD ===== */}
      {showOnboarding && (
        <div className="onboarding-overlay">
          <div className="onboarding-modal">
            <div className="onboarding-modal__progress">
              {[0, 1, 2, 3].map(i => (
                <div key={i} className={`onboarding-modal__dot ${onboardingStep >= i ? "onboarding-modal__dot--active" : ""}`} />
              ))}
            </div>

            {onboardingStep === 0 && (
              <div className="onboarding-modal__step">
                <div className="onboarding-modal__icon">🎮</div>
                <h1 className="onboarding-modal__title">Bienvenue sur EmuWorld</h1>
                <p className="onboarding-modal__desc">
                  Ton lanceur d'émulateurs tout-en-un. On va configurer quelques trucs en 30 secondes.
                </p>
                <button className="btn btn--primary btn--lg" onClick={() => setOnboardingStep(1)}>
                  C'est parti
                </button>
              </div>
            )}

            {onboardingStep === 1 && (
              <div className="onboarding-modal__step">
                <div className="onboarding-modal__icon">📁</div>
                <h1 className="onboarding-modal__title">Dossier ROMs</h1>
                <p className="onboarding-modal__desc">
                  Où sont tes ROMs ? EmuWorld va scanner ce dossier pour trouver tes jeux.
                </p>
                {config.roms_directory && (
                  <div className="onboarding-modal__path">{config.roms_directory}</div>
                )}
                <button className="btn btn--primary" onClick={async () => {
                  const selected = await open({ directory: true, title: "Choisis ton dossier ROMs" });
                  if (selected) {
                    const newConfig = { ...config, roms_directory: selected as string };
                    setConfig(newConfig);
                    await invoke("save_config", { config: newConfig });
                  }
                }}>
                  <FolderOpen size={16} /> Choisir le dossier
                </button>
                <div className="onboarding-modal__nav">
                  <button className="btn btn--ghost" onClick={() => setOnboardingStep(0)}>Retour</button>
                  <button className="btn btn--primary" onClick={() => setOnboardingStep(2)} disabled={!config.roms_directory}>Suivant</button>
                </div>
              </div>
            )}

            {onboardingStep === 2 && (
              <div className="onboarding-modal__step">
                <div className="onboarding-modal__icon">🕹️</div>
                <h1 className="onboarding-modal__title">Dossier Émulateurs</h1>
                <p className="onboarding-modal__desc">
                  Où installer les émulateurs ? EmuWorld téléchargera et gérera tout pour toi.
                </p>
                {config.emulators_directory && (
                  <div className="onboarding-modal__path">{config.emulators_directory}</div>
                )}
                <button className="btn btn--primary" onClick={async () => {
                  const selected = await open({ directory: true, title: "Choisis ton dossier émulateurs" });
                  if (selected) {
                    const newConfig = { ...config, emulators_directory: selected as string };
                    setConfig(newConfig);
                    await invoke("save_config", { config: newConfig });
                  }
                }}>
                  <FolderOpen size={16} /> Choisir le dossier
                </button>
                <div className="onboarding-modal__nav">
                  <button className="btn btn--ghost" onClick={() => setOnboardingStep(1)}>Retour</button>
                  <button className="btn btn--primary" onClick={() => setOnboardingStep(3)} disabled={!config.emulators_directory}>Suivant</button>
                </div>
              </div>
            )}

            {onboardingStep === 3 && (
              <div className="onboarding-modal__step">
                <div className="onboarding-modal__icon">✨</div>
                <h1 className="onboarding-modal__title">C'est prêt !</h1>
                <p className="onboarding-modal__desc">
                  Tu peux maintenant installer des émulateurs, scanner tes ROMs et télécharger des jeux depuis le Store. Amuse-toi bien !
                </p>
                <button className="btn btn--primary btn--lg" onClick={() => {
                  localStorage.setItem("emuworld_onboarding_done", "1");
                  setShowOnboarding(false);
                  loadData();
                  setTourStep(0);
                }}>
                  Lancer EmuWorld
                </button>
                <button className="btn btn--ghost" onClick={() => {
                  localStorage.setItem("emuworld_onboarding_done", "1");
                  setShowOnboarding(false);
                  loadData();
                }}>
                  Passer le tutoriel
                </button>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ===== GUIDED TOUR ===== */}
      {tourStep !== null && (() => {
        const steps = [
          { selector: "[data-tour='store']", title: "1. Télécharge un jeu", desc: "Va dans le Store pour télécharger des ROMs à l'unité.", position: "right" as const, action: () => setPage("store") },
          { selector: "[data-tour='emulators']", title: "2. Installe un émulateur", desc: "Installe l'émulateur correspondant à ta console en un clic.", position: "right" as const, action: () => setPage("catalog") },
          { selector: "[data-tour='library']", title: "3. Ta bibliothèque", desc: "Tous tes jeux apparaissent ici avec leurs covers.", position: "right" as const, action: () => setPage("library") },
          { selector: "[data-tour='play']", title: "4. Joue !", desc: "Clique sur un jeu pour le lancer. L'émulateur s'ouvre automatiquement.", position: "top" as const },
          { selector: "[data-tour='friends']", title: "5. Ajoute des amis", desc: "Retrouve tes potes, vois ce qu'ils jouent, et chatte avec eux.", position: "right" as const },
        ];
        const step = steps[tourStep];
        const el = document.querySelector<HTMLElement>(step.selector);
        const rect = el?.getBoundingClientRect();
        if (step.action && el) step.action();
        return (
          <div className="tour-overlay" onClick={() => setTourStep(null)}>
            {rect && (
              <>
                <div className="tour-highlight" style={{ top: rect.top - 4, left: rect.left - 4, width: rect.width + 8, height: rect.height + 8 }} />
                <div
                  className={`tour-tooltip tour-tooltip--${step.position}`}
                  style={{
                    top: step.position === "top" ? rect.top - 120 : rect.top + rect.height / 2 - 40,
                    left: step.position === "right" ? rect.right + 16 : rect.left + rect.width / 2 - 140,
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <h3 className="tour-tooltip__title">{step.title}</h3>
                  <p className="tour-tooltip__desc">{step.desc}</p>
                  <div className="tour-tooltip__nav">
                    {tourStep > 0 && <button className="btn btn--ghost btn--sm" onClick={() => setTourStep(tourStep - 1)}>Précédent</button>}
                    {tourStep < steps.length - 1 ? (
                      <button className="btn btn--primary btn--sm" onClick={() => setTourStep(tourStep + 1)}>Suivant</button>
                    ) : (
                      <button className="btn btn--primary btn--sm" onClick={() => setTourStep(null)}>Terminer</button>
                    )}
                  </div>
                  <span className="tour-tooltip__count">{tourStep + 1}/{steps.length}</span>
                </div>
              </>
            )}
            {!rect && (
              <div className="tour-tooltip" style={{ top: "50%", left: "50%", transform: "translate(-50%, -50%)" }} onClick={(e) => e.stopPropagation()}>
                <h3 className="tour-tooltip__title">{step.title}</h3>
                <p className="tour-tooltip__desc">{step.desc}</p>
                <div className="tour-tooltip__nav">
                  {tourStep > 0 && <button className="btn btn--ghost btn--sm" onClick={() => setTourStep(tourStep - 1)}>Précédent</button>}
                  {tourStep < steps.length - 1 ? (
                    <button className="btn btn--primary btn--sm" onClick={() => setTourStep(tourStep + 1)}>Suivant</button>
                  ) : (
                    <button className="btn btn--primary btn--sm" onClick={() => setTourStep(null)}>Terminer</button>
                  )}
                </div>
              </div>
            )}
          </div>
        );
      })()}

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
