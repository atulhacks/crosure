import { useEffect } from "react";
import { CommandPalette } from "./components/shell/CommandPalette";
import { AgentSettingsDialog } from "./components/agent/AgentSettingsDialog";
import { ErrorToast } from "./components/shell/ErrorToast";
import { StatusBar } from "./components/shell/StatusBar";
import { TitleBar } from "./components/shell/TitleBar";
import { Welcome } from "./components/shell/Welcome";
import { InvestigationPanel } from "./components/investigation/InvestigationPanel";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { Resizer } from "./components/ui/Resizer";
import { CenterPanel } from "./components/workbench/CenterPanel";
import { Console } from "./components/workbench/Console";
import { FunctionList } from "./components/workbench/FunctionList";
import { applyTheme, useUi } from "./store/ui";
import { useWorkbench } from "./store/workbench";

/** Workspace: functions | results + console | investigation. Every column resizes. */
function Workspace() {
  const { panes, setPane, consoleOpen } = useUi();
  return (
    <div className="flex min-h-0 flex-1">
      <div className="flex min-h-0 shrink-0 border-r" style={{ width: panes.left }}>
        <div className="flex min-w-0 flex-1 flex-col">
          <ErrorBoundary name="Functions">
            <FunctionList />
          </ErrorBoundary>
        </div>
      </div>
      <Resizer
        axis="x"
        start={panes.left}
        onResize={(px) => setPane("left", px)}
        onEnd={(px) => setPane("left", px, true)}
      />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <ErrorBoundary name="Results">
          <CenterPanel />
        </ErrorBoundary>
        {consoleOpen && (
          <>
            <Resizer
              axis="y"
              invert
              start={panes.console}
              onResize={(px) => setPane("console", px)}
              onEnd={(px) => setPane("console", px, true)}
            />
            <div className="shrink-0" style={{ height: panes.console }}>
              <ErrorBoundary name="Console">
                <Console />
              </ErrorBoundary>
            </div>
          </>
        )}
      </div>
      <Resizer
        axis="x"
        invert
        start={panes.right}
        onResize={(px) => setPane("right", px)}
        onEnd={(px) => setPane("right", px, true)}
      />
      <div className="flex min-h-0 shrink-0 border-l" style={{ width: panes.right }}>
        <div className="flex min-w-0 flex-1 flex-col">
          <ErrorBoundary name="Investigation">
            <InvestigationPanel />
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}

/** Root: title bar, workspace (or welcome), status bar, overlays. */
export default function App() {
  const opened = useWorkbench((s) => s.opened);
  const theme = useUi((s) => s.theme);
  const toggleConsole = useUi((s) => s.toggleConsole);
  const setDockTab = useUi((s) => s.setDockTab);
  useEffect(() => applyTheme(theme), [theme]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "`") {
        e.preventDefault();
        toggleConsole();
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") {
        e.preventDefault();
        setDockTab("agent");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggleConsole, setDockTab]);
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      {opened ? <Workspace /> : <Welcome />}
      <StatusBar />
      <ErrorBoundary name="Command palette">
        <CommandPalette />
      </ErrorBoundary>
      <ErrorBoundary name="AI settings">
        <AgentSettingsDialog />
      </ErrorBoundary>
      <ErrorToast />
    </div>
  );
}
