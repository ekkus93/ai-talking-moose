import unittest

from scripts.check_whisper_provenance import validate


class WhisperProvenanceTest(unittest.TestCase):
    def test_matching_revision_passes(self):
        sha = "1" * 40
        validate(sha, sha)

    def test_deliberately_wrong_revision_fails(self):
        with self.assertRaisesRegex(ValueError, "provenance mismatch"):
            validate("1" * 40, "2" * 40)


if __name__ == "__main__":
    unittest.main()
