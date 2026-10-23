import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type Material = "default" | "regular" | "clear" | "hud";

export function MaterialPreview({
  mode,
  change,
  samples,
  setSamples,
  disabled,
}: {
  mode: Material;
  change: (mode: Material) => void;
  samples: boolean;
  setSamples: (value: boolean) => void;
  disabled: boolean;
}) {
  const [enabled, setEnabled] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (isTauri())
      void invoke<boolean>("material_preview_enabled")
        .then(setEnabled)
        .catch(() => {});
  }, []);
  if (!enabled) return null;
  return (
    <fieldset className="material-preview" disabled={disabled || pending}>
      <legend>Development material comparison</legend>
      <p>
        Temporary; restart restores the existing default. A/B use HUD on macOS
        below 26.
      </p>
      <label>
        Native material
        <select
          aria-label="Native material"
          value={mode}
          onChange={(event) => {
            const next = event.target.value as Material;
            setPending(true);
            setError("");
            void invoke("preview_material", { mode: next })
              .then(() => change(next))
              .catch((reason: unknown) => setError(String(reason)))
              .finally(() => setPending(false));
          }}
        >
          <option value="default">Existing default</option>
          <option value="regular">A · Liquid Glass Regular</option>
          <option value="clear">B · Liquid Glass Clear</option>
          <option value="hud">C · HUD vibrancy</option>
        </select>
      </label>
      <label>
        <input
          type="checkbox"
          checked={samples}
          onChange={(e) => setSamples(e.target.checked)}
        />
        Synthetic thumbnails (capture, storage and drag disabled)
      </label>
      {error && <p role="alert">{error}</p>}
    </fieldset>
  );
}
