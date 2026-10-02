import clsx from "clsx";
import { ArrowDown, ArrowUp, Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import * as api from "../../api";
import { useAgent } from "../../store/agent";
import type { SettingsView } from "../../types";
import { Button, IconButton } from "../ui/Button";
import { isReady, ProviderForm, toConfig, type Draft } from "./ProviderForm";

/** Which AI providers the agent may use, which runs first, and fallback on decline. */
export function AgentSettingsDialog() {
  const { settingsOpen, setSettingsOpen } = useAgent();
  const [view, setView] = useState<SettingsView | null>(null);
  const [drafts, setDrafts] = useState<Draft[]>([]);
  const [active, setActive] = useState("");
  const [autoFallback, setAutoFallback] = useState(true);
  const [sel, setSel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!settingsOpen) return;
    api.agentSettings().then(
      (v) => {
        setView(v);
        setDrafts(v.providers.map((p) => ({ ...p, api_key: undefined })));
        setActive(v.active);
        setAutoFallback(v.auto_fallback);
        setSel(v.active || v.providers[0]?.id || null);
      },
      (e) => setError(String(e)),
    );
  }, [settingsOpen]);

  if (!settingsOpen || !view) return null;
  const current = drafts.find((d) => d.id === sel);
  const update = (d: Draft) => setDrafts(drafts.map((x) => (x.id === d.id ? d : x)));
  const move = (i: number, dir: -1 | 1) => {
    const j = i + dir;
    if (j < 0 || j >= drafts.length) return;
    const next = [...drafts];
    [next[i], next[j]] = [next[j], next[i]];
    setDrafts(next);
  };
  const add = (presetId: string) => {
    const p = view.presets.find((x) => x.id === presetId);
    if (!p) return;
    let id = p.id;
    for (let n = 2; drafts.some((d) => d.id === id); n++) id = `${p.id}-${n}`;
    setDrafts([...drafts, { ...p, id, key_source: "missing", ready: false }]);
    setSel(id);
  };
  const save = async () => {
    try {
      const providers = drafts.map(toConfig);
      await api.agentSaveSettings({ providers, active, auto_fallback: autoFallback });
      setSettingsOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onMouseDown={() => setSettingsOpen(false)}
    >
      <div
        className="flex h-[560px] w-[860px] flex-col overflow-hidden rounded-lg border border-line-strong bg-elevated shadow-2xl"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="flex h-11 shrink-0 items-center border-b px-4">
          <span className="text-sm font-semibold">AI providers</span>
          <span className="ml-3 text-xs text-muted">
            The agent starts on the active provider. If a model declines, the run can continue on
            the next one.
          </span>
          <IconButton className="ml-auto" onClick={() => setSettingsOpen(false)}>
            <X size={14} />
          </IconButton>
        </header>
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-64 shrink-0 flex-col border-r">
            <div className="scroll-host min-h-0 flex-1 overflow-auto py-1">
              {drafts.map((d, i) => (
                <div
                  key={d.id}
                  onClick={() => setSel(d.id)}
                  className={clsx(
                    "group flex cursor-pointer items-center gap-2 px-3 py-1.5",
                    sel === d.id ? "bg-active" : "hover:bg-hover",
                  )}
                >
                  <input
                    type="radio"
                    name="active"
                    checked={active === d.id}
                    onChange={() => setActive(d.id)}
                    title="Start runs on this provider"
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm">{d.label}</span>
                    <span className="block truncate font-mono text-2xs text-faint">
                      {d.model || "no model"}
                    </span>
                  </span>
                  <span
                    className={clsx(
                      "h-1.5 w-1.5 rounded-full",
                      isReady(d) ? "bg-good" : "bg-faint",
                    )}
                    title={isReady(d) ? "Ready" : "Needs a model or key"}
                  />
                  <span className="flex flex-col opacity-0 group-hover:opacity-100">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        move(i, -1);
                      }}
                      className="text-faint hover:text-fg"
                    >
                      <ArrowUp size={10} />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        move(i, 1);
                      }}
                      className="text-faint hover:text-fg"
                    >
                      <ArrowDown size={10} />
                    </button>
                  </span>
                </div>
              ))}
            </div>
            <label className="flex items-center gap-1.5 border-t px-3 py-2">
              <Plus size={12} className="text-faint" />
              <select
                value=""
                onChange={(e) => add(e.target.value)}
                className="min-w-0 flex-1 bg-transparent text-sm text-muted outline-none"
              >
                <option value="">Add provider…</option>
                {view.presets.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </label>
          </aside>
          <section className="scroll-host min-w-0 flex-1 overflow-auto p-4">
            {current ? (
              <ProviderForm
                key={current.id}
                draft={current}
                onChange={update}
                onRemove={() => {
                  setDrafts(drafts.filter((d) => d.id !== current.id));
                  setSel(drafts[0]?.id ?? null);
                }}
              />
            ) : (
              <p className="text-sm text-muted">Add a provider to get started.</p>
            )}
          </section>
        </div>
        <footer className="flex h-12 shrink-0 items-center gap-3 border-t px-4">
          <label className="flex items-center gap-1.5 text-sm text-muted">
            <input
              type="checkbox"
              checked={autoFallback}
              onChange={(e) => setAutoFallback(e.target.checked)}
            />
            If a model declines, continue the run on the next ready provider (list order)
          </label>
          {error && <span className="truncate text-xs text-bad">{error}</span>}
          <div className="ml-auto flex gap-2">
            <Button variant="ghost" onClick={() => setSettingsOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={save}>
              Save
            </Button>
          </div>
        </footer>
      </div>
    </div>
  );
}
