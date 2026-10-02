import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Bot } from "lucide-react";
import { KIND_COLOR, NODE_SIZE } from "../lib/format";
import type { GraphNode } from "../types";
import { Pill } from "./ui";

/** One investigation step on the canvas. */
export function StepNode({ data }: NodeProps) {
  const { node, selected } = data as { node: GraphNode; selected: boolean };
  const color = KIND_COLOR[node.kind];
  const dead = node.tags.includes("dead_end");
  const edge = selected ? "#22d3ee" : node.on_key_path ? "#f43f5e88" : "#232a37";
  return (
    <div
      className={`rounded-md border bg-panel-2 px-2.5 py-1.5 text-[11px] shadow ${dead ? "opacity-50" : ""}`}
      style={{
        width: NODE_SIZE.w,
        minHeight: NODE_SIZE.h,
        borderTopColor: edge,
        borderRightColor: edge,
        borderBottomColor: edge,
        borderLeftWidth: 3,
        borderLeftColor: color,
        boxShadow: selected ? "0 0 0 1px #22d3ee" : undefined,
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!h-1.5 !w-1.5 !border-0 !bg-slate-500"
      />
      <div className="flex items-center gap-1.5">
        <span className="font-mono text-[10px] text-dim">#{node.seq}</span>
        <span className="font-semibold uppercase" style={{ color }}>
          {node.kind}
        </span>
        {node.actor === "agent" && <Bot size={11} className="text-fuchsia-400" />}
        <span className="truncate font-mono text-fg">
          {node.label.replace(`${node.kind} `, "")}
        </span>
      </div>
      <div className="mt-0.5 line-clamp-2 text-dim">{node.summary}</div>
      {(node.tags.length > 0 || node.intent?.chip) && (
        <div className="mt-1 flex flex-wrap gap-1">
          {node.intent?.chip && <Pill color="#a78bfa">{node.intent.chip.replace(/_/g, " ")}</Pill>}
          {node.tags.map((t) => (
            <Pill
              key={t}
              color={t === "key_step" ? "#f43f5e" : t === "dead_end" ? "#64748b" : "#facc15"}
            >
              {t.replace("_", " ")}
            </Pill>
          ))}
        </div>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!h-1.5 !w-1.5 !border-0 !bg-slate-500"
      />
    </div>
  );
}
