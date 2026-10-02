# Crosure — Project Plan (Innoventure 4.0, Amity University)

> **One line:** Crosure is a reverse-engineering workbench that records *every
> step an analyst takes* as a live, replayable, tamper-evident graph, and
> turns those graphs into playbooks, tutorials, and the training data for
> the next generation of reverse-engineering and malware-analysis models.

---

## Build status (Oct 2026)

| Tier 1 item | State |
| --- | --- |
| 1. Open PE/ELF; functions, disasm, strings, imports, hex | Done (native engine). **Decompile pending** (rizin backend next) |
| 2. Console; every command and UI action recorded | Done (built-in console with r2-style aliases; rizin console comes with the rizin backend) |
| 3. Live Investigation Canvas, tags, click-to-jump | Done (click an old step → next action branches from it) |
| 4. Intent chips | Done |
| 5. Hash chain + "Verify session" | Done (tamper shows the exact step) |
| 6. Timeline replay | Done |
| 7. Export | Session JSON done. **SFT/DPO/trajectory exporters pending** (`crosure-dataset`) |

Crates built so far: `crosure-recorder`, `crosure-engine`, `crosure-graph`,
`crosure-session` (the single "run op + record step" choke point, used by
the app and later by the agent), plus the Tauri app with a dev HTTP bridge
for browser previews.

---

## Engine decision (Oct 2026, after running both side by side)

We built and ran Recurse locally on the same benign crackme and compared its
engine with ours.

| Check | Recurse engine | Crosure native engine |
| --- | --- | --- |
| Functions that begin with `endbr64` (GCC default on modern Linux) | Split in two: `check_password` reported as **4 bytes**, body becomes a fake `fcn_11dd` (also `fcn_118d`, `fcn_122b`) | Correct |
| PLT import stubs | `fcn_1090` | `strcmp@plt` |
| Disassembly annotations | Raw call targets | Callee names and string literals inline |
| Decompiler | Register-level pseudo-C: `__asm("...")` lines, calls as `call(0x1070)`, strings as `&[rip+0xdb8]` | None yet |
| Breadth (CFG, call graph, DWARF/PDB, debugger, capa, many archs) | Much wider | Narrower (CFG added now) |

**Decision:** keep Crosure's own engine as the core and close the gap:
1. Done: a native basic-block CFG (Linear and Graph views).
2. Next: a **rizin + rz-ghidra** backend behind the same `Engine` trait, for
   real Ghidra-quality decompilation, the same way Recurse treats r2/IDA as
   optional backends.

Not adopted: Recurse's `recurse-static` as a dependency. It isn't on
crates.io (it would be a git dependency on a competitor's repo), its function
discovery is wrong on standard binaries, its decompiler wouldn't improve
ours much, and it would blur what is original in Crosure.

---

## 1. What Recurse actually is (and what it is not)

We studied `Recurse-Labs/recurse` (commit `60a6741`, Oct 2026, ~65k LOC,
Apache-2.0) end to end.

| Area | What Recurse has |
| --- | --- |
| Shell | Tauri 2 desktop app, React 19 + TypeScript, Tailwind v4 + shadcn/ui, Zustand, `@xyflow/react` + dagre/elk |
| Engine | One `Engine` trait (`recurse-static/src/engine.rs`) with 3 backends: pure-Rust **native** (`object` + `capstone`), **radare2**, **IDA Pro** |
| Analysis | Functions, disasm, CFG, xrefs, strings, imports, recon (hashes, entropy, checksec), DWARF/PDB, C++ RTTI, FLIRT-like sigs, capa-like rules, binary diff, firmware carving |
| IL / decompiler | `recurse-vtil`: VTIL-style lifter, optimizer, C-like decompiler, taint, symbolic exec, optional emulation |
| Debugger | `recurse-debug`: ptrace/Mach/Win32 + gdb-remote |
| AI agent | `recurse-agent`: LLM tool-calling loop (OpenRouter/Anthropic/Ollama), SQLite + FTS5 memory, a claim-verification harness |
| Other | MCP server, Python bindings, eval harness with per-turn traces of the *agent* |

**Recurse is "Cursor for reverse engineering"**: an AI agent does the
reversing and a human checks it. It does **not**:

- record the **human analyst's** workflow as a step graph,
- show the *investigation* as a canvas (its graphs are the binary's CFG and call graph),
- keep dead-ends or the *reasons* behind steps, or allow replay,
- turn investigations into playbooks, tutorials, or a **training dataset**,
- make the investigation record tamper-evident for incident response or legal use.

Crosure doesn't try to out-build Recurse's engine. It competes on **what
happens around the engine**: capturing, understanding, teaching and learning
from how experts reverse.

> Licensing: Recurse is Apache-2.0, so reuse is legal if we keep its
> `LICENSE`/`NOTICE`. Check Innoventure's originality rules. This plan
> assumes our own code, in the same stack (Rust + TypeScript + Python),
> on top of mature open-source pieces (capstone, object, rizin).

---

## 2. The problem we solve

1. **Expert knowledge evaporates.** A senior analyst spends 6 hours on a
   sample. The report keeps the *answer*, but the *process* (what they
   checked, in what order, what they ruled out, why) is gone.
2. **Training new analysts is slow.** Students learn reversing by watching
   videos or reading write-ups, which are cleaned up after the fact. They
   never see the real path with its wrong turns.
3. **AI models can't learn the process.** LLMs have seen write-ups and
   code, but almost no data of *step-by-step expert actions on real binaries*.
   That's why RE agents flail on real malware.
4. **Investigations aren't auditable.** In incident response, "how did you
   reach this conclusion" matters. Today it's notes and screenshots.

**Crosure's answer:** record the process once, then reuse it four ways:
as a **graph** (understand), a **playbook** (automate), a **tutorial**
(teach), and a **dataset** (train).

---

## 3. The full idea: six pillars

```
   CAPTURE  →  GRAPH  →  INSIGHT  →  INTELLIGENCE
      │                     │              │
      └──────── LEARN ──────┴──── HUB ─────┘
```

### Pillar 1 — Capture (the recorder)
Everything that can be a step becomes a step, with as little effort for the analyst as possible.

- **In-app actions:** navigate, disasm, decompile, xrefs, strings, rename,
  comment, patch. Each one goes through one `record(step)` call.
- **Console commands:** an embedded rizin/r2 console. Every typed command is a node.
- **External tools (plugins):** Ghidra, x64dbg, IDA and gdb plugins stream
  actions into Crosure over a local WebSocket. Analysts keep their favorite
  tool, and Crosure records it.
- **Dynamic analysis:** sandbox runs (API calls, files, registry, network)
  and debugger events (breakpoint hit, memory read) become nodes too.
- **Intent capture ("why"):** the scarcest and most valuable data.
  - one-click **intent chips** after a step: *find C2*, *find
    decryption*, *check persistence*, *verify hunch*, *dead end*;
  - optional typed note;
  - optional **voice "think-aloud"**: push-to-talk, transcribed locally
    with Whisper and attached to the current node.
- **Implicit signals:** time spent on a view (dwell), scrolling, which
  functions were hovered or revisited. Never content, only attention.

### Pillar 2 — The Investigation Graph
- **Node = one step.** **Edges:** `next` (time), `derived_from` (data
  dependency: string → xref → function), `branch` (went back and tried
  something else), `confirms`/`refutes` (a step that proves or kills a hypothesis).
- **Hypothesis nodes:** the analyst can pin a hypothesis ("this is a
  config decryptor") and later mark it confirmed or refuted. This turns the
  graph into a *reasoning* graph, not just a log.
- **Tamper-evident (hash-chained):** each node stores
  `hash = sha256(prev_hash + canonical_json(node))`. A session has one root
  hash. Edit any old step and verification fails. This gives incident
  response a **chain of custody**, and the dataset gets **provenance**.
- **Branching and merging:** two analysts on the same sample can merge
  their graphs. Shared steps collapse, and different paths show side by side.

### Pillar 3 — Insight (making the graph useful right away)
- **Attention heatmap:** color the function list and call graph by how
  much time experts spent there. "Where to look first" for a new analyst.
- **Graph diff:** compare two investigations of the same or similar
  samples. Shows what one analyst checked that the other missed.
- **Cross-sample memory:** every function has a structural fingerprint
  (normalized-mnemonic simhash). Open a new sample and Crosure says: *"this
  function matches `rc4_decrypt`, renamed by an analyst in session #12
  (family: AgentTesla)."* Findings carry across samples and families.
- **Playbook mining:** find the step sequences that recur across many
  successful sessions (frequent-subsequence mining over node kinds and
  ops). Example: *strings matching URL → xrefs → decompile caller → find
  decrypt routine*. These become named **playbooks**.
- **Playbook replay:** run a playbook on a new binary. Targets are resolved
  by pattern, not by address ("the string that looks like a URL"), so
  automated triage reproduces expert steps and the results become nodes.

### Pillar 4 — Intelligence (models)
A ladder that is useful from day one and gets better with more data:

1. **Retrieval copilot (day one):** embed each node's state + intent +
   observation. In a new session, find similar past states and suggest
   "experts did *X* next here", with a link to the original session as evidence.
2. **Fine-tuned small model:** LoRA on an open model (Qwen2.5-Coder 1.5B–7B)
   with recorded trajectories. It predicts the next tool call and its reason.
   Runs locally, so samples never leave the machine.
3. **Preference tuning (DPO):** expert's successful path = *chosen*, their
   dead-end = *rejected*. Teaches the model to avoid the traps experts fell into.
4. **Crosure Gym (RL):** binaries with checkable answers (crackme flag,
   known C2 address, known family) turn into an environment where an agent
   is rewarded for correct, verified answers. Recorded human graphs are
   the warm start.
5. **Agent mode:** an LLM drives the same tools. Its steps are `actor: agent`
   nodes in the **same graph**, so human and AI work side by side, and the
   human can accept, reject or redirect any AI step (and that is data too).
6. **Auto-report:** the key-step path plus confirmed hypotheses become a
   malware report: summary, IOCs, MITRE ATT&CK mapping, and evidence links
   to nodes.

### Pillar 5 — Learn mode (education)
Very relevant for a university hackathon.

- **Expert replay as a tutorial:** step through a real expert session, with
  the expert's intent and voice note at each node and dead-ends included.
- **Challenge mode:** a student solves a crackme or sample while recorded.
  Afterwards Crosure **compares their graph with expert graphs**:
  - *coverage*: how many expert key steps did they hit (matched by kind +
    function fingerprint),
  - *efficiency*: steps and time to verdict,
  - *missed leads*: what experts checked that they didn't.
- **Hints:** the copilot gives graded hints ("look at imports" → "look at
  `CryptDecrypt` callers" → exact step).
- **Teacher dashboard:** class progress, common dead-ends, with every
  student's sessions also feeding the dataset (with consent).

### Pillar 6 — Hub (sharing and community)
- **Share sessions:** anonymized graphs (sample hash only, no binaries,
  no paths/usernames), license + consent flag on each.
- **Private org hubs:** SOC/CERT teams share internally only.
- **Reputation:** analysts earn reputation when their sessions are reused,
  forked, or turned into playbooks. This sets data quality weights.
- **CrosureBench:** public benchmark built from held-out expert sessions.
  Models are scored on "pick the next expert action" and "reach the
  verdict", with a leaderboard.

---

## 4. Step (node) schema, v1

```jsonc
{
  "id": "stp_01J9...",               // ULID, sortable by time
  "session_id": "ses_...",
  "parents": [{ "id": "stp_...", "rel": "next" | "derived_from" | "branch" | "confirms" | "refutes" }],
  "ts": "2026-10-02T10:14:03.120Z",
  "actor": { "type": "human" | "agent", "id": "anon_7f3a", "model": null },
  "kind": "xref",                    // load|recon|navigate|disasm|decompile|xref|strings|imports|
                                     // rename|comment|patch|debug|sandbox|shell|agent|hypothesis|finding|verdict
  "tool": "crosure" | "rizin" | "ghidra" | "x64dbg" | "ida" | "gdb" | "sandbox",
  "command": "axt 0x404020",
  "action": { "op": "xrefs", "target": "0x404020", "direction": "to" },
  "target": { "addr": "0x404020", "func": "sub_401200", "func_fp": "simhash:9c1e...", "section": ".rdata" },
  "observation": { "summary": "3 refs from sub_401200, sub_4013a0, main", "blob": "sha256:...", "truncated": false },
  "intent": { "chip": "find_c2", "note": "Who uses the URL string?", "voice_blob": null },
  "hypothesis_ref": "stp_...",       // which hypothesis this step tests, if any
  "tags": ["lead", "dead_end", "key_step"],
  "attention": { "dwell_ms": 8400, "revisits": 2 },
  "env": { "binary_sha256": "...", "arch": "x86-64", "format": "PE" },
  "prev_hash": "sha256:...",
  "hash": "sha256:..."
}
```

**Session labels** (at the end): verdict, family, capabilities (ATT&CK IDs),
IOCs, key-step path, analyst skill level (self-declared + reputation),
consent and license.

---

## 5. The dataset

| Export | Built from | Trains |
| --- | --- | --- |
| **Trajectories** `(state, action, observation)` | each path in the graph | agents, RL warm start |
| **SFT chat with tool calls** | state → expert's next command + intent | next-step models |
| **Rationale pairs** | step + intent chip / note / voice | "explain why" models |
| **Preference pairs (DPO)** | key path (chosen) vs dead end (rejected) at each branch | avoiding traps |
| **Attention maps** | dwell per function | "where to look" ranking |
| **Graph dataset** | full DAG + labels | family / capability prediction from investigation shape |
| **Function knowledge** | fingerprint → names, comments | auto-naming functions |
| **CrosureBench** | held-out sessions | evaluation |

**Quality controls:** reputation-weighted samples, dedup by sample hash and
graph similarity, hash-chain verification (no edited logs), anomaly checks
against poisoning (graphs that never reach a verdict, mass-identical
uploads), and a dataset card with each export.

---

## 6. Architecture (same stack as Recurse: Rust + TypeScript + Python)

We use the same family of technologies as Recurse: a **Tauri 2** desktop app
with a **React + TypeScript** frontend, a **Rust** Cargo workspace for
everything performance- or integrity-critical, and **Python** where its
ecosystem wins (ML training, plugins for Python-scriptable RE tools, voice).

```
┌──────────────────── Tauri 2 desktop app ─ frontend: React 19 + TypeScript ────────────────────┐
│ Workbench │ Console │ Investigation Canvas │ Timeline │ Heatmap │ Learn │ Dataset │ Hub          │
└────────────────┬──────────── Tauri commands (invoke) + events ("step-recorded") ──────────────┘
                 │
┌────────────────▼──────────── tauri/src-tauri (Rust app glue) ─────────────────────────────────┐
│ commands.rs · sessions.rs · events.rs: thin layer that calls the crates below                  │
└───┬──────────────┬──────────────┬──────────────┬──────────────┬──────────────┬────────────────┘
    │              │              │              │              │              │
 crosure-       crosure-       crosure-       crosure-       crosure-       crosure-
 engine         recorder       graph          insight        copilot        learn / dataset
 (analysis)     (steps, hash   (edges,        (fingerprints, (LLM, RAG,     (scoring,
                chain, store)  replay, diff)  heatmap, play- agent, report) exporters)
                                              books)
    │                                                                           │
 crosure-ingest: local WebSocket (127.0.0.1:7878) ◄── Ghidra / x64dbg / IDA / gdb / sandbox plugins
 crosure-py:     pyo3 bindings ──► python/crosure_ml (training, eval, CrosureBench), voice sidecar
                 │
   SQLite (rusqlite, bundled, FTS5) + content-addressed blob store (sha256)   ~/.crosure/
```

### Rust crates (Cargo workspace)

| Crate | Job | Key dependencies |
| --- | --- | --- |
| `crosure-engine` | `Engine` trait + backends. **native** (ELF/PE/Mach-O parsing, multi-arch disasm, functions, xrefs, CFG, strings, imports); **rizin** (child process, JSON commands, decompile via rz-ghidra, raw console); **ida** (optional, headless) | `object`, `capstone`, `yara-x`, `serde_json` |
| `crosure-recorder` | `Step` schema, `record(step)`, validation, **hash chain**, SQLite + blob store, chain verification. Depends on nothing else in the workspace | `serde`, `rusqlite`, `sha2`, `ulid` |
| `crosure-graph` | Edges (`next`/`derived_from`/`branch`/`confirms`/`refutes`), hypotheses, replay cursor, graph diff and merge, attention aggregation | `petgraph` |
| `crosure-insight` | Function fingerprints (normalized-mnemonic simhash), cross-sample memory, playbook mining (frequent subsequences) and replay | `crosure-engine`, `crosure-graph` |
| `crosure-copilot` | OpenAI-compatible LLM client (cloud or Ollama), local embeddings + retrieval, agent loop whose tool calls are recorded as `actor: agent` steps, report generator | `reqwest`, `tokio`, `fastembed` (ONNX) |
| `crosure-learn` | Challenges, score a student graph against expert graphs, graded hints | `crosure-graph`, `crosure-insight` |
| `crosure-dataset` | Exporters: trajectories, SFT chat, rationale, DPO pairs, graph, dataset card | `serde_json` |
| `crosure-ingest` | Local WebSocket server that turns plugin events into steps | `tokio-tungstenite` |
| `crosure-py` | Python bindings: open the store, iterate sessions, call exporters | `pyo3` |
| `crosure-mcp` *(roadmap)* | MCP server so outside agents use the engine, with every call recorded | `crosure-engine`, `crosure-recorder` |

Rules borrowed from Recurse's setup that we keep: no crate depends on Tauri,
each crate builds and tests on its own, `clippy` denies `unwrap`/`panic`,
and the `dev` profile builds dependencies at `opt-level = 3` so analysis
isn't slow in development.

### TypeScript (frontend, `tauri/src`)
React 19 + TypeScript + Vite, Tailwind v4 + shadcn/ui, Zustand stores,
**@xyflow/react** + dagre (Investigation Canvas and CFG), `@tanstack/react-virtual`
(big listings), Monaco (decompile view), Vitest + ESLint + Prettier.

### Python (`python/`)
| Package | Job | Key dependencies |
| --- | --- | --- |
| `crosure_ml` | LoRA fine-tuning, DPO, evaluation, CrosureBench scoring. Reads data through `crosure-py` | `transformers`, `peft`, `trl`, `unsloth`, `datasets` |
| `crosure_voice` | Local speech-to-text sidecar for think-aloud notes | `faster-whisper` |
| `plugins/ida`, `plugins/ghidra` | Stream actions to `crosure-ingest` | IDAPython, PyGhidra |
| `plugins/x64dbg` | Same, for the debugger | x64dbg Python plugin (or C++) |

### Tooling
`just` as the single entry point (`just dev`, `just test`, `just lint`),
`cargo clippy/fmt/test`, `npm` + Vitest/ESLint/Prettier, `uv` + `ruff` +
`pytest` for Python, GitHub Actions CI running all three.

### Repo layout
```
crosure/
  Cargo.toml  justfile
  crates/
    crosure-engine/  crosure-recorder/  crosure-graph/  crosure-insight/
    crosure-copilot/ crosure-learn/     crosure-dataset/ crosure-ingest/  crosure-py/
  tauri/
    src/            React + TypeScript frontend
    src-tauri/      Rust app glue (commands, events)
  python/
    crosure_ml/  crosure_voice/
  plugins/
    ghidra/  ida/  x64dbg/
  samples/        benign crackmes only, never live malware
  docs/           PLAN.md, schema.md, demo-script.md
```

### Keeping Rust realistic for a hackathon
- **Rizin backend first.** It gives decompile and a console on day one.
  The native engine covers fast parsing, disasm, strings, imports and
  fingerprints. A home-grown decompiler (what Recurse built with VTIL) is
  out of scope.
- **The recorder crate is the priority.** It's small, has no other
  dependencies, and is the core of Crosure. Build and test it first.
- **Keep crates thin and split by owner** so four people don't block each
  other. The Tauri layer only wires things together.
- **Python stays off the app's critical path.** The app runs without
  Python; Python is for training and plugins.

| Layer | Choice |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 19 + TypeScript + Vite, Tailwind v4 + shadcn/ui, @xyflow/react, Zustand, Monaco |
| Core | Rust workspace (crates above), tokio, serde |
| RE engine | native (`object` + `capstone`) + rizin backend (+ rz-ghidra) + optional IDA; `yara-x` |
| Storage | SQLite via `rusqlite` (bundled, FTS5) + content-addressed blobs |
| AI (in app) | OpenAI-compatible API or Ollama via `reqwest`; local embeddings via `fastembed` |
| AI (training) | Python: transformers, PEFT, TRL, Unsloth; faster-whisper for voice |
| Plugins | IDAPython, PyGhidra, x64dbg |
| Safety | Static by default. Dynamic only in an isolated VM. Samples zipped (`infected`). Never commit live malware |

---

## 7. Scope: what we build when

### Tier 1 — MVP (must work on demo day)
1. Open PE/ELF. Panes: functions, disasm, decompile, strings, imports, hex.
2. Embedded rizin console. Every command and UI action is recorded.
3. **Live Investigation Canvas** with tags (lead/dead-end/key-step) and click-to-jump.
4. **Intent chips** after each step.
5. **Hash chain + "Verify session" button.**
6. **Timeline replay.**
7. **Export:** JSONL (SFT + DPO pairs), graph JSON, Markdown report.

### Tier 2 — Headline features for the demo (pick 3–4)
8. **Retrieval copilot:** "experts did X next".
9. **Learn mode:** challenge, then a score against the expert graph.
10. **Attention heatmap** on the function list.
11. **Cross-sample memory:** "this function was named `rc4_decrypt` in session #12".
12. **Hypothesis nodes** (pin, confirm, refute).

### Tier 3 — Roadmap (show on slides, build if time allows)
13. Agent mode (AI steps in the same graph). 14. Playbook mining + replay.
15. Ghidra / x64dbg plugins. 16. Voice think-aloud. 17. Graph diff/merge.
18. Sandbox ingest. 19. Fine-tuned LoRA model + DPO. 20. Hub + reputation.
21. Crosure Gym + CrosureBench.

---

## 8. Timeline and team

**Prep (~2 weeks before the event)**
- Days 1–2: lock the schema, set up the repo skeleton and CI, collect crackmes + benign samples.
- Days 1–2 also: Cargo workspace, Tauri app shell, `justfile`, CI (cargo + npm + uv).
- Days 3–5: `crosure-recorder` (schema, hash chain, SQLite) and `crosure-engine` (rizin backend first, then native parsing/disasm).
- Days 6–9: workbench, live canvas, intent chips, replay.
- Days 10–12: **record 20–30 real sessions ourselves** (the seed dataset),
  fingerprints, retrieval index. Optional LoRA run on a cloud GPU.
- Days 13–14: Learn mode scoring, heatmap, export.

**Event (24–36 h)**
- H0–8: polish, hypothesis nodes, cross-sample memory.
- H8–16: copilot UI, Learn-mode screen, dataset view.
- H16–24: report generator, agent mode if time allows.
- H24–end: bug bash, demo rehearsal, backup video, slides.

| Role | Owns |
| --- | --- |
| Engine (Rust) | `crosure-engine`, `crosure-insight` fingerprints, `crosure-ingest`, safety |
| Recorder/Data (Rust) | `crosure-recorder`, `crosure-graph`, `crosure-dataset`, `crosure-learn`, Tauri commands |
| Frontend (TypeScript) | workbench, canvas, timeline, heatmap, Learn screens |
| AI/Pitch (Rust + Python) | `crosure-copilot`, `crosure-py`, `python/crosure_ml` LoRA run, report, slides and demo |

---

## 9. Demo script (6 minutes)

1. **Problem (30 s):** expert process vanishes, students learn from cleaned-up
   write-ups, models never see real RE steps.
2. **Capture (90 s):** open a sample. Strings → xref → decompile → rename. The
   **canvas grows live**, with intent chips on each step. Hit a dead end,
   branch back, pin a hypothesis, confirm it.
3. **Trust (20 s):** "Verify session" turns green. Edit a step in the DB and it turns red.
4. **Insight (60 s):** open a *second* sample of the same family. Crosure
   says "this function = `rc4_decrypt` from session #12". The heatmap shows
   where experts looked.
5. **Intelligence (60 s):** the copilot suggests the next step with evidence.
   Auto-report with IOCs + ATT&CK.
6. **Learn (60 s):** a student's attempt is scored against the expert graph,
   with missed leads highlighted.
7. **Data + vision (40 s):** click Export, show SFT + DPO samples. "Thousands
   of analysts give the largest expert RE trajectory dataset."

---

## 10. Impact and business model

- **Who uses it:** SOC/CERT teams, malware researchers, universities and
  training academies, CTF communities, AI labs building security models.
- **Measurable impact:** time-to-verdict (with vs without copilot), student
  coverage score improvement, number of reusable playbooks.
- **Model:** open-core desktop app (free) → **Team/Enterprise hub**
  (private sharing, audit chain, SSO) → **Education licenses** (Learn mode,
  dashboards) → **dataset licensing** to AI labs (consented, anonymized,
  revenue share with contributors).

---

## 11. Risks and ethics

| Risk | Mitigation |
| --- | --- |
| Live malware on our machines | Static by default, dynamic only in isolated VMs, zipped samples, no samples in git |
| Dual use (data helps attackers evade) | Share graphs, not binaries. Gated access to the hub/dataset. Abuse policy |
| Privacy of analysts | Opt-in per session, anonymized IDs, no paths/usernames, attention data only as aggregates |
| Low-quality or poisoned data | Reputation weighting, hash chain, anomaly checks, dataset cards |
| Recording feels intrusive / slows analysts | Recording is passive. Intent chips are one click and optional. Pause button |
| Scope too big for a hackathon | Strict tiers (Section 7). Tier 3 lives on slides |

---

## 12. Defaults we assume (change if needed)

- Team of 4, a 24–36 h event with ~2 weeks of prep.
- Same stack as Recurse: Tauri 2 desktop app, React + TypeScript frontend, Rust core crates, Python for ML and plugins.
- LLM: any OpenAI-compatible API for the demo, with Ollama as the offline fallback.
- Plugins, voice, sandbox and fine-tuning are roadmap unless prep goes fast.
