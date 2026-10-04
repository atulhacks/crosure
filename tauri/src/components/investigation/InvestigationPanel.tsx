import clsx from "clsx";
import { Sparkles } from "lucide-react";
import { useAgent } from "../../store/agent";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";
import { AgentPanel } from "../agent/AgentPanel";
import { ErrorBoundary } from "../ui/ErrorBoundary";
import { Resizer } from "../ui/Resizer";
import { Segmented } from "../ui/Segmented";
import { FlightLog } from "./FlightLog";
import { Inspector } from "./Inspector";
import { InvestigationCanvas } from "./InvestigationCanvas";
import { Timeline } from "./Timeline";

/** Right column: the recorded investigation (graph or log) and timeline over a dock with the AI agent and the step inspector. */
export function InvestigationPanel() {
  const stats = useWorkbench((s) => s.graph?.stats);
  const running = useAgent((s) => s.running);
  const { invMode, setInvMode, panes, setPane, dockTab, setDockTab } = useUi();
  const tab = (id: "agent" | "step", label: React.ReactNode) => (
    <button
      onClick={() => setDockTab(id)}
      className={clsx(
        "ease relative flex h-full items-center gap-1.5 px-2 text-sm",
        dockTab === id ? "text-fg" : "text-muted hover:text-fg",
      )}
    >
      {label}
      {dockTab === id && (
        <span className="absolute inset-x-1.5 bottom-0 h-[2px] rounded-full bg-brand" />
      )}
    </button>
  );
  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col bg-panel">
      <header className="flex h-[var(--bar-h)] shrink-0 items-center gap-2 border-b px-3">
        <span className="label">Investigation</span>
        {stats && (
          <span className="truncate text-2xs text-faint nums">
            {stats.steps} steps · {stats.agent_steps} by AI · {stats.findings} findings
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
        <ErrorBoundary key={invMode} name="Graph">
          {invMode === "graph" ? <InvestigationCanvas /> : <FlightLog />}
        </ErrorBoundary>
      </div>
      <Timeline />
      <Resizer
        axis="y"
        invert
        start={panes.inspector}
        onResize={(px) => setPane("inspector", px)}
        onEnd={(px) => setPane("inspector", px, true)}
      />
      <div className="flex shrink-0 flex-col border-t" style={{ height: panes.inspector }}>
        <nav className="flex h-8 shrink-0 items-stretch gap-1 border-b px-1.5">
          {tab(
            "agent",
            <>
              <Sparkles size={12} className={running ? "text-brand" : ""} /> Agent
              {running && <span className="rec-dot h-1.5 w-1.5 rounded-full bg-brand" />}
            </>,
          )}
          {tab("step", "Step")}
        </nav>
        <div className="min-h-0 flex-1">
          <ErrorBoundary key={dockTab} name={dockTab === "agent" ? "Agent" : "Step"}>
            {dockTab === "agent" ? <AgentPanel /> : <Inspector />}
          </ErrorBoundary>
        </div>
      </div>
    </section>
  );
}
