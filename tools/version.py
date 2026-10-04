#!/usr/bin/env python3
"""Keeps Crosure's version the same everywhere it is written down.

The version lives in four files: the Cargo workspace (all crates inherit
it), Cargo.lock, the Tauri config (installer and About box), and the
frontend's package.json / package-lock.json.

    python3 tools/version.py                 # print the version, fail if files disagree
    python3 tools/version.py set 0.2.0       # write it everywhere
    python3 tools/version.py check v0.2.0    # fail unless the tag matches (used by CI)
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
LOCK = ROOT / "Cargo.lock"
TAURI = ROOT / "tauri/src-tauri/tauri.conf.json"
PACKAGE = ROOT / "tauri/package.json"
PACKAGE_LOCK = ROOT / "tauri/package-lock.json"

SEMVER = re.compile(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")
WORKSPACE_VERSION = re.compile(r'(\[workspace\.package\][^\[]*?\nversion = ")([^"]+)(")', re.S)
# Workspace crates in Cargo.lock: `name = "crosure..."` then their version.
LOCK_ENTRY = re.compile(r'(name = "crosure[a-z-]*"\nversion = ")([^"]+)(")')


def current() -> dict[str, str]:
    """The version each file states."""
    found = {}
    m = WORKSPACE_VERSION.search(CARGO.read_text())
    found["Cargo.toml"] = m.group(2) if m else "?"
    found["Cargo.lock"] = ",".join(sorted({m.group(2) for m in LOCK_ENTRY.finditer(LOCK.read_text())})) or "?"
    found["tauri.conf.json"] = json.loads(TAURI.read_text())["version"]
    found["package.json"] = json.loads(PACKAGE.read_text())["version"]
    lock = json.loads(PACKAGE_LOCK.read_text())
    found["package-lock.json"] = ",".join(sorted({lock["version"], lock["packages"][""]["version"]}))
    return found


def agreed() -> str:
    """The version, or exit with the files that disagree."""
    found = current()
    versions = set(found.values())
    if len(versions) != 1:
        for name, v in found.items():
            print(f"  {name}: {v}", file=sys.stderr)
        sys.exit("versions disagree; run: python3 tools/version.py set X.Y.Z")
    return versions.pop()


def write_json(path: Path, update) -> None:
    data = json.loads(path.read_text())
    update(data)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def set_version(v: str) -> None:
    if not SEMVER.match(v):
        sys.exit(f"not a semantic version: {v}")
    CARGO.write_text(WORKSPACE_VERSION.sub(rf"\g<1>{v}\g<3>", CARGO.read_text(), count=1))
    LOCK.write_text(LOCK_ENTRY.sub(rf"\g<1>{v}\g<3>", LOCK.read_text()))
    write_json(TAURI, lambda d: d.__setitem__("version", v))
    write_json(PACKAGE, lambda d: d.__setitem__("version", v))

    def lock(d):
        d["version"] = v
        d["packages"][""]["version"] = v

    write_json(PACKAGE_LOCK, lock)
    agreed()
    print(f"version {v} written. Next:\n"
          f"  git commit -am 'Release {v}' && git tag v{v} && git push origin HEAD v{v}")


def main(argv: list[str]) -> None:
    if not argv:
        print(agreed())
    elif argv[0] == "set" and len(argv) == 2:
        set_version(argv[1])
    elif argv[0] == "check" and len(argv) == 2:
        tag = argv[1].removeprefix("refs/tags/").removeprefix("v")
        v = agreed()
        if tag != v:
            sys.exit(f"tag v{tag} does not match the version in the files ({v})")
        print(v)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
