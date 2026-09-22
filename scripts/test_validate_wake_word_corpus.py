import copy
import json
import unittest
from pathlib import Path
from validate_wake_word_corpus import MANIFEST, REQUIRED_LABELS, validate
BASE_FIXTURE={"id":"fixture-id","label":"positive_wake_phrase","path":"docs/fixtures/wake-word-v1/positive/fixture-id.wav","speaker_or_source":"synthetic-test-source","provenance":{"kind":"synthetic","tool":"unit-test"},"license":{"spdx":"CC0-1.0","redistributable":True},"bytes":1234,"sha256":"0"*64,"sample_rate_hz":16000,"channels":1,"sample_format":"pcm_s16le","expected_detection":True}
def fixture(label):
    r=copy.deepcopy(BASE_FIXTURE); r["id"]=label; r["label"]=label; r["path"]=f"docs/fixtures/wake-word-v1/{label}.wav"; r["expected_detection"]=label.startswith("positive_"); return r
class WakeWordCorpusContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.data=json.loads(Path(MANIFEST).read_text(encoding="utf-8"))
    def test_repository_manifest_is_valid_even_before_real_fixtures_exist(self): validate(copy.deepcopy(self.data))
    def test_non_empty_fixture_set_must_cover_all_required_labels(self):
        d=copy.deepcopy(self.data); d["fixtures"]=[fixture("positive_wake_phrase")]
        with self.assertRaises(AssertionError): validate(d)
    def test_complete_synthetic_fixture_set_is_valid(self):
        d=copy.deepcopy(self.data); d["fixtures"]=[fixture(x) for x in sorted(REQUIRED_LABELS)]; validate(d)
    def test_duplicate_fixture_id_fails(self):
        d=copy.deepcopy(self.data); one=fixture("positive_wake_phrase"); d["fixtures"]=[one,copy.deepcopy(one)]
        with self.assertRaises(AssertionError): validate(d)
    def test_path_traversal_fails(self):
        d=copy.deepcopy(self.data); bad=fixture("positive_wake_phrase"); bad["path"]="docs/fixtures/wake-word-v1/../private.wav"; d["fixtures"]=[bad]
        with self.assertRaises(AssertionError): validate(d)
    def test_policy_drift_fails(self):
        d=copy.deepcopy(self.data); d["policy"]["sample_rate_hz"]=48000
        with self.assertRaises(AssertionError): validate(d)
    def test_score_threshold_drift_fails(self):
        d=copy.deepcopy(self.data); d["policy"]["threshold"]=0.5
        with self.assertRaises(AssertionError): validate(d)
    def test_model_runtime_identity_drift_fails(self):
        d=copy.deepcopy(self.data); d["model_runtime_identity"]["model_archive_sha256"]="1"*64
        with self.assertRaises(AssertionError): validate(d)
if __name__ == "__main__": unittest.main()
