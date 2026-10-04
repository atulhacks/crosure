"""AArch64 reference benchmark: string xrefs and PLT stub names.

Ground truth pairs each `adrp xN, page` with the next `add xM, xN, #off` or
`ldr xM, [xN, #off]` in binutils' disassembly (reset at each function), and
counts objdump's `@plt` stubs. Needs binutils-aarch64-linux-gnu.

    cargo build --release -p crosure-engine --example xrefs
    python3 tools/armbench.py prog.stripped
"""
import re, subprocess, sys
from pathlib import Path
binary = sys.argv[1]
dis = subprocess.run(["aarch64-linux-gnu-objdump", "-d", "--no-show-raw-insn", binary], capture_output=True, text=True).stdout
refs = set()
page = {}
for line in dis.splitlines():
    if re.match(r"^[0-9a-f]+ <", line):
        page = {}
        continue
    m = re.match(r"\s+[0-9a-f]+:\s+(\S+)\s+(.*)", line)
    if not m:
        continue
    op, args = m[1], m[2]
    a = re.match(r"(x\d+), ([0-9a-f]+)", args)
    if op == "adrp" and a:
        page[a[1]] = int(a[2], 16)
        continue
    a = re.match(r"(x\d+|w\d+), (x\d+), #0x([0-9a-f]+)", args) if op == "add" else re.match(r"(x\d+|w\d+), \[(x\d+), #0x([0-9a-f]+)\]", args) if op.startswith("ldr") else None
    if a and a[2] in page:
        refs.add(page[a[2]] + int(a[3], 16))
out = subprocess.run([str(Path(__file__).resolve().parent.parent / "target/release/examples/xrefs"), binary], capture_output=True, text=True).stdout
strings = {int(l.split()[1], 16): int(l.split()[2]) for l in out.splitlines() if l.startswith("str ")}
stubs = [l for l in out.splitlines() if l.startswith("stub ")]
used = [a for a in strings if a in refs]
found = [a for a in used if strings[a] > 0]
plt = len(re.findall(r"^[0-9a-f]+ <\S+@plt>:", dis, re.M))
print(f"{binary}: strings referenced by code {len(used)}, with a Crosure xref {len(found)} ({len(found)/max(len(used),1):.0%}); PLT stubs {plt}, named by Crosure {len(stubs)}")
