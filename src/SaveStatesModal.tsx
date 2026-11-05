// Save states manager for one game: RetroArch states with their thumbnail,
// a name of your own, duplicate to a free slot, resume from a slot, delete
// (to the recycle bin), and share with a friend / add the ones friends sent.
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Download, Gift, Pencil, Play, Save, Send, Share2, Trash2, X } from "lucide-react";
import {
  dismissReceivedState,
  importReceivedState,
  isForGame,
  listReceivedStates,
  shareSaveState,
  type SharedSaveState,
} from "./savestateSharing";

export interface SaveState {
  path: string;
  file_name: string;
  slot: number | null;
  auto: boolean;
  folder: string | null;
  /** Core that made it, when RetroArch sorts states by core. */
  core: string | null;
  modified: string | null;
  size: number;
  thumbnail: string | null;
  label: string | null;
}

export interface ShareFriend {
  id: string;
  username: string;
}

type Notify = (message: string, type?: "success" | "error" | "info") => void;

interface Props {
  rom: { name: string; path: string; console: string };
  t: (key: string, params?: Record<string, string | number>) => string;
  locale: string;
  /** Signed-in user; sharing is hidden without one. */
  userId: string | null;
  friends: ShareFriend[];
  onClose: () => void;
  /** Launches the game and loads the given slot. */
  onResume: (slot: number) => void;
  /** Must be stable (it is an effect dependency). */
  onNotify: Notify;
  /** Tells the friend in the chat that a state is waiting for them. */
  onShared?: (friend: ShareFriend) => void;
}

export function SaveStatesModal({ rom, t, locale, userId, friends, onClose, onResume, onNotify, onShared }: Props) {
  const [states, setStates] = useState<SaveState[] | null>(null);
  const [received, setReceived] = useState<SharedSaveState[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [sharing, setSharing] = useState<string | null>(null);
  const [shareTo, setShareTo] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const cancelEdit = useRef(false);

  const dateFormat = (iso: string | null) =>
    iso ? new Date(iso).toLocaleString(locale === "fr" ? "fr-FR" : "en-GB", { dateStyle: "medium", timeStyle: "short" }) : "—";
  const formatSize = (bytes: number) =>
    bytes < 1024 * 1024
      ? t("saveStates.sizeKb", { size: Math.max(1, Math.round(bytes / 1024)) })
      : t("saveStates.sizeMb", { size: (bytes / 1024 / 1024).toFixed(1) });

  const reload = useCallback(async () => {
    try {
      setStates(await invoke<SaveState[]>("list_save_states", { romPath: rom.path }));
    } catch (e) {
      setStates([]);
      onNotify(String(e), "error");
    }
  }, [rom.path, onNotify]);

  const reloadReceived = useCallback(async () => {
    if (!userId) return setReceived([]);
    try {
      const all = await listReceivedStates(userId);
      setReceived(all.filter((s) => isForGame(s, { console: rom.console, name: rom.name })));
    } catch (e) {
      // Table not created yet (scratch/supabase_savestates_sharing.sql) or offline:
      // the local manager keeps working without the friends part.
      console.warn("[EmuWorld] shared save states unavailable:", e);
      setReceived([]);
    }
  }, [userId, rom.console, rom.name]);

  useEffect(() => { reload(); }, [reload]);
  useEffect(() => { reloadReceived(); }, [reloadReceived]);

  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      onNotify(String(e), "error");
    }
    await reload();
  };

  const saveLabel = (s: SaveState) => {
    setEditing(null);
    if (draft.trim() === (s.label ?? "")) return; // nothing changed: no write
    return run(() => invoke("rename_save_state", { path: s.path, label: draft }));
  };

  const slotName = (s: { auto?: boolean; slot: number | null }) =>
    s.auto ? t("saveStates.auto") : t("saveStates.slot", { slot: s.slot ?? 0 });

  const send = async (s: SaveState) => {
    const friend = friends.find((f) => f.id === shareTo);
    if (!userId || !friend) return;
    setBusy(s.path);
    try {
      await shareSaveState({ senderId: userId, recipientId: friend.id, state: s, rom });
      onNotify(t("saveStates.shared", { friend: friend.username }), "success");
      onShared?.(friend);
      setSharing(null);
    } catch (e) {
      onNotify(t("saveStates.shareFailed", { error: e instanceof Error ? e.message : String(e) }), "error");
    } finally {
      setBusy(null);
    }
  };

  const accept = async (shared: SharedSaveState) => {
    setBusy(shared.id);
    const from = shared.sender_name || t("saveStates.aFriend");
    try {
      const label = shared.label ? `${shared.label} · ${from}` : t("saveStates.fromFriend", { friend: from });
      await importReceivedState(shared, rom.path, label);
      onNotify(t("saveStates.imported", { friend: from }), "success");
    } catch (e) {
      onNotify(t("saveStates.importFailed", { error: e instanceof Error ? e.message : String(e) }), "error");
    } finally {
      setBusy(null);
    }
    await Promise.all([reload(), reloadReceived()]);
  };

  const decline = async (shared: SharedSaveState) => {
    setBusy(shared.id);
    try {
      await dismissReceivedState(shared);
    } catch (e) {
      onNotify(String(e), "error");
    } finally {
      setBusy(null);
    }
    await reloadReceived();
  };

  return (
    <div className="rom-context-overlay" onClick={onClose}>
      <div className="screenshot-gallery save-states" role="dialog" aria-label={t("saveStates.title")} onClick={(e) => e.stopPropagation()}>
        <div className="screenshot-gallery__header">
          <h3><Save size={16} /> {t("saveStates.title")} · {rom.name}</h3>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={onClose} aria-label={t("saveStates.close")}><X size={14} /></button>
        </div>

        {received.length > 0 && (
          <div className="save-states__received">
            <h4 className="save-states__section"><Gift size={14} /> {t("saveStates.receivedTitle")}</h4>
            {received.map((r) => (
              <div key={r.id} className="save-states__item save-states__item--received">
                <div className="save-states__thumb">
                  {r.thumbnail ? <img src={r.thumbnail} alt="" /> : <Gift size={22} />}
                </div>
                <div className="save-states__info">
                  <div className="save-states__name">{r.label || slotName(r)}</div>
                  <div className="save-states__meta">
                    {t("saveStates.from", { friend: r.sender_name || t("saveStates.aFriend") })} · {dateFormat(r.created_at)} · {formatSize(r.size_bytes)}
                    {r.core ? ` · ${r.core}` : ""}
                  </div>
                  {r.game_name !== rom.name && (
                    <div className="save-states__warning">{t("saveStates.otherVersion", { game: r.game_name })}</div>
                  )}
                </div>
                <div className="save-states__actions">
                  <button className="btn btn--primary btn--sm gamepad-nav-item" disabled={busy === r.id} onClick={() => accept(r)}>
                    <Download size={12} /> {t("saveStates.accept")}
                  </button>
                  <button className="btn btn--ghost btn--sm gamepad-nav-item" disabled={busy === r.id} onClick={() => decline(r)}>
                    {t("saveStates.decline")}
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

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
                    {dateFormat(s.modified)}
                    {" · "}{formatSize(s.size)}
                    {s.folder ? ` · ${s.folder}` : ""}
                  </div>
                  {sharing === s.path && (
                    <div className="save-states__share">
                      {friends.length === 0 ? (
                        <span className="save-states__meta">{t("saveStates.noFriends")}</span>
                      ) : (
                        <>
                          <select
                            className="library-sort-select gamepad-nav-item"
                            aria-label={t("saveStates.pickFriend")}
                            value={shareTo}
                            onChange={(e) => setShareTo(e.target.value)}
                          >
                            <option value="">{t("saveStates.pickFriend")}</option>
                            {friends.map((f) => <option key={f.id} value={f.id}>{f.username}</option>)}
                          </select>
                          <button className="btn btn--primary btn--sm gamepad-nav-item" disabled={!shareTo || busy === s.path} onClick={() => send(s)}>
                            {busy === s.path ? t("saveStates.sending") : <><Send size={12} /> {t("saveStates.send")}</>}
                          </button>
                        </>
                      )}
                      <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setSharing(null)}>{t("saveStates.cancel")}</button>
                    </div>
                  )}
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
                      {userId && (
                        <button
                          className={`btn btn--sm gamepad-nav-item ${sharing === s.path ? "btn--active" : "btn--ghost"}`}
                          onClick={() => { setSharing(sharing === s.path ? null : s.path); setShareTo(""); }}
                          title={t("saveStates.share")}
                          aria-label={t("saveStates.share")}
                        >
                          {busy === s.path ? <Check size={12} /> : <Share2 size={12} />}
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
