import { Background, Controls, ReactFlow, ReactFlowProvider, useReactFlow } from "@xyflow/react";
import { useEffect, useMemo } from "react";
import { layoutGraph } from "../../lib/layout";
import { useWorkbench } from "../../store/workbench";
import { StepNode } from "./StepNode";

const nodeTypes = { step: StepNode };

function Flow() {
  const graph = useWorkbench((s) => s.graph);
  const selected = useWorkbench((s) => s.selected);
  const branchFrom = useWorkbench((s) => s.branchFrom);
  const selectNode = useWorkbench((s) => s.selectNode);
  const { fitView } = useReactFlow();
  const laid = useMemo(
    () => layoutGraph(graph?.nodes ?? [], graph?.edges ?? [], selected, branchFrom),
    [graph, selected, branchFrom],
  );
  const count = graph?.nodes.length ?? 0;
  useEffect(() => {
    const t = setTimeout(() => fitView({ padding: 0.12, maxZoom: 1, duration: 200 }), 30);
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
    >
      <Background color="var(--border)" gap={18} size={1} />
      <Controls showInteractive={false} position="bottom-left" />
    </ReactFlow>
  );
}

/** The investigation as a graph: every step, how it connects, and the path to each finding. */
export function InvestigationCanvas() {
  return (
    <ReactFlowProvider>
      <Flow />
    </ReactFlowProvider>
  );
}
