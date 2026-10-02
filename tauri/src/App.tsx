import { CenterPanel } from "./components/CenterPanel";
import { Console } from "./components/Console";
import { ErrorToast } from "./components/ErrorToast";
import { FunctionList } from "./components/FunctionList";
import { InvestigationCanvas } from "./components/InvestigationCanvas";
import { NodeInspector } from "./components/NodeInspector";
import { Timeline } from "./components/Timeline";
import { TopBar } from "./components/TopBar";
import { Welcome } from "./components/Welcome";
import { useWorkbench } from "./store/workbench";

/** Root layout: functions | view + console | investigation canvas + inspector. */
export default function App() {
  const opened = useWorkbench((s) => s.opened);
  return (
    <div className="flex h-full flex-col">
      <TopBar />
      {opened ? (
        <>
          <div className="grid min-h-0 flex-1 grid-cols-[250px_minmax(0,1fr)_460px]">
            <FunctionList />
            <div className="flex min-h-0 flex-col border-x border-line">
              <CenterPanel />
              <Console />
            </div>
            <div className="flex min-h-0 flex-col">
              <InvestigationCanvas />
              <NodeInspector />
            </div>
          </div>
          <Timeline />
        </>
      ) : (
        <Welcome />
      )}
      <ErrorToast />
    </div>
  );
}
