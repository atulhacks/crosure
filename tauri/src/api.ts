import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type {
  AgentEventPage,
  AgentSettings,
  AgentStatus,
  ProviderConfig,
  SettingsView,
  FunctionInfo,
  InvestigationGraph,
  Op,
  Opened,
  Outcome,
  ParentRef,
  Session,
  Step,
  VerifyReport,
} from "./types";

/** True inside the desktop app; false in a plain browser (dev preview). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const BRIDGE = "http://127.0.0.1:1421";

/**
 * Calls a backend command: Tauri IPC in the app, or the dev bridge
 * (`crosure-devbridge`) when running in a browser.
 */
async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const res = await fetch(`${BRIDGE}/invoke/${cmd}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(args),
  });
  const body = await res.json();
  if (!res.ok) throw new Error(typeof body === "string" ? body : JSON.stringify(body));
  return body as T;
}

/** Opens a binary and starts a new recorded session. */
export const openBinary = (path: string) => invoke<Opened>("open_binary", { path });

/** Lists every recorded session, newest first. */
export const listSessions = () => invoke<Session[]>("list_sessions");

/** Reopens a recorded session. */
export const resumeSession = (id: string) => invoke<Opened>("resume_session", { id });

/** Functions of the open binary with renames applied. */
export const functions = () => invoke<FunctionInfo[]>("functions");

/** Runs an op from the UI; the backend records it as a step. */
export const runOp = (op: Op, parent: ParentRef | null) =>
  invoke<Outcome>("run_op", { op, parent });

/** Runs a console line; the backend records it as a step. */
export const runConsole = (line: string, parent: ParentRef | null) =>
  invoke<Outcome>("run_console", { line, parent });

/** Console help text. */
export const consoleHelp = () => invoke<string>("console_help");

/** Adds intent/tags to a step (recorded as a new annotate step). */
export const annotate = (
  stepId: string,
  chip: string | null,
  note: string | null,
  tags: string[],
) => invoke<Step>("annotate", { stepId, chip, note, tags });

/** The current investigation graph, optionally as of step `upto`. */
export const graph = (upto: number | null) => invoke<InvestigationGraph>("graph", { upto });

/** Re-derives the session's hash chain. */
export const verify = () => invoke<VerifyReport>("verify");

/** Writes the session to a JSON file and returns its path. */
export const exportSession = () => invoke<string>("export_session");

/** Re-opens an earlier step's stored result (does not record anything). */
export const stepOutcome = (stepId: string) => invoke<Outcome>("step_outcome", { stepId });

/** Agent setup status (the key itself is never returned). */
export const agentStatus = () => invoke<AgentStatus>("agent_status");

/** Provider settings (keys are never returned, only where each comes from). */
export const agentSettings = () => invoke<SettingsView>("agent_settings");

/** Saves provider settings. A provider without `api_key` keeps its saved key; "" clears it. */
export const agentSaveSettings = (settings: AgentSettings) =>
  invoke<SettingsView>("agent_save_settings", { settings });

/** Lists a provider's models; doubles as a connection test. */
export const agentListModels = (provider: ProviderConfig) =>
  invoke<string[]>("agent_list_models", { provider });

/** Starts the agent on the open binary; its steps are recorded like yours. */
export const agentStart = (prompt: string) => invoke<null>("agent_start", { prompt });

/** Agent events since a cursor. */
export const agentEvents = (since: number) => invoke<AgentEventPage>("agent_events", { since });

/** Asks the agent to stop after its current call. */
export const agentStop = () => invoke<null>("agent_stop");
