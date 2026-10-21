import { isTauri } from "@tauri-apps/api/core";
import { useRef, useState } from "react";
import { captureRegion, type Destination } from "./capture";
import RegionOverlay from "./RegionOverlay";
import "./App.css";

export default function App() {
  if (new URLSearchParams(window.location.search).has("region")) {
    return <RegionOverlay />;
  }
  return <Launcher />;
}

function Launcher() {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState(
    "Choose an action, then drag a region. Escape cancels.",
  );
  const inFlight = useRef(false);
  const native = isTauri();

  const capture = async (destination: Destination) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setMessage("Preparing region capture…");
    try {
      const result = await captureRegion(destination);
      if (result.status === "cancelled") setMessage("Capture cancelled.");
      else if (result.status === "copied")
        setMessage(
          `Copied ${result.width} × ${result.height} image to the clipboard.`,
        );
      else
        setMessage(
          `Saved ${result.width} × ${result.height} PNG to ${result.path}`,
        );
    } catch (error) {
      setMessage(String(error));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };

  return (
    <main className="app">
      <p className="status">Task 002 · Region capture</p>
      <h1>Slotshot</h1>
      <p>A lightweight screenshot utility for macOS and Windows.</p>
      <section aria-labelledby="status-heading">
        <h2 id="status-heading">Test native capture</h2>
        <div className="capture-actions">
          <button
            disabled={busy || !native}
            onClick={() => void capture("copy")}
          >
            Copy Region
          </button>
          <button
            disabled={busy || !native}
            onClick={() => void capture("save")}
          >
            Save Region
          </button>
        </div>
        <p role="status" aria-live="polite">
          {native
            ? message
            : "Open the Tauri desktop app to use native capture."}
        </p>
      </section>
      <p className="note">
        Planned: a visual screenshot clipboard and configurable shortcuts.
      </p>
    </main>
  );
}
