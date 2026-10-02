import { describe, expect, it } from "vitest";
import { duration, hex, shortHash } from "./format";
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
    expect(out.edges[0].style?.stroke).toBe("#f43f5e");
    expect(out.edges[1].animated).toBe(true);
  });
});
