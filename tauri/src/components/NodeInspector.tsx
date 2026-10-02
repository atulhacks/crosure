import { GitBranch } from "lucide-react";
import { useState } from "react";
import { INTENT_CHIPS, KIND_COLOR, shortHash, TAGS } from "../lib/format";
import { useWorkbench } from "../store/workbench";
import { Btn, PanelHeader } from "./ui";

type NoteKind = "hypothesis" | "finding";

/** Details of the selected step: intent chips, tags, notes. Annotations are new steps, never edits. */
export function NodeInspector() {
  const { graph, selected, branchFrom, annotate, act } = useWorkbench();
  const [text, setText] = useState("");
  const [kind, setKind] = useState<NoteKind>("hypothesis");
  const node = graph?.nodes.find((n) => n.id === selected);
  if (!node) {
    return (
      <div className="flex-[2] border-t border-line bg-panel p-3 text-xs text-dim">
        Click a step to see it, tag it, or branch from it.
      </div>
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
    <div className="flex min-h-0 flex-[2] flex-col border-t border-line bg-panel">
      <PanelHeader
        title={
          <span style={{ color: KIND_COLOR[node.kind] }}>
            #{node.seq} {node.kind}
          </span>
        }
      >
        <span className="font-mono text-[10px] text-dim" title={node.hash}>
          {shortHash(node.hash, 12)}
        </span>
      </PanelHeader>
      <div className="min-h-0 flex-1 space-y-2.5 overflow-auto p-3 text-xs">
        <div>
          <div className="font-mono text-fg">{node.command ?? node.label}</div>
          <div className="mt-0.5 text-dim">{node.summary}</div>
          {node.target?.func && (
            <div className="mt-0.5 font-mono text-dim">
              in {node.target.func} {node.target.addr}
            </div>
          )}
        </div>
        {branchFrom === node.id && (
          <div className="flex items-center gap-1.5 rounded border border-amber-400/40 px-2 py-1 text-amber-300">
            <GitBranch size={13} /> Your next action will branch from this step.
          </div>
        )}
        <div>
          <div className="mb-1 text-dim">Why? (intent)</div>
          <div className="flex flex-wrap gap-1">
            {INTENT_CHIPS.map((c) => (
              <button
                key={c.id}
                onClick={() => annotate(node.id, c.id, [])}
                className={`rounded-full border px-2 py-0.5 ${node.intent?.chip === c.id ? "border-accent-2 text-accent-2" : "border-line text-dim hover:text-fg"}`}
              >
                {c.label}
              </button>
            ))}
          </div>
        </div>
        <div>
          <div className="mb-1 text-dim">Tag</div>
          <div className="flex gap-1">
            {TAGS.map((t) => (
              <button
                key={t}
                disabled={node.tags.includes(t)}
                onClick={() => annotate(node.id, null, [t])}
                className="rounded-full border border-line px-2 py-0.5 text-dim hover:text-fg disabled:border-accent/50 disabled:text-accent"
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
            className="rounded-md border border-line bg-bg px-1.5"
          >
            <option value="hypothesis">Hypothesis</option>
            <option value="finding">Finding</option>
          </select>
          <input
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Linked to this step…"
            className="min-w-0 flex-1 rounded-md border border-line bg-bg px-2 py-1 outline-none focus:border-accent/60"
          />
          <Btn type="submit">Add</Btn>
        </form>
      </div>
    </div>
  );
}
