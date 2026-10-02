"""``python -m crosure verify SESSION.json`` or ``python -m crosure stats DATASET_DIR``."""

from __future__ import annotations

import json
import sys

from .chain import verify_session
from .dataset import load_dataset

USAGE = "usage: python -m crosure verify SESSION.json | stats DATASET_DIR"


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 2 or args[0] not in ("verify", "stats"):
        print(USAGE, file=sys.stderr)
        return 2
    if args[0] == "verify":
        with open(args[1], encoding="utf-8") as f:
            report = verify_session(json.load(f))
        if report.ok:
            print(f"ok: {report.checked} steps, head {report.head_hash}")
            return 0
        print(f"FAILED at step {report.failed_seq}: {report.reason}")
        return 1
    print(load_dataset(args[1]).summary())
    return 0


if __name__ == "__main__":
    sys.exit(main())
