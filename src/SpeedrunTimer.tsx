// Speedrun timer UI: the small always-on-top window driven by numpad
// hotkeys (SpeedrunWindow), and the per-game setup in the main app
// (SpeedrunModal). Logic: src/speedrun.ts, storage: src-tauri/src/speedrun.rs.
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Copy, Flag, Timer, Trash2, X } from "lucide-react";
import {
  deltaVsPb,
  elapsed,
  emptyData,
  exportSummary,
  formatDelta,
  formatTime,
  isGoldSegment,
  newRun,
  recordRun,
  segmentNames,
  skip,
  split,
  sumOfBest,
  togglePause,
  undo,
  type HotkeyAction,
  type RunState,
  type SpeedrunData,
} from "./speedrun";

type T = (key: string, params?: Record<string, string | number>) => string;

const now = () => performance.now();

function log(level: "INFO" | "WARN" | "ERROR", message: string) {
  invoke("log_event", { level, message: `Speedrun: ${message}` }).catch(() => {});
}

/** The timer window (`index.html?speedrun=1&game=<console::name>&name=<title>`). */
export function SpeedrunWindow({ t }: { t: T }) {
  const params = new URLSearchParams(window.location.search);
  const gameKey = params.get("game") ?? "";
  const gameName = params.get("name") ?? gameKey;
  const [data, setData] = useState<SpeedrunData>(emptyData);
  const [run, setRun] = useState<RunState>(newRun);
  const [clock, setClock] = useState(now);
  const [newPb, setNewPb] = useState(false);
  const recorded = useRef(false);
  const dataRef = useRef(data);
  dataRef.current = data;
  const runRef = useRef(run);
  runRef.current = run;

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    invoke<SpeedrunData>("get_speedrun", { gameKey }).then(setData).catch(() => {});
  }, [gameKey]);

  // Refs are updated right away (not on the next render): two hotkeys in a
  // row must see each other's result.
  const save = useCallback((next: SpeedrunData) => {
    dataRef.current = next;
    setData(next);
    invoke<SpeedrunData>("save_speedrun", { gameKey, data: next }).then(setData).catch((e) => log("ERROR", String(e)));
  }, [gameKey]);

  const handle = useCallback((action: HotkeyAction) => {
    const current = runRef.current;
    const count = segmentNames(dataRef.current).length;
    const at = now();
    const setRunNow = (next: RunState) => {
      runRef.current = next;
      setRun(next);
    };
    if (action === "split") {
      const next = split(current, at, count);
      if (current.status === "idle" && next.status === "running") {
        recorded.current = false;
        setNewPb(false);
        save({ ...dataRef.current, attempts: dataRef.current.attempts + 1 });
      }
      if (next.status === "finished" && current.status !== "finished" && !recorded.current) {
        const result = recordRun(dataRef.current, next);
        recorded.current = true;
        setNewPb(result.newPb);
        save(result.data);
        log("INFO", `run finie en ${formatTime(next.splits[count - 1] ?? 0)}${result.newPb ? " (record)" : ""}`);
      }
      setRunNow(next);
    } else if (action === "skip") {
      setRunNow(skip(current, count));
    } else if (action === "undo") {
      setRunNow(undo(current));
    } else if (action === "pause") {
      setRunNow(togglePause(current, at));
    } else if (action === "reset") {
      // A reset run still teaches its best segments.
      if (!recorded.current && current.splits.length > 0) save(recordRun(dataRef.current, current).data);
      recorded.current = false;
      setNewPb(false);
      setRunNow(newRun());
    }
  }, [save]);

  useEffect(() => {
    const unlisten = listen<HotkeyAction>("speedrun-hotkey", (e) => handle(e.payload));
    return () => { unlisten.then((fn) => fn()); };
  }, [handle]);

  useEffect(() => {
    if (run.status !== "running") return;
    const id = window.setInterval(() => setClock(now()), 31);
    return () => window.clearInterval(id);
  }, [run.status]);

  const names = segmentNames(data);
  const current = run.status === "running" || run.status === "paused" ? run.splits.length : -1;
  const time = elapsed(run, clock);
  const livePb = current >= 0 ? data.pb?.[current] : undefined;
  const behind = livePb !== undefined && time > livePb;
  const sob = sumOfBest(data);

  return (
    <div className="speedrun-timer" data-status={run.status}>
      <div className="speedrun-timer__title" title={gameName}>{gameName}</div>
      <div className="speedrun-timer__attempts">{t("speedrun.attempts", { done: data.completed, total: data.attempts })}</div>
      <ol className="speedrun-timer__splits">
        {names.map((name, i) => {
          const done = i < run.splits.length;
          const delta = done ? deltaVsPb(data, run, i) : null;
          const gold = done && isGoldSegment(data, { ...run }, i);
          return (
            <li key={i} className={`speedrun-timer__split ${i === current ? "speedrun-timer__split--current" : ""}`}>
              <span className="speedrun-timer__name">{name}</span>
              <span className={`speedrun-timer__delta ${gold ? "is-gold" : delta !== null ? (delta < 0 ? "is-ahead" : "is-behind") : ""}`}>
                {delta !== null ? formatDelta(delta) : ""}
              </span>
              <span className="speedrun-timer__time">
                {done ? (run.splits[i] !== null ? formatTime(run.splits[i] as number, 1) : "—") : data.pb ? formatTime(data.pb[i], 1) : "—"}
              </span>
            </li>
          );
        })}
      </ol>
      <div className={`speedrun-timer__clock ${run.status === "finished" ? (newPb ? "is-gold" : "") : behind ? "is-behind" : run.status !== "idle" ? "is-ahead" : ""}`}>
        {formatTime(time)}
      </div>
      {newPb && <div className="speedrun-timer__pb">{t("speedrun.newPb")}</div>}
      <div className="speedrun-timer__footer">
        <span>{t("speedrun.pb")} {data.pb ? formatTime(data.pb[data.pb.length - 1]) : "—"}</span>
        <span>{t("speedrun.sob")} {sob !== null ? formatTime(sob) : "—"}</span>
      </div>
      <div className="speedrun-timer__keys">{run.status === "paused" ? t("speedrun.paused") : t("speedrun.keys")}</div>
    </div>
  );
}

interface ModalProps {
  rom: { name: string; console: string };
  t: T;
  onClose: () => void;
  onNotify: (message: string, type?: "success" | "error" | "info") => void;
}

/** Per-game setup in the main app: splits, records, show the timer. */
export function SpeedrunModal({ rom, t, onClose, onNotify }: ModalProps) {
  const gameKey = `${rom.console}::${rom.name}`;
  const [data, setData] = useState<SpeedrunData | null>(null);
  const [draft, setDraft] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    invoke<SpeedrunData>("get_speedrun", { gameKey })
      .then((d) => { setData(d); setDraft(segmentNames(d).join("\n")); })
      .catch((e) => { setData(emptyData()); onNotify(String(e), "error"); });
  }, [gameKey, onNotify]);

  if (!data) return null;
  const lines = draft.split("\n").map((l) => l.trim()).filter(Boolean);
  const countChanged = data.pb !== null && lines.length !== data.pb.length;
  const sob = sumOfBest(data);

  const saveSplits = async (): Promise<SpeedrunData | null> => {
    const next: SpeedrunData = {
      ...data,
      splits: lines.length > 0 ? lines : segmentNames(data),
      ...(countChanged ? { pb: null, best_segments: [] } : {}),
    };
    try {
      const saved = await invoke<SpeedrunData>("save_speedrun", { gameKey, data: next });
      setData(saved);
      setDraft(segmentNames(saved).join("\n"));
      return saved;
    } catch (e) {
      onNotify(String(e), "error");
      return null;
    }
  };

  return (
    <div className="rom-context-overlay" onClick={onClose}>
      <div className="screenshot-gallery speedrun-modal" role="dialog" aria-label={t("speedrun.title")} onClick={(e) => e.stopPropagation()}>
        <div className="screenshot-gallery__header">
          <h3><Timer size={16} /> {t("speedrun.title")} · {rom.name}</h3>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={onClose} aria-label={t("saveStates.close")}><X size={14} /></button>
        </div>
        <div className="speedrun-modal__body">
          <div className="speedrun-modal__stats">
            <div><strong>{data.pb ? formatTime(data.pb[data.pb.length - 1]) : "—"}</strong><span>{t("speedrun.pb")}</span></div>
            <div><strong>{sob !== null ? formatTime(sob) : "—"}</strong><span>{t("speedrun.sob")}</span></div>
            <div><strong>{data.completed}/{data.attempts}</strong><span>{t("speedrun.runs")}</span></div>
          </div>
          <label className="settings__field-label" htmlFor="speedrun-splits">{t("speedrun.splitsLabel")}</label>
          <textarea
            id="speedrun-splits"
            className="notes-modal__textarea gamepad-nav-item speedrun-modal__splits"
            value={draft}
            rows={8}
            placeholder={t("speedrun.splitsPlaceholder")}
            onChange={(e) => setDraft(e.target.value)}
          />
          {countChanged && <p className="speedrun-modal__warning">{t("speedrun.countChanged")}</p>}
          <p className="settings__field-desc">{t("speedrun.keysHelp")}</p>
          <div className="speedrun-modal__actions">
            <button
              className="btn btn--primary btn--sm gamepad-nav-item"
              onClick={async () => {
                if (!(await saveSplits())) return;
                try {
                  await invoke("open_speedrun_window", { gameKey, gameName: rom.name });
                  onNotify(t("speedrun.opened"), "success");
                } catch (e) {
                  onNotify(String(e), "error");
                }
              }}
            >
              <Flag size={12} /> {t("speedrun.show")}
            </button>
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => invoke("close_speedrun_window").catch(() => {})}>
              {t("speedrun.hide")}
            </button>
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={async () => { if (await saveSplits()) onNotify(t("speedrun.saved"), "success"); }}>
              {t("speedrun.save")}
            </button>
            <button
              className="btn btn--ghost btn--sm gamepad-nav-item"
              onClick={async () => {
                await navigator.clipboard?.writeText(exportSummary(rom.name, data, { pb: t("speedrun.pb"), sob: t("speedrun.sob"), attempts: t("speedrun.runs") }));
                onNotify(t("speedrun.copied"), "success");
              }}
            >
              <Copy size={12} /> {t("speedrun.copy")}
            </button>
            {confirmClear ? (
              <button
                className="btn btn--danger btn--sm gamepad-nav-item"
                onClick={async () => {
                  setConfirmClear(false);
                  const saved = await invoke<SpeedrunData>("save_speedrun", { gameKey, data: { ...data, pb: null, best_segments: [], attempts: 0, completed: 0 } }).catch(() => null);
                  if (saved) setData(saved);
                }}
              >
                {t("speedrun.confirmClear")}
              </button>
            ) : (
              <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setConfirmClear(true)} aria-label={t("speedrun.clear")} title={t("speedrun.clear")}>
                <Trash2 size={12} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
