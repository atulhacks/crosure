import { ArrowUp, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useAgent } from "../../store/agent";
import { Empty } from "../ui/Pane";
import { AgentSetup } from "./AgentSetup";
import { Transcript } from "./Transcript";

const SUGGESTIONS = [
  "Reverse this binary and explain what it does",
  "Find how the input is validated",
  "Look for network indicators and C2",
];

/** Ask Claude to reverse the binary; its steps join the investigation graph live. */
export function AgentPanel() {
  const { status, events, running, error, refreshStatus, start, stop } = useAgent();
  const [prompt, setPrompt] = useState("");
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);
  useEffect(() => end.current?.scrollIntoView({ block: "end" }), [events.length, running]);

  if (!status) return <Empty title="Loading agent…" />;
  if (!status.configured) return <AgentSetup />;
  const send = (text: string) => {
    const t = text.trim();
    if (!t || running) return;
    setPrompt("");
    start(t);
  };
  return (
    <div className="flex h-full flex-col">
      <div className="scroll-host min-h-0 flex-1 overflow-auto px-3 py-2.5">
        {events.length === 0 && !running ? (
          <div className="flex h-full flex-col justify-end gap-1.5">
            <p className="mb-1 text-xs text-muted">
              Ask the agent to reverse this binary. Each tool call it makes becomes an AI step on
              the graph above, with its reason.
            </p>
            {SUGGESTIONS.map((s) => (
              <button
                key={s}
                onClick={() => send(s)}
                className="ease rounded-md border px-2.5 py-1.5 text-left text-xs text-muted hover:border-line-strong hover:text-fg"
              >
                {s}
              </button>
            ))}
          </div>
        ) : (
          <Transcript events={events} running={running} />
        )}
        {error && <p className="mt-2 text-xs text-bad">{error}</p>}
        <div ref={end} />
      </div>
      <form
        className="m-2 flex items-end gap-1.5 rounded-lg border bg-bg p-1.5 focus-within:border-brand/50"
        onSubmit={(e) => {
          e.preventDefault();
          send(prompt);
        }}
      >
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={2}
          disabled={running}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send(prompt);
            }
          }}
          placeholder={running ? "The agent is working…" : "Ask the agent…"}
          className="max-h-32 min-h-9 flex-1 resize-none bg-transparent px-1 text-sm outline-none placeholder:text-faint"
        />
        {running ? (
          <button
            type="button"
            onClick={stop}
            title="Stop"
            className="ease flex h-7 w-7 items-center justify-center rounded-md bg-active text-fg hover:bg-hover"
          >
            <Square size={12} />
          </button>
        ) : (
          <button
            type="submit"
            disabled={!prompt.trim()}
            title="Send (Enter)"
            className="ease flex h-7 w-7 items-center justify-center rounded-md bg-brand text-[#1a1208] disabled:opacity-30"
          >
            <ArrowUp size={14} />
          </button>
        )}
      </form>
      <div className="flex justify-between px-3 pb-1.5 font-mono text-2xs text-faint">
        <span>{status.model}</span>
        <span>
          {status.source === "demo"
            ? "scripted demo (not Claude)"
            : status.source === "env"
              ? "key from env"
              : "key saved"}
        </span>
      </div>
    </div>
  );
}
