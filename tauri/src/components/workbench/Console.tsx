import { ChevronRight, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import * as api from "../../api";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";
import { IconButton } from "../ui/Button";
import { Pane } from "../ui/Pane";

/** Command console. Every command is recorded as a step (tool = console). */
export function Console() {
  const consoleLog = useWorkbench((s) => s.consoleLog);
  const runConsole = useWorkbench((s) => s.runConsole);
  const busy = useWorkbench((s) => s.busy);
  const toggleConsole = useUi((s) => s.toggleConsole);
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
    if (!history.length) return;
    const next = Math.min(history.length - 1, Math.max(0, (cursor ?? history.length) + dir));
    setCursor(next);
    setLine(history[next]);
  };

  return (
    <Pane
      title="Console"
      className="h-full border-t bg-panel"
      scroll={false}
      actions={
        <IconButton onClick={toggleConsole} title="Hide console (Ctrl+`)">
          <X size={12} />
        </IconButton>
      }
    >
      <div className="flex h-full flex-col font-mono text-xs">
        <div className="scroll-host min-h-0 flex-1 overflow-auto px-3 py-1.5">
          {help && <pre className="mb-2 whitespace-pre-wrap text-muted">{help}</pre>}
          {consoleLog.map((l, i) => (
            <div key={i} className="mb-1">
              <div className="text-fg">
                <span className="text-brand">❯</span> {l.input}
              </div>
              <div className={l.ok ? "pl-3 text-muted" : "pl-3 text-bad"}>{l.output}</div>
            </div>
          ))}
          {!consoleLog.length && !help && (
            <div className="text-faint">
              help · dis main · xt strcmp · str http · hyp … · find …
            </div>
          )}
          <div ref={end} />
        </div>
        <form
          className="flex h-7 shrink-0 items-center gap-1.5 border-t px-3"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <ChevronRight size={12} className="text-brand" />
          <input
            value={line}
            onChange={(e) => setLine(e.target.value)}
            disabled={busy}
            placeholder="command"
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
            className="flex-1 bg-transparent outline-none placeholder:text-faint"
          />
        </form>
      </div>
    </Pane>
  );
}
