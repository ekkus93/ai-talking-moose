import copy
import json
import unittest
from pathlib import Path

from validate_wake_word_corpus import MANIFEST, validate


class WakeWordCorpusContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(Path(MANIFEST).read_text(encoding="utf-8"))

    def test_repository_manifest_is_valid(self):
        validate(copy.deepcopy(self.data))

    def test_duplicate_fixture_id_fails(self):
        data = copy.deepcopy(self.data)
        data["fixtures"].append(copy.deepcopy(data["fixtures"][0]))
        with self.assertRaises(AssertionError):
            validate(data)

    def test_missing_required_near_miss_fails(self):
        data = copy.deepcopy(self.data)
        data["fixtures"] = [item for item in data["fixtures"] if item["text"] != "Hey Bruce"]
        with self.assertRaises(AssertionError):
            validate(data)

    def test_policy_drift_fails(self):
        data = copy.deepcopy(self.data)
        data["threshold"] = 0.5
        with self.assertRaises(AssertionError):
            validate(data)


if __name__ == "__main__":
    unittest.main()
