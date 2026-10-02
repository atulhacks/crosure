import { Download, Moon, Search, ShieldAlert, ShieldCheck, Sparkles, Sun } from "lucide-react";
import { useState } from "react";
import { shortHash } from "../../lib/format";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";
import { Button, IconButton } from "../ui/Button";
import { Kbd } from "../ui/Pill";
import { Logo } from "./Logo";

/** Top bar: identity, the target, recording state, chain integrity, export. */
export function TitleBar() {
  const opened = useWorkbench((s) => s.opened);
  const report = useWorkbench((s) => s.verifyReport);
  const steps = useWorkbench((s) => s.graph?.stats.steps ?? 0);
  const replaying = useWorkbench((s) => s.upto !== null);
  const verify = useWorkbench((s) => s.verify);
  const exportSession = useWorkbench((s) => s.exportSession);
  const close = useWorkbench((s) => s.close);
  const { theme, setTheme, setPaletteOpen, setDockTab } = useUi();
  const [exported, setExported] = useState<string | null>(null);

  return (
    <header className="flex h-[var(--bar-h)] shrink-0 items-center gap-3 border-b bg-panel px-3">
      <button onClick={close} className="flex items-center gap-2" title="Home">
        <Logo />
        <span className="text-sm font-semibold tracking-tight">Crosure</span>
      </button>
      {opened && (
        <div className="flex min-w-0 items-center gap-2 text-xs text-muted">
          <span className="text-faint">/</span>
          <span className="truncate font-medium text-fg">{opened.session.name}</span>
          <span className="hidden truncate font-mono md:inline">
            {opened.info.format.toUpperCase()} · {opened.info.arch} · {opened.info.bits}-bit
            {opened.info.stripped ? " · stripped" : ""}
          </span>
        </div>
      )}

      {opened && (
        <button
          onClick={() => setPaletteOpen(true)}
          className="ease mx-auto flex h-6 w-72 items-center gap-2 rounded-md border bg-bg px-2 text-xs text-faint hover:border-line-strong"
        >
          <Search size={12} />
          <span>Go to function, run a command…</span>
          <span className="ml-auto flex gap-0.5">
            <Kbd>Ctrl</Kbd>
            <Kbd>K</Kbd>
          </span>
        </button>
      )}

      <div className="ml-auto flex items-center gap-1.5">
        {opened && (
          <span
            className="flex h-6 items-center gap-1.5 rounded-md px-2 font-mono text-2xs tracking-wider text-brand"
            title={
              replaying
                ? "Replaying — new actions go back to live"
                : "Every action is being recorded"
            }
          >
            <span className={`h-1.5 w-1.5 rounded-full bg-brand ${replaying ? "" : "rec-dot"}`} />
            {replaying ? "REPLAY" : "REC"} <span className="nums text-muted">{steps}</span>
          </span>
        )}
        {opened && (
          <Button
            variant="ghost"
            onClick={() => setDockTab("agent")}
            title="Ask the AI agent (Ctrl+L)"
          >
            <Sparkles size={13} className="text-brand" /> Ask AI
          </Button>
        )}
        {opened && report && (
          <Button
            variant="ghost"
            onClick={verify}
            className={report.ok ? "text-good hover:text-good" : "text-bad hover:text-bad"}
            title={report.failure?.reason ?? `Chain head ${report.head_hash}`}
          >
            {report.ok ? <ShieldCheck size={13} /> : <ShieldAlert size={13} />}
            {report.ok ? (
              <span className="font-mono text-xs nums">{shortHash(report.head_hash, 8)}</span>
            ) : (
              <span className="text-xs">Tampered at #{report.failure?.seq}</span>
            )}
          </Button>
        )}
        {opened && (
          <Button
            variant="ghost"
            onClick={async () => setExported(await exportSession())}
            title={exported ?? "Export session (JSON)"}
          >
            <Download size={13} /> {exported ? "Exported" : "Export"}
          </Button>
        )}
        <IconButton
          onClick={() => setTheme(theme === "crosure-dark" ? "crosure-light" : "crosure-dark")}
          title="Toggle theme"
        >
          {theme === "crosure-dark" ? <Sun size={13} /> : <Moon size={13} />}
        </IconButton>
      </div>
    </header>
  );
}
