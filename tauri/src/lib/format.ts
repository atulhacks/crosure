import type { StepKind } from "../types";

/**
 * Formats an address as `0x…` hex.
 * @example hex(4489) // "0x1189"
 */
export function hex(addr: number): string {
  return "0x" + addr.toString(16);
}

/**
 * Formats a duration in ms as a short human string.
 * @example duration(65_000) // "1m 5s"
 */
export function duration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  return `${m}m ${s % 60}s`;
}

/** Shortens a `sha256:…` hash for display. */
export function shortHash(h: string, n = 10): string {
  return h.replace(/^sha256:/, "").slice(0, n);
}

/** Accent colour (CSS) for each step kind on the canvas. */
export const KIND_COLOR: Record<StepKind, string> = {
  load: "#64748b",
  recon: "#64748b",
  navigate: "#94a3b8",
  functions: "#94a3b8",
  disasm: "#38bdf8",
  decompile: "#22d3ee",
  xref: "#a78bfa",
  strings: "#facc15",
  imports: "#fb923c",
  rename: "#4ade80",
  comment: "#4ade80",
  patch: "#f472b6",
  debug: "#f472b6",
  sandbox: "#f472b6",
  shell: "#94a3b8",
  agent: "#e879f9",
  hypothesis: "#fbbf24",
  finding: "#f43f5e",
  verdict: "#ef4444",
  annotate: "#64748b",
};

/** One-click intent chips offered after each step. */
export const INTENT_CHIPS: { id: string; label: string }[] = [
  { id: "find_c2", label: "Find C2" },
  { id: "find_crypto", label: "Find crypto/decoding" },
  { id: "find_persistence", label: "Find persistence" },
  { id: "find_entry_logic", label: "Find main logic" },
  { id: "understand_flow", label: "Understand flow" },
  { id: "verify_hunch", label: "Verify a hunch" },
];

/** Tags an analyst can put on a step. */
export const TAGS = ["lead", "dead_end", "key_step"] as const;

/** Canvas node box size (shared by the layout and the node component). */
export const NODE_SIZE = { w: 230, h: 68 };
