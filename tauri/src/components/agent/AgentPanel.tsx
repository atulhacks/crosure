import { Settings2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { useAgent } from "../../store/agent";
import { Empty } from "../ui/Pane";
import { AgentSetup } from "./AgentSetup";
import { Composer } from "./Composer";
import { ThreadBar } from "./ThreadBar";
import { Transcript } from "./Transcript";

const SUGGESTIONS = [
  "Reverse this binary and explain what it does",
  "Find how the input is validated",
  "Look for network indicators and C2",
];

/** Conversations with the agent; its steps join the investigation graph live. */
export function AgentPanel() {
  const { status, events, live, running, error, refreshStatus, start, setSettingsOpen } =
    useAgent();
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);
  useEffect(
    () => end.current?.scrollIntoView({ block: "end" }),
    [events.length, running, live?.text.length, live?.thinking.length],
  );

  if (!status) return <Empty title="Loading agent…" />;
  if (!status.configured) return <AgentSetup />;
  return (
    <div className="flex h-full flex-col">
      <ThreadBar />
      <div className="scroll-host min-h-0 flex-1 overflow-auto px-3 py-2.5">
        {events.length === 0 && !running ? (
          <div className="flex h-full flex-col justify-end gap-1.5">
            <p className="mb-1 text-xs text-muted">
              Each tool call the agent makes becomes an AI step on the graph above, with its reason.
              Type @ to attach a function or a step.
            </p>
            {SUGGESTIONS.map((s) => (
              <button
                key={s}
                onClick={() => start(s)}
                className="ease rounded-md border px-2.5 py-1.5 text-left text-xs text-muted hover:border-line-strong hover:text-fg"
              >
                {s}
              </button>
            ))}
          </div>
        ) : (
          <Transcript events={events} running={running} live={live} />
        )}
        {error && <p className="mt-2 text-xs text-bad">{error}</p>}
        <div ref={end} />
      </div>
      <Composer />
      <div className="flex items-center px-3 pb-1.5 font-mono text-2xs text-faint">
        <span className="truncate">
          {status.demo ? "scripted demo (not a real AI)" : `${status.label} · ${status.model}`}
        </span>
        {!status.demo && status.fallbacks > 0 && (
          <span className="ml-2 shrink-0">+{status.fallbacks} fallback</span>
        )}
        <button
          onClick={() => setSettingsOpen(true)}
          title="AI providers and behaviour"
          className="ml-auto text-faint hover:text-fg"
        >
          <Settings2 size={12} />
        </button>
      </div>
    </div>
  );
}
