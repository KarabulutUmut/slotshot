import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import {
  cancelRegion,
  finishRegion,
  regionReady,
  regionInteractive,
} from "./capture";
type Point = { x: number; y: number };

export default function RegionOverlay() {
  const capture = useRef<number | null>(null);
  const [start, setStart] = useState<Point>();
  const [end, setEnd] = useState<Point>();
  const [message, setMessage] = useState(
    "Drag a rectangle on this display. Escape cancels. Regions stay on one display.",
  );
  const submitting = useRef(false);

  useEffect(() => {
    let active = true;
    let frame = 0;
    let nextFrame = 0;
    const unlisten = listen<number>("region-activate", ({ payload }) => {
      if (!active) return;
      capture.current = payload;
      submitting.current = false;
      setStart(undefined);
      setEnd(undefined);
      setMessage(
        "Drag a rectangle on this display. Escape cancels. Regions stay on one display.",
      );
      // Acknowledge input readiness after the reset has painted in the visible window.
      frame = requestAnimationFrame(() => {
        nextFrame = requestAnimationFrame(() => {
          if (active && capture.current === payload)
            void regionInteractive(payload).catch(() => {});
        });
      });
    });
    void unlisten
      .then(() => {
        if (active) return regionReady();
      })
      .catch(() => {});
    const keydown = (event: KeyboardEvent) => {
      if (
        event.key === "Escape" &&
        capture.current !== null &&
        !submitting.current
      ) {
        event.preventDefault();
        submitting.current = true;
        void cancelRegion(capture.current).catch(() => {
          submitting.current = false;
        });
      }
    };
    window.addEventListener("keydown", keydown);
    return () => {
      active = false;
      cancelAnimationFrame(frame);
      cancelAnimationFrame(nextFrame);
      void unlisten.then((fn) => fn());
      window.removeEventListener("keydown", keydown);
    };
  }, []);

  const point = (event: PointerEvent<HTMLDivElement>): Point => {
    const bounds = event.currentTarget.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width)),
      y: Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height)),
    };
  };

  const finish = async (event: PointerEvent<HTMLDivElement>) => {
    if (!start || submitting.current || capture.current === null) return;
    const last = point(event);
    setEnd(last);
    event.currentTarget.releasePointerCapture(event.pointerId);
    if (start.x === last.x || start.y === last.y) {
      setStart(undefined);
      setEnd(undefined);
      setMessage(
        "The selection is empty. Drag a rectangle, or Escape to cancel.",
      );
      return;
    }
    submitting.current = true;
    try {
      await finishRegion(capture.current, {
        x1: start.x,
        y1: start.y,
        x2: last.x,
        y2: last.y,
      });
    } catch (error) {
      submitting.current = false;
      setStart(undefined);
      setEnd(undefined);
      setMessage(String(error));
    }
  };

  return (
    <div
      className="region-overlay"
      onContextMenu={(event) => event.preventDefault()}
      onPointerDown={(event) => {
        if (
          event.button !== 0 ||
          submitting.current ||
          capture.current === null
        )
          return;
        event.currentTarget.setPointerCapture(event.pointerId);
        const first = point(event);
        setStart(first);
        setEnd(first);
      }}
      onPointerMove={(event) => {
        if (start && !submitting.current) setEnd(point(event));
      }}
      onPointerUp={(event) => void finish(event)}
      onPointerCancel={() => {
        setStart(undefined);
        setEnd(undefined);
      }}
    >
      {start && end ? (
        <div
          className="region-boundary"
          style={{
            left: `${Math.min(start.x, end.x) * 100}%`,
            top: `${Math.min(start.y, end.y) * 100}%`,
            width: `${Math.abs(start.x - end.x) * 100}%`,
            height: `${Math.abs(start.y - end.y) * 100}%`,
          }}
        />
      ) : (
        <div className="region-shade" />
      )}
      <div className="region-instructions" role="status">
        {message}
      </div>
      <button
        className="region-cancel"
        onPointerDown={(event) => event.stopPropagation()}
        onPointerUp={(event) => event.stopPropagation()}
        onClick={() => {
          if (capture.current !== null && !submitting.current) {
            submitting.current = true;
            void cancelRegion(capture.current).catch(() => {
              submitting.current = false;
            });
          }
        }}
      >
        Cancel
      </button>
    </div>
  );
}
