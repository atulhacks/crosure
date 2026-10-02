"""Independent verification of a Crosure hash chain.

Each step's hash is ``sha256(prev_hash + "\\n" + canonical_json(step without "hash"))``,
where canonical JSON has object keys sorted and no whitespace. The first step
links to ``genesis_hash(session_id)``.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from typing import Any, Optional


def _digest(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def canonical_json(value: Any) -> str:
    """Serialize ``value`` the way Crosure does: sorted keys, compact, UTF-8.

    >>> canonical_json({"b": 1, "a": [True, None, "é"]})
    '{"a":[true,null,"é"],"b":1}'
    """
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def genesis_hash(session_id: str) -> str:
    """The ``prev_hash`` of a session's first step."""
    return _digest(f"crosure:genesis:{session_id}".encode())


def step_hash(step: dict) -> str:
    """Recompute a step's hash from its content."""
    body = {k: v for k, v in step.items() if k != "hash"}
    return _digest(step["prev_hash"].encode() + b"\n" + canonical_json(body).encode())


@dataclass
class VerifyReport:
    """Outcome of :func:`verify_session`."""

    ok: bool
    checked: int
    head_hash: str
    failed_seq: Optional[int] = None
    reason: Optional[str] = None


def verify_session(doc: dict) -> VerifyReport:
    """Verify an exported session (``crosure.session.v1``: ``session`` + ``steps``)."""
    session = doc["session"]
    expected = genesis_hash(session["id"])
    checked = 0

    def fail(seq: int, reason: str) -> VerifyReport:
        return VerifyReport(False, checked, expected, seq, reason)

    for step in sorted(doc["steps"], key=lambda s: s["seq"]):
        seq = step["seq"]
        if seq != checked:
            return fail(seq, "sequence gap or reordering")
        if step["session_id"] != session["id"]:
            return fail(seq, "step belongs to another session")
        if step["prev_hash"] != expected:
            return fail(seq, "prev_hash does not link to the previous step")
        if step_hash(step) != step["hash"]:
            return fail(seq, "content was modified after recording")
        expected = step["hash"]
        checked += 1
    if session.get("head_hash") != expected:
        return VerifyReport(False, checked, expected, None, "session head does not match the last step")
    return VerifyReport(True, checked, expected)
