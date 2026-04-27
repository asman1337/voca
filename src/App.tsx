import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { Orb } from "./components/Orb";
import { Settings } from "./components/Settings";
import History from "./components/History";
import { useOrbState } from "./hooks/useOrbState";

type Panel = "orb" | "settings" | "history";

export default function App() {
  const { state, clipboardText, setClipboardText } = useOrbState();
  const [panel, setPanel] = useState<Panel>("orb");

  // ── Centralised window resize — single source of truth ──────────────────
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    if (panel === "settings") {
      win.setSize(new LogicalSize(320, 490)).catch(() => {});
    } else if (panel === "history") {
      win.setSize(new LogicalSize(320, 520)).catch(() => {});
    } else if (state === "injected" && clipboardText) {
      win.setSize(new LogicalSize(286, 212)).catch(() => {});
    } else {
      win.setSize(new LogicalSize(80, 80)).catch(() => {});
    }
  }, [panel, state, clipboardText]);

  const handleDismissCard = useCallback(() => {
    setClipboardText(null);
    invoke("cmd_dismiss").catch(() => {});
  }, [setClipboardText]);

  return (
    <>
      {panel === "orb" && (
        <Orb
          state={state}
          clipboardText={clipboardText}
          onDismissCard={handleDismissCard}
          onOpenSettings={() => setPanel("settings")}
        />
      )}
      {panel === "settings" && (
        <Settings
          onClose={() => setPanel("orb")}
          onOpenHistory={() => setPanel("history")}
        />
      )}
      {panel === "history" && (
        <History onClose={() => setPanel("settings")} />
      )}
    </>
  );
}
