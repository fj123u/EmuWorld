// Settings section for SteamGridDB: the user's own API key (generated on their
// SteamGridDB profile, like the iiDB one) and the credit linked to the site.
// Backend: src-tauri/src/steamgriddb.rs.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CheckCircle, ExternalLink, Grid3x3, KeyRound } from "lucide-react";

export const SGDB_SITE = "https://www.steamgriddb.com";
export const SGDB_KEY_PAGE = "https://www.steamgriddb.com/profile/preferences/api";

type T = (key: string, params?: Record<string, string | number>) => string;

export function SgdbSettings({ t }: { t: T }) {
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ configured: boolean }>("get_sgdb_status").then((s) => setConfigured(s.configured)).catch(() => setConfigured(false));
  }, []);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      const status = await invoke<{ configured: boolean }>("set_sgdb_api_key", { key: draft.trim() });
      setConfigured(status.configured);
      setDraft("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    await invoke("clear_sgdb_api_key").catch(() => {});
    setConfigured(false);
  };

  const open = (url: string) => { openUrl(url).catch(() => window.open(url, "_blank")); };

  return (
    <div className="settings__group" aria-label="SteamGridDB">
      <div className="settings__group-title"><Grid3x3 size={16} /> {t("sgdb.title")}</div>
      <div className="settings__field">
        <div className="settings__field-info">
          <label className="settings__field-label" htmlFor="sgdb-key">{t("sgdb.keyLabel")}</label>
          <p className="settings__field-desc">{t("sgdb.desc")}</p>
        </div>
        {configured ? (
          <div className="iidb-settings__connected">
            <span className="iidb-settings__ok"><CheckCircle size={14} /> {t("sgdb.connected")}</span>
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={remove}>{t("sgdb.remove")}</button>
          </div>
        ) : (
          <form className="iidb-settings__form" onSubmit={(e) => { e.preventDefault(); if (draft.trim()) save(); }}>
            <input
              id="sgdb-key"
              type="password"
              autoComplete="off"
              className="settings__field-input gamepad-nav-item"
              placeholder={t("sgdb.keyPlaceholder")}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <button type="submit" className="btn btn--primary btn--sm gamepad-nav-item" disabled={busy || !draft.trim()}>
              <KeyRound size={12} /> {busy ? t("sgdb.checking") : t("sgdb.save")}
            </button>
            <button type="button" className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => open(SGDB_KEY_PAGE)}>
              <ExternalLink size={12} /> {t("sgdb.getKey")}
            </button>
          </form>
        )}
      </div>
      {error && <p className="iidb-settings__error" role="alert">{error}</p>}
      <p className="settings__field-desc iidb-settings__credit">
        {t("sgdb.creditBefore")}{" "}
        <button type="button" className="iidb-settings__link gamepad-nav-item" onClick={() => open(SGDB_SITE)}>SteamGridDB</button>
        {t("sgdb.creditAfter")}
      </p>
    </div>
  );
}
