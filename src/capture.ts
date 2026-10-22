import { invoke } from "@tauri-apps/api/core";

export type Destination = "store" | "save";
export type Selection = { x1: number; y1: number; x2: number; y2: number };
export type Outcome =
  | { status: "cancelled" }
  | { status: "stored"; slot: number; width: number; height: number }
  | { status: "saved"; path: string; width: number; height: number };

export const captureRegion = (
  destination: Destination,
  slot: number | null,
  revision: number,
  replace: boolean,
) =>
  invoke<Outcome>("capture_region", { destination, slot, revision, replace });
export const regionSnapshot = () => invoke<ArrayBuffer>("region_snapshot");
export const regionReady = () => invoke<void>("region_ready");
export const finishRegion = (selection: Selection) =>
  invoke<void>("finish_region", { selection });
export const cancelRegion = (failed = false) =>
  invoke<void>("cancel_region", { failed });
