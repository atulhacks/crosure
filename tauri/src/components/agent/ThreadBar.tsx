import { MessageSquarePlus } from "lucide-react";
import { useEffect } from "react";
import { useAgent } from "../../store/agent";
import { useWorkbench } from "../../store/workbench";
import { IconButton } from "../ui/Button";

/** Pick a past conversation about this binary, or start a new one. */
export function ThreadBar() {
  const { threads, currentThread, running, loadThreads, openThread } = useAgent();
  const sessionId = useWorkbench((s) => s.opened?.session.id);
  // A different binary means a different set of conversations.
  useEffect(() => {
    if (!sessionId || useAgent.getState().running) return;
    openThread(null).then(loadThreads);
  }, [sessionId, openThread, loadThreads]);
  return (
    <div className="flex h-8 shrink-0 items-center gap-1 border-b px-2">
      <select
        value={currentThread ?? ""}
        disabled={running}
        onChange={(e) => openThread(e.target.value || null)}
        className="h-6 min-w-0 flex-1 truncate rounded-md bg-transparent px-1 text-xs text-muted outline-none hover:bg-hover"
      >
        <option value="">New conversation</option>
        {threads.map((t) => (
          <option key={t.id} value={t.id}>
            {t.title} · {t.tool_calls} steps ·{" "}
            {new Date(t.created_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
          </option>
        ))}
      </select>
      <IconButton disabled={running} onClick={() => openThread(null)} title="New conversation">
        <MessageSquarePlus size={13} />
      </IconButton>
    </div>
  );
}
