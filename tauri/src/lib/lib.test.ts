import { describe, expect, it } from "vitest";
import { clock, duration, hex, shortHash } from "./format";
import { fuzzyFilter, fuzzyScore } from "./fuzzy";
import { clampPane, loadPanes, PANE_DEFAULTS } from "./panes";
import { layoutCfg } from "./cfgLayout";
import { hexRows } from "./hexdump";
import { layoutGraph } from "./layout";
import { nextParent } from "./parent";
import type { GraphNode } from "../types";

const node = (id: string, seq: number, key = false): GraphNode => ({
  id,
  seq,
  ts_ms: seq * 1000,
  kind: "disasm",
  actor: "human",
  tool: "crosure",
  label: id,
  summary: "",
  command: null,
  target: null,
  intent: null,
  tags: [],
  dwell_ms: 0,
  on_key_path: key,
  hash: "sha256:00",
});

describe("format", () => {
  it("formats hex, durations and hashes", () => {
    expect(hex(0x1189)).toBe("0x1189");
    expect(duration(400)).toBe("400ms");
    expect(duration(65_000)).toBe("1m 5s");
    expect(shortHash("sha256:abcdef0123456789", 6)).toBe("abcdef");
    expect(clock(65_000)).toBe("1:05");
  });
});

describe("nextParent", () => {
  const base = { branchFrom: null, headId: "c", viewStepId: "b", fromView: false };
  it("returns null for a plain action", () => expect(nextParent(base)).toBeNull());
  it("links clicks to the view's step", () =>
    expect(nextParent({ ...base, fromView: true })).toEqual({ id: "b", rel: "derived_from" }));
  it("branches when the analyst jumped back", () =>
    expect(nextParent({ ...base, branchFrom: "a", fromView: true })).toEqual({
      id: "a",
      rel: "branch",
    }));
  it("does not branch from the head itself", () =>
    expect(nextParent({ ...base, branchFrom: "c" })).toBeNull());
});

describe("hexRows", () => {
  it("splits into rows with ascii", () => {
    const rows = hexRows(0x10, "41420a" + "00".repeat(16));
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ addr: 16, ascii: "AB" + ".".repeat(14) });
    expect(rows[1].addr).toBe(32);
  });
});

describe("layoutGraph", () => {
  it("places nodes top-to-bottom and styles edges", () => {
    const nodes = [node("a", 0, true), node("b", 1, true), node("c", 2)];
    const out = layoutGraph(
      nodes,
      [
        { from: "a", to: "b", rel: "next" },
        { from: "a", to: "c", rel: "branch" },
      ],
      "b",
    );
    expect(out.nodes[1].position.y).toBeGreaterThan(out.nodes[0].position.y);
    expect(out.nodes[1].data.selected).toBe(true);
    expect(out.edges[0].style?.stroke).toBe("var(--brand)");
    expect(out.edges[1].style?.strokeDasharray).toBe("5 4");
    expect(out.edges[1].label).toBe("branch");
  });
});

describe("fuzzy", () => {
  it("prefers word starts and consecutive hits", () => {
    expect(fuzzyScore("zz", "main")).toBeNull();
    const ranked = fuzzyFilter("chk", ["cache_lookup_key", "check_password", "main"], (s) => s);
    expect(ranked[0]).toBe("check_password");
    expect(ranked).not.toContain("main");
  });
});

describe("panes", () => {
  it("clamps and falls back to defaults", () => {
    expect(clampPane("left", 10)).toBe(180);
    expect(clampPane("right", 9999)).toBe(760);
    localStorage.setItem("crosure.panes", "{not json");
    expect(loadPanes()).toEqual(PANE_DEFAULTS);
    localStorage.setItem("crosure.panes", JSON.stringify({ left: 300 }));
    expect(loadPanes().left).toBe(300);
  });
});

describe("layoutCfg", () => {
  it("colours edges by kind and slices instructions per block", () => {
    const insn = (addr: number) => ({
      addr,
      bytes: "90",
      mnemonic: "nop",
      operands: "",
      target: null,
      comment: null,
    });
    const out = layoutCfg(
      [
        {
          addr: 0,
          end: 2,
          first: 0,
          count: 2,
          succs: [
            { to: 2, kind: "taken" },
            { to: 3, kind: "fall" },
          ],
        },
        { addr: 2, end: 3, first: 2, count: 1, succs: [] },
        { addr: 3, end: 4, first: 3, count: 1, succs: [] },
      ],
      [insn(0), insn(1), insn(2), insn(3)],
    );
    expect(out.nodes[0].data.insns).toHaveLength(2);
    expect(out.nodes[0].data.entry).toBe(true);
    expect(out.edges.map((e) => e.style?.stroke)).toEqual(["var(--good)", "var(--bad)"]);
  });
});

describe("toConfig", () => {
  it("sends a key only when the user typed or cleared one", async () => {
    const { toConfig } = await import("../components/agent/ProviderForm");
    const base = {
      id: "ollama",
      kind: "openai_compatible" as const,
      label: "Ollama",
      base_url: "http://localhost:11434/v1",
      model: "qwen2.5-coder:14b",
      key_env: null,
      strict_tools: false,
      enabled: true,
      key_source: "not_needed" as const,
      ready: true,
    };
    expect(toConfig(base).api_key).toBeNull();
    expect(toConfig({ ...base, api_key: "" }).api_key).toBe("");
    expect(Object.keys(toConfig(base))).not.toContain("ready");
  });
});

describe("isReady", () => {
  it("mirrors the backend readiness rules", async () => {
    const { isReady, isLocal } = await import("../components/agent/ProviderForm");
    const d = {
      id: "openai",
      kind: "openai_compatible" as const,
      label: "OpenAI",
      base_url: "https://api.openai.com/v1",
      model: "",
      key_env: "OPENAI_API_KEY",
      strict_tools: true,
      enabled: true,
      key_source: "missing" as const,
    };
    expect(isLocal("http://127.0.0.1:11434/v1")).toBe(true);
    expect(isReady(d)).toBe(false);
    expect(isReady({ ...d, model: "m" })).toBe(false);
    expect(isReady({ ...d, model: "m", api_key: "sk" })).toBe(true);
    expect(isReady({ ...d, model: "m", key_source: "env" })).toBe(true);
    expect(isReady({ ...d, model: "m", base_url: "http://localhost:1234/v1" })).toBe(true);
    expect(isReady({ ...d, model: "m", api_key: "sk", enabled: false })).toBe(false);
  });
});
