import clsx from "clsx";
import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import { hex } from "../../lib/format";
import { functionOf, useWorkbench } from "../../store/workbench";
import { Pane } from "../ui/Pane";

const LIMIT = 2000;

/** Searchable function list. Opening one disassembles or decompiles it (a recorded step). */
export function FunctionList() {
  const functions = useWorkbench((s) => s.functions);
  const openFunction = useWorkbench((s) => s.openFunction);
  const current = useWorkbench((s) =>
    functionOf(s.views[s.activeTab === "decompile" ? "decompile" : "disasm"]),
  );
  const [q, setQ] = useState("");
  const shown = useMemo(() => {
    const n = q.toLowerCase();
    return n
      ? functions.filter((f) => f.name.toLowerCase().includes(n) || hex(f.addr).includes(n))
      : functions;
  }, [functions, q]);

  return (
    <Pane title="Functions" count={functions.length} scroll={false} className="flex-1 bg-panel">
      <div className="flex h-full flex-col">
        <label className="mx-2 my-2 flex h-7 items-center gap-1.5 rounded-md border bg-bg px-2 focus-within:border-brand/60">
          <Search size={12} className="text-faint" />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Filter functions"
            className="w-full bg-transparent text-sm outline-none placeholder:text-faint"
          />
        </label>
        <div className="scroll-host min-h-0 flex-1 overflow-auto pb-2 font-mono text-xs">
          {shown.slice(0, LIMIT).map((f) => {
            const stub = f.source === "import_stub";
            const active = current === f.addr;
            return (
              <button
                key={f.addr}
                onClick={() => openFunction(f.addr)}
                title={`${f.source} · ${f.size} bytes`}
                className={clsx(
                  "row ease relative w-full text-left hover:bg-hover",
                  active && "bg-active text-fg",
                )}
              >
                {active && <span className="absolute inset-y-0 left-0 w-[2px] bg-brand" />}
                <span className="nums w-14 shrink-0 text-asm-addr">{hex(f.addr)}</span>
                <span className={clsx("truncate", stub ? "text-muted" : "text-fg")}>
                  {stub ? f.name.replace("@plt", "") : f.name}
                  {stub && <span className="text-faint">@plt</span>}
                </span>
              </button>
            );
          })}
          {shown.length > LIMIT && (
            <div className="row text-faint">+{shown.length - LIMIT} more</div>
          )}
        </div>
      </div>
    </Pane>
  );
}
