import { create } from "zustand";
import * as api from "../api";
import { nextParent } from "../lib/parent";
import type {
  FunctionInfo,
  InvestigationGraph,
  Op,
  Opened,
  Outcome,
  ParentRef,
  Session,
  StepKind,
  VerifyReport,
} from "../types";

/** What the centre panel shows; always tied to the step that produced it. */
export type ViewKind = "disasm" | "xrefs" | "strings" | "imports" | "hex" | "info";
export interface View {
  kind: ViewKind;
  stepId: string;
  result: unknown;
}

export interface ConsoleLine {
  input: string;
  output: string;
  ok: boolean;
}

interface WorkbenchState {
  opened: Opened | null;
  sessions: Session[];
  functions: FunctionInfo[];
  view: View | null;
  graph: InvestigationGraph | null;
  selected: string | null;
  branchFrom: string | null;
  upto: number | null;
  verifyReport: VerifyReport | null;
  consoleLog: ConsoleLine[];
  error: string | null;
  busy: boolean;
  refreshSessions: () => Promise<void>;
  open: (path: string) => Promise<void>;
  resume: (id: string) => Promise<void>;
  close: () => void;
  /** Seq of the newest step (for the timeline), tracked from the live graph. */
  headSeq: number;
  act: (op: Op, opts?: { fromView?: boolean; parent?: ParentRef }) => Promise<Outcome | null>;
  runConsole: (line: string) => Promise<void>;
  selectNode: (id: string | null) => Promise<void>;
  annotate: (stepId: string, chip: string | null, tags: string[]) => Promise<void>;
  setUpto: (n: number | null) => Promise<void>;
  refreshGraph: () => Promise<void>;
  verify: () => Promise<void>;
  exportSession: () => Promise<string | null>;
  clearError: () => void;
}

const VIEW_OF: Partial<Record<StepKind, ViewKind>> = {
  disasm: "disasm",
  xref: "xrefs",
  strings: "strings",
  imports: "imports",
  navigate: "hex",
  recon: "info",
  load: "info",
};

/** Which centre view (if any) an outcome should replace the current one with. */
export function viewFor(outcome: Outcome): View | null {
  const kind = VIEW_OF[outcome.step.kind];
  return kind ? { kind, stepId: outcome.step.id, result: outcome.result } : null;
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

export const useWorkbench = create<WorkbenchState>((set, get) => {
  /** Runs `fn`, capturing errors into state. */
  async function guarded<T>(fn: () => Promise<T>): Promise<T | null> {
    set({ busy: true, error: null });
    try {
      return await fn();
    } catch (e) {
      set({ error: message(e) });
      return null;
    } finally {
      set({ busy: false });
    }
  }

  async function afterOpen(opened: Opened) {
    set({
      opened,
      functions: opened.functions,
      view: null,
      selected: null,
      branchFrom: null,
      upto: null,
      verifyReport: null,
      consoleLog: [],
    });
    await get().refreshGraph();
    await get().verify();
  }

  function parentFor(fromView: boolean) {
    const { branchFrom, graph, view } = get();
    const head = graph?.nodes.at(-1)?.id ?? null;
    return nextParent({ branchFrom, headId: head, viewStepId: view?.stepId ?? null, fromView });
  }

  async function recorded(outcome: Outcome) {
    const view = viewFor(outcome);
    set({ branchFrom: null, selected: outcome.step.id, upto: null, ...(view ? { view } : {}) });
    if (outcome.step.kind === "rename") set({ functions: await api.functions() });
    await get().refreshGraph();
    await get().verify();
  }

  return {
    opened: null,
    sessions: [],
    functions: [],
    view: null,
    graph: null,
    selected: null,
    branchFrom: null,
    upto: null,
    verifyReport: null,
    consoleLog: [],
    error: null,
    busy: false,
    headSeq: 0,

    refreshSessions: async () => {
      const sessions = await guarded(api.listSessions);
      if (sessions) set({ sessions });
    },
    open: async (path) => {
      const opened = await guarded(() => api.openBinary(path));
      if (opened) await afterOpen(opened);
    },
    resume: async (id) => {
      const opened = await guarded(() => api.resumeSession(id));
      if (opened) await afterOpen(opened);
    },
    close: () => set({ opened: null, graph: null, view: null, selected: null }),

    act: async (op, opts = {}) => {
      const parent = opts.parent ?? parentFor(opts.fromView ?? false);
      const outcome = await guarded(() => api.runOp(op, parent));
      if (outcome) await recorded(outcome);
      return outcome;
    },
    runConsole: async (line) => {
      set({ busy: true, error: null });
      try {
        const outcome = await api.runConsole(line, parentFor(false));
        set({
          consoleLog: [
            ...get().consoleLog,
            { input: line, output: outcome.step.observation.summary, ok: true },
          ],
        });
        await recorded(outcome);
      } catch (e) {
        set({ consoleLog: [...get().consoleLog, { input: line, output: message(e), ok: false }] });
      } finally {
        set({ busy: false });
      }
    },

    selectNode: async (id) => {
      if (!id) return set({ selected: null });
      const head = get().graph?.nodes.at(-1)?.id;
      set({ selected: id, branchFrom: id === head ? null : id });
      const outcome = await guarded(() => api.stepOutcome(id));
      const view = outcome && viewFor(outcome);
      if (view) set({ view });
    },
    annotate: async (stepId, chip, tags) => {
      const ok = await guarded(() => api.annotate(stepId, chip, null, tags));
      if (ok) {
        await get().refreshGraph();
        await get().verify();
      }
    },
    setUpto: async (n) => {
      set({ upto: n });
      await get().refreshGraph();
    },
    refreshGraph: async () => {
      const upto = get().upto;
      const graph = await guarded(() => api.graph(upto));
      if (graph)
        set({ graph, ...(upto === null ? { headSeq: graph.nodes.at(-1)?.seq ?? 0 } : {}) });
    },
    verify: async () => {
      const verifyReport = await guarded(api.verify);
      if (verifyReport) set({ verifyReport });
    },
    exportSession: () => guarded(api.exportSession),
    clearError: () => set({ error: null }),
  };
});
