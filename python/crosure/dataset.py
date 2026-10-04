"""Load a ``crosure-export`` directory and convert it for common trainers."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterator, Optional, Union


def _jsonl(path: Path) -> Iterator[dict]:
    if not path.exists():
        return
    with path.open(encoding="utf-8") as f:
        for line in f:
            if line.strip():
                yield json.loads(line)


@dataclass
class Dataset:
    """The four files of an export."""

    manifest: dict
    trajectories: list = field(default_factory=list)
    sft: list = field(default_factory=list)
    dpo: list = field(default_factory=list)

    def summary(self) -> str:
        """One line: counts and how many sessions were skipped."""
        skipped = sum(1 for s in self.manifest.get("sessions", []) if s.get("skipped"))
        return (
            f"{len(self.trajectories)} trajectories, {len(self.sft)} SFT examples, "
            f"{len(self.dpo)} DPO pairs, {skipped} sessions skipped"
        )


def load_dataset(directory: Union[str, Path], split: Optional[str] = None) -> Dataset:
    """Read ``manifest.json``, ``trajectories.jsonl``, ``sft.jsonl`` and ``dpo.jsonl``.

    ``split`` (``"train"`` or ``"test"``) keeps only that split. Splits are
    assigned per group: investigations of the same binary, or of binaries
    sharing significant code, are always on the same side.
    """
    d = Path(directory)
    manifest = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("format") != "crosure.dataset.v1":
        raise ValueError(f"not a Crosure dataset: {manifest.get('format')!r}")
    if split not in (None, "train", "test"):
        raise ValueError(f"split must be 'train' or 'test', not {split!r}")

    def keep(record_split: Optional[str]) -> bool:
        # Exports from before splits existed have none: everything is train.
        return split is None or (record_split or "train") == split

    return Dataset(
        manifest=manifest,
        trajectories=[t for t in _jsonl(d / "trajectories.jsonl") if keep(t.get("split"))],
        sft=[e for e in _jsonl(d / "sft.jsonl") if keep(e.get("meta", {}).get("split"))],
        dpo=[p for p in _jsonl(d / "dpo.jsonl") if keep(p.get("meta", {}).get("split"))],
    )


def to_trl_sft(example: dict) -> dict[str, Any]:
    """An SFT record in the conversational ``{"messages": [...]}`` format."""
    return {"messages": example["messages"]}


def to_trl_dpo(pair: dict) -> dict[str, Any]:
    """A DPO pair in the conversational preference format (prompt / chosen / rejected)."""
    return {
        "prompt": [
            {"role": "system", "content": pair["system"]},
            {"role": "user", "content": pair["prompt"]},
        ],
        "chosen": [{"role": "assistant", "content": pair["chosen"]}],
        "rejected": [{"role": "assistant", "content": pair["rejected"]}],
    }


def to_hf(ds: Dataset):  # pragma: no cover - needs the optional `datasets` package
    """A ``datasets.DatasetDict`` with ``sft`` and ``dpo`` splits (``pip install crosure[hf]``)."""
    from datasets import Dataset as HfDataset, DatasetDict

    return DatasetDict(
        sft=HfDataset.from_list([to_trl_sft(e) for e in ds.sft]),
        dpo=HfDataset.from_list([to_trl_dpo(p) for p in ds.dpo]),
    )
