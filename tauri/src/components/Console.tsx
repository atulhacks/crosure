import { TerminalSquare } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import { useWorkbench } from "../store/workbench";

/** Command console. Every command is recorded as a step (tool = console). */
export function Console() {
  const { consoleLog, runConsole, busy } = useWorkbench();
  const [line, setLine] = useState("");
  const [help, setHelp] = useState<string | null>(null);
  const [cursor, setCursor] = useState<number | null>(null);
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => end.current?.scrollIntoView({ block: "end" }), [consoleLog, help]);
  const history = consoleLog.map((l) => l.input);
  const submit = async () => {
    const cmd = line.trim();
    if (!cmd) return;
    setLine("");
    setCursor(null);
    if (cmd === "help" || cmd === "?") return setHelp(await api.consoleHelp());
    await runConsole(cmd);
  };
  const recall = (dir: -1 | 1) => {
    if (history.length === 0) return;
    const next = Math.min(history.length - 1, Math.max(0, (cursor ?? history.length) + dir));
    setCursor(next);
    setLine(history[next]);
  };
  return (
    <div className="flex h-44 shrink-0 flex-col border-t border-line bg-panel font-mono text-xs">
      <div className="min-h-0 flex-1 overflow-auto px-3 py-1.5">
        {help && <pre className="mb-1 whitespace-pre-wrap text-dim">{help}</pre>}
        {consoleLog.map((l, i) => (
          <div key={i}>
            <span className="text-accent">❯ {l.input}</span>
            <div className={l.ok ? "text-dim" : "text-bad"}>{l.output}</div>
          </div>
        ))}
        {consoleLog.length === 0 && !help && (
          <div className="text-dim">
            Type <span className="text-fg">help</span> for commands, e.g.{" "}
            <span className="text-fg">str http</span>, <span className="text-fg">xt strcmp</span>,{" "}
            <span className="text-fg">dis main</span>
          </div>
        )}
        <div ref={end} />
      </div>
      <form
        className="flex items-center gap-2 border-t border-line px-3 py-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <TerminalSquare size={14} className="text-dim" />
        <input
          value={line}
          onChange={(e) => setLine(e.target.value)}
          disabled={busy}
          placeholder="crosure> "
          onKeyDown={(e) => {
            if (e.key === "ArrowUp") {
              e.preventDefault();
              recall(-1);
            }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              recall(1);
            }
          }}
          className="flex-1 bg-transparent outline-none"
        />
      </form>
    </div>
  );
}
