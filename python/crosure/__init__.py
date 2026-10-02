"""Crosure for Python: verify exported investigations and load datasets.

The verifier re-derives the hash chain with no Crosure code involved, so a
dataset consumer can check provenance independently.
"""

from .chain import VerifyReport, canonical_json, genesis_hash, step_hash, verify_session
from .dataset import Dataset, load_dataset, to_trl_dpo, to_trl_sft

__all__ = [
    "Dataset",
    "VerifyReport",
    "canonical_json",
    "genesis_hash",
    "load_dataset",
    "step_hash",
    "to_trl_dpo",
    "to_trl_sft",
    "verify_session",
]
