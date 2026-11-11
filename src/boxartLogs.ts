// Last cover-search events ("boxart-log" from the backend), shown in
// Settings → logs. Kept outside React state on purpose: the backend sends one
// event per cover, and storing them in App re-rendered the whole app (and
// every game card) hundreds of times while a library was loading. Only the
// panel that shows them subscribes.
import { useSyncExternalStore } from "react";

export interface BoxartLog {
  game: string;
  url: string;
  status: string;
  error?: string;
}

const MAX_LOGS = 50;
let logs: BoxartLog[] = [];
const listeners = new Set<() => void>();

export function pushBoxartLog(log: BoxartLog) {
  logs = [...logs, log].slice(-MAX_LOGS);
  listeners.forEach((notify) => notify());
}

export function clearBoxartLogs() {
  logs = [];
  listeners.forEach((notify) => notify());
}

function subscribe(notify: () => void) {
  listeners.add(notify);
  return () => { listeners.delete(notify); };
}

/** The last cover-search events; re-renders only the component that calls it. */
export function useBoxartLogs(): BoxartLog[] {
  return useSyncExternalStore(subscribe, () => logs);
}
