import clsx from "clsx";
import {
  AlertTriangle,
  ArrowRightLeft,
  Brain,
  ChevronRight,
  CircleStop,
  Sparkles,
} from "lucide-react";
import { useState } from "react";
import { pendingApprovals, useAgent } from "../../store/agent";
import { useWorkbench } from "../../store/workbench";
import type { AgentEvent } from "../../types";
import { splitPrompt } from "../../lib/prompt";
import { Report } from "./Report";

const TOOL_LABEL: Record<string, string> = {
  binary_info: "info",
  list_functions: "functions",
  disassemble: "disasm",
  decompile: "decomp",
  xrefs_to: "xrefs",
  xrefs_from: "callees",
  search_strings: "strings",
  list_imports: "imports",
  read_bytes: "hex",
  rename_function: "rename",
  add_comment: "comment",
  record_hypothesis: "hypothesis",
  record_finding: "finding",
  record_verdict: "verdict",
};

function Thinking({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  return (
    <button
      onClick={() => setOpen(!open)}
      className="flex w-full items-start gap-1.5 text-left text-xs text-faint hover:text-muted"
    >
      <Brain size={12} className="mt-0.5 shrink-0" />
      <span className={clsx(!open && "line-clamp-1")}>{text}</span>
    </button>
  );
}

function ToolCard({ e }: { e: Extract<AgentEvent, { type: "tool_call" }> }) {
  const selectNode = useWorkbench((s) => s.selectNode);
  const selected = useWorkbench((s) => s.selected);
  const active = e.step_id !== null && selected === e.step_id;
  return (
    <button
      disabled={!e.step_id}
      onClick={() => e.step_id && selectNode(e.step_id)}
      className={clsx(
        "ease group w-full rounded-md border px-2.5 py-1.5 text-left hover:border-line-strong",
        active ? "border-brand/60 bg-brand-soft" : "bg-elevated",
        e.error && "border-bad/40",
      )}
    >
      <div className="flex items-center gap-2">
        <span className="w-16 shrink-0 text-2xs font-semibold tracking-wider text-muted uppercase">
          {TOOL_LABEL[e.tool] ?? e.tool}
        </span>
        <span className="truncate font-mono text-xs text-fg">{e.command}</span>
        <ChevronRight
          size={12}
          className="ml-auto shrink-0 text-faint opacity-0 group-hover:opacity-100"
        />
      </div>
      {e.why && <div className="mt-0.5 pl-[72px] text-xs text-muted">{e.why}</div>}
      {e.summary && (
        <div className="mt-0.5 truncate pl-[72px] text-2xs text-faint">{e.summary}</div>
      )}
      {e.error && <div className="mt-0.5 pl-[72px] text-2xs text-bad">{e.error}</div>}
    </button>
  );
}

function ApprovalCard({
  e,
  pending,
}: {
  e: Extract<AgentEvent, { type: "approval_requested" }>;
  pending: boolean;
}) {
  const decide = useAgent((s) => s.decide);
  return (
    <div className="rounded-md border border-brand/50 bg-brand-soft px-2.5 py-2">
      <div className="text-xs text-brand">The agent wants to run</div>
      <div className="mt-0.5 font-mono text-xs text-fg">{e.request.command}</div>
      {e.request.why && <div className="mt-0.5 text-xs text-muted">{e.request.why}</div>}
      {pending && (
        <div className="mt-2 flex gap-1.5">
          <button
            onClick={() => decide(e.request.id, true)}
            className="ease h-6 rounded-md bg-brand px-2.5 text-xs font-medium text-[#1a1208] hover:brightness-110"
          >
            Allow
          </button>
          <button
            onClick={() => decide(e.request.id, false)}
            className="ease h-6 rounded-md border px-2.5 text-xs text-muted hover:text-fg"
          >
            Deny
          </button>
        </div>
      )}
    </div>
  );
}

/** The agent run: request, reasoning, every recorded tool call, and the report. */
export function Transcript({ events, running }: { events: AgentEvent[]; running: boolean }) {
  const pending = new Set(pendingApprovals(events).map((r) => r.id));
  return (
    <div className="space-y-1.5">
      {events.map((e, i) => {
        switch (e.type) {
          case "started": {
            const p = splitPrompt(e.prompt);
            return (
              <div key={i} className="mb-2 rounded-md bg-active px-2.5 py-1.5 text-sm text-fg">
                {p.text}
                <div className="mt-0.5 font-mono text-2xs text-faint">
                  {e.provider} · {e.model}
                  {p.attached && " · context attached"}
                </div>
              </div>
            );
          }
          case "thinking":
            return <Thinking key={i} text={e.text} />;
          case "message":
            return (
              <p key={i} className="text-sm text-fg">
                {e.text}
              </p>
            );
          case "tool_call":
            return <ToolCard key={i} e={e} />;
          case "refusal":
            return (
              <div
                key={i}
                className="rounded-md border border-warn/40 px-2.5 py-2 text-xs text-warn"
              >
                <div className="flex items-center gap-1.5 font-medium">
                  <AlertTriangle size={12} /> {e.provider} declined ({e.category ?? "policy"})
                </div>
                <p className="mt-1 text-muted">
                  {e.explanation ?? "A safety system stopped this request."} Add another provider (a
                  local model, for example) so the run can continue, or apply to the provider's
                  security-research program (Anthropic: Cyber Verification Program).
                </p>
              </div>
            );
          case "approval_requested":
            return <ApprovalCard key={i} e={e} pending={pending.has(e.request.id)} />;
          case "approval_resolved":
            return (
              <div key={i} className={`text-2xs ${e.allowed ? "text-good" : "text-bad"}`}>
                {e.allowed ? "Allowed" : "Denied"} by you
              </div>
            );
          case "usage":
            return null;
          case "context_trimmed":
            return (
              <div key={i} className="text-2xs text-faint">
                Elided {e.elided} old tool results to stay within the context window (~
                {(e.tokens / 1000).toFixed(0)}k tokens). They stay on the graph.
              </div>
            );
          case "switched":
            return (
              <div key={i} className="flex items-center gap-1.5 text-xs text-muted">
                <ArrowRightLeft size={12} className="text-brand" /> Continuing on{" "}
                <span className="font-mono text-fg">{e.to}</span>
              </div>
            );
          case "finished":
            return (
              <div key={i} className="mt-3 border-t pt-3">
                <div className="mb-2 flex items-center gap-1.5 text-xs text-brand">
                  <Sparkles size={12} /> Report
                </div>
                <Report text={e.report} />
                <div className="mt-3 font-mono text-2xs text-faint nums">
                  {e.tool_calls} recorded steps · {e.turns} turns ·{" "}
                  {(e.input_tokens / 1000).toFixed(1)}k in · {(e.output_tokens / 1000).toFixed(1)}k
                  out
                </div>
              </div>
            );
          case "failed":
            return (
              <div
                key={i}
                className="rounded-md border border-bad/40 px-2.5 py-1.5 text-xs text-bad"
              >
                {e.error}
              </div>
            );
          case "stopped":
            return (
              <div key={i} className="flex items-center gap-1.5 text-xs text-muted">
                <CircleStop size={12} /> Stopped.
              </div>
            );
        }
      })}
      {running && (
        <div className="flex items-center gap-2 py-1 text-xs text-muted">
          <span className="rec-dot h-1.5 w-1.5 rounded-full bg-brand" /> Working…
        </div>
      )}
    </div>
  );
}
