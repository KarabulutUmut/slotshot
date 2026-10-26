import { listen } from "@tauri-apps/api/event";
import ShortcutSettings from "./ShortcutSettings";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent,
  type ReactNode,
} from "react";
import {
  captureRegion,
  chooseCaptureSlot,
  type Destination,
  type Outcome,
} from "./capture";
import RegionOverlay from "./RegionOverlay";
import { selectSlot, destinationSlot, type SelectionState } from "./selection";
import { fitSlots, nextSlotIndex } from "./layout";
import "./App.css";

type Entry = { token: string; width: number; height: number };
type SlotView = {
  slots: (Entry | null)[];
  revision: number;
  exportDirectory: string | null;
  defaultDirectory: string;
  warning: string | null;
};
type DesktopView = {
  sequence: number;
  busy: boolean;
  outcome: Outcome | null;
  message: string | null;
  error: boolean;
  settings: boolean;
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

function Icon({
  name,
}: {
  name: "capture" | "save" | "clear" | "settings" | "plus";
}) {
  const paths = {
    capture: "M5 2H2v3m9-3h3v3M2 11v3h3m9-3v3h-3M5 8h6M8 5v6",
    save: "M8 2v8m-3-3 3 3 3-3M2 10v4h12v-4",
    clear: "M2 4h12M6 4V2h4v2M4 4l1 10h6l1-10M7 6v5m2-5v5",
    settings:
      "m6 2 .5-1h3L10 2l1 .6 1.2.1 1.5 2.6-.5 1.1v1.2l.5 1.1-1.5 2.6-1.2.1-1 .6-.5 1h-3l-.5-1-1-.6-1.2-.1L2.3 9.7l.5-1.1V7.4l-.5-1.1 1.5-2.6L5 3.6 6 3M10 8a2 2 0 1 1-4 0 2 2 0 0 1 4 0",
    plus: "M8 3v10M3 8h10",
  };
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.25"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={paths[name]} />
    </svg>
  );
}

function Sheet({
  children,
  close,
  label,
}: {
  children: ReactNode;
  close?: () => void;
  label: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="settings"
      aria-label={label}
      onCancel={(event) => {
        if (close) close();
        else event.preventDefault();
      }}
    >
      {children}
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
  const [localBusy, setBusy] = useState(false);
  const [nativeBusy, setNativeBusy] = useState(false);
  const [pending, setPending] = useState<Extract<
    Outcome,
    { status: "choosing" }
  > | null>(null);
  const choosing = useRef(false);
  const [committing, setCommitting] = useState(false);
  const busy = localBusy || nativeBusy;
  const desktopSequence = useRef(0);
  const loadSequence = useRef(0);
  const [settings, setSettings] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [failure, setFailure] = useState(false);
  const [details, setDetails] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [layout, setLayout] = useState(() => fitSlots(5, 624, 156));
  const viewport = useRef<HTMLDivElement>(null);
  const [message, setMessage] = useState(native ? "Loading local slots…" : "");
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
    const sequence = ++loadSequence.current;
    const next = await invoke<SlotView>("slot_view");
    if (sequence !== loadSequence.current) return;
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
    void refresh()
      .then(() => {
        if (!disposed && desktopSequence.current === 0) setMessage("");
      })
      .catch((e: unknown) => {
        if (!disposed) {
          setMessage(String(e));
          setFailure(true);
        }
      });
    return () => {
      disposed = true;
      // Invalidate an in-flight bootstrap read when React tears down this effect.
      loadSequence.current += 1;
    };
  }, [native, refresh]);

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
      if (results.some((r) => r.status === "rejected")) {
        setFailure(true);
        setMessage(
          "Some thumbnails could not load. Retry loading slots; originals remain on disk.",
        );
      }
    });
    return () => {
      disposed = true;
      urls.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [native, view]);

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const apply = (next: DesktopView) => {
      if (disposed || next.sequence <= desktopSequence.current) return;
      desktopSequence.current = next.sequence;
      setNativeBusy(next.busy);
      if (next.busy) {
        setSettings(false);
        setConfirmation(null);
        setDetails(false);
      }
      if (next.settings) setSettings(true);
      if (next.message) {
        setMessage(next.message);
        setFailure(next.error);
      }
      if (next.error) {
        setDetails(true);
        // Filesystem errors can follow a committed metadata write. Refresh once
        // so recovery never leaves the picker showing an obsolete assignment.
        void refresh().catch((e: unknown) =>
          setMessage(`${next.message ?? ""} ${String(e)}`),
        );
      }
      const outcome = next.outcome;
      setPending(outcome?.status === "choosing" ? outcome : null);
      choosing.current = false;
      setCommitting(false);
      if (outcome?.status === "stored") {
        setSelection({ ids: [outcome.slot], anchor: outcome.slot });
        setMessage(
          `Stored ${outcome.width} × ${outcome.height} PNG in slot ${outcome.slot}.`,
        );
        setFailure(false);
        void refresh().catch((e: unknown) => {
          setFailure(true);
          setMessage(String(e));
        });
      } else if (outcome?.status === "saved") {
        setMessage(
          `Saved ${outcome.width} × ${outcome.height} PNG to ${outcome.path}`,
        );
        setFailure(false);
      } else if (outcome?.status === "cancelled") {
        setMessage("Capture cancelled.");
        setFailure(false);
      }
    };
    // Subscribe before reading the snapshot: native actions cannot be lost during bootstrap.
    const unlisten = listen<DesktopView>("desktop-state", ({ payload }) =>
      apply(payload),
    );
    void unlisten
      .then(() => invoke<DesktopView>("desktop_view"))
      .then(apply)
      .catch((e: unknown) => {
        if (!disposed) {
          setMessage(String(e));
          setFailure(true);
        }
      });
    return () => {
      disposed = true;
      void unlisten.then((fn) => fn());
    };
  }, [native, refresh]);

  const run = async (action: () => Promise<string>) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setFailure(false);
    try {
      setMessage(await action());
    } catch (e) {
      setFailure(true);
      setMessage(String(e));
    } finally {
      try {
        await refresh();
      } catch (e) {
        setFailure(true);
        setMessage(String(e));
      }
      inFlight.current = false;
      setBusy(false);
      setDragging(false);
    }
  };

  const occupied = selection.ids.filter((id) => view?.slots[id - 1]);
  const ready = native && !!view && !busy;
  const setup = !!view && !view.exportDirectory;

  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      setLayout(
        fitSlots(
          view?.slots.length ?? 5,
          entry.contentRect.width - 8,
          entry.contentRect.height - 8,
        ),
      );
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [view?.slots.length]);

  const chooseDestination = useCallback(
    async (slot: number | null) => {
      if (!pending || choosing.current) return;
      choosing.current = true;
      setCommitting(true);
      try {
        await chooseCaptureSlot(pending.capture, slot);
      } catch (error) {
        choosing.current = false;
        setCommitting(false);
        setMessage(String(error));
        setFailure(true);
      }
    },
    [pending],
  );

  useEffect(() => {
    if (!pending) return;
    grid.current?.querySelector<HTMLButtonElement>(".slot")?.focus();
    const keydown = (event: KeyboardEvent) => {
      if (event.repeat || event.metaKey || event.ctrlKey || event.altKey)
        return;
      const slot = destinationSlot(event.key, view?.slots.length ?? 0);
      if (event.key === "Escape" || slot !== null) {
        event.preventDefault();
        void chooseDestination(event.key === "Escape" ? null : slot);
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [pending, view?.slots.length, chooseDestination]);

  const capture = (destination: Destination) => {
    if (!view) return;
    void run(async () => {
      const result = await captureRegion(destination);
      if (result.status === "cancelled") return "Capture cancelled.";
      if (result.status === "stored") {
        setSelection({ ids: [result.slot], anchor: result.slot });
        return `Stored ${result.width} × ${result.height} PNG in slot ${result.slot}.`;
      }
      if (result.status === "saved")
        return `Saved ${result.width} × ${result.height} PNG to ${result.path}`;
      return "Choose a destination slot.";
    });
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
    setDragging(true);
    void run(() => invoke<string>("drag_slots", { ids }));
  };

  return (
    <main className={`picker ${native && mac ? "mac-native" : ""}`}>
      <header className="picker-header" data-tauri-drag-region>
        <h1 className="sr-only">Slotshot</h1>
        <div className="capture-actions">
          {pending ? (
            <>
              <strong className="destination-title" role="status">
                Choose a slot
              </strong>
              <button
                disabled={committing}
                onClick={() => void chooseDestination(null)}
              >
                Cancel
              </button>
            </>
          ) : (
            <>
              <button
                disabled={!ready || setup}
                title="Capture a region, then click any destination slot"
                onClick={() => capture("store")}
              >
                <Icon name="capture" /> Store in Slot
              </button>
              <button
                disabled={!ready || setup}
                onClick={() => capture("save")}
              >
                <Icon name="save" /> Save to Device
              </button>
            </>
          )}
        </div>
        <button
          className="icon-button"
          aria-label="Settings"
          title="Settings"
          aria-haspopup="dialog"
          aria-expanded={settings || setup}
          disabled={!view || busy}
          onClick={() => setSettings(!settings)}
        >
          <Icon name="settings" />
        </button>
      </header>
      {(settings || setup) && view && (
        <Sheet
          label={setup ? "Choose where exports go" : "Settings"}
          close={setup ? undefined : () => setSettings(false)}
        >
          <h2 id="settings-title">
            {setup ? "Choose where exports go" : "Settings"}
          </h2>
          <p>
            Exports are separate PNG files. Your slots stay private in
            Slotshot’s local storage.
          </p>
          <h3>General</h3>
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
          </div>
          {!setup && native && <ShortcutSettings mac={mac} />}
          {!setup && (
            <div className="dialog-actions settings-done">
              <button onClick={() => setSettings(false)}>Done</button>
            </div>
          )}
        </Sheet>
      )}
      <div className="slot-viewport" ref={viewport}>
        <div
          className={`slots ${layout.scroll ? "scrolling" : ""} ${dragging ? "dragging" : ""}`}
          style={{
            gridTemplateColumns: `repeat(${layout.columns}, ${layout.width}px)`,
            gridAutoRows: `${layout.height}px`,
          }}
          ref={grid}
          role="group"
          aria-label={
            pending
              ? "Choose screenshot destination; occupied slots replace immediately"
              : "Screenshot slots"
          }
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
            const next = nextSlotIndex(
              current,
              event.key,
              layout.columns,
              buttons.length,
            );
            buttons[next]?.focus();
            if (!pending)
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
                title={
                  pending
                    ? `Store in slot ${id}${entry ? " · Replace immediately" : ""}`
                    : entry
                      ? `Slot ${id} · ${entry.width} × ${entry.height} pixels`
                      : `Slot ${id} · Empty`
                }
                aria-pressed={selected}
                aria-label={`Slot ${id}, ${entry ? `${entry.width} by ${entry.height} pixels` : "empty"}`}
                disabled={(busy && !pending) || committing}
                onContextMenu={(e) => e.preventDefault()}
                onDragStart={(e) => e.preventDefault()}
                onPointerDown={(e) => {
                  if (!pending) pointerDown(e, id);
                }}
                onPointerMove={pointerMove}
                onPointerCancel={() => {
                  press.current = null;
                }}
                onPointerUp={(e) => {
                  if (pending) return;
                  const start = press.current;
                  if (
                    start &&
                    !start.dragged &&
                    !e.shiftKey &&
                    !(mac ? e.metaKey : e.ctrlKey)
                  )
                    setSelection(
                      selectSlot(selection, id, {
                        shift: false,
                        toggle: false,
                      }),
                    );
                }}
                onClick={(e) => {
                  if (pending) {
                    void chooseDestination(id);
                    return;
                  }
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
                  </>
                ) : (
                  <span className="empty-content">
                    <Icon name="plus" />
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>
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
          <Icon name="save" /> Save
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
          <Icon name="clear" /> Clear
        </button>
        <span className="selection-count">
          {pending
            ? `Click or press 1–${Math.min(view?.slots.length ?? 1, 9)}${view?.slots.length === 10 ? "; 0 = 10" : ""} · Escape cancels`
            : occupied.length
              ? `${occupied.length} selected`
              : `${view?.slots.length ?? "…"} ${view?.slots.length === 1 ? "slot" : "slots"}`}
        </span>
        {view?.warning && (
          <span className="sr-only" role="alert">
            {view.warning}
          </span>
        )}
        <span className="sr-only" role="status" aria-live="polite">
          {pending
            ? "Choose any slot. Occupied slots replace immediately. Escape cancels."
            : busy
              ? "Working…"
              : message}
        </span>
        {!pending && (message || view?.warning || busy) && (
          <button
            className={`activity ${failure || view?.warning ? "has-error" : ""}`}
            title={view?.warning ?? message}
            onClick={() => setDetails(true)}
            aria-label="Activity details"
          >
            {busy
              ? "Working…"
              : failure || view?.warning
                ? "Needs attention"
                : "Details"}
          </button>
        )}
      </footer>
      {details && (
        <Sheet label="Activity" close={() => setDetails(false)}>
          <h2>Activity</h2>
          <p className="activity-message">{message}</p>
          {view?.warning && (
            <p role="alert" className="warning">
              {view.warning}
            </p>
          )}
          <div className="dialog-actions">
            {!busy && native && (failure || view?.warning) && (
              <button
                onClick={() =>
                  void run(async () => "Slots reloaded; cleanup retried.")
                }
              >
                Reload
              </button>
            )}
            <button autoFocus onClick={() => setDetails(false)}>
              Done
            </button>
          </div>
        </Sheet>
      )}
      {confirmation && (
        <Confirm value={confirmation} close={() => setConfirmation(null)} />
      )}
    </main>
  );
}
