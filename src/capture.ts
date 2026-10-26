import { invoke } from "@tauri-apps/api/core";
export type Destination = "store" | "save";
export type Selection = { x1: number; y1: number; x2: number; y2: number };
export type Outcome =
  | { status: "cancelled" }
  | { status: "choosing"; capture: number; width: number; height: number }
  | { status: "stored"; slot: number; width: number; height: number }
  | { status: "saved"; path: string; width: number; height: number };
export const captureRegion = (destination: Destination) =>
  invoke<Outcome>("capture_region", { destination });
export const regionReady = () => invoke<void>("region_ready");
export const regionInteractive = (capture: number) =>
  invoke<void>("region_interactive", { capture });
export const finishRegion = (capture: number, selection: Selection) =>
  invoke<void>("finish_region", { capture, selection });
export const cancelRegion = (capture: number, failed = false) =>
  invoke<void>("cancel_region", { capture, failed });
export const chooseCaptureSlot = (capture: number, slot: number | null) =>
  invoke<void>("choose_capture_slot", { capture, slot });
