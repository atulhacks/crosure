//! `cargo run -p crosure-engine --example inspect -- <binary>`: prints what the
//! native engine sees. Handy for checking a new fixture.

use crosure_engine::{Engine, NativeEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: inspect <binary>")?;
    let e = NativeEngine::open(std::path::Path::new(&path))?;
    let info = e.info()?;
    println!(
        "{} {} {}-bit entry={:#x} stripped={}",
        info.format, info.arch, info.bits, info.entry, info.stripped
    );
    for f in e.functions()? {
        println!("fn {:#x} {:>6} {:<14} {}", f.addr, f.size, f.source, f.name);
    }
    for i in e.imports()? {
        println!(
            "import {:<24} {:<20} {:?}",
            i.name,
            i.library,
            i.addr.map(|a| format!("{a:#x}"))
        );
    }
    for s in e.strings(6)?.iter().filter(|s| s.mapped).take(15) {
        println!("str {:#x} {:?}", s.addr, s.value);
    }
    if let Some(addr) = e.resolve("check_password")?.or(e.resolve("main")?) {
        for insn in e.disasm_function(addr)? {
            println!(
                "  {:#x} {:<8} {:<40} {}",
                insn.addr,
                insn.mnemonic,
                insn.operands,
                insn.comment.unwrap_or_default()
            );
        }
    }
    Ok(())
}
