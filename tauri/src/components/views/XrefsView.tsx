import { hex } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import type { Xref } from "../../types";

/** Cross-references; clicking one disassembles the referencing function (derived from this step). */
export function XrefsView({ result }: { result: unknown }) {
  const r = result as { addr: number; name?: string; refs: Xref[]; names?: string[] };
  const act = useWorkbench((s) => s.act);
  const outgoing = Array.isArray(r.names);
  return (
    <div className="text-xs">
      <div className="border-b border-line px-3 py-2 text-sm">
        {r.refs.length} {outgoing ? "references from" : "references to"}{" "}
        <span className="font-mono text-accent">{r.name ?? hex(r.addr)}</span>
      </div>
      {r.refs.map((x, idx) => {
        const goto = outgoing ? x.to : x.from;
        return (
          <button
            key={`${x.from}-${x.to}-${idx}`}
            onClick={() => act({ op: "disasm", target: hex(goto) }, { fromView: true })}
            className="flex w-full gap-3 px-3 py-1 text-left font-mono hover:bg-panel-2"
          >
            <span className="w-20 text-dim">{hex(x.from)}</span>
            <span className="w-12 text-accent-2">{x.kind}</span>
            <span className="truncate">{outgoing ? r.names?.[idx] : (x.from_func ?? "?")}</span>
          </button>
        );
      })}
    </div>
  );
}
