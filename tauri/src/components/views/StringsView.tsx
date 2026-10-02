import { useState } from "react";
import { hex } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import type { StringRef } from "../../types";

/** Strings; filter is its own recorded step, clicking a string finds its xrefs (derived). */
export function StringsView({ result }: { result: unknown }) {
  const r = result as { strings: StringRef[]; truncated: boolean };
  const act = useWorkbench((s) => s.act);
  const [q, setQ] = useState("");
  return (
    <div className="text-xs">
      <form
        className="flex items-center gap-2 border-b border-line px-3 py-2"
        onSubmit={(e) => {
          e.preventDefault();
          act({ op: "strings", filter: q || null, min_len: null });
        }}
      >
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search strings (recorded)…"
          className="flex-1 rounded-md border border-line bg-bg px-2 py-1 outline-none focus:border-accent/60"
        />
        <span className="text-dim">
          {r.strings.length}
          {r.truncated ? "+" : ""} strings
        </span>
      </form>
      {r.strings.map((s) => (
        <button
          key={`${s.addr}-${s.encoding}`}
          disabled={!s.mapped}
          onClick={() => act({ op: "xrefs_to", target: hex(s.addr) }, { fromView: true })}
          className="flex w-full gap-3 px-3 py-0.5 text-left font-mono hover:bg-panel-2 disabled:opacity-50"
        >
          <span className="w-20 shrink-0 text-dim">{hex(s.addr)}</span>
          <span className="w-16 shrink-0 text-dim">{s.section ?? "-"}</span>
          <span className="truncate text-yellow-200/90">{s.value}</span>
        </button>
      ))}
    </div>
  );
}
