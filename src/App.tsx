import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent,
} from "react";
import { captureRegion, type Destination } from "./capture";
import RegionOverlay from "./RegionOverlay";
import { selectSlot, type SelectionState } from "./selection";
import "./App.css";

type Entry = { token: string; width: number; height: number };
type SlotView = {
  slots: (Entry | null)[];
  revision: number;
  exportDirectory: string | null;
  defaultDirectory: string;
  warning: string | null;
};
type Confirmation = { text: string; action: () => void };
const preview: SlotView = {
  slots: Array.from({ length: 5 }, () => null),
  revision: 0,
  exportDirectory: null,
  defaultDirectory: "Pictures",
  warning: null,
};

export default function App() {
  if (new URLSearchParams(window.location.search).has("region")) {
    return <RegionOverlay />;
  }
  return <Picker />;
}

function Confirm({ value, close }: { value: Confirmation; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog ref={dialog} className="confirmation" onCancel={close}>
      <h2>Confirm permanent change</h2>
      <p>{value.text}</p>
      <div className="dialog-actions">
        <button autoFocus onClick={close}>
          Cancel
        </button>
        <button
          className="danger"
          onClick={() => {
            close();
            value.action();
          }}
        >
          Continue
        </button>
      </div>
    </dialog>
  );
}

function Picker() {
  const native = isTauri();
  const [view, setView] = useState<SlotView | null>(native ? null : preview);
  const [selection, setSelection] = useState<SelectionState>({
    ids: [],
    anchor: null,
  });
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [settings, setSettings] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [message, setMessage] = useState(
    native
      ? "Loading local slots…"
      : "Desktop preview · Open the Tauri app to capture and manage screenshots.",
  );
  const inFlight = useRef(false);
  const press = useRef<{
    x: number;
    y: number;
    id: number;
    ids: number[];
    dragged: boolean;
  } | null>(null);
  const grid = useRef<HTMLDivElement>(null);
  const mac = /Mac/i.test(navigator.userAgent);

  const refresh = useCallback(async () => {
    if (!native) return;
    const next = await invoke<SlotView>("slot_view");
    setView(next);
    setSelection((current) => ({
      ids: current.ids.filter((id) => id <= next.slots.length),
      anchor:
        current.anchor !== null && current.anchor <= next.slots.length
          ? current.anchor
          : null,
    }));
  }, [native]);

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    void invoke<SlotView>("slot_view")
      .then((next) => {
        if (disposed) return;
        setView(next);
        setMessage("Select screenshots, then drag them into another app.");
      })
      .catch((e: unknown) => {
        if (!disposed) setMessage(String(e));
      });
    return () => {
      disposed = true;
    };
  }, [native]);

  useEffect(() => {
    if (!native || !view) return;
    let disposed = false;
    const urls: string[] = [];
    const entries = view.slots;
    void Promise.allSettled(
      entries.map(async (entry, index) => {
        if (!entry) return null;
        const bytes = await invoke<ArrayBuffer>("slot_thumbnail", {
          id: index + 1,
        });
        if (disposed) return null;
        const url = URL.createObjectURL(
          new Blob([bytes], { type: "image/png" }),
        );
        urls.push(url);
        return [entry.token, url] as const;
      }),
    ).then((results) => {
      if (disposed) return;
      setThumbnails(
        Object.fromEntries(
          results.flatMap((r) =>
            r.status === "fulfilled" && r.value ? [r.value] : [],
          ),
        ),
      );
      if (results.some((r) => r.status === "rejected"))
        setMessage(
          "Some thumbnails could not load. Retry loading slots; originals remain on disk.",
        );
    });
    return () => {
      disposed = true;
      urls.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [native, view]);

  const run = async (action: () => Promise<string>) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    try {
      setMessage(await action());
    } catch (e) {
      setMessage(String(e));
    } finally {
      try {
        await refresh();
      } catch (e) {
        setMessage(String(e));
      }
      inFlight.current = false;
      setBusy(false);
    }
  };

  const occupied = selection.ids.filter((id) => view?.slots[id - 1]);
  const ready = native && !!view && !busy;
  const setup = !!view && !view.exportDirectory;

  const capture = (destination: Destination) => {
    if (!view) return;
    const slot =
      destination === "store" && selection.ids.length === 1
        ? selection.ids[0]
        : null;
    const replace = slot !== null && !!view.slots[slot - 1];
    const action = () =>
      void run(async () => {
        const result = await captureRegion(
          destination,
          slot,
          view.revision,
          replace,
        );
        if (result.status === "cancelled") return "Capture cancelled.";
        if (result.status === "stored") {
          setSelection({ ids: [result.slot], anchor: result.slot });
          return `Stored ${result.width} × ${result.height} PNG in slot ${result.slot}.`;
        }
        return `Saved ${result.width} × ${result.height} PNG to ${result.path}`;
      });
    if (replace)
      setConfirmation({
        text: `Replace slot ${slot}? Its current PNG and thumbnail will be permanently deleted after the new capture is saved. Cancelled capture keeps the current image.`,
        action,
      });
    else action();
  };

  const changeCount = (count: number) => {
    if (!view || count === view.slots.length) return;
    const affected = view.slots.slice(count).filter(Boolean).length;
    const action = () =>
      void run(async () => {
        await invoke("resize_slots", {
          count,
          revision: view.revision,
          confirmed: affected > 0,
        });
        return `Now showing ${count} slots.`;
      });
    if (affected)
      setConfirmation({
        text: `Reduce to ${count} slots? Permanently delete ${affected} screenshot${affected === 1 ? "" : "s"} and their thumbnails above slot ${count}. Exported files are kept.`,
        action,
      });
    else action();
  };

  const pointerDown = (event: PointerEvent<HTMLButtonElement>, id: number) => {
    if (event.button !== 0 || busy) return;
    const next = selectSlot(
      selection,
      id,
      { shift: event.shiftKey, toggle: mac ? event.metaKey : event.ctrlKey },
      true,
    );
    setSelection(next);
    press.current = {
      x: event.clientX,
      y: event.clientY,
      id,
      ids: next.ids,
      dragged: false,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const pointerMove = (event: PointerEvent<HTMLButtonElement>) => {
    const start = press.current;
    if (
      !start ||
      start.dragged ||
      !(event.buttons & 1) ||
      !ready ||
      !view?.slots[start.id - 1]
    )
      return;
    if (Math.hypot(event.clientX - start.x, event.clientY - start.y) < 7)
      return;
    start.dragged = true;
    event.currentTarget.releasePointerCapture(event.pointerId);
    const ids = start.ids.filter((id) => view.slots[id - 1]);
    void run(() => invoke<string>("drag_slots", { ids }));
  };

  return (
    <main className="picker">
      <header className="picker-header">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            ▣
          </span>
          <h1>Slotshot</h1>
          <span className="slot-count">{view?.slots.length ?? "…"} slots</span>
        </div>
        <button
          className="icon-button"
          aria-label="Settings"
          aria-expanded={settings}
          disabled={!view || busy}
          onClick={() => setSettings(!settings)}
        >
          ⚙
        </button>
      </header>
      <div className="capture-actions">
        <button
          className="primary"
          disabled={!ready || setup}
          onClick={() => capture("store")}
        >
          ＋ Store in Slot
        </button>
        <button disabled={!ready || setup} onClick={() => capture("save")}>
          Save to Device
        </button>
        <span className="destination-hint">
          {selection.ids.length === 1
            ? `Into slot ${selection.ids[0]}`
            : "Into first empty slot"}
        </span>
      </div>
      {(settings || setup) && view && (
        <section className="settings" aria-labelledby="settings-title">
          <h2 id="settings-title">
            {setup ? "Choose where exports go" : "Settings"}
          </h2>
          <p>
            Exports are separate PNG files. Your slots stay private in
            Slotshot’s local storage.
          </p>
          <div className="folder-path">
            {view.exportDirectory ?? view.defaultDirectory}
          </div>
          <div className="settings-actions">
            {setup && (
              <button
                disabled={!ready}
                onClick={() =>
                  void run(async () => {
                    await invoke("configure_exports", { useDefault: true });
                    return "Export folder set to Pictures.";
                  })
                }
              >
                Use Pictures
              </button>
            )}
            <button
              disabled={!ready}
              onClick={() =>
                void run(async () =>
                  (await invoke<boolean>("configure_exports", {
                    useDefault: false,
                  }))
                    ? "Export folder updated."
                    : "Folder selection cancelled.",
                )
              }
            >
              Choose Folder…
            </button>
            <label>
              Slots{" "}
              <select
                aria-label="Number of slots"
                disabled={!ready}
                value={view.slots.length}
                onChange={(e) => changeCount(Number(e.target.value))}
              >
                {Array.from({ length: 10 }, (_, i) => (
                  <option key={i + 1}>{i + 1}</option>
                ))}
              </select>
            </label>
            {!setup && <button onClick={() => setSettings(false)}>Done</button>}
          </div>
        </section>
      )}
      <div
        className="slots"
        ref={grid}
        role="group"
        aria-label="Screenshot slots"
        onKeyDown={(event) => {
          if (
            ![
              "ArrowLeft",
              "ArrowRight",
              "ArrowUp",
              "ArrowDown",
              "Home",
              "End",
            ].includes(event.key)
          )
            return;
          const buttons = Array.from(
            grid.current?.querySelectorAll<HTMLButtonElement>(".slot") ?? [],
          );
          const current = buttons.indexOf(
            document.activeElement as HTMLButtonElement,
          );
          if (current < 0) return;
          event.preventDefault();
          const next =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? buttons.length - 1
                : Math.max(
                    0,
                    Math.min(
                      buttons.length - 1,
                      current +
                        (["ArrowLeft", "ArrowUp"].includes(event.key) ? -1 : 1),
                    ),
                  );
          buttons[next]?.focus();
          setSelection(
            selectSlot(selection, next + 1, {
              shift: event.shiftKey,
              toggle: false,
            }),
          );
        }}
      >
        {view?.slots.map((entry, index) => {
          const id = index + 1;
          const selected = selection.ids.includes(id);
          return (
            <button
              key={id}
              className={`slot ${entry ? "occupied" : "empty"} ${selected ? "selected" : ""}`}
              aria-pressed={selected}
              aria-label={`Slot ${id}, ${entry ? `${entry.width} by ${entry.height} pixels` : "empty"}`}
              disabled={busy}
              onContextMenu={(e) => e.preventDefault()}
              onDragStart={(e) => e.preventDefault()}
              onPointerDown={(e) => pointerDown(e, id)}
              onPointerMove={pointerMove}
              onPointerCancel={() => {
                press.current = null;
              }}
              onPointerUp={(e) => {
                const start = press.current;
                if (
                  start &&
                  !start.dragged &&
                  !e.shiftKey &&
                  !(mac ? e.metaKey : e.ctrlKey)
                )
                  setSelection(
                    selectSlot(selection, id, { shift: false, toggle: false }),
                  );
              }}
              onClick={(e) => {
                if (e.detail === 0)
                  setSelection(
                    selectSlot(selection, id, {
                      shift: e.shiftKey,
                      toggle: mac ? e.metaKey : e.ctrlKey,
                    }),
                  );
              }}
            >
              <span className="slot-number">{id}</span>
              {entry ? (
                <>
                  {thumbnails[entry.token] ? (
                    <img
                      src={thumbnails[entry.token]}
                      alt=""
                      draggable={false}
                    />
                  ) : (
                    <span className="thumbnail-loading">
                      Preview unavailable
                    </span>
                  )}
                  <span className="dimensions">
                    {entry.width} × {entry.height}
                  </span>
                </>
              ) : (
                <span className="empty-content">
                  <span aria-hidden="true">＋</span>
                  <span>Empty slot</span>
                </span>
              )}
            </button>
          );
        })}
      </div>
      {view && view.slots.every((slot) => !slot) && (
        <p className="empty-hint">
          Your screenshots, right where you need them.
          <br />
          Select an empty slot and capture a region to get started.
        </p>
      )}
      <footer className="toolbar">
        <button
          disabled={!ready || setup || !occupied.length}
          onClick={() =>
            void run(async () => {
              const result = await invoke<{
                saved: string[];
                failed: string[];
              }>("save_slots", { ids: occupied });
              return [
                `Saved ${result.saved.length} PNG${result.saved.length === 1 ? "" : "s"}.`,
                ...result.saved,
                ...result.failed,
              ].join("\n");
            })
          }
        >
          <span aria-hidden="true">↓</span> Save
          {occupied.length > 1 ? ` (${occupied.length})` : ""}
        </button>
        <button
          disabled={!ready || !occupied.length}
          onClick={() =>
            setConfirmation({
              text: `Permanently clear ${occupied.length} selected screenshot${occupied.length === 1 ? "" : "s"} and their thumbnails? Exported files are kept.`,
              action: () =>
                void run(async () => {
                  await invoke("clear_slots", {
                    ids: occupied,
                    revision: view?.revision,
                  });
                  return "Selected screenshots cleared.";
                }),
            })
          }
        >
          <span aria-hidden="true">⌫</span> Clear
        </button>
        <span className="drag-hint">
          {occupied.length
            ? `${occupied.length} selected · Drag to another app`
            : "Select · Shift range · " +
              (mac ? "⌘" : "Ctrl") +
              " multi-select"}
        </span>
      </footer>
      <div className="feedback">
        <p role="status" aria-live="polite">
          {busy ? "Working… " : ""}
          {message}
        </p>
        {!busy && native && (
          <button
            className="retry"
            onClick={() =>
              void run(async () => "Slots reloaded; cleanup retried.")
            }
          >
            Reload
          </button>
        )}
      </div>
      {view?.warning && (
        <p role="alert" className="warning">
          {view.warning}
        </p>
      )}
      {confirmation && (
        <Confirm value={confirmation} close={() => setConfirmation(null)} />
      )}
    </main>
  );
}
