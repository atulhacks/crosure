//! Prints `str <addr> <xref count>` for every mapped string and
//! `stub <addr> <name>` for every recognised import stub.
use crosure_engine::{Engine, NativeEngine};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: xrefs <binary>")?;
    let e = NativeEngine::open(std::path::Path::new(&path))?;
    for s in e.strings(4)?.iter().filter(|s| s.mapped) {
        println!("str {:#x} {}", s.addr, e.xrefs_to(s.addr)?.len());
    }
    for f in e.functions()?.iter().filter(|f| f.source == "import_stub") {
        println!("stub {:#x} {}", f.addr, f.name);
    }
    Ok(())
}
