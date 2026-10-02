import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  ReactFlowProvider,
  type NodeProps,
} from "@xyflow/react";
import { useMemo } from "react";
import { BLOCK_W, layoutCfg, type BlockData } from "../../../lib/cfgLayout";
import { hex } from "../../../lib/format";
import { InsnText, type Follow } from "./asm";
import { useFollow, type DisasmResult } from "./DisasmView";

function makeBlockNode(follow: Follow) {
  return function BlockNode({ data }: NodeProps) {
    const { block, insns, entry } = data as BlockData;
    return (
      <div
        className="overflow-hidden rounded-md border bg-elevated font-mono text-2xs"
        style={{ width: BLOCK_W, borderColor: entry ? "var(--brand)" : undefined }}
      >
        <Handle
          type="target"
          position={Position.Top}
          className="!h-1 !w-1 !border-0 !bg-transparent"
        />
        <div className="flex h-[22px] items-center justify-between border-b px-2 text-faint nums">
          <span>loc_{block.addr.toString(16)}</span>
          <span>{block.count} insns</span>
        </div>
        <div className="py-1">
          {insns.map((i) => (
            <div key={i.addr} className="flex h-4 items-center gap-2 px-2 whitespace-nowrap">
              <span className="nums w-12 shrink-0 text-asm-addr">{hex(i.addr).slice(-6)}</span>
              <InsnText i={i} follow={follow} compact />
            </div>
          ))}
        </div>
        <Handle
          type="source"
          position={Position.Bottom}
          className="!h-1 !w-1 !border-0 !bg-transparent"
        />
      </div>
    );
  };
}

function Graph({ r }: { r: DisasmResult }) {
  const follow = useFollow();
  const nodeTypes = useMemo(() => ({ block: makeBlockNode(follow) }), [follow]);
  const laid = useMemo(() => layoutCfg(r.blocks ?? [], r.instructions), [r]);
  return (
    <ReactFlow
      nodes={laid.nodes}
      edges={laid.edges}
      nodeTypes={nodeTypes}
      fitView
      fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
      nodesDraggable={false}
      nodesConnectable={false}
      minZoom={0.15}
      proOptions={{ hideAttribution: true }}
    >
      <Background color="var(--border)" gap={20} size={1} />
      <Controls showInteractive={false} position="bottom-left" />
    </ReactFlow>
  );
}

/** Basic-block graph of the current function. Switching to it records nothing. */
export function CfgView({ result }: { result: unknown }) {
  const r = result as DisasmResult;
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center gap-3 border-b px-4 text-xs text-muted">
        <span className="font-mono font-semibold text-fg">{r.function.name}</span>
        <span className="nums">{r.blocks?.length ?? 0} blocks</span>
        <span className="ml-auto flex items-center gap-3 text-2xs">
          <span className="flex items-center gap-1">
            <i className="h-px w-3 bg-good" /> taken
          </span>
          <span className="flex items-center gap-1">
            <i className="h-px w-3 bg-bad" /> not taken
          </span>
        </span>
      </div>
      <div className="min-h-0 flex-1">
        <ReactFlowProvider key={r.function.addr}>
          <Graph r={r} />
        </ReactFlowProvider>
      </div>
    </div>
  );
}
