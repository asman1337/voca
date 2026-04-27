import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { OrbState } from "../types";

export function useOrbState() {
  const [state, setState] = useState<OrbState>("idle");
  const [clipboardText, setClipboardText] = useState<string | null>(null);

  useEffect(() => {
    // Sync initial state from the backend on mount
    invoke<string>("cmd_get_state")
      .then((s) => setState(s as OrbState))
      .catch(() => {/* backend not yet running in pure-frontend dev mode */});

    // Stream state transitions from Rust
    const unlistenState = listen<OrbState>("orb-state-changed", (event) => {
      setState(event.payload);
      if (event.payload !== "injected") {
        setClipboardText(null);
      }
    });

    // Backend emits the transcript as a plain string when injection falls back
    // to clipboard (no focused text input detected in the active window).
    const unlistenClipboard = listen<string>("orb-clipboard-ready", (event) => {
      setClipboardText(event.payload);
    });

    return () => {
      unlistenState.then((f) => f());
      unlistenClipboard.then((f) => f());
    };
  }, []);

  return { state, clipboardText, setClipboardText };
}
