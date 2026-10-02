import { useMemo, useState } from "react";
import { hex } from "../lib/format";
import { useWorkbench } from "../store/workbench";
import { PanelHeader } from "./ui";

const LIMIT = 2000;

/** Searchable function list; clicking one disassembles it (recorded). */
export function FunctionList() {
  const { functions, act, view } = useWorkbench();
  const [q, setQ] = useState("");
  const current =
    view?.kind === "disasm" ? (view.result as { function: { addr: number } }).function.addr : null;
  const shown = useMemo(() => {
    const needle = q.toLowerCase();
    return functions.filter(
      (f) => !needle || f.name.toLowerCase().includes(needle) || hex(f.addr).includes(needle),
    );
  }, [functions, q]);
  return (
    <div className="flex min-h-0 flex-col bg-panel">
      <PanelHeader title={`Functions · ${functions.length}`} />
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Filter…"
        className="m-2 rounded-md border border-line bg-bg px-2 py-1 text-xs outline-none focus:border-accent/60"
      />
      <div className="min-h-0 flex-1 overflow-auto font-mono text-xs">
        {shown.slice(0, LIMIT).map((f) => (
          <button
            key={f.addr}
            onClick={() => act({ op: "disasm", target: hex(f.addr) })}
            className={`flex w-full items-center gap-2 px-3 py-0.5 text-left hover:bg-panel-2 ${
              current === f.addr ? "bg-accent/10 text-accent" : ""
            }`}
            title={`${f.source} · ${f.size} bytes`}
          >
            <span className="w-16 shrink-0 text-dim">{hex(f.addr)}</span>
            <span className={`truncate ${f.source === "import_stub" ? "text-orange-300/80" : ""}`}>
              {f.name}
            </span>
          </button>
        ))}
        {shown.length > LIMIT && (
          <div className="px-3 py-1 text-dim">+{shown.length - LIMIT} more — filter to narrow</div>
        )}
      </div>
    </div>
  );
}
