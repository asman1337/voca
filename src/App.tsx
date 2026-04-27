import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { Orb } from "./components/Orb";
import { Settings } from "./components/Settings";
import { useOrbState } from "./hooks/useOrbState";

export default function App() {
  const { state, clipboardText, setClipboardText } = useOrbState();
  const [settingsOpen, setSettingsOpen] = useState(false);

  // ── Centralised window resize — single source of truth ──────────────────
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    if (settingsOpen) {
      win.setSize(new LogicalSize(320, 490)).catch(() => {});
    } else if (state === "injected" && clipboardText) {
      win.setSize(new LogicalSize(286, 212)).catch(() => {});
    } else {
      win.setSize(new LogicalSize(80, 80)).catch(() => {});
    }
  }, [settingsOpen, state, clipboardText]);

  const handleDismissCard = useCallback(() => {
    setClipboardText(null);
    invoke("cmd_dismiss").catch(() => {});
  }, [setClipboardText]);

  return (
    <>
      {!settingsOpen && (
        <Orb
          state={state}
          clipboardText={clipboardText}
          onDismissCard={handleDismissCard}
          onOpenSettings={() => setSettingsOpen(true)}
        />
      )}
      {settingsOpen && (
        <Settings onClose={() => setSettingsOpen(false)} />
      )}
    </>
  );
}
