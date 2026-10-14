# External provider / System voice qualification checkpoint — 2026-10-14

- `f51e442`: native voice selection regression verifies that a removed saved voice fails without switching to another installed voice, while explicit System Default remains valid.
- `ac3dd21`: native PCM parity regression covers planar f32, planar f64, interleaved f32/i16, and planar i32 conversion, with sample-rate preservation.
- `bd1bd6d`: exact rustfmt correction for the PCM regression.
- Exact-head GitHub Actions run `38653568928` **passed** on `bd1bd6d4641fdcd85510b32e360dc6290f6ee726`, including Rust quality, Rust tests, and generated backend-contract verification. Earlier runs were superseded or failed formatting; they are not qualification evidence.

## Still outstanding

The macOS native `AVSpeechSynthesizer` adapter, OS voice enumeration, buffer callback integration, playback/cancellation, Control Panel enablement, physical Mac audition, complete provider-routing matrix, and full `check:all` remain unqualified. All canonical ELSV checklist items stay unchecked until their named acceptance conditions are met.

The overlapping-provider credential-redaction issue is recorded in `docs/ELSV_REDACTION_PREFIX_REGRESSION_2026-10-14.md`. Source-write safety checks rejected attempted updates to the existing secret-store implementation. This remains an ELSV-200 security issue, not a completed fix.
