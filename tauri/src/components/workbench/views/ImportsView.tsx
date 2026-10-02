import { hex } from "../../../lib/format";
import { useWorkbench } from "../../../store/workbench";
import type { Import } from "../../../types";
import { ColHead } from "./Table";

/** Imports, grouped by library. Opening one finds its callers (derived). */
export function ImportsView({ result }: { result: unknown }) {
  const { imports } = result as { imports: Import[] };
  const act = useWorkbench((s) => s.act);
  const sorted = [...imports].sort(
    (a, b) => a.library.localeCompare(b.library) || a.name.localeCompare(b.name),
  );
  return (
    <div>
      <ColHead
        cols={[
          ["Address", "w-20"],
          ["Library", "w-36"],
          ["Name", ""],
        ]}
      />
      <div className="py-1 font-mono text-xs">
        {sorted.map((i) => (
          <button
            key={`${i.library}-${i.name}`}
            disabled={i.addr === null}
            onClick={() =>
              i.addr !== null && act({ op: "xrefs_to", target: hex(i.addr) }, { fromView: true })
            }
            className="row ease w-full text-left hover:bg-hover disabled:opacity-50"
          >
            <span className="nums w-20 shrink-0 text-asm-addr">
              {i.addr !== null ? hex(i.addr) : "—"}
            </span>
            <span className="w-36 shrink-0 truncate text-faint">{i.library || "—"}</span>
            <span className="truncate text-fg">{i.name}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
