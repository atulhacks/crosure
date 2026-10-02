# Crosure — Project Plan (Innoventure 4.0, Amity University)

> **One line:** Crosure is a reverse-engineering workbench that records *every
> step an analyst takes* as a live, replayable graph, and turns those graphs
> into a structured dataset for training and evaluating reverse-engineering
> and malware-analysis models.

---

## 1. What Recurse actually is (and what it is not)

We studied `Recurse-Labs/recurse` (commit `60a6741`, Oct 2026, ~65k LOC,
Apache-2.0) end to end. Summary:

| Area | What Recurse has |
| --- | --- |
| Shell | Tauri 2 desktop app, React 19 + TypeScript, Tailwind v4 + shadcn/ui, Zustand stores, `@xyflow/react` + dagre/elk for graphs |
| Engine | A single `Engine` trait (`recurse-static/src/engine.rs`) with 3 backends: pure-Rust **native** (`object` + `capstone`, ELF/PE/Mach-O/WASM), **radare2**, **IDA Pro** (headless `idat`) |
| Analysis | Functions, disassembly, CFG, xrefs, strings, imports, recon (hashes, entropy, checksec), DWARF/PDB, C++ RTTI/vtables, FLIRT-like signatures, capa-like capability rules, binary diffing, firmware carving, kernel-driver IOCTLs |
| IL / decompiler | `recurse-vtil`: VTIL-style IL lifter, optimizer, structured C-like decompiler, taint analysis, symbolic exec, optional unicorn emulation |
| Debugger | `recurse-debug`: ptrace/Mach/Win32 + gdb-remote, breakpoints, stepping, watchpoints |
| AI agent | `recurse-agent`: LLM tool-calling loop (OpenAI-compatible / OpenRouter / Anthropic / local Ollama), one `analyze` tool, SQLite + FTS5 memory, a "verify" harness that re-checks agent claims |
| Other | `recurse-mcp` (MCP server over the engine), `recurse-py` (Python bindings), `recurse-eval` (crackme eval tiers, per-turn JSON traces of the *agent*) |

**The key point:** Recurse is "Cursor for reverse engineering", where **an AI
agent does the reversing** and a human checks it. It does **not**:

- record the **human analyst's** workflow as a structured step graph,
- show that workflow live on a canvas, node by node (its graphs are the
  CFG and call graph of the *binary*, not of the *investigation*),
- let you replay, branch, annotate "why" or mark dead-ends on those steps,
- export investigations as a **training dataset** (its README even says
  "do not train on eval tasks"; its only traces are agent eval logs),
- aggregate many analysts' sessions into a corpus for models.

So the core of your idea — **the investigation graph as data** — is still
open. Crosure should not try to out-build Recurse's engine (they have
months of Rust work). It should win on the thing they don't do.

> Licensing note: Recurse is Apache-2.0. Reusing its code is legal if we keep
> its `LICENSE`/`NOTICE` and mark changes, but check Innoventure's rules on
> original work. Plan below assumes we build our own code and use mature
> open-source engines (rizin/radare2, capstone, LIEF) underneath.

---

## 2. Positioning

| | Recurse | **Crosure** |
| --- | --- | --- |
| Who does the RE | AI agent, human verifies | **Human (and optionally agent)**, every step recorded |
| Main graph | Binary's CFG / call graph | **Investigation graph** (steps, commands, findings) + CFG |
| Output | Answers, renames, reports | Answers **plus a reusable dataset** of expert trajectories |
| Learning | None from users | Every session improves suggestions / trains models |
| Pitch | "Cursor for RE" | **"Flight recorder + black box for malware analysis"** |

Tagline options: *"Every reverse, remembered."* / *"Turn analyst
intuition into training data."*

---

## 3. Core concept: the Investigation Graph

Each analysis session is a **directed graph** of steps.

- **Node = one step.** One command or UI action plus what came back.
- **Edges:**
  - `next`: time order (A then B),
  - `derived_from`: data dependency (string `"cmd.exe"` → xrefs → function
    `0x401200` → decompile),
  - `branch`: the analyst went back to an older node and tried something
    else. That makes it a tree, not a line, so dead-ends are kept.
- **Node kinds:** `load`, `recon`, `navigate`, `disasm`, `decompile`, `xref`,
  `strings`, `imports`, `rename`, `comment`, `patch`, `debug` (break, step,
  read memory), `shell` (r2/rizin console command), `agent` (LLM step),
  `hypothesis`, `finding`, `verdict`.

### Step (node) schema, v0

```jsonc
{
  "id": "stp_01J9...",            // ULID, sortable by time
  "session_id": "ses_...",
  "parents": ["stp_..."],          // derived_from / branch sources
  "ts": "2026-10-02T10:14:03.120Z",
  "actor": "human" | "agent",
  "kind": "xref",
  "tool": "crosure" | "rizin" | "ghidra" | "x64dbg" | "gdb",
  "command": "axt 0x404020",       // exact raw command if any
  "action": { "op": "xrefs", "target": "0x404020", "direction": "to" },
  "target": { "addr": "0x404020", "func": "sub_401200", "section": ".rdata" },
  "observation": {                 // what the analyst saw
    "summary": "3 refs from sub_401200, sub_4013a0, main",
    "digest": "sha256:...",        // full result stored separately
    "truncated": false
  },
  "intent": "Who uses the 'cmd.exe' string?",   // optional, 1-click or typed
  "tags": ["lead", "dead_end", "key_step"],
  "duration_ms": 8400,             // time spent before next step
  "env": { "binary_sha256": "...", "arch": "x86-64", "format": "PE" }
}
```

A session also carries **labels** set at the end: verdict
(malicious/benign), family, capabilities (ATT&CK IDs), IOCs, and which nodes
were key to the conclusion. Those labels are what make the data useful for
supervised training.

---

## 4. Product features

### MVP (must work on demo day)
1. **Open a binary** (PE/ELF), with safe static analysis only; nothing runs by default.
2. **Workbench panes:** function list, disassembly, decompile (via rizin's
   `pdg`/r2ghidra or `pdc`), strings, imports, hex, CFG.
3. **Integrated console**: a rizin/r2 command line. Every command typed
   becomes a node.
4. **Recorder:** every UI action and console command goes through one
   `record(step)` call, so nothing goes unlogged.
5. **Live Investigation Canvas:** React Flow graph that grows as you work.
   Click a node to jump back to that view; color by kind; mark
   `lead`/`dead_end`/`key_step`.
6. **Timeline replay:** a scrubber that replays the session step by step.
7. **Export:** session to JSONL / graph JSON / Markdown report.
8. **Dataset view:** sessions list, stats (steps, kinds, time per step),
   one-click export of the training dataset (Section 6).

### Stretch (great for judges)
9. **"Next step" copilot:** retrieval over past recorded graphs (embeddings
   of node `intent` + `observation`). It suggests what experts did next in
   similar situations. This works without fine-tuning, so it is demo-safe.
10. **Agent mode:** an LLM drives the same tools. Its steps are recorded
    as `actor: agent` nodes in the same graph, side by side with human ones.
11. **Auto-report:** turn the key-step path into a malware report with
    IOCs and ATT&CK mapping.
12. **Capture from external tools:** small plugins for Ghidra, x64dbg and
    IDA that stream their actions into Crosure's recorder (port 7878,
    JSON over WebSocket).
13. **Small fine-tuned model:** LoRA on a small open model (Qwen2.5-Coder-1.5B
    / Llama-3.2-3B) using recorded trajectories. Do this **before** the
    hackathon and show the results; don't train live.

---

## 5. Architecture

```
┌───────────────────────── Frontend (React + TS + Vite) ─────────────────────────┐
│ Workbench panes │ Console │ Investigation Canvas (React Flow) │ Timeline │ Dataset │
└──────────────┬────────────────────────── WebSocket / REST ──────────────────────┘
               │
┌──────────────▼──────────── Backend: Python (FastAPI) ───────────────────────────┐
│ api/            REST + WS (live node push)                                       │
│ engine/         adapter over rizin (rzpipe) + LIEF + capstone                     │
│ recorder/       record(step) → validate → store → broadcast                       │
│ graph/          build edges (next / derived_from / branch), replay, diff          │
│ dataset/        exporters: SFT chat, trajectories, preference pairs, graph JSON   │
│ copilot/        embeddings + retrieval over past steps; optional LLM agent        │
│ plugins/        ingest from Ghidra / x64dbg / IDA / gdb                           │
└──────────────┬──────────────────────────────────────────────────────────────────┘
               │
        SQLite (sessions, steps, edges, labels) + blob store (full outputs, by sha256)
```

**Why this stack (and not Recurse's Rust/Tauri):** it's a hackathon. Python
gives us rzpipe, LIEF, capstone, pefile, yara-python, and ML tooling
(sentence-transformers, HF, Unsloth) in one place. We reuse a mature engine
(rizin) and spend our time on the recorder, graph and dataset, which is
where Crosure differs from Recurse. We can wrap the web app in Tauri or
Electron later for a desktop build.

| Layer | Choice |
| --- | --- |
| Frontend | React + TypeScript + Vite, Tailwind + shadcn/ui, **@xyflow/react** (canvas), dagre (layout), Zustand, Monaco (code views) |
| Backend | Python 3.11, FastAPI, uvicorn, websockets, pydantic |
| RE engine | rizin + `rzpipe` (disasm, xrefs, CFG, `pdg` decompile via rz-ghidra), LIEF/pefile (headers), capstone (fallback) |
| Storage | SQLite (+ FTS5), content-addressed blob folder |
| AI | sentence-transformers (embeddings), any OpenAI-compatible LLM / Ollama, Unsloth/PEFT for LoRA |
| Safety | Static-only by default; debugging only inside a VM/container; samples stored zipped with password `infected` |

### Proposed repo layout

```
crosure/
  frontend/            React app
  backend/
    crosure/api/  engine/  recorder/  graph/  dataset/  copilot/  plugins/
    tests/
  plugins/             ghidra/  x64dbg/  ida/  (stretch)
  samples/             benign crackmes only (no live malware in git)
  docs/                PLAN.md, schema.md, demo-script.md
```

---

## 6. The dataset: what we export and why it matters

From one recorded graph we generate several training formats:

1. **Trajectories (agent / RL style):** `(state, action, observation)`
   sequences along each path. They teach a model *how to proceed*.
2. **SFT chat format:** a user turn ("binary context + what's known so far"),
   and an assistant turn with a tool call (the expert's next command) and
   the rationale (`intent`). Usable directly with OpenAI-style tool-calling
   fine-tunes.
3. **Preference pairs (DPO):** at each `branch` point, the path that led to
   a `key_step`/`finding` is *chosen* and the `dead_end` path is *rejected*.
   This is the data nobody else has: experts' wrong turns.
4. **Graph dataset:** full DAGs with labels, for GNN/structure models
   ("predict family / capability from investigation shape").
5. **Eval sets:** held-out sessions become benchmarks ("given this state,
   does the model pick an action an expert would?").

**Privacy and safety:** we store hashes and not raw samples by default, and
strip usernames and local paths. Analysts opt in per session. Exports carry
a license and consent flag.

---

## 7. Hackathon timeline (adjust to actual dates)

**Before the event (prep, ~1–2 weeks)**
- Day 1–2: lock the step schema, set up the repo skeleton, CI, sample crackmes.
- Day 3–5: backend engine adapter (rizin) and recorder with SQLite.
- Day 6–8: frontend workbench and live canvas over WebSocket.
- Day 9–10: record 10–20 real sessions ourselves on crackmes and benign
  samples. This is our seed dataset. Optionally run a LoRA on it.

**During the event (24–36 h)**
- H0–6: polish replay and timeline, node tagging, export.
- H6–14: dataset view and exporters (SFT, DPO pairs, graph).
- H14–22: "Next step" copilot (retrieval), agent mode if time allows.
- H22–28: auto-report, UI polish, bug bash.
- H28–end: demo script rehearsal, slides, backup video.

### Team split (4 people)
| Role | Owns |
| --- | --- |
| Backend/RE | rizin adapter, engine API, safety |
| Recorder/Data | schema, recorder, graph builder, exporters |
| Frontend | workbench panes, canvas, timeline, dataset view |
| AI/Pitch | copilot retrieval, LoRA experiment, report, slides and demo |

---

## 8. Demo script (5 minutes)

1. Problem: malware analysis knowledge lives in experts' heads and is lost
   after every investigation. Models can't learn the *process*.
2. Open a crackme/sample. Analyst does strings → xref → decompile → rename →
   finding. **The canvas grows live.**
3. Show a dead end, branch back, and mark the key path.
4. Replay the session with the timeline scrubber.
5. Click **Export**: show the JSONL SFT sample and a DPO pair.
6. Open a new sample: the **copilot suggests the next step** based on past
   graphs.
7. Vision: thousands of analysts give the largest expert RE trajectory
   dataset; specialized models cut triage time.

---

## 9. Open questions for the team

- Hackathon dates and duration, and team size? (They set the MVP cut line.)
- Desktop (Tauri) or web-first? (Recommendation: web-first.)
- Python backend OK, or does the team prefer Rust/Node?
- Which LLM access do we have (OpenAI/Anthropic key, or local Ollama only)?
- Must the external-tool plugins (Ghidra/x64dbg) be in the demo, or are they roadmap?
