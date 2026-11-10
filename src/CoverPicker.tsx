// "Choisir la cover": the box art iiDB has for a game (or, when iiDB hesitates
// between several games, for each of them), to pick the one shown in the
// library. Backend: iidb_cover_choices (iidb_api.rs) and set_cover_from_url
// (lib.rs). Needs the user's own iiDB API key (Settings → Covers iiDB).
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Check, FolderOpen, Image as ImageIcon, X } from "lucide-react";
import { IIDB_SITE } from "./IidbSettings";

export interface CoverChoice {
  url: string;
  thumbnail: string | null;
  region: string | null;
  console_id: string | null;
}

export interface CoverGame {
  parent_id: number;
  name: string;
  platform: string | null;
  year: number | null;
  covers: CoverChoice[];
  /** The iiDB game picked earlier for this ROM (shown first). */
  remembered?: boolean;
}

/** Window event telling every card of a game to show its new cover. */
export const COVER_CHANGED_EVENT = "emuworld:cover-changed";

export interface CoverChangedDetail {
  name: string;
  console: string;
  dataUrl: string;
}

export function announceCoverChange(detail: CoverChangedDetail) {
  window.dispatchEvent(new CustomEvent<CoverChangedDetail>(COVER_CHANGED_EVENT, { detail }));
}

/** Shows a game's new cover as soon as one is picked (cards keep their own state). */
export function useCoverChanges(rom: { name: string; console: string }, setCover: (dataUrl: string) => void) {
  useEffect(() => {
    const onChange = (e: Event) => {
      const d = (e as CustomEvent<CoverChangedDetail>).detail;
      if (d.name === rom.name && d.console === rom.console) setCover(d.dataUrl);
    };
    window.addEventListener(COVER_CHANGED_EVENT, onChange);
    return () => window.removeEventListener(COVER_CHANGED_EVENT, onChange);
  }, [rom.name, rom.console, setCover]);
}

type T = (key: string, params?: Record<string, string | number>) => string;

interface Props {
  rom: { name: string; console: string };
  t: T;
  onClose: () => void;
  onNotify: (message: string, type?: "success" | "error" | "info") => void;
}

export function CoverPicker({ rom, t, onClose, onNotify }: Props) {
  const [games, setGames] = useState<CoverGame[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [applying, setApplying] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    invoke<CoverGame[]>("iidb_cover_choices", { gameName: rom.name, console: rom.console })
      .then((g) => { if (alive) setGames(g); })
      .catch((e) => { if (alive) setError(String(e)); });
    return () => { alive = false; };
  }, [rom.name, rom.console]);

  const applyWith = async (marker: string, command: string, args: Record<string, string | number>) => {
    setApplying(marker);
    try {
      const dataUrl = await invoke<string>(command, { gameName: rom.name, console: rom.console, ...args });
      announceCoverChange({ name: rom.name, console: rom.console, dataUrl });
      onNotify(t("coverPicker.applied"), "success");
      onClose();
    } catch (e) {
      onNotify(t("coverPicker.failed", { error: String(e) }), "error");
      setApplying(null);
    }
  };

  // The game is remembered: later cover searches use it without asking iiDB again.
  const apply = (cover: CoverChoice, game: CoverGame) =>
    applyWith(cover.url, "set_cover_from_url", { url: cover.url, parentId: game.parent_id });

  const fromFile = async () => {
    const picked = await openFileDialog({
      multiple: false,
      filters: [{ name: t("coverPicker.images"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }],
    }).catch(() => null);
    if (typeof picked === "string") await applyWith(picked, "set_cover_from_file", { path: picked });
  };

  const open = (url: string) => { openUrl(url).catch(() => window.open(url, "_blank")); };

  let body;
  if (error === "no_key") body = <p className="cover-picker__empty">{t("coverPicker.noKey")}</p>;
  else if (error) body = <p className="cover-picker__empty">{t("coverPicker.unavailable")}</p>;
  else if (games === null) body = <p className="cover-picker__empty">{t("coverPicker.loading")}</p>;
  else if (games.length === 0) body = <p className="cover-picker__empty">{t("coverPicker.none")}</p>;
  else body = games.map((game) => (
    <section key={game.parent_id} className="cover-picker__game">
      <h4 className="cover-picker__game-title">
        {game.name}
        {(game.platform || game.year) && <span className="cover-picker__meta"> · {[game.platform, game.year].filter(Boolean).join(", ")}</span>}
        {game.remembered && <span className="cover-picker__chosen">{t("coverPicker.yourChoice")}</span>}
      </h4>
      <div className="cover-picker__grid">
        {game.covers.map((cover) => (
          <button
            key={cover.url}
            className="cover-picker__cover gamepad-nav-item"
            disabled={applying !== null}
            onClick={() => apply(cover, game)}
            aria-label={t("coverPicker.use", { region: cover.region ?? "?" })}
          >
            <img src={cover.thumbnail ?? cover.url} alt="" loading="lazy" />
            {cover.region && <span className="cover-picker__region">{cover.region}</span>}
            {applying === cover.url && <span className="cover-picker__applying"><Check size={16} /></span>}
          </button>
        ))}
      </div>
    </section>
  ));

  return (
    <div className="rom-context-overlay" onClick={onClose}>
      <div className="screenshot-gallery cover-picker" role="dialog" aria-label={t("coverPicker.title")} onClick={(e) => e.stopPropagation()}>
        <div className="screenshot-gallery__header">
          <h3><ImageIcon size={16} /> {t("coverPicker.title")} · {rom.name}</h3>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={onClose} aria-label={t("coverPicker.close")}><X size={14} /></button>
        </div>
        <div className="cover-picker__actions">
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={fromFile} disabled={applying !== null}>
            <FolderOpen size={14} /> {t("coverPicker.fromFile")}
          </button>
        </div>
        {games && games.length > 1 && <p className="cover-picker__hint">{t("coverPicker.severalGames")}</p>}
        <div className="cover-picker__body">{body}</div>
        <p className="cover-picker__credit">
          {t("coverPicker.creditBefore")}{" "}
          <button type="button" className="iidb-settings__link gamepad-nav-item" onClick={() => open(IIDB_SITE)}>iiDB</button>
        </p>
      </div>
    </div>
  );
}
