// Dev-only: the real app in a browser, on the fake backend of the tests
// (src/test/tauriMock.ts), to look at the v3 redesign page by page.
// Open http://localhost:1420/scratch/ui-v3/app.html while `npm run dev` runs.
// window.__pad(button) simulates the controller.
// URL options: ?theme=dark (or default); ?demo=lobby shows the Lobbies page with an open lobby;
// ?noanim=1 turns animations off (needed when the preview pane is hidden).
import React from "react";
import ReactDOM from "react-dom/client";
import { mockBackend } from "../../src/test/tauriMock";
import App from "../../src/App";
import "../../src/App.css";
import { audit, auditAll } from "./contrast-audit";
import { LobbyDemo } from "./lobby-demo";
import { MotionGlobalConfig } from "framer-motion";
import { emit } from "@tauri-apps/api/event";

const params = new URLSearchParams(location.search);
localStorage.setItem("emuworld_onboarding_done", "true");
if (params.get("theme")) localStorage.setItem("emuworld_theme", params.get("theme")!);
mockBackend();
// ?noanim=1: page transitions finish at once (a hidden browser pane runs no animation frame).
if (params.get("noanim")) MotionGlobalConfig.skipAnimations = true;
// window.__pad(13): one press of a pad button (12 up, 13 down, 14 left, 15 right, 0 A, 4 LB, 5 RB).
const padFrame = (pressed: number | null) =>
  emit("gamepad-state", { connected: true, name: "Pad", buttons: Array.from({ length: 16 }, (_, i) => i === pressed), axes: [0, 0, 0, 0] });
const pad = async (button: number) => { await padFrame(button); await new Promise((r) => setTimeout(r, 60)); await padFrame(null); await new Promise((r) => setTimeout(r, 160)); };
Object.assign(window, { __audit: audit, __auditAll: auditAll, __pad: pad });

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {params.get("demo") === "lobby" ? <LobbyDemo /> : <App />}
  </React.StrictMode>,
);
