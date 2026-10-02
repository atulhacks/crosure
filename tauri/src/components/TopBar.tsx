import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Download, FolderOpen, ShieldAlert, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { inTauri } from "../api";
import { shortHash } from "../lib/format";
import { useWorkbench } from "../store/workbench";
import { Logo } from "./Logo";
import { Btn } from "./ui";

/** Picks a file with the native dialog and opens it. */
export async function pickAndOpen(open: (p: string) => Promise<void>) {
  const path = inTauri
    ? await openDialog({ multiple: false, directory: false, title: "Open binary" })
    : window.prompt("Path to a binary (dev preview)");
  if (typeof path === "string") await open(path);
}

/** App bar: session info, chain verification badge, export. */
export function TopBar() {
  const { opened, verifyReport, open, verify, exportSession, close } = useWorkbench();
  const [exported, setExported] = useState<string | null>(null);
  return (
    <div className="flex h-11 shrink-0 items-center gap-3 border-b border-line bg-panel px-3">
      <button onClick={close} className="flex items-center gap-2" title="Home">
        <Logo />
        <span className="text-sm font-semibold tracking-wide">Crosure</span>
      </button>
      {opened && (
        <div className="flex min-w-0 items-center gap-2 text-xs text-dim">
          <span className="truncate font-medium text-fg">{opened.session.name}</span>
          <span>
            {opened.info.format.toUpperCase()} · {opened.info.arch} · {opened.info.bits}-bit
            {opened.info.stripped ? " · stripped" : ""}
          </span>
          <span className="font-mono">{shortHash(opened.info.sha256, 12)}</span>
        </div>
      )}
      <div className="ml-auto flex items-center gap-2">
        {opened && verifyReport && (
          <button
            onClick={verify}
            title={verifyReport.failure?.reason ?? `head ${verifyReport.head_hash}`}
            className={`flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-xs ${
              verifyReport.ok ? "border-good/40 text-good" : "border-bad/60 text-bad"
            }`}
          >
            {verifyReport.ok ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
            {verifyReport.ok
              ? `Chain verified · ${verifyReport.checked} steps · ${shortHash(verifyReport.head_hash, 8)}`
              : `Tampered at step ${verifyReport.failure?.seq}: ${verifyReport.failure?.reason}`}
          </button>
        )}
        {opened && (
          <Btn
            onClick={async () => setExported(await exportSession())}
            title={exported ?? "Export session as JSON"}
          >
            <Download size={14} /> {exported ? "Exported" : "Export"}
          </Btn>
        )}
        <Btn onClick={() => pickAndOpen(open)}>
          <FolderOpen size={14} /> Open binary
        </Btn>
      </div>
    </div>
  );
}
