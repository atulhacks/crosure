import { create } from "zustand";
import { clampPane, loadPanes, savePanes, type PaneSizes } from "../lib/panes";

export type Theme = "crosure-dark" | "crosure-light";

interface UiState {
  theme: Theme;
  panes: PaneSizes;
  paletteOpen: boolean;
  consoleOpen: boolean;
  /** Investigation panel: the graph, or the chronological flight log. */
  invMode: "graph" | "log";
  /** Disassembly tab: linear listing or basic-block graph. */
  disasmMode: "linear" | "graph";
  /** Bottom of the investigation column: the AI agent or the selected step. */
  dockTab: "agent" | "step";
  setDockTab: (t: "agent" | "step") => void;
  setTheme: (t: Theme) => void;
  setPane: (key: keyof PaneSizes, px: number, persist?: boolean) => void;
  setPaletteOpen: (open: boolean) => void;
  toggleConsole: () => void;
  setInvMode: (m: "graph" | "log") => void;
  setDisasmMode: (m: "linear" | "graph") => void;
}

function storedTheme(): Theme {
  try {
    return localStorage.getItem("crosure.theme") === "crosure-light"
      ? "crosure-light"
      : "crosure-dark";
  } catch {
    return "crosure-dark";
  }
}

/** Applies a theme to the document root. */
export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
}

export const useUi = create<UiState>((set, get) => ({
  theme: storedTheme(),
  panes: loadPanes(),
  paletteOpen: false,
  consoleOpen: true,
  invMode: "graph",
  disasmMode: "linear",
  dockTab: "agent",
  setDockTab: (dockTab) => set({ dockTab }),
  setTheme: (theme) => {
    applyTheme(theme);
    try {
      localStorage.setItem("crosure.theme", theme);
    } catch {
      /* convenience only */
    }
    set({ theme });
  },
  setPane: (key, px, persist = false) => {
    const panes = { ...get().panes, [key]: clampPane(key, px) };
    set({ panes });
    if (persist) savePanes(panes);
  },
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  toggleConsole: () => set({ consoleOpen: !get().consoleOpen }),
  setInvMode: (invMode) => set({ invMode }),
  setDisasmMode: (disasmMode) => set({ disasmMode }),
}));
