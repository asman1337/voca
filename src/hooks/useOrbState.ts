import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { OrbState, ClipboardPayload } from "../types";

export function useOrbState() {
  const [state, setState] = useState<OrbState>("idle");
  const [clipboardText, setClipboardText] = useState<string | null>(null);

  useEffect(() => {
    // Listen for state changes emitted from the Rust backend
    const unlistenState = listen<OrbState>("orb-state-changed", (event) => {
      setState(event.payload);
      // Clear clipboard card when leaving injected/idle states
      if (event.payload !== "injected") {
        setClipboardText(null);
      }
    });

    // Listen for clipboard-fallback transcriptions
    const unlistenClipboard = listen<ClipboardPayload>(
      "orb-clipboard-ready",
      (event) => {
        setClipboardText(event.payload.text);
      }
    );

    return () => {
      unlistenState.then((f) => f());
      unlistenClipboard.then((f) => f());
    };
  }, []);

  return { state, clipboardText, setClipboardText };
}
