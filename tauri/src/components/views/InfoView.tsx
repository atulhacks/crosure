import { hex } from "../../lib/format";
import type { BinaryInfo } from "../../types";

/** Binary facts and section table. */
export function InfoView({ result }: { result: unknown }) {
  const info = result as BinaryInfo;
  const rows: [string, string][] = [
    ["Path", info.path],
    [
      "Format",
      `${info.format.toUpperCase()} · ${info.arch} · ${info.bits}-bit · ${info.little_endian ? "LE" : "BE"}`,
    ],
    ["Entry", hex(info.entry)],
    ["Size", `${info.size.toLocaleString()} bytes`],
    ["SHA-256", info.sha256.replace("sha256:", "")],
    ["Symbols", info.stripped ? "stripped" : "present"],
  ];
  return (
    <div className="p-4 text-xs">
      <table className="mb-4">
        <tbody>
          {rows.map(([k, v]) => (
            <tr key={k}>
              <td className="pr-6 pb-1 text-dim">{k}</td>
              <td className="font-mono break-all">{v}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <table className="w-full font-mono">
        <thead className="text-left text-dim">
          <tr>
            <th>Section</th>
            <th>Address</th>
            <th>Size</th>
            <th>Exec</th>
          </tr>
        </thead>
        <tbody>
          {info.sections.map((s) => (
            <tr key={`${s.name}-${s.addr}`}>
              <td>{s.name}</td>
              <td>{hex(s.addr)}</td>
              <td>{s.size}</td>
              <td>{s.executable ? "x" : ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
