import clsx from "clsx";
import { ArrowUp, Square } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "../../api";
import { fuzzyFilter } from "../../lib/fuzzy";
import { contextFill, latestUsage, useAgent } from "../../store/agent";
import { useWorkbench } from "../../store/workbench";
import type { Profile, ProviderView } from "../../types";

const PROFILES: { value: Profile; label: string; hint: string }[] = [
  {
    value: "investigate",
    label: "Investigate",
    hint: "All tools: read, rename, comment, record findings",
  },
  {
    value: "read_only",
    label: "Read-only",
    hint: "Analysis and findings, but no renames or comments",
  },
  {
    value: "ask",
    label: "Ask",
    hint: "No tools: answers from the conversation and @-attached context",
  },
];

/** Picker entry that opens the provider settings instead of switching. */
const MANAGE = "__manage__";

/** The `@token` being typed at the caret, if any. */
export function mentionAt(text: string, caret: number): { start: number; query: string } | null {
  const before = text.slice(0, caret);
  const m = /(^|\s)@([#\w.@]*)$/.exec(before);
  return m ? { start: caret - m[2].length - 1, query: m[2] } : null;
}

/** Prompt box with @-mentions, profile and model pickers, usage and send/stop. */
export function Composer() {
  const {
    draft,
    setDraft,
    running,
    start,
    stop,
    profile,
    setProfile,
    status,
    setActive,
    events,
    setSettingsOpen,
  } = useAgent();
  const functions = useWorkbench((s) => s.functions);
  const steps = useWorkbench((s) => s.timeline);
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [caret, setCaret] = useState(0);
  const [pick, setPick] = useState(0);
  const box = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    api.agentSettings().then(
      (v) => setProviders(v.providers.filter((p) => p.enabled)),
      () => setProviders([]),
    );
  }, [status?.provider]);
  useEffect(() => {
    if (draft) box.current?.focus();
  }, [draft]);

  const mention = mentionAt(draft, caret);
  const options = useMemo(() => {
    if (!mention) return [];
    if (mention.query.startsWith("#")) {
      const n = mention.query.slice(1);
      return steps
        .filter((s) => String(s.seq).startsWith(n))
        .slice(-8)
        .map((s) => ({ insert: `#${s.seq}`, label: `#${s.seq} ${s.command ?? s.label}` }));
    }
    return fuzzyFilter(mention.query, functions, (f) => f.name, 8).map((f) => ({
      insert: f.name,
      label: f.name,
    }));
  }, [mention, functions, steps]);

  const complete = (insert: string) => {
    if (!mention) return;
    const next = `${draft.slice(0, mention.start)}@${insert} ${draft.slice(caret)}`;
    setDraft(next);
    setPick(0);
    requestAnimationFrame(() =>
      box.current?.setSelectionRange(
        mention.start + insert.length + 2,
        mention.start + insert.length + 2,
      ),
    );
  };
  const send = () => {
    if (draft.trim() && !running) start(draft.trim());
  };
  const usage = latestUsage(events);
  const fill = contextFill(events);
  const pct = fill ? Math.round((100 * fill.used) / fill.limit) : 0;

  return (
    <div className="relative m-2 rounded-lg border bg-bg focus-within:border-brand/50">
      {options.length > 0 && (
        <div className="absolute bottom-full left-0 z-20 mb-1 w-full overflow-hidden rounded-md border border-line-strong bg-elevated py-1 shadow-xl">
          {options.map((o, i) => (
            <button
              key={o.insert}
              onMouseDown={(e) => {
                e.preventDefault();
                complete(o.insert);
              }}
              className={clsx(
                "block w-full truncate px-2.5 py-1 text-left font-mono text-xs",
                i === pick ? "bg-active text-fg" : "text-muted",
              )}
            >
              @{o.label}
            </button>
          ))}
        </div>
      )}
      <textarea
        ref={box}
        value={draft}
        rows={2}
        disabled={running}
        onChange={(e) => {
          setDraft(e.target.value);
          setCaret(e.target.selectionStart);
        }}
        onSelect={(e) => setCaret(e.currentTarget.selectionStart)}
        onKeyDown={(e) => {
          if (options.length) {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setPick((p) => (p + 1) % options.length);
              return;
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setPick((p) => (p - 1 + options.length) % options.length);
              return;
            }
            if (e.key === "Tab" || e.key === "Enter") {
              e.preventDefault();
              complete(options[Math.min(pick, options.length - 1)].insert);
              return;
            }
          }
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            send();
          }
        }}
        placeholder={
          running
            ? "The agent is working…"
            : "Ask the agent… (@function or @#step to attach context)"
        }
        className="block max-h-40 min-h-12 w-full resize-none bg-transparent px-2.5 pt-2 text-sm outline-none placeholder:text-faint"
      />
      <div className="flex items-center gap-1 px-1.5 pb-1.5">
        <select
          value={profile}
          onChange={(e) => setProfile(e.target.value as Profile)}
          title={PROFILES.find((p) => p.value === profile)?.hint}
          className="h-6 rounded-md bg-transparent px-1 text-2xs text-muted outline-none hover:bg-hover hover:text-fg"
        >
          {PROFILES.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>
        {status && !status.demo && (
          <select
            value={status.provider}
            onChange={(e) =>
              e.target.value === MANAGE ? setSettingsOpen(true) : setActive(e.target.value)
            }
            title="Model"
            className="h-6 max-w-44 truncate rounded-md bg-transparent px-1 font-mono text-2xs text-muted outline-none hover:bg-hover hover:text-fg"
          >
            {providers.map((p) => (
              <option key={p.id} value={p.id} disabled={!p.ready}>
                {p.label} · {p.model}
                {p.ready ? "" : " (needs key)"}
              </option>
            ))}
            <option value={MANAGE}>Add or manage models…</option>
          </select>
        )}
        {status?.demo && <span className="px-1 font-mono text-2xs text-faint">scripted demo</span>}
        <span
          className="ml-auto font-mono text-2xs text-faint nums"
          title="Tokens used by the last run"
        >
          {usage > 0 ? `${(usage / 1000).toFixed(1)}k tok` : ""}
        </span>
        {fill && (
          <span
            className={`font-mono text-2xs nums ${pct >= 80 ? "text-warn" : "text-faint"}`}
            title={`Context: ${fill.used.toLocaleString()} of ${fill.limit.toLocaleString()} tokens. Above the limit, the oldest tool results are elided (they stay on the graph).`}
          >
            ctx {pct}%
          </span>
        )}
        {running ? (
          <button
            onClick={stop}
            title="Stop"
            className="ease flex h-7 w-7 items-center justify-center rounded-md bg-active text-fg hover:bg-hover"
          >
            <Square size={12} />
          </button>
        ) : (
          <button
            onClick={send}
            disabled={!draft.trim()}
            title="Send (Enter)"
            className="ease flex h-7 w-7 items-center justify-center rounded-md bg-brand text-[#1a1208] disabled:opacity-30"
          >
            <ArrowUp size={14} />
          </button>
        )}
      </div>
    </div>
  );
}
