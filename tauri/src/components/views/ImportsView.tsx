import { hex } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import type { Import } from "../../types";

/** Imports grouped by library; clicking one finds its callers (derived). */
export function ImportsView({ result }: { result: unknown }) {
  const { imports } = result as { imports: Import[] };
  const act = useWorkbench((s) => s.act);
  return (
    <div className="text-xs">
      {imports.map((i) => (
        <button
          key={`${i.library}-${i.name}`}
          disabled={i.addr === null}
          onClick={() =>
            i.addr !== null && act({ op: "xrefs_to", target: hex(i.addr) }, { fromView: true })
          }
          className="flex w-full gap-3 px-3 py-0.5 text-left font-mono hover:bg-panel-2 disabled:opacity-50"
        >
          <span className="w-20 shrink-0 text-dim">{i.addr !== null ? hex(i.addr) : "-"}</span>
          <span className="w-40 shrink-0 truncate text-orange-300/80">{i.library || "?"}</span>
          <span className="truncate">{i.name}</span>
        </button>
      ))}
    </div>
  );
}
