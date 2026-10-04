use std::path::Path;

use serde_json::json;

use crate::run::Done;
use crate::{SessionError, Workspace};

/// Rizin prefixes dropped from names in decompiled code.
const PREFIXES: [&str; 5] = ["sym.imp.", "sym.", "dbg.", "obj.", "_obj."];

impl Workspace {
    /// The name to show for a rizin identifier: prefixes dropped, `fcn.<hex>`
    /// resolved to our function name, analyst renames applied.
    fn crosure_name(&self, ident: &str) -> String {
        if let Some(hex) = ident.strip_prefix("fcn.") {
            if let Ok(addr) = u64::from_str_radix(hex, 16) {
                return self.name_of(addr).unwrap_or_else(|| ident.to_string());
            }
        }
        let Some(bare) = PREFIXES.iter().find_map(|p| ident.strip_prefix(p)) else {
            return ident.to_string();
        };
        match self.engine.resolve(bare) {
            Ok(Some(addr)) => self
                .renames
                .get(&addr)
                .cloned()
                .unwrap_or_else(|| bare.to_string()),
            _ => bare.to_string(),
        }
    }

    /// Rewrites identifiers in one line of pseudo-C, leaving string and
    /// character literals alone.
    pub(crate) fn rename_c(&self, line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let mut ident = String::new();
        let mut quote: Option<char> = None;
        let mut escaped = false;
        let flush = |ident: &mut String, out: &mut String| {
            if !ident.is_empty() {
                out.push_str(&self.crosure_name(ident));
                ident.clear();
            }
        };
        for c in line.chars() {
            if let Some(q) = quote {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
                continue;
            }
            let starts = c.is_ascii_alphabetic() || c == '_';
            if starts || (!ident.is_empty() && (c.is_ascii_alphanumeric() || c == '.')) {
                ident.push(c);
                continue;
            }
            flush(&mut ident, &mut out);
            if c == '"' || c == '\'' {
                quote = Some(c);
            }
            out.push(c);
        }
        flush(&mut ident, &mut out);
        out
    }

    /// Runs the decompiler on the function containing `target`.
    pub(crate) fn decompile_fn(&self, target: &str) -> Result<Done, SessionError> {
        let addr = self.resolve(target)?;
        let f = self
            .engine
            .function_at(addr)?
            .ok_or(SessionError::Unresolved(target.to_string()))?;
        let mut d = crosure_engine::decompile(Path::new(&self.session.binary_path), f.addr)?;
        for l in &mut d.lines {
            l.text = self.rename_c(&l.text);
        }
        let name = self.name_of(f.addr).unwrap_or(f.name.clone());
        let code = d.lines.iter().filter(|l| !l.text.trim().is_empty()).count();
        Ok(Done {
            summary: format!("{name}: {code} lines of pseudo-C ({})", d.backend),
            result: json!({
                "function": { "addr": f.addr, "name": name, "size": f.size },
                "backend": d.backend,
                "lines": d.lines,
            }),
            target: Some(self.target_for(f.addr)),
            addr: Some(f.addr),
        })
    }
}

#[cfg(test)]
mod tests {
    use crosure_recorder::Store;

    use crate::{Op, Origin, Workspace};

    fn crackme() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../crosure-engine/tests/fixtures/crackme-x64")
    }

    #[test]
    fn rizin_names_become_crosure_names() -> Result<(), Box<dyn std::error::Error>> {
        let store = Store::open_in_memory()?;
        let (mut ws, _) = Workspace::open(&store, &crackme(), None)?;
        ws.run(
            &store,
            Op::Rename {
                target: "decode".into(),
                name: "xor_decode".into(),
            },
            None,
            Origin::Ui,
        )?;
        let line =
            r#"    sym.decode(&s2, 7); x = sym.imp.strcmp(a, "sym.decode"); fcn.000011d9();"#;
        assert_eq!(
            ws.rename_c(line),
            r#"    xor_decode(&s2, 7); x = strcmp(a, "sym.decode"); check_password();"#
        );
        Ok(())
    }

    #[test]
    fn decompile_is_recorded() -> Result<(), Box<dyn std::error::Error>> {
        let store = Store::open_in_memory()?;
        let (mut ws, _) = Workspace::open(&store, &crackme(), None)?;
        let op = Op::Decompile {
            target: "check_password".into(),
            offset: None,
        };
        if !crosure_engine::decompiler_status().available {
            assert!(ws.run(&store, op, None, Origin::Ui).is_err());
            return Ok(());
        }
        let out = ws.run(&store, op, None, Origin::Ui)?;
        assert_eq!(out.step.command.as_deref(), Some("dec check_password"));
        assert!(out.step.observation.summary.contains("pseudo-C"));
        let code = out.result["lines"].to_string();
        assert!(code.contains("strcmp(") && !code.contains("sym."), "{code}");
        Ok(())
    }
}
