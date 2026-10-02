import { hex } from "../../lib/format";
import { hexRows } from "../../lib/hexdump";

/** Hex dump of a byte range. */
export function HexView({ result }: { result: unknown }) {
  const r = result as { addr: number; hex: string };
  return (
    <div className="p-3 font-mono text-xs leading-5">
      {hexRows(r.addr, r.hex).map((row) => (
        <div key={row.addr} className="flex gap-4">
          <span className="w-20 text-dim">{hex(row.addr)}</span>
          <span className="w-[26rem]">{row.hex}</span>
          <span className="text-yellow-200/80">{row.ascii}</span>
        </div>
      ))}
    </div>
  );
}
