import clsx from "clsx";
import { hex } from "../../../lib/format";
import type { Instruction } from "../../../types";

/** Mnemonic colour class: calls, jumps, everything else. */
export function mnemonicClass(m: string): string {
  if (m.includes("call") || m === "bl" || m === "blx") return "text-asm-call";
  if (m.startsWith("j") || m.startsWith("b.") || m === "b" || m === "ret") return "text-asm-jump";
  return "text-asm-mnemonic";
}

/** Callbacks for following references out of an instruction. */
export interface Follow {
  /** Disassemble a function at `addr` (derived from the current view). */
  fn: (addr: number) => void;
  /** Show cross-references to `addr`. */
  xrefs: (addr: number) => void;
  isFunction: (addr: number) => boolean;
}

/** Mnemonic, operands and resolved comment for one instruction. */
export function InsnText({
  i,
  follow,
  compact = false,
}: {
  i: Instruction;
  follow: Follow;
  compact?: boolean;
}) {
  const target = i.target;
  const isFn = target !== null && follow.isFunction(target);
  const isString = i.comment?.startsWith('"') ?? false;
  return (
    <>
      <span className={clsx("shrink-0", compact ? "w-12" : "w-16", mnemonicClass(i.mnemonic))}>
        {i.mnemonic}
      </span>
      {isFn ? (
        <button
          onClick={() => follow.fn(target as number)}
          className="truncate text-left text-asm-operand underline decoration-line-strong underline-offset-2 hover:text-brand hover:decoration-brand"
        >
          {i.operands}
        </button>
      ) : (
        <span className="truncate text-asm-operand">{i.operands}</span>
      )}
      {i.comment && target !== null && (
        <button
          onClick={() => follow.xrefs(target)}
          title={`Xrefs to ${hex(target)}`}
          className={clsx(
            "shrink-0 truncate text-left hover:underline",
            isString ? "text-asm-string" : "text-asm-symbol",
            compact ? "max-w-40" : "max-w-72",
          )}
        >
          ; {i.comment}
        </button>
      )}
    </>
  );
}
