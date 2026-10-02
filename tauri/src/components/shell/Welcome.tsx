import { ArrowRight, FolderOpen, GitBranch, ShieldCheck, Waypoints } from "lucide-react";
import { useEffect } from "react";
import { shortHash } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import { Button } from "../ui/Button";
import { Logo } from "./Logo";
import { pickAndOpen } from "./open";

const POINTS = [
  {
    icon: Waypoints,
    title: "Every step is a node",
    text: "Clicks and commands become a live investigation graph.",
  },
  {
    icon: GitBranch,
    title: "Branches and dead ends",
    text: "Jump back, try again; the path to each finding is kept.",
  },
  {
    icon: ShieldCheck,
    title: "Tamper-evident",
    text: "Hash-chained steps. Edit one and verification fails.",
  },
];

/** Start screen: open a binary or resume a recorded investigation. */
export function Welcome() {
  const sessions = useWorkbench((s) => s.sessions);
  const refreshSessions = useWorkbench((s) => s.refreshSessions);
  const open = useWorkbench((s) => s.open);
  const resume = useWorkbench((s) => s.resume);
  useEffect(() => {
    refreshSessions();
  }, [refreshSessions]);

  return (
    <div className="scroll-host flex-1 overflow-auto">
      <div className="mx-auto flex w-full max-w-[640px] flex-col px-6 pt-[12vh] pb-12">
        <Logo size={36} />
        <h1 className="mt-5 text-[28px] font-semibold tracking-tight">
          Every reverse, remembered.
        </h1>
        <p className="mt-2 max-w-[52ch] text-base text-muted">
          Crosure records how you reverse a binary, step by step, into a replayable, verifiable
          investigation graph: evidence for reports, lessons for students, and training data for
          models.
        </p>
        <div className="mt-6 flex gap-2">
          <Button variant="primary" className="h-9 px-4" onClick={() => pickAndOpen(open)}>
            <FolderOpen size={15} /> Open binary
          </Button>
        </div>

        <div className="mt-10 grid grid-cols-3 gap-5">
          {POINTS.map(({ icon: Icon, title, text }) => (
            <div key={title}>
              <Icon size={15} className="text-brand" />
              <div className="mt-2 text-sm font-medium">{title}</div>
              <div className="mt-0.5 text-xs text-muted">{text}</div>
            </div>
          ))}
        </div>

        <div className="mt-12 flex items-baseline justify-between">
          <span className="label">Recent investigations</span>
          <span className="font-mono text-2xs text-faint">~/.crosure</span>
        </div>
        <div className="mt-2 overflow-hidden rounded-lg border">
          {sessions.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-faint">Nothing recorded yet.</div>
          )}
          {sessions.map((s, i) => (
            <button
              key={s.id}
              onClick={() => resume(s.id)}
              className={`ease group flex w-full items-center gap-4 px-4 py-2.5 text-left hover:bg-hover ${i > 0 ? "border-t" : ""}`}
            >
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{s.name}</div>
                <div className="truncate font-mono text-2xs text-faint">{s.binary_path}</div>
              </div>
              <div className="shrink-0 text-right">
                <div className="text-xs text-muted">
                  {new Date(s.created_ms).toLocaleDateString(undefined, {
                    month: "short",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </div>
                <div className="font-mono text-2xs text-faint nums">
                  {shortHash(s.head_hash, 8)}
                </div>
              </div>
              <ArrowRight size={14} className="ease text-faint opacity-0 group-hover:opacity-100" />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
