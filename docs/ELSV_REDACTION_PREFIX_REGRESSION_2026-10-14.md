# Provider credential overlap regression

The current `SecretStore::redact` processes the Google credential before the OpenAI credential. When one configured credential is a prefix of the other, this ordering can leave the longer credential's trailing characters visible in diagnostics. The fix should process longer credentials first, with regression tests for both provider assignments. This is an unresolved ELSV-200 security follow-up; do not claim qualification until the source fix and exact-head Rust CI pass.
