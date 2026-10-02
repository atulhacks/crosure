import { hex } from "../../../lib/format";
import type { BinaryInfo } from "../../../types";

function Field({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      <span className="label">{k}</span>
      <span className="truncate font-mono text-sm text-fg nums" title={v}>
        {v}
      </span>
    </div>
  );
}

/** Overview of the binary: identity, headline facts, sections. */
export function InfoView({ result }: { result: unknown }) {
  const info = result as BinaryInfo;
  const exec = info.sections.filter((s) => s.executable).length;
  return (
    <div className="max-w-4xl px-5 py-4">
      <div className="truncate font-mono text-xs text-faint">{info.path}</div>
      <div className="mt-4 grid grid-cols-4 gap-x-6 gap-y-4">
        <Field k="Format" v={info.format.toUpperCase()} />
        <Field k="Architecture" v={`${info.arch} · ${info.bits}-bit`} />
        <Field k="Endianness" v={info.little_endian ? "little" : "big"} />
        <Field k="Symbols" v={info.stripped ? "stripped" : "present"} />
        <Field k="Entry" v={hex(info.entry)} />
        <Field k="Size" v={`${info.size.toLocaleString()} B`} />
        <Field k="Sections" v={`${info.sections.length} (${exec} exec)`} />
      </div>
      <div className="mt-4">
        <Field k="SHA-256" v={info.sha256.replace("sha256:", "")} />
      </div>

      <div className="label mt-7 mb-1.5">Sections</div>
      <div className="overflow-hidden rounded-md border font-mono text-xs">
        <div className="row h-7 border-b bg-panel">
          {["Name", "Address", "Size", "Offset", ""].map((h) => (
            <span key={h} className="label w-28">
              {h}
            </span>
          ))}
        </div>
        {info.sections.map((s) => (
          <div key={`${s.name}-${s.addr}`} className="row nums">
            <span className="w-28 truncate text-fg">{s.name}</span>
            <span className="w-28 text-asm-addr">{hex(s.addr)}</span>
            <span className="w-28 text-muted">{s.size.toLocaleString()}</span>
            <span className="w-28 text-muted">
              {s.file_offset !== null ? hex(s.file_offset) : "—"}
            </span>
            {s.executable && <span className="text-2xs text-asm-mnemonic">exec</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
