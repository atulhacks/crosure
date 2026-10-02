import { hex } from "../../../lib/format";
import { useWorkbench } from "../../../store/workbench";
import type { Xref } from "../../../types";
import { ColHead, ResultTitle } from "./Table";

const KIND_CLASS = {
  call: "text-asm-call",
  jump: "text-asm-jump",
  data: "text-asm-string",
} as const;

/** Cross-references. Opening one disassembles that function (derived from this step). */
export function XrefsView({ result }: { result: unknown }) {
  const r = result as { addr: number; name?: string; refs: Xref[]; names?: string[] };
  const act = useWorkbench((s) => s.act);
  const outgoing = Array.isArray(r.names);
  return (
    <div>
      <ResultTitle>
        <span className="nums text-fg">{r.refs.length}</span>{" "}
        {outgoing ? "references from" : "references to"}
        <span className="font-mono text-fg">{r.name ?? hex(r.addr)}</span>
      </ResultTitle>
      <ColHead
        cols={[
          ["From", "w-20"],
          ["Kind", "w-12"],
          [outgoing ? "Target" : "Function", ""],
        ]}
      />
      <div className="py-1 font-mono text-xs">
        {r.refs.map((x, i) => {
          const goto = outgoing ? x.to : x.from;
          return (
            <button
              key={`${x.from}-${x.to}-${i}`}
              onClick={() => act({ op: "disasm", target: hex(goto) }, { fromView: true })}
              className="row ease w-full text-left hover:bg-hover"
            >
              <span className="nums w-20 text-asm-addr">{hex(x.from)}</span>
              <span className={`w-12 ${KIND_CLASS[x.kind]}`}>{x.kind}</span>
              <span className="truncate text-fg">
                {outgoing ? r.names?.[i] : (x.from_func ?? "?")}
              </span>
            </button>
          );
        })}
        {r.refs.length === 0 && (
          <div className="px-4 py-3 text-sm text-faint">
            No code references. It may be reached through a data pointer.
          </div>
        )}
      </div>
    </div>
  );
}
