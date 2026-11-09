// Settings section for iiDB: the user's own API key (iiDB's terms require one
// per user for automated access) and the credit iiDB asks public apps to
// show, linked to https://iidb.iisu.network. Backend: src-tauri/src/iidb_api.rs.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CheckCircle, ExternalLink, Image as ImageIcon, KeyRound } from "lucide-react";

export interface IidbStatus {
  configured: boolean;
  username: string | null;
  tier: string | null;
  requests_per_second: number | null;
}

export const IIDB_SITE = "https://iidb.iisu.network";
export const IIDB_ACCOUNTS = "https://accounts.iisu.network/";

type T = (key: string, params?: Record<string, string | number>) => string;

export function IidbSettings({ t }: { t: T }) {
  const [status, setStatus] = useState<IidbStatus | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<IidbStatus>("get_iidb_status").then(setStatus).catch(() => setStatus(null));
  }, []);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await invoke<IidbStatus>("set_iidb_api_key", { key: draft.trim() }));
      setDraft("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    await invoke("clear_iidb_api_key").catch(() => {});
    setStatus({ configured: false, username: null, tier: null, requests_per_second: null });
  };

  const open = (url: string) => { openUrl(url).catch(() => window.open(url, "_blank")); };

  return (
    <div className="settings__group" aria-label="iiDB">
      <div className="settings__group-title"><ImageIcon size={16} /> {t("iidb.title")}</div>
      <div className="settings__field">
        <div className="settings__field-info">
          <label className="settings__field-label" htmlFor="iidb-key">{t("iidb.keyLabel")}</label>
          <p className="settings__field-desc">{t("iidb.desc")}</p>
        </div>
        {status?.configured ? (
          <div className="iidb-settings__connected">
            <span className="iidb-settings__ok">
              <CheckCircle size={14} />{" "}
              {status.tier
                ? t("iidb.connectedTier", { user: status.username ?? "iiSU", tier: status.tier, rps: status.requests_per_second ?? 1 })
                : t("iidb.connected")}
            </span>
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={remove}>{t("iidb.remove")}</button>
          </div>
        ) : (
          <form className="iidb-settings__form" onSubmit={(e) => { e.preventDefault(); if (draft.trim()) save(); }}>
            <input
              id="iidb-key"
              type="password"
              autoComplete="off"
              className="settings__field-input gamepad-nav-item"
              placeholder={t("iidb.keyPlaceholder")}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <button type="submit" className="btn btn--primary btn--sm gamepad-nav-item" disabled={busy || !draft.trim()}>
              <KeyRound size={12} /> {busy ? t("iidb.checking") : t("iidb.save")}
            </button>
            <button type="button" className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => open(IIDB_ACCOUNTS)}>
              <ExternalLink size={12} /> {t("iidb.getKey")}
            </button>
          </form>
        )}
      </div>
      {error && <p className="iidb-settings__error" role="alert">{error}</p>}
      <p className="settings__field-desc iidb-settings__credit">
        {t("iidb.creditBefore")}{" "}
        <button type="button" className="iidb-settings__link gamepad-nav-item" onClick={() => open(IIDB_SITE)}>iiDB</button>
        {t("iidb.creditAfter")}
      </p>
    </div>
  );
}
