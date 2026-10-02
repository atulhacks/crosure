import { Background, Controls, ReactFlow, ReactFlowProvider, useReactFlow } from "@xyflow/react";
import { useEffect, useMemo } from "react";
import { NODE_SIZE } from "../../lib/format";
import { layoutGraph } from "../../lib/layout";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";
import { StepNode } from "./StepNode";

const nodeTypes = { step: StepNode };

function Flow() {
  const graph = useWorkbench((s) => s.graph);
  const selected = useWorkbench((s) => s.selected);
  const branchFrom = useWorkbench((s) => s.branchFrom);
  const selectNode = useWorkbench((s) => s.selectNode);
  const setDockTab = useUi((s) => s.setDockTab);
  const { fitView, setCenter } = useReactFlow();
  const laid = useMemo(
    () => layoutGraph(graph?.nodes ?? [], graph?.edges ?? [], selected, branchFrom),
    [graph, selected, branchFrom],
  );
  const count = graph?.nodes.length ?? 0;
  const last = laid.nodes.at(-1)?.position;
  // Small graphs fit the view; larger ones follow the newest step at a readable zoom.
  useEffect(() => {
    const t = setTimeout(() => {
      if (count <= 7 || !last) fitView({ padding: 0.12, maxZoom: 1, duration: 200 });
      else setCenter(last.x + NODE_SIZE.w / 2, last.y - NODE_SIZE.h, { zoom: 0.85, duration: 250 });
    }, 30);
    return () => clearTimeout(t);
  }, [count, last?.x, last?.y, fitView, setCenter]); // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <ReactFlow
      nodes={laid.nodes}
      edges={laid.edges}
      nodeTypes={nodeTypes}
      onNodeClick={(_, n) => {
        selectNode(n.id);
        setDockTab("step");
      }}
      onPaneClick={() => selectNode(null)}
      nodesDraggable={false}
      nodesConnectable={false}
      minZoom={0.2}
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
