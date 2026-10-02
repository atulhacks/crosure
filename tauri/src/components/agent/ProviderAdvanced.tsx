import { useState } from "react";
import type { Draft } from "./ProviderForm";

const EFFORTS = ["", "none", "low", "medium", "high"] as const;

const field =
  "rounded-md border bg-bg px-2 text-sm outline-none placeholder:text-faint focus:border-brand/60";

/** Parses `Name: value` lines into a header map, skipping blank or malformed lines. */
export function parseHeaders(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const i = line.indexOf(":");
    if (i <= 0) continue;
    const name = line.slice(0, i).trim();
    if (name) out[name] = line.slice(i + 1).trim();
  }
  return out;
}

/** Formats a header map as `Name: value` lines. */
export function formatHeaders(h: Record<string, string> | undefined): string {
  return Object.entries(h ?? {})
    .map(([k, v]) => `${k}: ${v}`)
    .join("\n");
}

/** Collapsible per-provider options: reasoning effort, output-limit field, custom headers. */
export function ProviderAdvanced({
  draft,
  set,
}: {
  draft: Draft;
  set: (patch: Partial<Draft>) => void;
}) {
  const [headers, setHeaders] = useState(() => formatHeaders(draft.headers));
  const openai = draft.kind === "openai_compatible";
  const effortHint = openai
    ? "Sent as reasoning_effort. Leave on default for models without reasoning."
    : /api\.anthropic\.com/i.test(draft.base_url)
      ? "Claude effort level; none turns thinking off."
      : "Turns on extended thinking with a matching token budget.";
  return (
    <details className="rounded-md border px-2 py-1.5 text-sm">
      <summary className="cursor-pointer text-muted">Advanced</summary>
      <div className="mt-2 flex flex-col gap-2">
        <label className="flex flex-col gap-1">
          <span className="label">Reasoning effort</span>
          <select
            className={`${field} h-7`}
            value={draft.reasoning_effort ?? ""}
            onChange={(e) => set({ reasoning_effort: e.target.value || null })}
          >
            {EFFORTS.map((e) => (
              <option key={e} value={e}>
                {e || "default"}
              </option>
            ))}
          </select>
          <span className="text-2xs text-faint">{effortHint}</span>
        </label>
        {openai && (
          <label
            className="flex items-center gap-1.5 text-muted"
            title="OpenAI reasoning models reject max_tokens; always on for api.openai.com"
          >
            <input
              type="checkbox"
              checked={draft.max_completion_tokens ?? false}
              onChange={(e) => set({ max_completion_tokens: e.target.checked })}
            />{" "}
            Send max_completion_tokens instead of max_tokens
          </label>
        )}
        <label className="flex flex-col gap-1">
          <span className="label">Custom headers</span>
          <textarea
            className={`${field} min-h-14 py-1 font-mono text-xs`}
            value={headers}
            placeholder={"X-Gateway-Tag: crosure\nHelicone-Auth: Bearer …"}
            onChange={(e) => {
              setHeaders(e.target.value);
              set({ headers: parseHeaders(e.target.value) });
            }}
          />
          <span className="text-2xs text-faint">
            One per line. Authorization, x-api-key and other headers Crosure sets are ignored.
          </span>
        </label>
      </div>
    </details>
  );
}
