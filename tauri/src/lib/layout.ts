import dagre from "@dagrejs/dagre";
import type { Edge, Node } from "@xyflow/react";
import type { GraphEdge, GraphNode } from "../types";

import { NODE_SIZE } from "./format";

const NODE_W = NODE_SIZE.w;
const NODE_H = NODE_SIZE.h + 14;

/**
 * Lays the investigation graph out top-to-bottom with dagre and returns
 * React Flow nodes/edges. Branch edges are dashed; key-path edges glow.
 */
export function layoutGraph(
  nodes: GraphNode[],
  edges: GraphEdge[],
  selected: string | null,
): { nodes: Node<{ node: GraphNode; selected: boolean }>[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 28, ranksep: 36 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const n of nodes) g.setNode(n.id, { width: NODE_W, height: NODE_H });
  for (const e of edges) g.setEdge(e.from, e.to);
  dagre.layout(g);
  const keyIds = new Set(nodes.filter((n) => n.on_key_path).map((n) => n.id));
  return {
    nodes: nodes.map((n) => {
      const p = g.node(n.id);
      return {
        id: n.id,
        type: "step",
        position: { x: (p?.x ?? 0) - NODE_W / 2, y: (p?.y ?? 0) - NODE_H / 2 },
        data: { node: n, selected: n.id === selected },
      };
    }),
    edges: edges.map((e) => {
      const key = keyIds.has(e.from) && keyIds.has(e.to);
      return {
        id: `${e.from}-${e.to}`,
        source: e.from,
        target: e.to,
        animated: e.rel === "branch",
        label: e.rel === "next" ? undefined : e.rel.replace("_", " "),
        style: {
          stroke: key ? "#f43f5e" : e.rel === "derived_from" ? "#a78bfa" : "#475569",
          strokeWidth: key ? 2.5 : 1.5,
          strokeDasharray: e.rel === "branch" ? "6 4" : undefined,
        },
        labelStyle: { fill: "#94a3b8", fontSize: 10 },
        labelBgStyle: { fill: "#0f172a" },
      };
    }),
  };
}
