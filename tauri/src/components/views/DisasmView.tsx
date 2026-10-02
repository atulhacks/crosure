import { useState } from "react";
import { hex } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import type { Instruction } from "../../types";
import { Btn } from "../ui";

interface DisasmResult {
  function: { addr: number; name: string; size: number };
  instructions: Instruction[];
  comments: [number, string][];
}

/** Function disassembly. Clicking a call target or string follows it (recorded, derived from this step). */
export function DisasmView({ result }: { result: unknown }) {
  const r = result as DisasmResult;
  const { act, functions } = useWorkbench();
  const [renaming, setRenaming] = useState<string | null>(null);
  const isFunc = new Set(functions.map((f) => f.addr));
  const comments = new Map(r.comments ?? []);
  const fn = r.function;
  return (
    <div className="font-mono text-xs">
      <div className="sticky top-0 z-10 flex items-center gap-2 border-b border-line bg-bg/95 px-3 py-2 backdrop-blur">
        {renaming === null ? (
          <span className="text-sm font-semibold text-accent">{fn.name}</span>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (renaming.trim())
                act(
                  { op: "rename", target: hex(fn.addr), name: renaming.trim() },
                  { fromView: true },
                );
              setRenaming(null);
            }}
          >
            <input
              autoFocus
              value={renaming}
              onChange={(e) => setRenaming(e.target.value)}
              onBlur={() => setRenaming(null)}
              className="rounded border border-accent/60 bg-bg px-1.5 py-0.5 text-sm outline-none"
            />
          </form>
        )}
        <span className="text-dim">
          {hex(fn.addr)} · {fn.size} bytes · {r.instructions.length} insns
        </span>
        <div className="ml-auto flex gap-1.5 font-sans">
          <Btn onClick={() => setRenaming(fn.name)}>Rename</Btn>
          <Btn onClick={() => act({ op: "xrefs_to", target: hex(fn.addr) }, { fromView: true })}>
            Xrefs to
          </Btn>
          <Btn onClick={() => act({ op: "xrefs_from", target: hex(fn.addr) }, { fromView: true })}>
            Calls from
          </Btn>
        </div>
      </div>
      {r.instructions.map((i) => {
        const follow = i.target !== null && isFunc.has(i.target);
        const isString = i.comment?.startsWith('"');
        return (
          <div key={i.addr} className="group flex gap-3 px-3 leading-5 hover:bg-panel-2">
            <span className="w-20 shrink-0 text-dim">{hex(i.addr)}</span>
            <span className="w-28 shrink-0 truncate text-slate-600">{i.bytes}</span>
            <span
              className={`w-14 shrink-0 ${i.mnemonic.includes("call") ? "text-accent-2" : i.mnemonic.startsWith("j") ? "text-amber-300" : "text-sky-300"}`}
            >
              {i.mnemonic}
            </span>
            {follow ? (
              <button
                className="truncate text-left underline decoration-dotted hover:text-accent"
                onClick={() =>
                  act({ op: "disasm", target: hex(i.target as number) }, { fromView: true })
                }
              >
                {i.operands}
              </button>
            ) : (
              <span className="truncate">{i.operands}</span>
            )}
            {i.comment && (
              <button
                className={`ml-auto shrink-0 truncate text-left ${isString ? "text-yellow-300/90" : "text-good/90"} hover:underline`}
                title={isString ? "Xrefs to this string" : "Xrefs to this target"}
                onClick={() =>
                  i.target !== null &&
                  act({ op: "xrefs_to", target: hex(i.target) }, { fromView: true })
                }
              >
                ; {i.comment}
              </button>
            )}
            {comments.has(i.addr) && (
              <span className="shrink-0 text-good">; {comments.get(i.addr)}</span>
            )}
          </div>
        );
      })}
    </div>
  );
}
