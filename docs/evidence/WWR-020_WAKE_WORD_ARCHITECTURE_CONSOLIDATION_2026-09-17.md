# WWR-020 Wake Word architecture consolidation evidence

WWR-020 establishes `crate::app::wake_word` as the one authoritative Wake Word V1 subsystem facade. The remediation specification permits an equivalent module boundary when ownership is equally clear; this placement avoids unrelated crate-root churn while the subsystem is still application-internal.

Selected authoritative components:

- settings: validated `app::wake_word_settings`, crate-private and exposed through `app::wake_word::settings`;
- KWS engine policy: strict `app::wake_word_engine`, crate-private and exposed through `app::wake_word::engine`;
- runtime lifecycle: the richer shared-state `asr::wake_word_runtime`, crate-private and exposed through `app::wake_word::runtime`;
- handoff: `asr::wake_word_handoff`, crate-private and exposed through `app::wake_word::handoff`;
- diagnostics: `asr::wake_word_diagnostics`, crate-private and exposed through `app::wake_word::diagnostics`;
- artifact manifest: `asr::wake_word_sherpa_manifest`, crate-private and exposed through `app::wake_word::manifest`.

The competing `app::wake_word_runtime` and `asr::wake_word_sherpa` module declarations are removed. Their tracked files contain only non-compiled migration markers because Ralph Bridge does not expose a file-delete operation. Thus the crate contains one compiled `WakeWordRuntimeManager` and one compiled V1 `SherpaKwsConfig`/`SherpaKwsEngine` policy.

Existing tests stay with the selected authoritative component sources, so no test depends on the removed duplicate runtime or engine. `app::wake_word::architecture_tests` additionally verifies that the duplicate declarations cannot silently return and that the facade resolves the selected manager, engine policy, settings, diagnostics, handoff, and manifest.
