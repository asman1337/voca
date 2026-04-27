import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Orb } from "./components/Orb";
import { useOrbState } from "./hooks/useOrbState";

export default function App() {
  const { state, clipboardText, setClipboardText } = useOrbState();

  const handleDismissCard = useCallback(() => {
    setClipboardText(null);
    // Tell the backend to transition Injected → Idle
    invoke("cmd_dismiss").catch(() => {});
  }, [setClipboardText]);

  return (
    <Orb
      state={state}
      clipboardText={clipboardText}
      onDismissCard={handleDismissCard}
    />
  );
}
