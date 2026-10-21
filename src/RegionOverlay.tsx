import { useEffect, useRef, useState, type PointerEvent } from "react";
import {
  cancelRegion,
  finishRegion,
  regionReady,
  regionSnapshot,
} from "./capture";

type Point = { x: number; y: number };

export default function RegionOverlay() {
  const [source, setSource] = useState<string>();
  const [start, setStart] = useState<Point>();
  const [end, setEnd] = useState<Point>();
  const [message, setMessage] = useState(
    "Drag a rectangle on this display. Escape cancels. Regions stay on one display.",
  );
  const submitting = useRef(false);

  useEffect(() => {
    let active = true;
    let url: string | undefined;
    regionSnapshot()
      .then((bytes) => {
        if (!active) return;
        url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
        setSource(url);
      })
      .catch(() => {
        if (active) void cancelRegion(true);
      });
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void cancelRegion().catch(() => {});
      }
    };
    window.addEventListener("keydown", keydown);
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
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
    if (!start || submitting.current) return;
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
      await finishRegion({ x1: start.x, y1: start.y, x2: last.x, y2: last.y });
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
        if (event.button !== 0 || submitting.current || !source) return;
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
      {source && (
        <img
          className="region-snapshot"
          src={source}
          alt="Display snapshot for region selection"
          draggable={false}
          onLoad={() => {
            void regionReady().catch(() => cancelRegion(true));
          }}
          onError={() => void cancelRegion(true)}
        />
      )}
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
        onClick={() => void cancelRegion().catch(() => {})}
      >
        Cancel
      </button>
    </div>
  );
}
