import { Database, Download, FileJson } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import * as api from "../../api";
import { useWorkbench } from "../../store/workbench";
import { Button } from "../ui/Button";

type Result = { title: string; detail: string; path: string } | { error: string };

function Item({
  icon,
  title,
  hint,
  onClick,
}: {
  icon: React.ReactNode;
  title: string;
  hint: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="ease flex w-full items-start gap-2.5 rounded-md px-2.5 py-2 text-left hover:bg-hover"
    >
      <span className="mt-0.5 text-muted">{icon}</span>
      <span>
        <span className="block text-sm text-fg">{title}</span>
        <span className="block text-xs text-muted">{hint}</span>
      </span>
    </button>
  );
}

/** Export: this session as JSON, or every verified session as a training dataset. */
export function ExportMenu() {
  const exportSession = useWorkbench((s) => s.exportSession);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<Result | null>(null);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);

  const run = async (job: () => Promise<Result>) => {
    setBusy(true);
    try {
      setResult(await job());
    } catch (e) {
      setResult({ error: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const session = () =>
    run(async () => {
      const path = await exportSession();
      if (!path) return { error: "Export failed." };
      return { title: "Session exported", detail: "Steps, graph and verification.", path };
    });

  const dataset = () =>
    run(async () => {
      const { dir, manifest: m } = await api.exportDataset();
      const skipped = m.sessions.filter((s) => s.skipped).length;
      return {
        title: "Dataset exported",
        detail:
          `${m.trajectories} investigations · ${m.sft_examples} SFT examples · ` +
          `${m.dpo_pairs} preference pairs` +
          (skipped ? ` · ${skipped} skipped` : ""),
        path: dir,
      };
    });

  return (
    <div ref={root} className="relative">
      <Button
        variant="ghost"
        onClick={() => {
          setOpen(!open);
          setResult(null);
        }}
      >
        <Download size={13} /> Export
      </Button>
      {open && (
        <div className="absolute right-0 z-50 mt-1 w-80 rounded-lg border bg-elevated p-1 shadow-xl">
          <Item
            icon={<FileJson size={14} />}
            title="This session (JSON)"
            hint="Every step, the graph and the chain verification."
            onClick={session}
          />
          <Item
            icon={<Database size={14} />}
            title="Training dataset"
            hint="All verified sessions as trajectories, SFT examples and DPO pairs (JSONL)."
            onClick={dataset}
          />
          {(busy || result) && (
            <div className="mt-1 border-t px-2.5 pt-2 pb-1.5 text-xs">
              {busy && <span className="text-muted">Exporting…</span>}
              {!busy && result && "error" in result && (
                <span className="text-bad">{result.error}</span>
              )}
              {!busy && result && "path" in result && (
                <>
                  <div className="text-good">{result.title}</div>
                  <div className="mt-0.5 text-muted">{result.detail}</div>
                  <div className="mt-1 font-mono text-2xs break-all text-faint">{result.path}</div>
                </>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
