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

    def test_architectures_cannot_swap_runner_families(self) -> None:
        arm = "          - arch: arm64\n            runner: macos-15\n"
        intel = "          - arch: x86_64\n            runner: macos-15-intel\n"
        self.assertIn(arm, self.workflow)
        self.assertIn(intel, self.workflow)
        swapped = self.workflow.replace(
            arm, arm.replace("runner: macos-15\n", "runner: macos-15-intel\n"), 1
        ).replace(
            intel, intel.replace("runner: macos-15-intel\n", "runner: macos-15\n"), 1
        )
        with self.assertRaisesRegex(ValueError, "architecture-to-runner"):
            check_workflow(swapped)

    def test_missing_seatbelt_network_deny_fails_closed(self) -> None:
        changed = self.workflow.replace("(deny network*)", "(allow network*)", 1)
        self.assertNotEqual(changed, self.workflow)
        with self.assertRaisesRegex(ValueError, "macOS Seatbelt profile"):
            check_workflow(changed)

    def test_comment_cannot_masquerade_as_network_denial(self) -> None:
        # A raw substring search would accept this permissive policy.
        original = "          (deny network*)\n"
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(
            original, "          (allow network*)\n          # (deny network*)\n", 1
        )
        with self.assertRaisesRegex(ValueError, "macOS Seatbelt profile"):
            check_workflow(changed)

    def test_decoy_network_denial_outside_policy_is_rejected(self) -> None:
        original = "          (deny network*)\n"
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(original, "          (allow network*)\n", 1)
        changed = changed.replace(
            "          SEATBELT\n", "          SEATBELT\n          # (deny network*)\n", 1
        )
        with self.assertRaisesRegex(ValueError, "macOS Seatbelt profile"):
            check_workflow(changed)

    def test_duplicate_seatbelt_policy_declaration_is_rejected(self) -> None:
        original = """          cat > "$sandbox_profile" <<'SEATBELT'"""
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(original, original + "\n" + original, 1)
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

    def test_commented_network_denial_assertion_is_not_evidence(self) -> None:
        original = '          assert transcript["network_denial_probe_passed"] is True'
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(original, '          # ' + original.strip(), 1)
        with self.assertRaisesRegex(ValueError, "executable top-level"):
            check_workflow(changed)

    def test_literal_string_cannot_masquerade_as_attestation(self) -> None:
        original = '          assert repeated["network_denied_required"] is True'
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(original, '          ' + repr(original.strip()), 1)
        with self.assertRaisesRegex(ValueError, "executable top-level"):
            check_workflow(changed)

    def test_unreachable_conditional_assertion_is_not_evidence(self) -> None:
        original = '          assert repeated["network_denial_probe_passed"] is True'
        self.assertIn(original, self.workflow)
        changed = self.workflow.replace(
            original,
            '          if False:\n              ' + original.strip(),
            1,
        )
        with self.assertRaisesRegex(ValueError, "executable top-level"):
            check_workflow(changed)

    def test_linux_namespace_substitution_is_rejected(self) -> None:
        changed = self.workflow + "\n# sudo unshare --net\n"
        with self.assertRaisesRegex(ValueError, "Linux namespace-based"):
            check_workflow(changed)


    def test_commented_sandbox_command_cannot_mask_unprotected_real_inference(self) -> None:
        for report in ("transcribe.json", "transcribe-repeat.json"):
            with self.subTest(report=report):
                original = next(
                    line for line in self.workflow.splitlines()
                    if "if ! sandbox-exec" in line and f'"$reports/{report}"' in line
                )
                bare = original.replace('sandbox-exec -f "$sandbox_profile" ', "", 1)
                bare = bare.replace(" --require-network-denied", "", 1)
                # The decoy preserves all required raw substrings and the
                # original flag count, but is only a shell comment.
                changed = self.workflow.replace(
                    original, bare + "\n          # " + original.strip(), 1
                )
                with self.assertRaisesRegex(ValueError, "active transcription"):
                    check_workflow(changed)

    def test_extra_unprotected_real_inference_fails_closed(self) -> None:
        marker = '          "$binary" delete "$model" "$reports/delete.json"'
        self.assertIn(marker, self.workflow)
        extra = (
            '          "$binary" transcribe "$model" '
            'third_party/whisper.cpp/samples/jfk.wav "$reports/extra.json"'
        )
        changed = self.workflow.replace(marker, extra + "\n" + marker, 1)
        with self.assertRaisesRegex(ValueError, "active transcription"):
            check_workflow(changed)

    def test_missing_model_negative_probe_must_remain_guarded(self) -> None:
        original = next(
            line for line in self.workflow.splitlines()
            if 'if "$binary" transcribe ' in line and "missing-model.json" in line
        )
        changed = self.workflow.replace(original, original.replace("if ", "", 1), 1)
        with self.assertRaisesRegex(ValueError, "active transcription"):
            check_workflow(changed)

    def test_native_whisper_cannot_be_skipped_by_ci_checkout(self) -> None:
        import re

        ci = (WORKFLOW.parents[2] / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        for job in ("rust-clippy", "rust-tests", "backend-contract", "canonical-check-all"):
            with self.subTest(job=job):
                match = re.search(
                    rf"(?ms)^  {re.escape(job)}:\n(.*?)(?=^  [a-z][\w-]*:\n|\Z)",
                    ci,
                )
                self.assertIsNotNone(match, f"missing CI job {job}")
                section = match.group(1)
                self.assertRegex(
                    section,
                    r'(?m)^    env:\n      TALKING_MOOSE_REQUIRE_WHISPER_NATIVE: "1"$',
                )
                self.assertIn(
                    "      - name: Check out repository\n"
                    "        uses: actions/checkout@v7\n"
                    "        with:\n"
                    "          submodules: true\n",
                    section,
                )

    def test_full_validation_executes_real_canonical_checks(self) -> None:
        ci = (WORKFLOW.parents[2] / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        marker = "\n  canonical-check-all:\n"
        self.assertEqual(ci.count(marker), 1)
        canonical = ci.split(marker, 1)[1]
        for required in (
            "bash scripts/prepare_ubuntu_apt.sh",
            "components: rustfmt, clippy",
            "run: npm ci",
            'CARGO_BUILD_JOBS: "2"',
            "run: npm run check:all",
            "python3 scripts/check_whisper_macos_acceptance_workflow.py",
            "python3 -m unittest scripts.test_check_whisper_macos_acceptance_workflow",
        ):
            with self.subTest(required=required):
                self.assertIn(required, canonical)

    def test_policy_changes_remain_classified_as_ci_plumbing(self) -> None:
        ci = (WORKFLOW.parents[2] / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        classifier = next(
            line for line in ci.splitlines()
            if ".github/workflows/*|.github/actions/*|" in line
        )
        for path in (
            "scripts/check_whisper_macos_acceptance_workflow.py",
            "scripts/test_check_whisper_macos_acceptance_workflow.py",
        ):
            self.assertIn(path, classifier)
        self.assertIn(
            "python3 -m unittest scripts.test_check_whisper_macos_acceptance_workflow",
            ci,
        )


if __name__ == "__main__":
    unittest.main()
