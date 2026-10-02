import { hex } from "../../../lib/format";
import { hexRows } from "../../../lib/hexdump";
import { ColHead } from "./Table";

/** Hex dump of a byte range. */
export function HexView({ result }: { result: unknown }) {
  const r = result as { addr: number; hex: string };
  return (
    <div>
      <ColHead
        cols={[
          ["Offset", "w-20"],
          ["Bytes", "w-[25rem]"],
          ["ASCII", ""],
        ]}
      />
      <div className="py-1 font-mono text-xs">
        {hexRows(r.addr, r.hex).map((row) => (
          <div key={row.addr} className="row">
            <span className="nums w-20 text-asm-addr">{hex(row.addr)}</span>
            <span className="w-[25rem] text-asm-operand">{row.hex}</span>
            <span className="text-asm-string">{row.ascii}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
