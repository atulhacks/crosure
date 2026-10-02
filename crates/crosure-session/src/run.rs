use crosure_engine::{Instruction, Xref};
use crosure_recorder::{NewStep, Observation, ParentRef, Step, Store, Target};
use serde::Serialize;
use serde_json::{json, Value};

use crate::summary;
use crate::{Op, Origin, SessionError, Workspace};

/// What an op returned, plus the step that recorded it.
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub step: Step,
    pub result: Value,
}

struct Done {
    result: Value,
    summary: String,
    target: Option<Target>,
    addr: Option<u64>,
}

const MAX_STRINGS: usize = 5000;

impl Workspace {
    /// Runs `op` on the engine and records it as one step.
    ///
    /// `parent` adds an explicit edge (for example `derived_from` the step
    /// whose result the analyst clicked, or `branch` when they went back to
    /// an older step). A `next` edge to the previous step is added otherwise.
    pub fn run(
        &mut self,
        store: &Store,
        op: Op,
        parent: Option<ParentRef>,
        origin: Origin,
    ) -> Result<Outcome, SessionError> {
        let done = self.execute(&op)?;
        let mut action = serde_json::to_value(&op)?;
        if let (Value::Object(map), Some(a)) = (&mut action, done.addr) {
            map.insert("addr".into(), json!(a));
        }
        let mut step = NewStep::human(
            op.kind(),
            match origin {
                Origin::Ui => "crosure",
                Origin::Console => "console",
            },
        );
        step.parents = parent.into_iter().collect();
        step.command = Some(op.command());
        step.action = action;
        step.target = done.target;
        step.observation = Observation {
            summary: done.summary,
            blob: Some(store.put_blob(&serde_json::to_vec(&done.result)?)?),
            truncated: false,
        };
        let step = store.record(&self.session.id, step)?;
        Ok(Outcome {
            step,
            result: done.result,
        })
    }

    fn rename_refs(&self, mut refs: Vec<Xref>) -> Vec<Xref> {
        for r in &mut refs {
            if let Some(n) = self.name_of(r.from) {
                r.from_func = Some(n.split('+').next().unwrap_or(&n).to_string());
            }
        }
        refs
    }

    fn rename_insns(&self, insns: &mut [Instruction]) {
        for i in insns {
            if let Some(n) = i.target.and_then(|t| self.renames.get(&t)) {
                i.comment = Some(n.clone());
            }
        }
    }

    fn execute(&mut self, op: &Op) -> Result<Done, SessionError> {
        let simple = |summary: String, result: Value| Done {
            result,
            summary,
            target: None,
            addr: None,
        };
        Ok(match op {
            Op::Info => {
                let info = self.engine.info()?;
                let s = format!(
                    "{} {} {}-bit, entry {:#x}, {} sections",
                    info.format,
                    info.arch,
                    info.bits,
                    info.entry,
                    info.sections.len()
                );
                simple(s, serde_json::to_value(info)?)
            }
            Op::Disasm { target } => {
                let addr = self.resolve(target)?;
                let f = self
                    .engine
                    .function_at(addr)?
                    .ok_or(SessionError::Unresolved(target.clone()))?;
                let mut insns = self.engine.disasm_function(f.addr)?;
                self.rename_insns(&mut insns);
                let name = self.name_of(f.addr).unwrap_or(f.name.clone());
                let comments: Vec<_> = self.comments.range(f.addr..f.addr + f.size).collect();
                Done {
                    summary: summary::disasm(&name, &insns),
                    result: json!({ "function": { "addr": f.addr, "name": name, "size": f.size }, "instructions": insns, "comments": comments }),
                    target: Some(self.target_for(f.addr)),
                    addr: Some(f.addr),
                }
            }
            Op::XrefsTo { target } => {
                let addr = self.resolve(target)?;
                let refs = self.rename_refs(self.engine.xrefs_to(addr)?);
                let what = self.name_of(addr).unwrap_or_else(|| target.clone());
                Done {
                    summary: summary::xrefs(&what, &refs),
                    result: json!({ "addr": addr, "name": what, "refs": refs }),
                    target: Some(self.target_for(addr)),
                    addr: Some(addr),
                }
            }
            Op::XrefsFrom { target } => {
                let addr = self.resolve(target)?;
                let refs = self.engine.xrefs_from(addr)?;
                let to: Vec<String> = refs
                    .iter()
                    .map(|r| self.name_of(r.to).unwrap_or_else(|| format!("{:#x}", r.to)))
                    .collect();
                Done {
                    summary: format!(
                        "{} refs from {target}: {}",
                        refs.len(),
                        summary::list(&to, 5)
                    ),
                    result: json!({ "addr": addr, "refs": refs, "names": to }),
                    target: Some(self.target_for(addr)),
                    addr: Some(addr),
                }
            }
            Op::Strings { filter, min_len } => {
                let needle = filter.as_ref().map(|f| f.to_lowercase());
                let all = self.engine.strings(min_len.unwrap_or(4))?;
                let hits: Vec<_> = all
                    .into_iter()
                    .filter(|s| {
                        needle
                            .as_ref()
                            .is_none_or(|n| s.value.to_lowercase().contains(n))
                    })
                    .collect();
                let shown: Vec<String> = hits
                    .iter()
                    .filter(|s| s.mapped)
                    .map(|s| format!("{:?}", s.value))
                    .collect();
                let what = filter
                    .as_ref()
                    .map_or(String::new(), |f| format!(" matching {f:?}"));
                let truncated = hits.len() > MAX_STRINGS;
                simple(
                    format!("{} strings{what}: {}", hits.len(), summary::list(&shown, 3)),
                    json!({ "strings": hits.into_iter().take(MAX_STRINGS).collect::<Vec<_>>(), "truncated": truncated }),
                )
            }
            Op::Imports => {
                let imports = self.engine.imports()?;
                let mut libs: Vec<String> = imports
                    .iter()
                    .map(|i| i.library.clone())
                    .filter(|l| !l.is_empty())
                    .collect();
                libs.sort();
                libs.dedup();
                simple(
                    format!("{} imports from {}", imports.len(), summary::list(&libs, 4)),
                    json!({ "imports": imports }),
                )
            }
            Op::Hex { target, len } => {
                let addr = self.resolve(target)?;
                let bytes = self.engine.read_bytes(addr, len.unwrap_or(256).min(4096))?;
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                Done {
                    summary: format!("{} bytes at {addr:#x}", bytes.len()),
                    result: json!({ "addr": addr, "hex": hex }),
                    target: Some(self.target_for(addr)),
                    addr: Some(addr),
                }
            }
            Op::Rename { target, name } => {
                let addr = self.resolve(target)?;
                let start = self.engine.function_at(addr)?.map_or(addr, |f| f.addr);
                let old = self.name_of(start).unwrap_or_else(|| format!("{start:#x}"));
                self.renames.insert(start, name.clone());
                Done {
                    summary: format!("renamed {old} → {name}"),
                    result: json!({ "addr": start, "old": old, "name": name }),
                    target: Some(self.target_for(start)),
                    addr: Some(start),
                }
            }
            Op::Comment { target, text } => {
                let addr = self.resolve(target)?;
                self.comments.insert(addr, text.clone());
                Done {
                    summary: format!("comment at {addr:#x}: {text}"),
                    result: json!({ "addr": addr, "text": text }),
                    target: Some(self.target_for(addr)),
                    addr: Some(addr),
                }
            }
            Op::Hypothesis { text } => {
                simple(format!("hypothesis: {text}"), json!({ "text": text }))
            }
            Op::Finding { text } => simple(format!("finding: {text}"), json!({ "text": text })),
            Op::Verdict {
                verdict,
                family,
                text,
            } => simple(
                format!(
                    "verdict: {verdict}{} {text}",
                    family.as_ref().map_or(String::new(), |f| format!(" ({f})"))
                ),
                json!({ "verdict": verdict, "family": family, "text": text }),
            ),
        })
    }
}
