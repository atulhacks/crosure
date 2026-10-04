# Improvement review: radare2, rizin, Ghidra, Zed

October 2026. This review compares Crosure with the current source of
radare2, rizin (`dev`), Ghidra (`master`) and Zed. It covers:
- function discovery, xrefs and jump tables (r2, rizin);
- command declarations and project formats (rizin, r2);
- function ID and similarity (Ghidra FunctionID, BSim, Version Tracking);
- agent robustness (Zed).

Every claim the decisions rest on was checked against our own code. Where it
could be measured, it was measured.

## Method

Engine accuracy is measured, not argued. `tools/fnbench.py` scores function
discovery on stripped binaries against the unstripped originals' symbol
tables: start precision, start recall, and exact size. rizin `aaa` is run on
the same files as a reference.

The corpus is real code:
- cabextract + libmspack, built at -O2 with gcc, with clang, and with mingw (PE);
- a gcc build without unwind tables;
- `librz_util.so`, built at -O3.

## Results

Each cell reads *before → after* for Crosure, with rizin `aaa` on the same
stripped file in the last column.

| Stripped binary | Starts found | Exact sizes | rizin `aaa` starts |
| --- | --- | --- | --- |
| gcc -O2 ELF | 65.8% → **98.8%** | 7.7% → **100%** | 48.2% |
| clang -O2 ELF | 64.5% → **98.8%** | 6.1% → **100%** | 46.2% |
| gcc, no unwind tables | ~66% → **90.4%** | ~8% → **91.7%** | 48.2% |
| mingw PE | 75.3% → **99.6%** | n/a (COFF has no sizes) | 60.9% |
| librz_util.so -O3 | 95.8% → **100%** | 92.0% → **100%** | 93.0% |

Precision is 100% everywhere except the PE, where it is 99.6%. The one ELF
miss is `register_tm_clones`, a CRT function that only a tail jump reaches.

**Why the jump.** Stripped ELF files keep `.eh_frame`, and its entries give
the exact start and size of every compiled function. On these builds that
was 100% of starts and 100% of sizes. PE x64 has the same information in
`.pdata`, for every function except leaves. Neither r2 nor rizin uses either
table to find functions: both use them only for exception handling. That is
the main reason their recall on stripped binaries is low.

The remaining misses were functions reached only through pointers stored in
data. The loader's relocations list those pointers, so Crosure now reads them
as well.

## Done in this round

| Change | Evidence |
| --- | --- |
| Function ranges from `.eh_frame` (gimli) and `.pdata` (chained entries skipped); code pointers from `R_*_RELATIVE` and PE `DIR64` relocations; inferred starts inside a known range dropped (switch labels, `call $+5`); `.plt` FDEs ignored; padding trimmed when there is no unwind data | Table above; `crates/crosure-engine/tests/extents.rs` |
| `Workspace::resume` refuses a file whose sha256 differs from the session's, so new steps can never attach to a different binary | `resume_refuses_a_changed_binary` |
| A reply cut off by the output limit no longer bricks the thread. Before, its unanswered `tool_use` blocks stayed in the transcript and every provider rejected the next request | `a_reply_cut_off_mid_tool_call_does_not_break_the_thread` |
| Thread files are written atomically. An unreadable file is set aside as `*.corrupt-<ms>`; before, it was treated as empty and overwritten, erasing every thread in the session | `an_unreadable_file_is_set_aside_not_overwritten` |
| The store format version is kept in `PRAGMA user_version`, with an ordered migration list. A store from a newer Crosure is refused, not misread | `crates/crosure-recorder/tests/format.rs` |

## Done in the second round

| Change | Evidence |
| --- | --- |
| **One op registry** (`crosure-session/src/table.rs`). It generates console parsing and help, `Op::command()`, the agent/MCP tool schemas and parser, the read-only profile and the confirm preset. Fixed the drift it found: `hex` round-trip, verdict values, unreachable `min_len`, the missing `functions` in the TS type. | Schema snapshot identical before and after; round-trip test over every op; TS type check |
| **Paging** with `offset` / `--from`, recorded in the step. **Context budget**: the oldest large tool results are elided (lossless, they are recorded steps); a "prompt too long" error trims and retries once. | `tests/context.rs`: run bounded, pairs intact, overflow recovered; exact next offset |
| **Retries in the agent loop**: only temporary failures (408/429/5xx except 501/529, network), backoff with jitter, `retry-after` honoured, each wait shown, Stop works during waits and between tool calls | `tests/retry.rs`; status classification against a mock server |
| **Function fingerprints** (FID-style, `fid1:`) on every step target; store format v2 indexes them. **`recall`** lists what other sessions recorded about the same code; ambiguous matches are flagged, nothing is applied automatically. | Relinked build: 71/71 identical after fixing offset signs; collisions in librz_util are all genuinely identical code; recall test across stripped/unstripped copies |
| **Leakage-safe splits** by binary and shared significant code (library code excluded); duplicate SFT examples dropped; Python `load_dataset(split=…)` | `crates/crosure-dataset/tests/split.rs`; Python test |

### ARM, measured

cabextract, stripped, built with `aarch64-linux-gnu-gcc` and
`arm-linux-gnueabihf-gcc` (Thumb-2) at -O2.

**AArch64** (scored with `tools/armbench.py`):
- string references: 0% → **100%** (67/67);
- PLT stubs named: 0 → **51/51**;
- function starts and exact sizes: **100%** (rizin `aaa` finds 67.9%).

**ARM32 / Thumb-2:**
- function starts: 1.1% → **88.6%**, at 100% precision (rizin `aaa`
  54.5%);
- exact sizes: 89.2%;
- PLT stubs named: 0 → **52/52**;
- literal-pool string references: **18/18**.

The fixes:
- `native/arm64.rs` pairs `adrp` with its `add`/`ld*`/`st*`.
- `native/arm32.rs` follows literal pools and pc-relative `add` chains.
- Thumb code is detected from the odd entry point or odd symbols, and
  decoded as Thumb, except in `.plt`.
- ARM conditional branches are recognized.
- `R_ARM_RELATIVE` pointers are read from the data.

### Strings, every architecture

The scanner read whole files, so runs of instruction bytes became
"strings": 461 of 638 in the gcc build of cabextract. A string inside code
is now kept only if code takes its address. Strings in data sections, file
headers and overlays are kept as before. After the fix:
- cabextract has **177** strings on x86-64, 175 on ARM64 and 177 on ARM32;
- the strings code references are 71 / 67 / 67: the same source gives the
  same answer on each architecture.

## Known limits

- ARM-mode functions inside a Thumb binary are decoded as Thumb. Here
  that is 3 of 92 functions, all C runtime startup code.
- No noreturn propagation or jump-table recovery yet. These matter only
  when there are no unwind tables: a gcc build without them reaches 90.4%
  of function starts.
- Mach-O `LC_FUNCTION_STARTS` is not read. There is no Mach-O corpus to
  measure it on.

## Next, in order

Items 1–5 below are done (see the tables above). The remaining items are
the known limits just listed.

**1. One declarative op registry.** Effort: M.

Each operation is currently described by hand in several places: the `Op`
enum, `Op::command()`, the console parser and help, the agent tool schemas
and parser, `WRITE_TOOLS`, and the TypeScript types. They have already
drifted:
- `hex` writes `hex <addr> 256` when no length was given, so the recorded
  command does not round-trip.
- The console help lists 3 verdicts but accepts any string. The agent
  accepts 4, including `suspicious`.
- `strings` has a `min_len` that neither the console nor the agent can set.

rizin solved the same problem with one descriptor per command, from which it
generates parsing, validation and help (`librz/core/cmd_descs/*.yaml`,
`cmd_api.c`). The plan is a `const` table in `crosure-session/src/registry.rs`
that generates the console grammar and help, the agent and MCP schemas, and
the list of write tools. Current tool and field names stay as aliases,
because they already appear in recorded datasets. A round-trip test runs over
every op: console → `Op` → tool JSON.

**2. Agent context budget.** Effort: M.

Today the whole transcript is resent on every turn. Once it exceeds the
model's window, the run fails and the thread is dead. Zed summarizes old
turns with an extra model call (`agent/src/thread.rs`). Crosure can do
better, losslessly: every tool result is a recorded step. Above about 70% of
the window, the oldest results are replaced by
`[elided: step <id>, <summary>; re-run to view]`, in large chunks so the
prompt cache survives.

Paging belongs with this. `disassemble`, `decompile` and `search_strings`
get `offset` and `count`, the truncation note names the next offset, and the
`recon` output is capped.

**3. Interruptible, visible retries.** Effort: S.

The current state:
- Retries sleep for up to 60s and cannot be stopped.
- The request timeout is 600s.
- Stop is only checked between turns.

Zed sleeps in short slices that check for cancellation and shows a retry
status. Each tool call left unrun then gets a "canceled" result, so every
`tool_use` is still answered.

**4. FID-style masked function hash.** Effort: M. This needs exact extents,
which item 1 of this round delivered.

The hash, following Ghidra FunctionID (`MessageDigestFidHasher.java`):
- Hash the opcode bytes with address operands masked out.
- Keep a second, "specific" hash that also includes small constants.
- Skip functions shorter than 4 instructions.
- Accept a match only above FID's score threshold (14.6), using callee
  matches.

This enables two things:
- **Cross-session recall.** "You investigated this function in another
  binary: finding …". It is offered as a recorded suggestion, never applied
  automatically.
- **Leakage-safe dataset splits.** Trajectories are grouped by binary and by
  shared function hashes, so train and test never contain the same function.
  `crosure-dataset` has no train/test split today.

**5. ARM correctness.** Effort: M.

Three gaps:
- `br` and `blr` are not treated as branches.
- AArch64 `ADRP+ADD/LDR` pairs are never combined, so ARM64 string xrefs and
  PLT names are missing.
- 32-bit ARM is always decoded as ARM, never Thumb.

The fix is to use Capstone's operand detail API instead of parsing the
operand text. r2 does this throughout (`arch/p/x86/plugin_cs.c`,
`anal/fcn.c`). Before this work starts, an aarch64 cross toolchain must be
added to the benchmark, so the change is measured like this round's.

**6. Noreturn list and x86-64 jump tables.** Effort: M.

These improve the CFG and function ends when there is no unwind data:
- **Noreturn:** use r2's list (`types-linux.sdb.txt`) with exact names, and
  propagate it to fixpoint (`canal.c` `aanr`, following r2's rule on indirect
  jumps rather than rizin's).
- **Jump tables:** bound by the `cmp`/`ja` guard, with targets kept inside the
  function's range.

**7. Smaller items.**
- Scan strings only in allocated, non-executable sections, and record how
  many references each string has.
- Read Mach-O `LC_FUNCTION_STARTS`.
- Tag `.cold` fragments as parts of their parent function.

## Not copying, and why

| What | Why not |
| --- | --- |
| Whole-section prologue scanning; prologue-based tail-call guesses; r2's tail-call distance heuristic | False starts in the middle of functions. With CET, `endbr64` also marks targets that are not functions. Unwind tables made these unnecessary. |
| Full ESIL emulation for references | Slow, and r2/rizin turn it off on x86 themselves. |
| r2 projects as command scripts | Loading a project runs commands (`prj.sandbox` is off by default). That is a hazard in a malware tool and bypasses the recorder. |
| String-based undo (r2 `RCoreUndo`) | Lossy: undoing a comment deletes it instead of restoring the old one. An undo should be a new step that compensates, with the old value taken from replay. |
| Mutable project snapshots instead of an append-only log | Snapshots lose provenance, and provenance is the point of Crosure. |
| BSim's trained weights and LSH tables; `.fidb` libraries | They are calibrated to p-code features, licence-bound and large. Crosure builds its index from its own recorded sessions. |
| LLM summaries as the main context strategy | An extra paid call per compaction, and summaries lose addresses. Eliding recorded results is lossless. |
| Removing the agent's turn cap (Zed has none) | Unattended analysis runs need a bound on cost. |

## Reproducing the benchmark

```bash
cargo build --release -p crosure-engine --example inspect
strip -o prog.stripped prog
python3 tools/fnbench.py prog prog.stripped
```

When `rizin` is on the PATH, its `aaa` results are printed alongside.
