"""Fail-closed mutation tests for the opt-in macOS network-denied CPU gate.

All cases operate on workflow source text. No models, network services,
credentials, microphones, or real inference are used.
"""

import unittest

from scripts.check_whisper_macos_acceptance_workflow import WORKFLOW, check_workflow


class MacWhisperNetworkPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_current_manual_workflow_preserves_sandbox_contract(self) -> None:
        check_workflow(self.workflow)

    def test_missing_seatbelt_network_deny_fails_closed(self) -> None:
        changed = self.workflow.replace("(deny network*)", "(allow network*)", 1)
        self.assertNotEqual(changed, self.workflow)
        with self.assertRaisesRegex(ValueError, "macOS Seatbelt profile"):
            check_workflow(changed)

    def test_unprotected_primary_transcription_is_rejected(self) -> None:
        sandboxed = 'sandbox-exec -f "$sandbox_profile" "$binary" transcribe '
        self.assertEqual(self.workflow.count(sandboxed), 2)
        changed = self.workflow.replace(sandboxed, '"$binary" transcribe ', 1)
        with self.assertRaisesRegex(ValueError, "first inference under sandbox"):
            check_workflow(changed)

    def test_repeated_transcription_without_required_denial_is_rejected(self) -> None:
        argument = '"$reports/transcribe-repeat.json" --require-network-denied'
        self.assertIn(argument, self.workflow)
        changed = self.workflow.replace(
            argument, '"$reports/transcribe-repeat.json"', 1
        )
        with self.assertRaisesRegex(ValueError, "repeated inference under sandbox"):
            check_workflow(changed)

    def test_unsubstantiated_json_offline_attestation_is_rejected(self) -> None:
        original = 'assert transcript["network_denial_probe_passed"] is True'
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(
            original, 'assert transcript["network_denial_probe_passed"] is False', 1
        )
        with self.assertRaisesRegex(ValueError, "first probe passed"):
            check_workflow(changed)

    def test_linux_namespace_substitution_is_rejected(self) -> None:
        changed = self.workflow + "\n# sudo unshare --net\n"
        with self.assertRaisesRegex(ValueError, "Linux namespace-based"):
            check_workflow(changed)


if __name__ == "__main__":
    unittest.main()
