import copy
import json
import unittest
from pathlib import Path
from validate_wake_word_corpus import MANIFEST, REQUIRED_LABELS, validate

def fixture(label):
    positive=label.startswith("positive_")
    return {"id":label,"label":label,"output":f"{label}.pcm","speaker_or_source":"espeak-ng test","text":"Hey Moose" if positive else "Hello there","voice":"en-us","speed_wpm":150,"gain_db":0.0,"distance_scale":1.0,"noise_amplitude":0.0,"expected_detection":positive,"provenance":{"kind":"deterministic_synthetic_speech"},"license":{"spdx":"CC0-1.0","redistributable":True}}

class WakeWordCorpusContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.data=json.loads(Path(MANIFEST).read_text(encoding="utf-8"))
    def test_repository_manifest_is_valid(self): validate(copy.deepcopy(self.data))
    def test_fixture_set_must_cover_all_required_labels(self):
        d=copy.deepcopy(self.data); d["fixtures"]=[fixture("positive_wake_phrase")]
        with self.assertRaises(AssertionError): validate(d)
    def test_complete_synthetic_fixture_set_is_valid(self):
        d=copy.deepcopy(self.data); d["fixtures"]=[fixture(x) for x in sorted(REQUIRED_LABELS)]; validate(d)
    def test_duplicate_fixture_id_fails(self):
        d=copy.deepcopy(self.data); one=fixture("positive_wake_phrase"); d["fixtures"]=[one,copy.deepcopy(one)]
        with self.assertRaises(AssertionError): validate(d)
    def test_output_path_traversal_fails(self):
        d=copy.deepcopy(self.data); d["fixtures"][0]["output"]="../private.pcm"
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
    def test_acceptance_criteria_cannot_return_to_pending(self):
        d=copy.deepcopy(self.data); d["acceptance_criteria"]["criteria_status"]="pending_real_fixture_calibration"
        with self.assertRaises(AssertionError): validate(d)
if __name__=="__main__": unittest.main()
