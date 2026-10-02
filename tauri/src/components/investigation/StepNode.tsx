import { Handle, Position, type NodeProps } from "@xyflow/react";
import clsx from "clsx";
import { Bot, GitBranch } from "lucide-react";
import { KIND_COLOR, NODE_SIZE } from "../../lib/format";
import type { StepData } from "../../lib/layout";
import { Pill } from "../ui/Pill";

/** One recorded step on the investigation graph. */
export function StepNode({ data }: NodeProps) {
  const { node, selected, branchFrom } = data as StepData;
  const dead = node.tags.includes("dead_end");
  const key = node.tags.includes("key_step") || node.kind === "finding" || node.kind === "verdict";
  const title = node.label.startsWith(node.kind)
    ? node.label.slice(node.kind.length).trim()
    : node.label;
  return (
    <div
      className={clsx(
        "ease rounded-md border bg-elevated px-2.5 py-1.5",
        dead && "opacity-45",
        selected
          ? "border-brand shadow-[0_0_0_1px_var(--brand)]"
          : node.on_key_path
            ? "border-brand/35"
            : "border-line-strong",
      )}
      style={{ width: NODE_SIZE.w, minHeight: NODE_SIZE.h }}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!h-1 !w-1 !border-0 !bg-transparent"
      />
      <div className="flex items-center gap-1.5 text-2xs">
        <span
          className="h-1.5 w-1.5 shrink-0 rounded-full"
          style={{ background: KIND_COLOR[node.kind] }}
        />
        <span className="font-semibold tracking-wider text-muted uppercase">{node.kind}</span>
        <span className="font-mono text-faint nums">#{node.seq}</span>
        {node.actor === "agent" && <Bot size={11} className="text-[var(--kind-agent)]" />}
        {branchFrom && <GitBranch size={11} className="ml-auto text-brand" />}
        {key && !branchFrom && (
          <span className="ml-auto h-1.5 w-1.5 rounded-full bg-brand" title="Key step" />
        )}
      </div>
      <div className="mt-0.5 truncate font-mono text-xs text-fg">{title || node.summary}</div>
      <div className="truncate text-2xs text-muted">{node.summary}</div>
      {(node.intent?.chip || node.tags.length > 0) && (
        <div className="mt-1 flex flex-wrap gap-1">
          {node.intent?.chip && <Pill tone="brand">{node.intent.chip.replace(/_/g, " ")}</Pill>}
          {node.tags
            .filter((t) => t !== "key_step")
            .map((t) => (
              <Pill key={t} tone={t === "dead_end" ? "muted" : "warn"}>
                {t.replace("_", " ")}
              </Pill>
            ))}
        </div>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!h-1 !w-1 !border-0 !bg-transparent"
      />
    </div>
  );
}
