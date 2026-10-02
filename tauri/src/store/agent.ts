import { create } from "zustand";
import * as api from "../api";
import type { AgentEvent, AgentStatus } from "../types";
import { useWorkbench } from "./workbench";

interface AgentState {
  status: AgentStatus | null;
  events: AgentEvent[];
  running: boolean;
  error: string | null;
  refreshStatus: () => Promise<void>;
  settingsOpen: boolean;
  setSettingsOpen: (open: boolean) => void;
  start: (prompt: string) => Promise<void>;
  stop: () => Promise<void>;
}

const POLL_MS = 450;

/** Refreshes graph, chain badge and (after renames) the function list. */
async function syncWorkbench(renamed: boolean) {
  const wb = useWorkbench.getState();
  await wb.refreshGraph();
  await wb.verify();
  if (renamed) useWorkbench.setState({ functions: await api.functions() });
}

export const useAgent = create<AgentState>((set, get) => ({
  status: null,
  events: [],
  running: false,
  error: null,
  refreshStatus: async () => {
    try {
      set({ status: await api.agentStatus() });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  settingsOpen: false,
  setSettingsOpen: (settingsOpen) => {
    set({ settingsOpen });
    if (!settingsOpen) get().refreshStatus();
  },
  start: async (prompt) => {
    set({ error: null });
    try {
      await api.agentStart(prompt);
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) });
      return;
    }
    set({ events: [], running: true });
    await syncWorkbench(false);
    let cursor = 0;
    // Poll until the run ends; each recorded tool call refreshes the graph live.
    while (true) {
      await new Promise((r) => setTimeout(r, POLL_MS));
      let page;
      try {
        page = await api.agentEvents(cursor);
      } catch (e) {
        set({ error: String(e), running: false });
        return;
      }
      cursor = page.next;
      if (page.events.length) {
        set({ events: [...get().events, ...page.events] });
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
  },
  stop: async () => {
    await api.agentStop();
  },
}));
