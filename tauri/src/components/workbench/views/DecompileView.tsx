import clsx from "clsx";
import { hex } from "../../../lib/format";
import { tokenizeC, type CToken } from "../../../lib/ctokens";
import { useWorkbench } from "../../../store/workbench";
import { FunctionHeader, type DisasmResult } from "./DisasmView";

/** Result of a `decompile` step. */
export interface DecompileResult {
  function: DisasmResult["function"];
  backend: string;
  lines: { text: string; addr: number | null }[];
}

const COLOR: Record<CToken["kind"], string> = {
  keyword: "text-asm-jump",
  type: "text-asm-mnemonic",
  number: "text-asm-symbol",
  string: "text-asm-string",
  comment: "text-faint italic",
  call: "text-asm-call",
  ident: "text-asm-operand",
  plain: "text-asm-operand",
};

/** Pseudo-C with line addresses; calls to known functions open them (recorded). */
export function DecompileView({ result }: { result: unknown }) {
  const r = result as DecompileResult;
  const functions = useWorkbench((s) => s.functions);
  const openFunction = useWorkbench((s) => s.openFunction);
  const byName = new Map(functions.map((f) => [f.name, f.addr]));
  const code = r.lines.filter((l) => l.text.trim()).length;
  return (
    <div>
      <FunctionHeader fn={r.function} count={code} unit="lines" />
      <div className="py-2 font-mono text-xs">
        {r.lines.map((l, i) => (
          <div key={i} className="row ease hover:bg-hover">
            <span className="nums w-8 shrink-0 text-right text-faint select-none">{i + 1}</span>
            <span className="nums w-20 shrink-0 pl-3 text-asm-addr select-none">
              {l.addr !== null ? hex(l.addr) : ""}
            </span>
            <span className="whitespace-pre">
              {tokenizeC(l.text).map((t, j) => {
                const target = t.kind === "call" ? byName.get(t.text) : undefined;
                return target !== undefined && target !== r.function.addr ? (
                  <button
                    key={j}
                    onClick={() => openFunction(target, true)}
                    title={`Decompile ${t.text}`}
                    className="text-asm-call underline decoration-line-strong underline-offset-2 hover:text-brand hover:decoration-brand"
                  >
                    {t.text}
                  </button>
                ) : (
                  <span key={j} className={clsx(COLOR[t.kind])}>
                    {t.text}
                  </span>
                );
              })}
            </span>
          </div>
        ))}
      </div>
      <div className="px-4 pb-3 text-2xs text-faint">{r.backend}</div>
    </div>
  );
}
