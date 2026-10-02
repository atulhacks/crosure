import { FolderOpen, History } from "lucide-react";
import { useEffect } from "react";
import { shortHash } from "../lib/format";
import { useWorkbench } from "../store/workbench";
import { Logo } from "./Logo";
import { pickAndOpen } from "./TopBar";
import { Btn } from "./ui";

/** Start screen: open a binary or resume a recorded session. */
export function Welcome() {
  const { sessions, refreshSessions, open, resume } = useWorkbench();
  useEffect(() => {
    refreshSessions();
  }, [refreshSessions]);
  return (
    <div className="flex flex-1 items-center justify-center overflow-auto p-8">
      <div className="w-full max-w-2xl">
        <div className="mb-6 flex items-center gap-3">
          <Logo size={44} />
          <div>
            <h1 className="text-2xl font-semibold">Crosure</h1>
            <p className="text-dim">
              Every reverse, remembered. Each step you take becomes a node in a tamper-evident
              investigation graph.
            </p>
          </div>
        </div>
        <Btn className="h-10 px-4 text-sm" onClick={() => pickAndOpen(open)}>
          <FolderOpen size={16} /> Open a binary
        </Btn>
        <h2 className="mt-8 mb-2 flex items-center gap-2 text-xs font-semibold tracking-wide text-dim uppercase">
          <History size={14} /> Recorded sessions
        </h2>
        <div className="divide-y divide-line rounded-md border border-line bg-panel">
          {sessions.length === 0 && <div className="p-4 text-dim">No sessions yet.</div>}
          {sessions.map((s) => (
            <button
              key={s.id}
              onClick={() => resume(s.id)}
              className="flex w-full items-center justify-between gap-4 px-4 py-2.5 text-left hover:bg-panel-2"
            >
              <div className="min-w-0">
                <div className="truncate font-medium">{s.name}</div>
                <div className="truncate text-xs text-dim">{s.binary_path}</div>
              </div>
              <div className="shrink-0 text-right text-xs text-dim">
                <div>{new Date(s.created_ms).toLocaleString()}</div>
                <div className="font-mono">head {shortHash(s.head_hash, 8)}</div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
