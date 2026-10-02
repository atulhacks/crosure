# Crosure

**Every reverse, remembered.** Crosure is a reverse-engineering workbench that
records every step an analyst takes as a live, replayable, **tamper-evident**
investigation graph, so investigations can be audited, taught, and turned
into training data for reverse-engineering and malware-analysis models.

Built for Innoventure 4.0 (Amity University). Full plan: [docs/PLAN.md](docs/PLAN.md).

![Crosure workbench](docs/media/workbench.png)

## What works today

- **Open ELF / PE / Mach-O binaries** with a pure-Rust engine (`object` + Capstone):
  functions (symbols, entry, call targets, code pointers, PLT/IAT import
  stubs, so it works on stripped binaries too), annotated disassembly,
  strings (ASCII + UTF-16LE), imports, cross-references, hex.
- **Every action is a recorded step.** Clicking a function, following a call,
  searching strings, typing a console command (`dis main`, `xt strcmp`,
  `str http` …), renaming, adding a hypothesis or finding. Each one is a node with:
  - the exact command,
  - what it targeted,
  - a summary of what it showed,
  - the full result stored by content hash.
- **Native control-flow graph:** each function splits into basic blocks with
  taken / not-taken / jump edges. Switch Disassembly between **Linear** and
  **Graph**.
- **Investigation canvas** (React Flow). The graph grows live and edges are typed:
  - `next`: time order;
  - `derived_from`: you clicked something in a previous result;
  - `branch`: you jumped back to an older step and tried something else.

  The path to each finding is highlighted, and dead ends fade out.
- **Intent chips and tags.** One click records *why* a step was taken
  (*find C2*, *find crypto*, …) and how it turned out (lead, dead end, key
  step). Annotations are new steps and never edit old ones.
- **Hash chain.** Each step stores `sha256(prev_hash + canonical step)`. The
  verify badge re-derives the whole chain. Edit any stored step and it turns red:

  ![Tamper detected](docs/media/tampered.png)
- **Replay and flight log.** A scrubber shows the graph as it was after any
  step, and a **Log** view lists every step in time order with its command
  (`dis check_password`, `xt strcmp@plt`), outcome, intent and tags.
- **IDE shell:**
  - resizable panes;
  - tabbed results, where each tab links back to the step that produced it;
  - a status bar;
  - a **Ctrl+K** command palette (go to a function, run an action or a console command);
  - a console toggle (Ctrl+\`);
  - dark and light themes.

  ![Flight log](docs/media/flight-log.png)
- **Export.** Writes the session (steps, graph, verification) to `~/.crosure/exports/*.json`.
- **Sessions persist** in SQLite (`~/.crosure/crosure.db`) and can be resumed.
  Renames are replayed from the log.

## Design

Crosure's look is a quiet IDE with one accent colour, *recorder amber*. It is
used only for recording, the key path and focus. The rules:

- **Colour carries meaning.** All colours are theme tokens in `tauri/src/theme.css`:
  - a desaturated disassembly palette;
  - one dot colour per step kind;
  - green and red for taken and not taken.
- **Type and size.**
  - A type scale of 10/11/12/13 px, with tabular numbers in every column.
  - Inter and JetBrains Mono are bundled, so the desktop app renders the same offline.
  - One `Pane` primitive gives every docked surface the same header, count and spacing.

## Layout

```
crates/
  crosure-recorder/   step schema, hash chain, SQLite + blob store, verify   (no other workspace deps)
  crosure-engine/     Engine trait + native backend (ELF/PE/Mach-O, x86/x64/ARM/AArch64)
  crosure-graph/      investigation graph: typed edges, annotations folded, key paths, replay, stats
  crosure-session/    runs ops on the engine and records each one; console parser
tauri/
  src/                React 19 + TypeScript frontend (Tailwind v4, React Flow, Zustand)
  src-tauri/          Tauri 2 app: thin commands over crosure-session; optional dev bridge
docs/PLAN.md          full product plan
```

## Run it

Prerequisites: Rust ≥ 1.77, Node ≥ 20, and the
[Tauri Linux packages](https://v2.tauri.app/start/prerequisites/)
(`libwebkit2gtk-4.1-dev` etc.). Optionally install [`just`](https://github.com/casey/just).

```bash
just dev        # desktop app (cd tauri && npm install && npm run tauri dev)
just preview    # same UI in a browser: dev bridge on :1421 + Vite on :1420
just test       # cargo test --workspace + vitest
just lint       # clippy -D warnings + eslint
```

`CROSURE_HOME` overrides the data directory (default `~/.crosure`).

Try it on the bundled benign crackme:
`crates/crosure-engine/tests/fixtures/crackme-x64` (also `-stripped` and `.exe`).

## Console

```
info                         binary info
dis <fn|addr>                disassemble function
xt <fn|addr>                 xrefs to
xf <fn|addr>                 xrefs from function
str [filter]                 strings
imp                          imports
hex <addr> [len]             hex dump
ren <fn|addr> <name>         rename function
cmt <addr> <text>            comment
hyp <text>                   pin a hypothesis
find <text>                  record a finding
verdict <malicious|benign|unknown> <family|-> <text>
```

r2-style aliases also work (`pdf`, `axt`, `iz`, `ii`, `px`, `afn`).

## Safety

Analysis is static by default and nothing is executed. Never commit live malware
(see [samples/README.md](samples/README.md)).

## License

Apache-2.0
