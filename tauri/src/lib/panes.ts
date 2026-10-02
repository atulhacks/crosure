/** Persisted pane sizes (px). */
export interface PaneSizes {
  left: number;
  right: number;
  console: number;
  inspector: number;
}

export const PANE_DEFAULTS: PaneSizes = { left: 248, right: 440, console: 168, inspector: 380 };

export const PANE_LIMITS: Record<keyof PaneSizes, [number, number]> = {
  left: [180, 480],
  right: [320, 760],
  console: [72, 480],
  inspector: [140, 760],
};

/**
 * Clamps a pane size to its limits.
 * @example clampPane("left", 10) // 180
 */
export function clampPane(key: keyof PaneSizes, px: number): number {
  const [lo, hi] = PANE_LIMITS[key];
  return Math.round(Math.min(hi, Math.max(lo, px)));
}

const KEY = "crosure.panes";

/** Loads sizes from localStorage, falling back to defaults on any problem. */
export function loadPanes(): PaneSizes {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<PaneSizes>;
    const out = { ...PANE_DEFAULTS };
    for (const k of Object.keys(PANE_DEFAULTS) as (keyof PaneSizes)[]) {
      if (typeof raw[k] === "number") out[k] = clampPane(k, raw[k] as number);
    }
    return out;
  } catch {
    return { ...PANE_DEFAULTS };
  }
}

/** Saves sizes; storage failures are ignored (sizes are a convenience). */
export function savePanes(sizes: PaneSizes): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(sizes));
  } catch {
    /* private mode or quota: keep the in-memory sizes */
  }
}
