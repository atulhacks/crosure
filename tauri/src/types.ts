/** Mirrors of the Rust types exchanged over Tauri commands. */

export type StepKind =
  | "load"
  | "recon"
  | "navigate"
  | "functions"
  | "disasm"
  | "decompile"
  | "xref"
  | "strings"
  | "imports"
  | "rename"
  | "comment"
  | "patch"
  | "debug"
  | "sandbox"
  | "shell"
  | "agent"
  | "hypothesis"
  | "finding"
  | "verdict"
  | "annotate";

export type Relation = "next" | "derived_from" | "branch" | "confirms" | "refutes" | "annotates";

export interface ParentRef {
  id: string;
  rel: Relation;
}
export interface Target {
  addr: string | null;
  func: string | null;
  func_fp: string | null;
  section: string | null;
}
export interface Intent {
  chip: string | null;
  note: string | null;
}

export interface Step {
  id: string;
  session_id: string;
  seq: number;
  ts_ms: number;
  parents: ParentRef[];
  actor: { type: "human" | "agent"; id: string; model: string | null };
  kind: StepKind;
  tool: string;
  command: string | null;
  action: Record<string, unknown>;
  target: Target | null;
  observation: { summary: string; blob: string | null; truncated: boolean };
  intent: Intent | null;
  tags: string[];
  prev_hash: string;
  hash: string;
}

export interface Session {
  id: string;
  name: string;
  binary_path: string;
  binary_sha256: string;
  created_ms: number;
  head_hash: string;
}

export interface SectionInfo {
  name: string;
  addr: number;
  size: number;
  file_offset: number | null;
  executable: boolean;
}

export interface BinaryInfo {
  path: string;
  format: string;
  arch: string;
  bits: number;
  little_endian: boolean;
  entry: number;
  size: number;
  sha256: string;
  stripped: boolean;
  sections: SectionInfo[];
}

export interface FunctionInfo {
  addr: number;
  name: string;
  size: number;
  source: string;
}

export interface Instruction {
  addr: number;
  bytes: string;
  mnemonic: string;
  operands: string;
  target: number | null;
  comment: string | null;
}

export interface BlockEdge {
  to: number;
  kind: "taken" | "fall" | "jump";
}
export interface BasicBlock {
  addr: number;
  end: number;
  first: number;
  count: number;
  succs: BlockEdge[];
}

export interface StringRef {
  addr: number;
  mapped: boolean;
  value: string;
  encoding: string;
  section: string | null;
}
export interface Import {
  name: string;
  library: string;
  addr: number | null;
}
export interface Xref {
  from: number;
  to: number;
  kind: "call" | "jump" | "data";
  from_func: string | null;
}

export interface Opened {
  session: Session;
  info: BinaryInfo;
  functions: FunctionInfo[];
}

export interface Outcome {
  step: Step;
  result: unknown;
}

export interface GraphNode {
  id: string;
  seq: number;
  ts_ms: number;
  kind: StepKind;
  actor: "human" | "agent";
  tool: string;
  label: string;
  summary: string;
  command: string | null;
  target: Target | null;
  intent: Intent | null;
  tags: string[];
  dwell_ms: number;
  on_key_path: boolean;
  hash: string;
}

export interface GraphEdge {
  from: string;
  to: string;
  rel: Relation;
}

export interface GraphStats {
  steps: number;
  human_steps: number;
  agent_steps: number;
  by_kind: Record<string, number>;
  dead_ends: number;
  findings: number;
  branches: number;
  duration_ms: number;
}

export interface InvestigationGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  stats: GraphStats;
}

export interface VerifyReport {
  ok: boolean;
  checked: number;
  head_hash: string;
  failure: { seq: number; step_id: string | null; reason: string } | null;
}

export type Op =
  | { op: "info" }
  | { op: "disasm"; target: string }
  | { op: "xrefs_to"; target: string }
  | { op: "xrefs_from"; target: string }
  | { op: "strings"; filter: string | null; min_len: number | null }
  | { op: "imports" }
  | { op: "hex"; target: string; len: number | null }
  | { op: "rename"; target: string; name: string }
  | { op: "comment"; target: string; text: string }
  | { op: "hypothesis"; text: string }
  | { op: "finding"; text: string }
  | { op: "verdict"; verdict: string; family: string | null; text: string };
