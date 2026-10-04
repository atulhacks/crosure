//! Prints `count` instructions at an address: `dis <binary> <hex addr> [count]`.
use crosure_engine::{Engine, NativeEngine};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = std::env::args().skip(1);
    let path = a.next().ok_or("usage: dis <binary> <addr> [count]")?;
    let addr = u64::from_str_radix(a.next().ok_or("addr")?.trim_start_matches("0x"), 16)?;
    let count = a.next().and_then(|c| c.parse().ok()).unwrap_or(16);
    let e = NativeEngine::open(std::path::Path::new(&path))?;
    for i in e.disasm(addr, count)? {
        println!(
            "{:#x}  {} {}  ; {:?} {:?}",
            i.addr, i.mnemonic, i.operands, i.target, i.comment
        );
    }
    Ok(())
}
