import { Search } from "lucide-react";
import { useState } from "react";
import { hex } from "../../../lib/format";
import { useWorkbench } from "../../../store/workbench";
import type { StringRef } from "../../../types";
import { ColHead } from "./Table";

/** Strings. A search is its own recorded step; opening a string finds its xrefs (derived). */
export function StringsView({ result }: { result: unknown }) {
  const r = result as { strings: StringRef[]; truncated: boolean };
  const act = useWorkbench((s) => s.act);
  const [q, setQ] = useState("");
  return (
    <div>
      <form
        className="flex h-10 items-center gap-2 border-b px-3"
        onSubmit={(e) => {
          e.preventDefault();
          act({ op: "strings", filter: q || null, min_len: null });
        }}
      >
        <label className="flex h-7 flex-1 items-center gap-1.5 rounded-md border bg-bg px-2 focus-within:border-brand/60">
          <Search size={12} className="text-faint" />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search strings — Enter records the search"
            className="w-full bg-transparent text-sm outline-none placeholder:text-faint"
          />
        </label>
        <span className="nums text-xs text-muted">
          {r.strings.length}
          {r.truncated ? "+" : ""}
        </span>
      </form>
      <ColHead
        cols={[
          ["Address", "w-20"],
          ["Section", "w-20"],
          ["Value", ""],
        ]}
      />
      <div className="py-1 font-mono text-xs">
        {r.strings.map((s) => (
          <button
            key={`${s.addr}-${s.encoding}`}
            disabled={!s.mapped}
            onClick={() => act({ op: "xrefs_to", target: hex(s.addr) }, { fromView: true })}
            className="row ease w-full text-left hover:bg-hover disabled:opacity-50"
          >
            <span className="nums w-20 shrink-0 text-asm-addr">{hex(s.addr)}</span>
            <span className="w-20 shrink-0 truncate text-faint">{s.section ?? "—"}</span>
            <span className="truncate text-asm-string">{s.value}</span>
            {s.encoding !== "ascii" && <span className="text-2xs text-faint">{s.encoding}</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
