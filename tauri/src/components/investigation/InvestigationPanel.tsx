import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";
import { Resizer } from "../ui/Resizer";
import { Segmented } from "../ui/Segmented";
import { FlightLog } from "./FlightLog";
import { Inspector } from "./Inspector";
import { InvestigationCanvas } from "./InvestigationCanvas";
import { ReplayBar } from "./ReplayBar";

/** Right column: the recorded investigation (graph or log), replay, and the step inspector. */
export function InvestigationPanel() {
  const stats = useWorkbench((s) => s.graph?.stats);
  const { invMode, setInvMode, panes, setPane } = useUi();
  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col bg-panel">
      <header className="flex h-[var(--bar-h)] shrink-0 items-center gap-2 border-b px-3">
        <span className="label">Investigation</span>
        {stats && (
          <span className="truncate text-2xs text-faint nums">
            {stats.steps} steps · {stats.branches} br · {stats.dead_ends} dead · {stats.findings}{" "}
            found
          </span>
        )}
        <div className="ml-auto">
          <Segmented
            value={invMode}
            onChange={setInvMode}
            options={[
              { value: "graph", label: "Graph" },
              { value: "log", label: "Log" },
            ]}
          />
        </div>
      </header>
      <div className="min-h-0 flex-1 bg-bg">
        {invMode === "graph" ? <InvestigationCanvas /> : <FlightLog />}
      </div>
      <ReplayBar />
      <Resizer
        axis="y"
        invert
        start={panes.inspector}
        onResize={(px) => setPane("inspector", px)}
        onEnd={(px) => setPane("inspector", px, true)}
      />
      <div className="shrink-0 border-t" style={{ height: panes.inspector }}>
        <Inspector />
      </div>
    </section>
  );
}
