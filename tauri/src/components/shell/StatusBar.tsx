import { TerminalSquare } from "lucide-react";
import { hex } from "../../lib/format";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";

/** 22px IDE status strip: engine, target, current location, session. */
export function StatusBar() {
  const opened = useWorkbench((s) => s.opened);
  const fnCount = useWorkbench((s) => s.functions.length);
  const disasm = useWorkbench((s) => s.views.disasm);
  const consoleOpen = useUi((s) => s.consoleOpen);
  const toggleConsole = useUi((s) => s.toggleConsole);
  if (!opened) return null;
  const fn = (disasm?.result as { function?: { addr: number; name: string } } | undefined)
    ?.function;
  const item = "flex items-center gap-1 px-2";
  return (
    <footer className="flex h-[var(--status-h)] shrink-0 items-center border-t bg-panel font-mono text-2xs text-muted nums">
      <span className={item}>native</span>
      <span className={item}>
        {opened.info.arch} · {opened.info.bits}-bit · {opened.info.format}
      </span>
      <span className={item}>{fnCount} functions</span>
      {fn && (
        <span className={`${item} text-fg`}>
          {fn.name} <span className="text-faint">{hex(fn.addr)}</span>
        </span>
      )}
      <span className="ml-auto" />
      <span className={item} title={opened.session.id}>
        session {opened.session.id.slice(4, 14)}
      </span>
      <button
        onClick={toggleConsole}
        className={`${item} ease h-full hover:bg-hover ${consoleOpen ? "text-fg" : ""}`}
      >
        <TerminalSquare size={11} /> console
      </button>
    </footer>
  );
}
