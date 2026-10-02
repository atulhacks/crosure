import clsx from "clsx";
import { Copy, GitBranch, MousePointerClick } from "lucide-react";
import { useState } from "react";
import { INTENT_CHIPS, KIND_COLOR, shortHash, TAGS } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import { Button } from "../ui/Button";
import { Empty } from "../ui/Pane";

type NoteKind = "hypothesis" | "finding";

/** The selected step: what it did, why (intent), how it went (tags), and notes linked to it. */
export function Inspector() {
  const { graph, selected, branchFrom, annotate, act } = useWorkbench();
  const [text, setText] = useState("");
  const [kind, setKind] = useState<NoteKind>("hypothesis");
  const node = graph?.nodes.find((n) => n.id === selected);
  if (!node) {
    return (
      <Empty
        icon={<MousePointerClick size={18} />}
        title="Select a step"
        hint="Tag it, say why you took it, or branch from it."
      />
    );
  }
  const submit = async () => {
    if (!text.trim()) return;
    const parent = { id: node.id, rel: "derived_from" as const };
    await act(kind === "hypothesis" ? { op: "hypothesis", text } : { op: "finding", text }, {
      parent,
    });
    setText("");
  };
  return (
    <div className="scroll-host h-full space-y-3 overflow-auto px-3 py-2.5">
      <div>
        <div className="flex items-center gap-1.5 text-2xs">
          <span
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: KIND_COLOR[node.kind] }}
          />
          <span className="font-semibold tracking-wider text-muted uppercase">{node.kind}</span>
          <span className="font-mono text-faint nums">#{node.seq}</span>
          <button
            onClick={() => navigator.clipboard?.writeText(node.hash)}
            title={node.hash}
            className="ml-auto flex items-center gap-1 font-mono text-faint hover:text-fg"
          >
            {shortHash(node.hash, 10)} <Copy size={10} />
          </button>
        </div>
        <div className="mt-1 font-mono text-sm break-all text-fg">{node.command ?? node.label}</div>
        <div className="mt-0.5 text-xs text-muted">{node.summary}</div>
        {node.target?.func && (
          <div className="mt-0.5 font-mono text-2xs text-faint">
            in {node.target.func} · {node.target.addr}
          </div>
        )}
        <div className="mt-1.5 text-2xs text-faint">
          {node.actor === "agent" ? "Taken by the AI agent" : "Taken by you"} ·{" "}
          {new Date(node.ts_ms).toLocaleTimeString()}
        </div>
        {node.intent?.note && (
          <blockquote className="mt-1.5 border-l-2 border-[var(--kind-agent)]/50 pl-2 text-xs text-muted">
            {node.intent.note}
          </blockquote>
        )}
      </div>

      {branchFrom === node.id && (
        <div className="flex items-center gap-1.5 rounded-md border border-brand/40 bg-brand-soft px-2 py-1.5 text-xs text-brand">
          <GitBranch size={12} /> Your next action branches from here.
        </div>
      )}

      <div>
        <div className="label mb-1.5">Why</div>
        <div className="flex flex-wrap gap-1">
          {INTENT_CHIPS.map((c) => (
            <button
              key={c.id}
              onClick={() => annotate(node.id, c.id, [])}
              className={clsx(
                "ease h-6 rounded-full border px-2 text-xs",
                node.intent?.chip === c.id
                  ? "border-brand/50 bg-brand-soft text-brand"
                  : "text-muted hover:border-line-strong hover:text-fg",
              )}
            >
              {c.label}
            </button>
          ))}
        </div>
      </div>

      <div>
        <div className="label mb-1.5">Outcome</div>
        <div className="flex gap-1">
          {TAGS.map((t) => (
            <button
              key={t}
              disabled={node.tags.includes(t)}
              onClick={() => annotate(node.id, null, [t])}
              className="ease h-6 rounded-full border px-2 text-xs text-muted hover:border-line-strong hover:text-fg disabled:border-brand/50 disabled:bg-brand-soft disabled:text-brand"
            >
              {t.replace("_", " ")}
            </button>
          ))}
        </div>
      </div>

      <form
        className="flex gap-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <select
          value={kind}
          onChange={(e) => setKind(e.target.value as NoteKind)}
          className="h-7 rounded-md border bg-bg px-1.5 text-xs text-fg"
        >
          <option value="hypothesis">Hypothesis</option>
          <option value="finding">Finding</option>
        </select>
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Linked to this step"
          className="h-7 min-w-0 flex-1 rounded-md border bg-bg px-2 text-sm outline-none placeholder:text-faint focus:border-brand/60"
        />
        <Button type="submit">Add</Button>
      </form>
    </div>
  );
}
