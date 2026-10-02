import { useMemo, useState } from "react";
import { hex } from "../../../lib/format";
import { useWorkbench } from "../../../store/workbench";
import type { BasicBlock, Instruction } from "../../../types";
import { Button } from "../../ui/Button";
import { InsnText, type Follow } from "./asm";

/** Result of a `disasm` step. */
export interface DisasmResult {
  function: { addr: number; name: string; size: number };
  instructions: Instruction[];
  comments: [number, string][];
  blocks?: BasicBlock[];
}

/** Follow callbacks wired to recorded ops (derived from the current view). */
export function useFollow(): Follow {
  const act = useWorkbench((s) => s.act);
  const functions = useWorkbench((s) => s.functions);
  return useMemo(() => {
    const fns = new Set(functions.map((f) => f.addr));
    return {
      fn: (a) => act({ op: "disasm", target: hex(a) }, { fromView: true }),
      xrefs: (a) => act({ op: "xrefs_to", target: hex(a) }, { fromView: true }),
      isFunction: (a) => fns.has(a),
    };
  }, [act, functions]);
}

/** Function header: name (renameable), size, and its actions. */
export function FunctionHeader({ fn, count }: { fn: DisasmResult["function"]; count: number }) {
  const act = useWorkbench((s) => s.act);
  const [renaming, setRenaming] = useState<string | null>(null);
  return (
    <div className="flex h-10 items-center gap-3 border-b px-4">
      {renaming === null ? (
        <button
          onDoubleClick={() => setRenaming(fn.name)}
          title="Double-click to rename"
          className="font-mono text-base font-semibold text-fg"
        >
          {fn.name}
        </button>
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
            className="h-7 rounded-md border border-brand/60 bg-bg px-2 font-mono text-base outline-none"
          />
        </form>
      )}
      <span className="font-mono text-xs text-muted nums">
        {hex(fn.addr)} · {fn.size} bytes · {count} insns
      </span>
      <div className="ml-auto flex gap-1">
        <Button variant="ghost" onClick={() => setRenaming(fn.name)}>
          Rename
        </Button>
        <Button
          variant="ghost"
          onClick={() => act({ op: "xrefs_to", target: hex(fn.addr) }, { fromView: true })}
        >
          Callers
        </Button>
        <Button
          variant="ghost"
          onClick={() => act({ op: "xrefs_from", target: hex(fn.addr) }, { fromView: true })}
        >
          Callees
        </Button>
      </div>
    </div>
  );
}

/** Linear listing with block labels, aligned columns and followable references. */
export function DisasmView({ result }: { result: unknown }) {
  const r = result as DisasmResult;
  const follow = useFollow();
  const comments = new Map(r.comments ?? []);
  const leaders = new Set((r.blocks ?? []).slice(1).map((b) => b.addr));
  return (
    <div>
      <FunctionHeader fn={r.function} count={r.instructions.length} />
      <div className="py-2 font-mono text-xs">
        {r.instructions.map((i) => (
          <div key={i.addr}>
            {leaders.has(i.addr) && (
              <div className="row mt-1.5 text-faint">loc_{i.addr.toString(16)}:</div>
            )}
            <div className="row ease hover:bg-hover">
              <span className="nums w-16 shrink-0 text-asm-addr">{hex(i.addr)}</span>
              <span className="w-32 shrink-0 truncate text-asm-bytes">
                {i.bytes.match(/../g)?.join(" ")}
              </span>
              <InsnText i={i} follow={follow} />
              {comments.has(i.addr) && (
                <span className="shrink-0 text-asm-symbol">; {comments.get(i.addr)}</span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
