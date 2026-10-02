# Overview

## One line

Crosure is a reverse-engineering workbench that **records every step of an
investigation**, whether a human or an AI takes it, as a live, replayable,
**tamper-evident graph**. Those graphs then become evidence, lessons and
training data.

## The problem

1. **Expert process disappears.** A report keeps the answer. The path is lost:
   what was checked, in what order, what was ruled out, and why.
2. **Students learn from cleaned-up write-ups**, never from real investigations
   with their dead ends.
3. **AI models never see expert step-by-step reversing**, so AI agents
   struggle on real binaries.
4. **Investigations are hard to audit.** "How did you conclude this?" is
   answered with notes and screenshots.

## The idea

Every action goes through one recorder:

```
 analyst click ─┐
 console command ─┼──►  run op on the engine  ──►  record step  ──►  graph · log · timeline
 AI tool call ──┘          (functions, disasm,       (hash-chained,       (live, replayable,
                            xrefs, strings, …)        with "why")          verifiable)
```

- A **step** is one action plus what it showed, why it was taken, and how it
  turned out (lead, dead end, key step).
- Steps link into a **graph**:
  - *next*: time order;
  - *derived from*: you clicked something in a previous result;
  - *branch*: you went back and tried something else.
- Every step is **hash-chained**, so editing any stored step makes
  verification fail.
- The **AI agent** uses the same tools as you. Every one of its calls is a
  recorded step with the model's stated reason.

## What it is for

| Use | How Crosure helps |
| --- | --- |
| Analysis | IDE-style workbench: functions, disassembly (linear and block graph), xrefs, strings, imports, hex, console, AI agent |
| Audit / incident response | Tamper-evident chain of custody of every step; export |
| Teaching | Replay real investigations including dead ends; compare paths (planned) |
| AI training data | Human and AI trajectories with rationale, branches and outcomes (exporters planned) |

## Status at a glance

- **Built:**
  - the native analysis engine and the recorder with its hash chain;
  - the investigation graph, flight log and replay timeline;
  - the desktop app;
  - the AI agent, with multiple providers and fallback on refusal;
  - the CLI.
- **Next:** a decompiler (rizin + Ghidra backend) and the dataset exporters.

The [README](../README.md) lists every working feature.
