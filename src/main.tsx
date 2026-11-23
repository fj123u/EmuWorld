import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { installErrorLogging } from "./errorLogging";

// Errors of the window go to the daily log file (Settings → Logs).
installErrorLogging((level, message) => { invoke("log_event", { level, message }).catch(() => {}); });

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
