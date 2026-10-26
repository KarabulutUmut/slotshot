import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  defaultShortcuts,
  displayShortcut,
  recordedShortcut,
  type ShortcutConfig,
  type ShortcutView,
} from "./shortcuts";

export default function ShortcutSettings({ mac }: { mac: boolean }) {
  const [view, setView] = useState<ShortcutView | null>(null);
  const [recording, setRecording] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const recorder = useRef<number | null>(null);
  const working = useRef(false);
  const stop = useCallback(() => {
    recorder.current = null;
    setRecording(null);
    void invoke("shortcut_recording", { recording: false }).catch(() => {});
  }, []);
  const save = useCallback(
    async (config: ShortcutConfig) => {
      if (working.current) return;
      working.current = true;
      setSaving(true);
      setError("");
      stop();
      try {
        setView(await invoke<ShortcutView>("configure_shortcuts", { config }));
      } catch (e) {
        setError(String(e));
        // A conflict leaves the previous assignment; report any rollback failure too.
        try {
          setView(await invoke<ShortcutView>("shortcut_view"));
        } catch (reloadError) {
          setError(`${String(e)} ${String(reloadError)}`);
        }
      } finally {
        working.current = false;
        setSaving(false);
      }
    },
    [stop],
  );
  useEffect(() => {
    void invoke<ShortcutView>("shortcut_view")
      .then(setView)
      .catch((e: unknown) => setError(String(e)));
    return stop;
  }, [stop]);
  useEffect(() => {
    const unlisten = listen<string>("shortcut-recorded", ({ payload }) => {
      if (recorder.current === null || !view) return;
      const keys = [...view.config.keys];
      keys[recorder.current] = payload;
      void save({ ...view.config, keys });
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [view, save]);
  useEffect(() => {
    window.addEventListener("blur", stop);
    return () => window.removeEventListener("blur", stop);
  }, [stop]);
  const assign = useCallback(
    (index: number, key: string | null) => {
      if (!view) return;
      const keys = [...view.config.keys];
      keys[index] = key;
      void save({ ...view.config, keys });
    },
    [view, save],
  );
  useEffect(() => {
    if (recording === null) return;
    const record = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopImmediatePropagation();
        stop();
        return;
      }
      if (event.key === "Tab") {
        stop();
        return;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
      const key = recordedShortcut(event);
      if (key) assign(recording, key);
    };
    window.addEventListener("keydown", record, true);
    return () => window.removeEventListener("keydown", record, true);
  }, [recording, assign, stop]);
  return (
    <section className="shortcut-settings" aria-label="Global shortcuts">
      <h3>Shortcuts</h3>
      <p>
        Click a shortcut, then press a combination with at least two modifiers.
        Escape cancels.
      </p>
      {mac && (
        <p>
          macOS can share shortcuts between apps. Choose an unused combination
          if another app also responds.
        </p>
      )}
      {["Store in Slot", "Save to Device", "Show/Hide Picker"].map(
        (label, index) => (
          <div key={label} className="shortcut-row">
            <span>{label}</span>
            <button
              disabled={!view || saving}
              aria-label={`${label} shortcut`}
              aria-pressed={recording === index}
              onClick={() => {
                setError("");
                if (recording === index) {
                  stop();
                  return;
                }
                void invoke("shortcut_recording", { recording: true })
                  .then(() => {
                    recorder.current = index;
                    setRecording(index);
                  })
                  .catch((e: unknown) => setError(String(e)));
              }}
            >
              {recording === index
                ? "Press shortcut…"
                : view
                  ? displayShortcut(view.config.keys[index], mac)
                  : "Loading…"}
            </button>
            <button
              aria-label={`Disable ${label} shortcut`}
              disabled={!view?.config.keys[index] || saving}
              onClick={() => assign(index, null)}
            >
              Off
            </button>
            {view?.errors[index] && (
              <small role="alert">{view.errors[index]}</small>
            )}
          </div>
        ),
      )}
      {error && (
        <p className="warning" role="alert">
          {error}
        </p>
      )}
      <div className="settings-actions">
        {recording !== null && <button onClick={stop}>Cancel recording</button>}
        <button
          disabled={saving || !view}
          onClick={() => void save(defaultShortcuts())}
        >
          Reset shortcuts
        </button>
      </div>
    </section>
  );
}
