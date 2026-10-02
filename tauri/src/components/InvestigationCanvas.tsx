import { Background, Controls, ReactFlow, ReactFlowProvider, useReactFlow } from "@xyflow/react";
import { useEffect, useMemo } from "react";
import { duration } from "../lib/format";
import { layoutGraph } from "../lib/layout";
import { useWorkbench } from "../store/workbench";
import { StepNode } from "./StepNode";
import { PanelHeader } from "./ui";

const nodeTypes = { step: StepNode };

function Flow() {
  const { graph, selected, selectNode } = useWorkbench();
  const { fitView } = useReactFlow();
  const laid = useMemo(
    () => layoutGraph(graph?.nodes ?? [], graph?.edges ?? [], selected),
    [graph, selected],
  );
  const count = graph?.nodes.length ?? 0;
  useEffect(() => {
    const t = setTimeout(() => fitView({ padding: 0.15, maxZoom: 1, duration: 250 }), 30);
    return () => clearTimeout(t);
  }, [count, fitView]);
  return (
    <ReactFlow
      nodes={laid.nodes}
      edges={laid.edges}
      nodeTypes={nodeTypes}
      onNodeClick={(_, n) => selectNode(n.id)}
      onPaneClick={() => selectNode(null)}
      nodesDraggable={false}
      nodesConnectable={false}
      minZoom={0.2}
      proOptions={{ hideAttribution: true }}
      colorMode="dark"
    >
      <Background color="#1e2533" gap={18} />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}

/** The live investigation graph: every recorded step, how it connects, and the key path. */
export function InvestigationCanvas() {
  const stats = useWorkbench((s) => s.graph?.stats);
  return (
    <div className="flex min-h-0 flex-[3] flex-col">
      <PanelHeader title="Investigation">
        {stats && (
          <span className="text-[11px] whitespace-nowrap text-dim">
            {stats.steps} steps · {stats.branches} branches · {stats.dead_ends} dead ends ·{" "}
            {stats.findings} findings · {duration(stats.duration_ms)}
          </span>
        )}
      </PanelHeader>
      <div className="min-h-0 flex-1 bg-bg">
        <ReactFlowProvider>
          <Flow />
        </ReactFlowProvider>
      </div>
    </div>
  );
}
