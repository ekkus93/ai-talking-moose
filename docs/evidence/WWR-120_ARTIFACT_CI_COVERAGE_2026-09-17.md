# WWR-120 — Wake artifact CI coverage evidence

WWR-120 makes the Wake artifact gate sensitive to every production identity boundary rather than only the legacy verifier/preparer tests.

The Wake Artifact Verification workflow now triggers on both identity freezers, their dedicated offline test suites, the production manifest, model/runtime preparation and verification code, runtime cache tests, the manifest validator, and the workflow itself. It executes the four explicit artifact/freezer suites requested by the remediation plan (verifier, preparation, model freezer, runtime freezer) plus the WWR-110 runtime preparation/cache suite.

`scripts/validate_wake_word_artifact_manifest.py --production` validates schema version, the explicit production-mode flag, model/freezer constants, exact consumed model-file set, keyword identity, runtime/freezer version and platform set, deterministic install/bundle roots, archive filenames/source URLs, architecture policy, and every required non-zero size/SHA-256 identity. Placeholder identities fail closed.

Safe extraction is exercised against malicious tar and zip traversal entries in the artifact preparation suite and against malicious model/runtime freezer archives. Cache bypass is covered by the runtime preparation suite, which corrupts an installed cached library and requires re-verification/re-preparation rather than trusting presence.

Exact PR-head and merge evidence is recorded in the remediation TODO when this change qualifies.
