# WWR-510 Privacy Error Audit Guard — 2026-09-22

## Scope

This evidence records an additional deterministic audit guard for Wake Word privacy-safe diagnostics and production error/log surfaces.

## Change

The Wake Word privacy audit now requires:

- concrete sanitizer regression coverage for path-like and token-like data;
- sanitized missing-artifact and missing-native-library error evidence;
- sanitized native architecture mismatch evidence;
- no production Wake Word logging macros in Wake Word Rust surfaces;
- no sensitive production error/log string literal fragments for credentials, secrets, transcripts, raw audio, private audio content, absolute paths, or file paths;
- no potentially path-leaking production formatting fragments such as direct path/model/runtime formatting.

## Non-claims

This is a source/privacy guard only. It does not claim real Wake Word audio acceptance, platform acceptance, performance acceptance, or final WWR-950/960 qualification.

## Related TODO items

This supports WWR-510 and WWR-900 source/privacy audit requirements:

- audit errors/logs for credentials;
- audit errors/logs for unnecessary absolute paths;
- audit errors/logs for audio content;
- audit logs/errors/metrics for raw audio;
- audit logs/errors/metrics for secrets;
- audit logs/errors/metrics for unnecessary paths.
