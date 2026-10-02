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

/** Dot colour (a theme variable) for each step kind. */
export const KIND_COLOR: Record<StepKind, string> = {
  load: "var(--kind-neutral)",
  recon: "var(--kind-neutral)",
  navigate: "var(--kind-neutral)",
  functions: "var(--kind-neutral)",
  disasm: "var(--kind-code)",
  decompile: "var(--kind-code)",
  xref: "var(--kind-xref)",
  strings: "var(--kind-data)",
  imports: "var(--kind-import)",
  rename: "var(--kind-edit)",
  comment: "var(--kind-edit)",
  patch: "var(--kind-edit)",
  debug: "var(--kind-code)",
  sandbox: "var(--kind-import)",
  shell: "var(--kind-neutral)",
  agent: "var(--kind-agent)",
  hypothesis: "var(--kind-hypothesis)",
  finding: "var(--kind-finding)",
  verdict: "var(--kind-finding)",
  annotate: "var(--kind-neutral)",
};

/**
 * Time since the session started, as `m:ss`.
 * @example clock(65_000) // "1:05"
 */
export function clock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

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
export const NODE_SIZE = { w: 240, h: 58 };
