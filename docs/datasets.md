# Datasets

Crosure records every step of an investigation, whether a human or an AI
took it. `crosure-dataset` turns those recordings into training data, and
`python/` loads it and checks where it came from.

## Export

There are two ways to export:

- **In the app:** **Export → Training dataset**. The files are written to
  `~/.crosure/datasets/<time>/`.
- **From the terminal:** run the commands below.

```bash
cargo run -p crosure-dataset --bin crosure-export -- ./my-dataset
cargo run -p crosure-dataset --bin crosure-export -- --session ses_01... --humans-only ./out
```

| Option | Effect |
| --- | --- |
| `--session ID` | Export only these sessions (repeatable). By default every session is exported. |
| `--all-steps` | Use every analysis step as an SFT target, not only steps on a key path. |
| `--humans-only` | Use only steps a human took as SFT targets. |
| `--history N` | Number of recent steps shown in each prompt (default 40). |
| `--allow-unverified` | Also export sessions whose hash chain fails. They are marked `verified: false`. |

Every session is **verified first**. If a session's chain does not verify,
it is skipped and the reason is listed in `manifest.json`.

Local paths are reduced to the binary's file name. Actor ids are already
anonymous.

## Files

| File | One line per | Holds |
| --- | --- | --- |
| `trajectories.jsonl` | investigation | binary, sha256, head hash, and every step |
| `sft.jsonl` | step | a chat example: system prompt, context, then the step as the answer |
| `dpo.jsonl` | preference pair | the same context, the step that led somewhere, the step that did not |
| `manifest.json` | export | options, counts, and each session with its head hash and any skip reason |

Each step in a trajectory carries:
- the command (`xt strcmp`) and its target;
- why it was taken (the stated reason or intent chip) and what it showed;
- its tags and whether it lies on a **key path** (it leads to a finding,
  verdict or `key_step`);
- its typed parent links and its hash.

### SFT

```json
{"messages": [
  {"role": "system", "content": "You are a reverse engineer working in Crosure ..."},
  {"role": "user", "content": "Binary: crackme-x64 (sha256:1aef…)\nTask: Explain what @check_password does…\nSteps so far:\n#1 dis check_password: 22 instructions; calls decode, strcmp@plt …\nWhat is the next step?"},
  {"role": "assistant", "content": "xt strcmp@plt\nwhy: The comparison routine is the likely check."}
 ],
 "meta": {"session_id": "ses_…", "seq": 5, "actor": "agent", "model": "anthropic:…", "on_key_path": true, "hash": "sha256:…"}}
```

The task in each prompt is the analyst's most recent question before that
step, or a default task if there was none.

Some steps are never SFT targets:
- the load;
- the question itself;
- the agent's report;
- steps taken only to attach `@` context to a prompt (intent chip
  `ai_context`).

### DPO

Pairs come from two places in the graph:

- **Branch.** Sometimes the analyst goes back to step P and tries something
  else. If that branch leads to a result, it is *chosen*. P's other children
  that led nowhere are *rejected*. The prompt is the history up to P.
- **Dead end.** A step tagged `dead_end` is *rejected*. The next step that
  led to a result is *chosen*. The prompt is the history before the dead end.

Tagging steps in the Inspector (lead, dead end, key step) and branching from
older steps both make the dataset richer.

## Splits

Every record carries `split` (`train` or `test`) and `group`. The manifest
gives per-split counts.

Two investigations end up in the same group when either is true:
- they looked at the **same binary** (same sha256);
- they touched a **significant function with the same code**: the same
  structural fingerprint, at least 12 instructions. This catches a stripped
  copy, a relinked build or a variant that reuses code.

Fingerprints found in many different binaries are statically linked library
code and do not join groups. The cutoff is 3 binaries, or 10% of the
binaries in the export if that is more.

Each group goes to `test` by a hash of its id, about `--test-percent` of
groups (default 10). Re-exporting gives the same split. So a model is never
tested on a function it was trained on.

SFT examples with the same prompt and answer, for example from two identical
investigations, are exported once. The manifest reports how many were
dropped as `sft_duplicates`.

```bash
cargo run -p crosure-dataset --bin crosure-export -- --test-percent 20 ./my-dataset
```

```python
train = load_dataset("./my-dataset", split="train")
test = load_dataset("./my-dataset", split="test")
```

## Python

The Python tools use the standard library only. With `pip install
./python[hf]` you also get Hugging Face datasets.

```bash
python -m crosure verify session.json   # re-derive the hash chain independently
python -m crosure stats ./my-dataset
```

```python
from crosure import load_dataset, to_trl_sft, to_trl_dpo, verify_session

ds = load_dataset("./my-dataset")
sft = [to_trl_sft(e) for e in ds.sft]     # {"messages": [...]}
dpo = [to_trl_dpo(p) for p in ds.dpo]     # {"prompt": [...], "chosen": [...], "rejected": [...]}
```

`verify_session` checks a **This session (JSON)** export against the chain
rule:

```
hash = sha256(prev_hash + "\n" + canonical_json(step without "hash"))
```

Canonical JSON means sorted keys and no whitespace. The first step links to
`sha256("crosure:genesis:<session id>")`, and the session's head must equal
the last step's hash.

The verifier shares no code with the recorder, so a dataset consumer does
not have to trust Crosure to check where the data came from.
