import { Binary, FileSearch, Info, PackageOpen } from "lucide-react";
import { useWorkbench } from "../store/workbench";
import { DisasmView } from "./views/DisasmView";
import { HexView } from "./views/HexView";
import { ImportsView } from "./views/ImportsView";
import { InfoView } from "./views/InfoView";
import { StringsView } from "./views/StringsView";
import { XrefsView } from "./views/XrefsView";
import { Btn, PanelHeader } from "./ui";

/** Shows the result of the current (or revisited) step. */
export function CenterPanel() {
  const { view, act, graph, opened } = useWorkbench();
  const node = graph?.nodes.find((n) => n.id === view?.stepId);
  const entry = opened ? `0x${opened.info.entry.toString(16)}` : "0x0";
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <PanelHeader title={node ? `${view?.kind} · step #${node.seq} · ${node.label}` : "Workbench"}>
        <Btn onClick={() => act({ op: "info" })}>
          <Info size={13} /> Info
        </Btn>
        <Btn onClick={() => act({ op: "strings", filter: null, min_len: null })}>
          <FileSearch size={13} /> Strings
        </Btn>
        <Btn onClick={() => act({ op: "imports" })}>
          <PackageOpen size={13} /> Imports
        </Btn>
        <Btn onClick={() => act({ op: "disasm", target: entry })}>
          <Binary size={13} /> Entry
        </Btn>
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto">
        {!view && (
          <div className="p-6 text-dim">
            Pick a function, look at strings or imports, or type in the console. Every action
            becomes a node on the investigation canvas →
          </div>
        )}
        {view?.kind === "disasm" && <DisasmView result={view.result} />}
        {view?.kind === "xrefs" && <XrefsView result={view.result} />}
        {view?.kind === "strings" && <StringsView result={view.result} />}
        {view?.kind === "imports" && <ImportsView result={view.result} />}
        {view?.kind === "hex" && <HexView result={view.result} />}
        {view?.kind === "info" && <InfoView result={view.result} />}
      </div>
    </div>
  );
}
