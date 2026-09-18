# WWR-120 — Wake artifact CI coverage evidence

WWR-120 makes the Wake artifact gate sensitive to every production identity boundary rather than only the legacy verifier/preparer tests.

The Wake Artifact Verification workflow now triggers on both current production identity freezers, the pre-existing deterministic freezer helpers and their tests, the production manifest, model/runtime preparation and verification code, runtime cache tests, the manifest validator, and the workflow itself. It executes the repository's four pre-existing Wake artifact Python suites (`test-verify-wake-word-artifacts.py`, `test-prepare-wake-word-artifacts.py`, `test-freeze-wake-word-model-identities.py`, and `test-freeze-wake-word-runtime-identities.py`) plus focused tests for the newer production freezers and the WWR-110 runtime preparation/cache suite.

`scripts/validate_wake_word_artifact_manifest.py --production` validates schema version, the explicit production-mode flag, model/freezer constants, exact consumed model-file set, keyword identity, runtime/freezer version and platform set, deterministic install/bundle roots, archive filenames/source URLs, architecture policy, and every required non-zero size/SHA-256 identity. Placeholder identities fail closed.

Safe extraction is exercised against malicious tar and zip traversal entries in the artifact preparation suite and against malicious model/runtime freezer archives. Cache bypass is covered by the runtime preparation suite, which corrupts an installed cached library and requires re-verification/re-preparation rather than trusting presence.

Exact PR-head and merge evidence is recorded in the remediation TODO when this change qualifies.
