import { useCallback } from "react";
import { Orb } from "./components/Orb";
import { useOrbState } from "./hooks/useOrbState";

export default function App() {
  const { state, clipboardText, setClipboardText } = useOrbState();

  const handleDismissCard = useCallback(() => {
    setClipboardText(null);
  }, [setClipboardText]);

  return (
    <Orb
      state={state}
      clipboardText={clipboardText}
      onDismissCard={handleDismissCard}
    />
  );
}
