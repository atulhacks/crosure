import { create } from "zustand";
import * as api from "../api";
import type { AgentEvent, AgentStatus, Profile, ThreadSummary } from "../types";
import { useWorkbench } from "./workbench";

interface AgentState {
  status: AgentStatus | null;
  /** Everything shown for the current thread. */
  events: AgentEvent[];
  threads: ThreadSummary[];
  currentThread: string | null;
  profile: Profile;
  /** Composer text; other panels can prefill it ("Ask AI about this function"). */
  draft: string;
  running: boolean;
  error: string | null;
  settingsOpen: boolean;
  refreshStatus: () => Promise<void>;
  setSettingsOpen: (open: boolean) => void;
  setDraft: (text: string) => void;
  setProfile: (p: Profile) => void;
  loadThreads: () => Promise<void>;
  openThread: (id: string | null) => Promise<void>;
  start: (prompt: string) => Promise<void>;
  stop: () => Promise<void>;
  decide: (id: string, allow: boolean) => Promise<void>;
  setActive: (providerId: string) => Promise<void>;
}

const POLL_MS = 450;
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Refreshes graph, chain badge and (after renames) the function list. */
async function syncWorkbench(renamed: boolean) {
  const wb = useWorkbench.getState();
  await wb.refreshGraph();
  await wb.verify();
  if (renamed) useWorkbench.setState({ functions: await api.functions() });
}

/** Approval requests that have not been answered yet. */
export function pendingApprovals(events: AgentEvent[]) {
  const resolved = new Set(events.flatMap((e) => (e.type === "approval_resolved" ? [e.id] : [])));
  return events.flatMap((e) =>
    e.type === "approval_requested" && !resolved.has(e.request.id) ? [e.request] : [],
  );
}

/** Latest cumulative token usage in the thread's current run. */
export function latestUsage(events: AgentEvent[]) {
  for (let i = events.length - 1; i >= 0; i--) {
    const e = events[i];
    if (e.type === "usage") return e.input_tokens + e.output_tokens;
  }
  return 0;
}

/**
 * How full the context was on the last request: the server's count of the
 * prompt against the size at which old results are elided. Null before the
 * first reply, or for threads recorded before this was reported.
 */
export function contextFill(events: AgentEvent[]): { used: number; limit: number } | null {
  for (let i = events.length - 1; i >= 0; i--) {
    const e = events[i];
    if (e.type === "usage") {
      return e.context_tokens && e.context_limit
        ? { used: e.context_tokens, limit: e.context_limit }
        : null;
    }
  }
  return null;
}

export const useAgent = create<AgentState>((set, get) => ({
  status: null,
  events: [],
  threads: [],
  currentThread: null,
  profile: "investigate",
  draft: "",
  running: false,
  error: null,
  settingsOpen: false,

  refreshStatus: async () => {
    try {
      set({ status: await api.agentStatus() });
    } catch (e) {
      set({ error: message(e) });
    }
  },
  setSettingsOpen: (settingsOpen) => {
    set({ settingsOpen });
    if (!settingsOpen) get().refreshStatus();
  },
  setDraft: (draft) => set({ draft }),
  setProfile: (profile) => set({ profile }),

  loadThreads: async () => {
    try {
      const list = await api.agentThreads();
      set({ threads: list.threads, currentThread: list.current });
    } catch {
      set({ threads: [], currentThread: null });
    }
  },
  openThread: async (id) => {
    try {
      await api.agentOpenThread(id);
      const page = await api.agentEvents(0);
      const t = get().threads.find((x) => x.id === id);
      set({
        currentThread: id,
        events: page.events,
        error: null,
        ...(t ? { profile: t.profile } : {}),
      });
    } catch (e) {
      set({ error: message(e) });
    }
  },

  start: async (prompt) => {
    set({ error: null });
    try {
      await api.agentStart(prompt, get().profile);
    } catch (e) {
      set({ error: message(e) });
      return;
    }
    set({ running: true, draft: "" });
    await syncWorkbench(false);
    let cursor = 0;
    let fresh = true;
    // Poll until the run ends; each recorded tool call refreshes the graph live.
    while (true) {
      await new Promise((r) => setTimeout(r, POLL_MS));
      let page;
      try {
        page = await api.agentEvents(cursor);
      } catch (e) {
        set({ error: message(e), running: false });
        return;
      }
      cursor = page.next;
      if (fresh || page.events.length) {
        set({ events: fresh ? page.events : [...get().events, ...page.events] });
        fresh = false;
        const calls = page.events.filter((e) => e.type === "tool_call" && e.step_id);
        if (calls.length)
          await syncWorkbench(
            calls.some((e) => e.type === "tool_call" && e.tool === "rename_function"),
          );
      }
      if (!page.running) break;
    }
    set({ running: false });
    await syncWorkbench(false);
    await get().loadThreads();
  },
  stop: async () => {
    await api.agentStop();
  },
  decide: async (id, allow) => {
    try {
      await api.agentDecide(id, allow);
    } catch (e) {
      set({ error: message(e) });
    }
  },
  setActive: async (providerId) => {
    try {
      set({ status: await api.agentSetActive(providerId) });
    } catch (e) {
      set({ error: message(e) });
    }
  },
}));
