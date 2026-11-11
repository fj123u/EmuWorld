// Dev-only: the real app in a browser, on the fake backend of the tests
// (src/test/tauriMock.ts), to look at the v3 redesign page by page.
// Open http://localhost:1420/scratch/ui-v3/app.html while `npm run dev` runs.
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

const params = new URLSearchParams(location.search);
localStorage.setItem("emuworld_onboarding_done", "true");
if (params.get("theme")) localStorage.setItem("emuworld_theme", params.get("theme")!);
mockBackend();
// ?noanim=1: page transitions finish at once (a hidden browser pane runs no animation frame).
if (params.get("noanim")) MotionGlobalConfig.skipAnimations = true;
Object.assign(window, { __audit: audit, __auditAll: auditAll });

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {params.get("demo") === "lobby" ? <LobbyDemo /> : <App />}
  </React.StrictMode>,
);
