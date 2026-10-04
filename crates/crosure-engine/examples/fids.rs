//! Prints `addr size fingerprint` for every function.
use crosure_engine::{Engine, NativeEngine};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: fids <binary>")?;
    let e = NativeEngine::open(std::path::Path::new(&path))?;
    for f in e.functions()? {
        let h = e
            .function_hash(f.addr)?
            .map(|h| h.fingerprint())
            .unwrap_or_default();
        println!("{:#x} {} {} {}", f.addr, f.size, h, f.name);
    }
    Ok(())
}
