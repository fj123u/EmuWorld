// Save states manager for one game: RetroArch states with their thumbnail,
// a name of your own, duplicate to a free slot, resume from a slot, and
// delete (to the recycle bin).
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Copy, Pencil, Play, Save, Trash2, X } from "lucide-react";

export interface SaveState {
  path: string;
  file_name: string;
  slot: number | null;
  auto: boolean;
  folder: string | null;
  modified: string | null;
  size: number;
  thumbnail: string | null;
  label: string | null;
}

interface Props {
  rom: { name: string; path: string; console: string };
  t: (key: string, params?: Record<string, string | number>) => string;
  locale: string;
  onClose: () => void;
  /** Launches the game and loads the given slot. */
  onResume: (slot: number) => void;
  onError: (message: string) => void;
}


export function SaveStatesModal({ rom, t, locale, onClose, onResume, onError }: Props) {
  const [states, setStates] = useState<SaveState[] | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const cancelEdit = useRef(false);

  const formatSize = (bytes: number) =>
    bytes < 1024 * 1024
      ? t("saveStates.sizeKb", { size: Math.max(1, Math.round(bytes / 1024)) })
      : t("saveStates.sizeMb", { size: (bytes / 1024 / 1024).toFixed(1) });

  const reload = useCallback(async () => {
    try {
      setStates(await invoke<SaveState[]>("list_save_states", { romPath: rom.path }));
    } catch (e) {
      setStates([]);
      onError(String(e));
    }
  }, [rom.path, onError]);

  useEffect(() => { reload(); }, [reload]);

  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      onError(String(e));
    }
    await reload();
  };

  const saveLabel = (s: SaveState) => run(async () => {
    setEditing(null);
    await invoke("rename_save_state", { path: s.path, label: draft });
  });

  const slotName = (s: SaveState) => (s.auto ? t("saveStates.auto") : t("saveStates.slot", { slot: s.slot ?? 0 }));

  return (
    <div className="rom-context-overlay" onClick={onClose}>
      <div className="screenshot-gallery save-states" role="dialog" aria-label={t("saveStates.title")} onClick={(e) => e.stopPropagation()}>
        <div className="screenshot-gallery__header">
          <h3><Save size={16} /> {t("saveStates.title")} · {rom.name}</h3>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={onClose} aria-label={t("saveStates.close")}><X size={14} /></button>
        </div>
        {states === null ? (
          <p className="settings__field-desc save-states__empty">{t("saveStates.loading")}</p>
        ) : states.length === 0 ? (
          <p className="settings__field-desc save-states__empty">{t("saveStates.empty")}</p>
        ) : (
          <div className="save-states__list">
            {states.map((s) => (
              <div key={s.path} className="save-states__item">
                <div className="save-states__thumb">
                  {s.thumbnail ? <img src={s.thumbnail} alt="" /> : <Save size={22} />}
                </div>
                <div className="save-states__info">
                  {editing === s.path ? (
                    <input
                      className="settings__field-input gamepad-nav-item"
                      autoFocus
                      value={draft}
                      maxLength={80}
                      placeholder={t("saveStates.namePlaceholder")}
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") e.currentTarget.blur();
                        if (e.key === "Escape") {
                          cancelEdit.current = true;
                          e.currentTarget.blur();
                        }
                      }}
                      onBlur={() => {
                        if (cancelEdit.current) {
                          cancelEdit.current = false;
                          setEditing(null);
                        } else {
                          saveLabel(s);
                        }
                      }}
                    />
                  ) : (
                    <div className="save-states__name">{s.label || slotName(s)}</div>
                  )}
                  <div className="save-states__meta">
                    {s.label ? `${slotName(s)} · ` : ""}
                    {s.modified ? new Date(s.modified).toLocaleString(locale === "fr" ? "fr-FR" : "en-GB", { dateStyle: "medium", timeStyle: "short" }) : "—"}
                    {" · "}{formatSize(s.size)}
                    {s.folder ? ` · ${s.folder}` : ""}
                  </div>
                </div>
                <div className="save-states__actions">
                  {confirmDelete === s.path ? (
                    <>
                      <button className="btn btn--danger btn--sm gamepad-nav-item" onClick={() => { setConfirmDelete(null); run(() => invoke("delete_save_state", { path: s.path })); }}>
                        {t("saveStates.confirmDelete")}
                      </button>
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setConfirmDelete(null)}>{t("saveStates.cancel")}</button>
                    </>
                  ) : (
                    <>
                      {s.slot !== null && (
                        <button className="btn btn--primary btn--sm gamepad-nav-item" onClick={() => onResume(s.slot!)} title={t("saveStates.resumeHint")}>
                          <Play size={12} /> {t("saveStates.resume")}
                        </button>
                      )}
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => { setEditing(s.path); setDraft(s.label ?? ""); }} title={t("saveStates.rename")} aria-label={t("saveStates.rename")}>
                        <Pencil size={12} />
                      </button>
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => run(() => invoke("duplicate_save_state", { path: s.path }))} title={t("saveStates.duplicate")} aria-label={t("saveStates.duplicate")}>
                        <Copy size={12} />
                      </button>
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setConfirmDelete(s.path)} title={t("saveStates.delete")} aria-label={t("saveStates.delete")}>
                        <Trash2 size={12} />
                      </button>
                    </>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
        <p className="settings__field-desc save-states__hint">{t("saveStates.hint")}</p>
      </div>
    </div>
  );
}
