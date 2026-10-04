import copy
import json
import unittest
from pathlib import Path

from crosure import load_dataset, to_trl_dpo, verify_session

FIXTURES = Path(__file__).parent / "fixtures"


def session():
    return json.loads((FIXTURES / "crackme-session.json").read_text(encoding="utf-8"))


class ChainTest(unittest.TestCase):
    def test_recorded_session_verifies(self):
        report = verify_session(session())
        self.assertTrue(report.ok, report.reason)
        self.assertEqual(report.checked, 13)

    def test_edit_is_detected(self):
        doc = session()
        doc["steps"][5]["observation"]["summary"] = "nothing here"
        report = verify_session(doc)
        self.assertFalse(report.ok)
        self.assertEqual(report.failed_seq, 5)

    def test_dropped_step_is_detected(self):
        doc = session()
        del doc["steps"][3]
        self.assertEqual(verify_session(doc).failed_seq, 4)

    def test_rehashed_forgery_breaks_the_head(self):
        from crosure import step_hash

        doc = copy.deepcopy(session())
        last = doc["steps"][-1]
        last["observation"]["summary"] = "forged"
        last["hash"] = step_hash(last)
        report = verify_session(doc)
        self.assertFalse(report.ok)
        self.assertIn("head", report.reason)


class DatasetTest(unittest.TestCase):
    def test_load_export(self):
        ds = load_dataset(FIXTURES / "dataset")
        self.assertEqual(len(ds.trajectories), 1)
        self.assertEqual(len(ds.sft), 7)
        first = ds.sft[0]["messages"]
        self.assertEqual([m["role"] for m in first], ["system", "user", "assistant"])
        self.assertNotIn("/home/", json.dumps(ds.trajectories))

    def test_splits_partition_the_export(self):
        full = load_dataset(FIXTURES / "dataset")
        train = load_dataset(FIXTURES / "dataset", split="train")
        test = load_dataset(FIXTURES / "dataset", split="test")
        self.assertEqual(len(train.sft) + len(test.sft), len(full.sft))
        self.assertEqual(len(train.trajectories) + len(test.trajectories), len(full.trajectories))
        self.assertTrue(all(t["split"] in ("train", "test") for t in full.trajectories))
        with self.assertRaises(ValueError):
            load_dataset(FIXTURES / "dataset", split="validation")

    def test_dpo_conversion(self):
        pair = {"system": "s", "prompt": "p", "chosen": "xt strcmp", "rejected": "str http"}
        out = to_trl_dpo(pair)
        self.assertEqual(out["prompt"][1], {"role": "user", "content": "p"})
        self.assertEqual(out["chosen"][0]["content"], "xt strcmp")


if __name__ == "__main__":
    unittest.main()
