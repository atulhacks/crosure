import dagre from "@dagrejs/dagre";
import type { Edge, Node } from "@xyflow/react";
import type { BasicBlock, Instruction } from "../types";

export const BLOCK_W = 420;
const ROW = 16;
const HEAD = 22;

/** Data carried by a CFG block node. */
export interface BlockData extends Record<string, unknown> {
  block: BasicBlock;
  insns: Instruction[];
  entry: boolean;
}

/**
 * Lays out a function's basic blocks top-to-bottom. Taken edges are green,
 * fall-through red, unconditional jumps neutral — the reading every RE tool
 * uses, so it needs no legend.
 */
export function layoutCfg(
  blocks: BasicBlock[],
  insns: Instruction[],
): { nodes: Node<BlockData>[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 36, ranksep: 46 });
  g.setDefaultEdgeLabel(() => ({}));
  const height = (b: BasicBlock) => HEAD + b.count * ROW + 8;
  for (const b of blocks) g.setNode(String(b.addr), { width: BLOCK_W, height: height(b) });
  for (const b of blocks) for (const s of b.succs) g.setEdge(String(b.addr), String(s.to));
  dagre.layout(g);
  const color = { taken: "var(--good)", fall: "var(--bad)", jump: "var(--edge-strong)" } as const;
  return {
    nodes: blocks.map((b, i) => {
      const p = g.node(String(b.addr));
      return {
        id: String(b.addr),
        type: "block",
        position: { x: (p?.x ?? 0) - BLOCK_W / 2, y: (p?.y ?? 0) - height(b) / 2 },
        data: { block: b, insns: insns.slice(b.first, b.first + b.count), entry: i === 0 },
      };
    }),
    edges: blocks.flatMap((b) =>
      b.succs.map((s) => ({
        id: `${b.addr}-${s.to}-${s.kind}`,
        source: String(b.addr),
        target: String(s.to),
        type: "smoothstep",
        style: { stroke: color[s.kind], strokeWidth: 1.5 },
      })),
    ),
  };
}
