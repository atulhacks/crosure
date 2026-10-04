import { RefreshCw } from "lucide-react";
import { useState } from "react";
import * as api from "../../api";
import type { KeySource, ModelInfo, ProviderConfig } from "../../types";
import { Button } from "../ui/Button";
import { ProviderAdvanced } from "./ProviderAdvanced";

/** Draft of one provider being edited; `api_key` set only when the user typed one. */
export type Draft = ProviderConfig & { key_source?: KeySource; ready?: boolean };

/**
 * The config to send to the backend: just the provider fields. `api_key` is
 * sent only when the user typed (or cleared) one, so a saved key is kept.
 */
export function toConfig(d: Draft): ProviderConfig {
  return {
    id: d.id,
    kind: d.kind,
    label: d.label,
    base_url: d.base_url,
    model: d.model,
    api_key: d.api_key === undefined ? null : d.api_key,
    key_env: d.key_env,
    strict_tools: d.strict_tools,
    enabled: d.enabled,
    max_completion_tokens: d.max_completion_tokens ?? false,
    reasoning_effort: d.reasoning_effort || null,
    headers: d.headers ?? {},
    context_window: d.context_window ?? null,
    max_output: d.max_output ?? null,
  };
}

/** True for servers on this machine, which need no key. */
export function isLocal(url: string): boolean {
  return /:\/\/(localhost|127\.0\.0\.1|\[::1\])/i.test(url);
}

/**
 * Whether the draft can run: enabled, has an endpoint and a model, and has a
 * key (saved, from the environment, just typed) unless it is local. Mirrors
 * the backend.
 */
export function isReady(d: Draft): boolean {
  if (!d.enabled || !d.base_url.trim() || !d.model.trim()) return false;
  if (isLocal(d.base_url)) return true;
  if (d.api_key !== undefined && d.api_key !== null) return d.api_key.trim() !== "";
  return d.key_source === "saved" || d.key_source === "env";
}

const KEY_HINT: Record<KeySource, string> = {
  saved: "A key is saved. Type a new one to replace it, or clear to remove.",
  env: "Using the key from the environment.",
  not_needed: "Local server: no key needed.",
  missing: "No key yet.",
};

function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: React.ReactNode;
  hint?: string;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="label">{label}</span>
      {children}
      {hint && <span className="text-2xs text-faint">{hint}</span>}
    </label>
  );
}

const input =
  "h-7 rounded-md border bg-bg px-2 text-sm outline-none placeholder:text-faint focus:border-brand/60";

/** Edit one provider: endpoint, model (fetched from the server), key, options. */
export function ProviderForm({
  draft,
  onChange,
  onRemove,
}: {
  draft: Draft;
  onChange: (d: Draft) => void;
  onRemove: () => void;
}) {
  const [models, setModels] = useState<ModelInfo[] | null>(null);
  const [probe, setProbe] = useState<{ busy: boolean; error: string | null }>({
    busy: false,
    error: null,
  });
  const set = (patch: Partial<Draft>) => onChange({ ...draft, ...patch });

  const fetchModels = async () => {
    setProbe({ busy: true, error: null });
    try {
      setModels(await api.agentListModels(toConfig(draft)));
      setProbe({ busy: false, error: null });
    } catch (e) {
      setModels(null);
      setProbe({ busy: false, error: e instanceof Error ? e.message : String(e) });
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-2 gap-3">
        <Field label="Name">
          <input
            className={input}
            value={draft.label}
            onChange={(e) => set({ label: e.target.value })}
          />
        </Field>
        <Field label="Protocol">
          <span className="flex h-7 items-center text-sm text-muted">
            {draft.kind === "anthropic" ? "Anthropic Messages API" : "OpenAI-compatible"}
          </span>
        </Field>
      </div>
      <Field label="Base URL">
        <input
          className={`${input} font-mono text-xs`}
          value={draft.base_url}
          placeholder={
            draft.kind === "anthropic"
              ? "https://api.z.ai/api/anthropic (calls <url>/v1/messages)"
              : "https://host/v1 (calls <url>/chat/completions)"
          }
          onChange={(e) => set({ base_url: e.target.value })}
        />
      </Field>
      <Field label="Model">
        <div className="flex gap-1.5">
          <input
            className={`${input} min-w-0 flex-1 font-mono text-xs`}
            value={draft.model}
            list={`models-${draft.id}`}
            placeholder="Type a model id or fetch the list"
            onChange={(e) => {
              // A listed model brings the limits its server reports.
              const m = models?.find((m) => m.id === e.target.value);
              set(
                m && (m.context_window || m.max_output)
                  ? { model: m.id, context_window: m.context_window, max_output: m.max_output }
                  : { model: e.target.value },
              );
            }}
          />
          <datalist id={`models-${draft.id}`}>
            {models?.map((m) => (
              <option key={m.id} value={m.id} />
            ))}
          </datalist>
          <Button
            onClick={fetchModels}
            disabled={probe.busy}
            title="Ask the server which models it offers (also tests the connection)"
          >
            <RefreshCw size={12} className={probe.busy ? "animate-spin" : ""} /> Fetch models
          </Button>
        </div>
        {models && (
          <span className="text-2xs text-good">Connected · {models.length} models available</span>
        )}
        {probe.error && <span className="text-2xs text-bad">{probe.error}</span>}
      </Field>
      <Field
        label="API key"
        hint={`${KEY_HINT[draft.api_key === undefined ? (draft.key_source ?? "missing") : "missing"]}${draft.key_env ? ` Env: ${draft.key_env}.` : ""}`}
      >
        <div className="flex gap-1.5">
          <input
            type="password"
            className={`${input} min-w-0 flex-1 font-mono text-xs`}
            value={draft.api_key ?? ""}
            placeholder={
              draft.key_source === "saved"
                ? "•••••••• (saved)"
                : draft.key_source === "not_needed"
                  ? "not needed"
                  : "paste key"
            }
            onChange={(e) => set({ api_key: e.target.value })}
          />
          {draft.key_source === "saved" && (
            <Button variant="ghost" onClick={() => set({ api_key: "" })}>
              Clear
            </Button>
          )}
        </div>
      </Field>
      <ProviderAdvanced draft={draft} set={set} />
      <div className="flex items-center gap-4 text-sm">
        <label className="flex items-center gap-1.5 text-muted">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => set({ enabled: e.target.checked })}
          />{" "}
          Enabled
        </label>
        {draft.kind === "openai_compatible" && (
          <label
            className="flex items-center gap-1.5 text-muted"
            title="Only for servers that support strict JSON-schema tools (OpenAI)"
          >
            <input
              type="checkbox"
              checked={draft.strict_tools}
              onChange={(e) => set({ strict_tools: e.target.checked })}
            />{" "}
            Strict tool schemas
          </label>
        )}
        <Button variant="ghost" className="ml-auto text-bad hover:text-bad" onClick={onRemove}>
          Remove
        </Button>
      </div>
    </div>
  );
}
