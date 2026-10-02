import clsx from "clsx";
import { FileSearch } from "lucide-react";
import { useUi } from "../../store/ui";
import { useWorkbench, type ViewKind } from "../../store/workbench";
import { Empty } from "../ui/Pane";
import { Segmented } from "../ui/Segmented";
import { CfgView } from "./views/CfgView";
import { DecompileView } from "./views/DecompileView";
import { DisasmView } from "./views/DisasmView";
import { HexView } from "./views/HexView";
import { ImportsView } from "./views/ImportsView";
import { InfoView } from "./views/InfoView";
import { StringsView } from "./views/StringsView";
import { XrefsView } from "./views/XrefsView";

const TABS: { kind: ViewKind; label: string }[] = [
  { kind: "info", label: "Overview" },
  { kind: "disasm", label: "Disassembly" },
  { kind: "decompile", label: "Decompiled" },
  { kind: "xrefs", label: "Xrefs" },
  { kind: "strings", label: "Strings" },
  { kind: "imports", label: "Imports" },
  { kind: "hex", label: "Hex" },
];

const HINT: Record<ViewKind, string> = {
  info: "",
  strings: "",
  imports: "",
  disasm: "Pick a function on the left, or press Ctrl+K.",
  decompile: "Pick a function on the left to see it as pseudo-C.",
  xrefs: "Click a string, import, or call target to see who references it.",
  hex: "Type hex <addr> in the console.",
};

/** Tabbed results. Each tab shows the latest step of its kind, linked back to that step. */
export function CenterPanel() {
  const { views, activeTab, openTab, graph, selectNode, decompiler } = useWorkbench();
  const { disasmMode, setDisasmMode } = useUi();
  const view = views[activeTab];
  const node = graph?.nodes.find((n) => n.id === view?.stepId);
  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col">
      <nav className="flex h-[var(--bar-h)] shrink-0 items-stretch gap-1 border-b px-2">
        {TABS.map((t) => (
          <button
            key={t.kind}
            onClick={() => openTab(t.kind)}
            className={clsx(
              "ease relative px-2.5 text-sm",
              activeTab === t.kind ? "text-fg" : "text-muted hover:text-fg",
            )}
          >
            {t.label}
            {activeTab === t.kind && (
              <span className="absolute inset-x-2 bottom-0 h-[2px] rounded-full bg-brand" />
            )}
          </button>
        ))}
        <div className="ml-auto flex items-center gap-2">
          {node && (
            <button
              onClick={() => selectNode(node.id)}
              title="Show this step on the investigation graph"
              className="ease rounded-md px-1.5 py-0.5 font-mono text-2xs text-faint hover:bg-hover hover:text-fg nums"
            >
              step #{node.seq}
            </button>
          )}
          {activeTab === "disasm" && view && (
            <Segmented
              value={disasmMode}
              onChange={setDisasmMode}
              options={[
                { value: "linear", label: "Linear" },
                { value: "graph", label: "Graph" },
              ]}
            />
          )}
        </div>
      </nav>
      <div
        className={clsx(
          "min-h-0 flex-1",
          !(activeTab === "disasm" && disasmMode === "graph") && "scroll-host overflow-auto",
        )}
      >
        {!view && activeTab === "decompile" && decompiler && !decompiler.available ? (
          <Empty
            icon={<FileSearch size={22} />}
            title="No decompiler found"
            hint={decompiler.hint ?? ""}
          />
        ) : (
          !view && (
            <Empty
              icon={<FileSearch size={22} />}
              title="Nothing here yet"
              hint={HINT[activeTab]}
            />
          )
        )}
        {view?.kind === "decompile" && <DecompileView result={view.result} />}
        {view?.kind === "disasm" &&
          (disasmMode === "graph" ? (
            <CfgView result={view.result} />
          ) : (
            <DisasmView result={view.result} />
          ))}
        {view?.kind === "xrefs" && <XrefsView result={view.result} />}
        {view?.kind === "strings" && <StringsView result={view.result} />}
        {view?.kind === "imports" && <ImportsView result={view.result} />}
        {view?.kind === "hex" && <HexView result={view.result} />}
        {view?.kind === "info" && <InfoView result={view.result} />}
      </div>
    </section>
  );
}
