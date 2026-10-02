import dagre from "@dagrejs/dagre";
import type { Edge, Node } from "@xyflow/react";
import type { GraphEdge, GraphNode } from "../types";
import { NODE_SIZE } from "./format";

/** Data carried by an investigation-graph node. */
export interface StepData extends Record<string, unknown> {
  node: GraphNode;
  selected: boolean;
  branchFrom: boolean;
}

/**
 * Lays the investigation graph out top-to-bottom with dagre. The key path
 * (steps that led to a finding) is drawn in the brand colour; branches are
 * dashed; everything else stays quiet.
 */
export function layoutGraph(
  nodes: GraphNode[],
  edges: GraphEdge[],
  selected: string | null,
  branchFrom: string | null = null,
): { nodes: Node<StepData>[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 24, ranksep: 30 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const n of nodes) g.setNode(n.id, { width: NODE_SIZE.w, height: NODE_SIZE.h });
  for (const e of edges) g.setEdge(e.from, e.to);
  dagre.layout(g);
  const keyIds = new Set(nodes.filter((n) => n.on_key_path).map((n) => n.id));
  return {
    nodes: nodes.map((n) => {
      const p = g.node(n.id);
      return {
        id: n.id,
        type: "step",
        position: { x: (p?.x ?? 0) - NODE_SIZE.w / 2, y: (p?.y ?? 0) - NODE_SIZE.h / 2 },
        data: { node: n, selected: n.id === selected, branchFrom: n.id === branchFrom },
      };
    }),
    edges: edges.map((e) => {
      const key = keyIds.has(e.from) && keyIds.has(e.to);
      const branch = e.rel === "branch";
      return {
        id: `${e.from}-${e.to}`,
        source: e.from,
        target: e.to,
        type: "smoothstep",
        animated: false,
        label: branch ? "branch" : undefined,
        style: {
          stroke: key ? "var(--brand)" : e.rel === "next" ? "var(--edge)" : "var(--edge-strong)",
          strokeWidth: key ? 2 : 1.25,
          strokeDasharray: branch ? "5 4" : undefined,
        },
        labelStyle: { fill: "var(--muted)", fontSize: 10 },
        labelBgPadding: [4, 2] as [number, number],
      };
    }),
  };
}
