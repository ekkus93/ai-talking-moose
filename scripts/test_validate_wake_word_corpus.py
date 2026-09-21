import copy
import json
import unittest
from pathlib import Path

from validate_wake_word_corpus import MANIFEST, REQUIRED_LABELS, validate


BASE_FIXTURE = {
    "id": "fixture-id",
    "label": "positive_wake_phrase",
    "path": "docs/fixtures/wake-word-v1/positive/fixture-id.wav",
    "speaker_or_source": "synthetic-test-source",
    "provenance": {"kind": "synthetic", "tool": "unit-test"},
    "license": {"spdx": "CC0-1.0", "redistributable": True},
    "bytes": 1234,
    "sha256": "0" * 64,
    "sample_rate_hz": 16000,
    "channels": 1,
    "sample_format": "pcm_s16le",
    "expected_detection": True,
}


def fixture(label):
    result = copy.deepcopy(BASE_FIXTURE)
    result["id"] = label
    result["label"] = label
    result["path"] = f"docs/fixtures/wake-word-v1/{label}.wav"
    result["expected_detection"] = label.startswith("positive_")
    return result


class WakeWordCorpusContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(Path(MANIFEST).read_text(encoding="utf-8"))

    def test_repository_manifest_is_valid_even_before_real_fixtures_exist(self):
        validate(copy.deepcopy(self.data))

    def test_non_empty_fixture_set_must_cover_all_required_labels(self):
        data = copy.deepcopy(self.data)
        data["fixtures"] = [fixture("positive_wake_phrase")]
        with self.assertRaises(AssertionError):
            validate(data)

    def test_complete_synthetic_fixture_set_is_valid(self):
        data = copy.deepcopy(self.data)
        data["fixtures"] = [fixture(label) for label in sorted(REQUIRED_LABELS)]
        validate(data)

    def test_duplicate_fixture_id_fails(self):
        data = copy.deepcopy(self.data)
        one = fixture("positive_wake_phrase")
        data["fixtures"] = [one, copy.deepcopy(one)]
        with self.assertRaises(AssertionError):
            validate(data)

    def test_path_traversal_fails(self):
        data = copy.deepcopy(self.data)
        bad = fixture("positive_wake_phrase")
        bad["path"] = "docs/fixtures/wake-word-v1/../private.wav"
        data["fixtures"] = [bad]
        with self.assertRaises(AssertionError):
            validate(data)

    def test_policy_drift_fails(self):
        data = copy.deepcopy(self.data)
        data["policy"]["sample_rate_hz"] = 48000
        with self.assertRaises(AssertionError):
            validate(data)


if __name__ == "__main__":
    unittest.main()
