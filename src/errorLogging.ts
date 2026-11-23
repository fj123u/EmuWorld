// Errors of the interface go to the daily log file too (log_event →
// push_log): an uncaught exception or a rejected promise, with where it
// happened, so a user's log tells what broke in the window, not only in Rust.

export type SendLog = (level: "INFO" | "WARN" | "ERROR", message: string) => void;

/** "message @ file:line:col", the first lines of the stack when there is one. */
export function describeError(error: unknown, where?: { file?: string; line?: number; column?: number }): string {
  const err = error instanceof Error ? error : null;
  const message = err ? `${err.name}: ${err.message}` : typeof error === "string" ? error : JSON.stringify(error) ?? String(error);
  const place = where?.file ? ` @ ${where.file.split("/").slice(-1)[0]}:${where.line ?? "?"}:${where.column ?? "?"}` : "";
  const stack = err?.stack
    ? " | " + err.stack.split("\n").slice(1, 4).map((l) => l.trim()).join(" | ")
    : "";
  return `${message}${place}${stack}`;
}

/** Sends uncaught errors and rejected promises to `send`. Returns a function that stops it. */
export function installErrorLogging(send: SendLog, target: Window = window): () => void {
  const onError = (e: ErrorEvent) => {
    send("ERROR", `Erreur JS: ${describeError(e.error ?? e.message, { file: e.filename, line: e.lineno, column: e.colno })}`);
  };
  const onRejection = (e: PromiseRejectionEvent) => {
    send("ERROR", `Promesse rejetée: ${describeError(e.reason)}`);
  };
  target.addEventListener("error", onError);
  target.addEventListener("unhandledrejection", onRejection);
  return () => {
    target.removeEventListener("error", onError);
    target.removeEventListener("unhandledrejection", onRejection);
  };
}
