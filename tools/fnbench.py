"""Function-discovery benchmark.

Scores Crosure (and rizin, when installed) on stripped binaries against the
unstripped originals' symbol tables: start precision/recall and exact sizes.

    cargo build --release -p crosure-engine --example inspect
    python3 tools/fnbench.py prog prog.stripped [lib.so lib.stripped.so ...]

PE ground truth comes from mingw's nm (COFF symbols carry no sizes).
"""
import json, re, shutil, subprocess, sys, time
from pathlib import Path

def truth_elf(path):
    """FUNC symbols with a size, from .symtab (+ .dynsym), in executable sections."""
    secs = {}
    for l in subprocess.run(["readelf", "-SW", path], capture_output=True, text=True).stdout.splitlines():
        m = re.match(r"\s*\[\s*(\d+)\]\s+(\S+)\s+\S+\s+([0-9a-f]+)\s+[0-9a-f]+\s+([0-9a-f]+)\s+\S+\s+(\S*X\S*)", l)
        if m:
            secs[int(m[1])] = m[2]
    out = {}
    thumb = "ARM" in subprocess.run(["readelf", "-hW", path], capture_output=True, text=True).stdout.split("Machine:")[1].split("\n")[0]
    for l in subprocess.run(["readelf", "-sW", path], capture_output=True, text=True).stdout.splitlines():
        p = l.split()
        if len(p) >= 8 and p[3] == "FUNC" and p[6].isdigit() and int(p[6]) in secs and secs[int(p[6])] not in (".plt", ".plt.got", ".plt.sec", ".init", ".fini"):
            # ARM: bit 0 of a function symbol marks Thumb code, not the address.
            a, sz = int(p[1], 16) & ~1 if thumb else int(p[1], 16), int(p[2])
            out[a] = max(sz, out.get(a, 0))
    return out

def truth_pe(path):
    """Text symbols of the unstripped PE (no sizes in COFF)."""
    out = {}
    r = subprocess.run(["x86_64-w64-mingw32-nm", "-n", path], capture_output=True, text=True).stdout
    for l in r.splitlines():
        p = l.split()
        if len(p) == 3 and p[1] in "Tt" and not p[2].startswith((".", "__imp_")):
            out[int(p[0], 16)] = 0
    return out

def ours(path):
    t = time.time()
    r = subprocess.run([str(Path(__file__).resolve().parent.parent / "target/release/examples/inspect"), path], capture_output=True, text=True)
    dt = time.time() - t
    fs = {}
    for l in r.stdout.splitlines():
        if l.startswith("fn "):
            p = l.split()
            fs[int(p[1], 16)] = int(p[2])
    return fs, dt

def rizin(path, cmd):
    t = time.time()
    r = subprocess.run(["rizin", "-q", "-e", "scr.color=0", "-e", "bin.relocs.apply=true", "-c", f"{cmd};aflj", path], capture_output=True, text=True, timeout=1800)
    dt = time.time() - t
    j = r.stdout[r.stdout.find("["):]
    fs = {}
    for f in json.loads(j or "[]"):
        if f.get("name", "").startswith(("sym.imp.", "loc.imp.")):
            continue
        fs[f["offset"]] = f.get("size", 0)
    return fs, dt

def score(name, found, truth, dt, has_size):
    text_lo, text_hi = min(truth), max(a + max(s, 1) for a, s in truth.items())
    found = {a: s for a, s in found.items() if text_lo <= a < text_hi + 0x1000}
    hit = set(found) & set(truth)
    p = len(hit) / max(len(found), 1)
    r = len(hit) / max(len(truth), 1)
    line = f"  {name:<14} found {len(found):5}  precision {p:6.1%}  recall {r:6.1%}"
    if has_size:
        sized = [a for a in hit if truth[a] > 0]
        exact = sum(1 for a in sized if found[a] == truth[a]) * len(hit) / max(len(sized), 1)
        short = sum(1 for a in sized if 0 < found[a] < truth[a])
        long_ = sum(1 for a in sized if found[a] > truth[a] + 15)
        line += f"  size exact {exact/max(len(hit),1):6.1%}  too-short {short:4}  overrun>15B {long_:4}"
    print(line + f"  {dt:6.1f}s")

if __name__ == "__main__" and len(sys.argv) < 3:
    sys.exit(__doc__)

for unstripped, stripped in zip(sys.argv[1::2], sys.argv[2::2]):
    pe = unstripped.endswith(".exe")
    truth = truth_pe(unstripped) if pe else truth_elf(unstripped)
    print(f"{stripped}: {len(truth)} true functions")
    f, dt = ours(stripped); score("crosure", f, truth, dt, not pe)
    if shutil.which("rizin"):
        f, dt = rizin(stripped, "aaa"); score("rizin aaa", f, truth, dt, not pe)
