import copy
import json
import unittest

from validate_wake_word_corpus import POINTER, validate


class WakeWordCorpusPointerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(POINTER.read_text(encoding="utf-8"))

    def test_repository_pointer_is_valid(self):
        validate(copy.deepcopy(self.data))

    def test_pointer_cannot_claim_independent_policy(self):
        data = copy.deepcopy(self.data)
        data["threshold"] = 0.25
        with self.assertRaises(AssertionError):
            validate(data)

    def test_pointer_cannot_target_another_manifest(self):
        data = copy.deepcopy(self.data)
        data["canonical_manifest"] = "other.json"
        with self.assertRaises(AssertionError):
            validate(data)


if __name__ == "__main__":
    unittest.main()
