# Whisper Migration Closeout — Acceptance Still Open

## Source-bound acceptance

Observed source: `c3d32fc213fc9a52cc3cb229af77925565429997`. All eight
workflows below completed successfully. A workflow pass applies to its executed
jobs, not skipped jobs or later source changes.

| Workflow | Run | Scope |
| --- | --- | --- |
| CI | [39902352829](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352829) | Linux frontend, Rust tests/Clippy, generated contracts, release metadata, provenance, packaging policy and license inventory |
| Whisper app smoke | [39902352871](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352871) | Production unsigned Tauri apps, arm64 job `120246541988` and Intel job `120246542243` |
| System TTS | [39902375294](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902375294) | Native arm64 synthesis/routing job `120246616680`; unsigned bundles arm64 `120246616329`, Intel `120246616602` |
| KittenTTS Whisper roundtrip | [39902352938](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352938) | Real pinned synthesis/transcription on arm64 under network denial |
| KittenTTS voice audition proxy | [39902352795](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352795) | All eight voices pass; objective tie-break recommends Bella, without changing Luna or claiming human audition |
| Wake real KWS | [39902352821](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352821) | Workflow's real-KWS acceptance |
| Wake performance evidence | [39902352846](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352846) | Workflow's corpus/listener evidence gates |
| Wake required gates | [39902352845](https://github.com/ekkus93/ai-talking-moose/actions/runs/39902352845) | Required-gates audit |

The CI run **skipped** the manual canonical-check-all job, dependency audit,
full compile-proof matrix and ordinary macOS matrix. It is not a full manual
CI qualification. Native settings/installer/pipeline and provider-isolation
tests on both Macs passed earlier at removal source
`f9d7310cf1a4c38d7c85180211eb48b2136d0a36`, native run `39895042644`,
arm64 job `120223399132` and Intel job `120223399238`.

The [archived roundtrip and audition JSON](whisper-closeout-c3d32fc2/)
records WER **0** and content recall **1** for every voice on the fixed corpus
phrase. `artifact-receipt.json` records GitHub artifact IDs, ZIP digests/bytes
and individual JSON hashes. This is intelligibility evidence, not physical
playback, subjective quality, thermal qualification or an owner voice decision.

Whisper source remains `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`;
model revision `5359861c739e955e79d9a303bcbc70fb988958b1`, 487,601,967 bytes,
SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`.
KittenTTS keeps its independently pinned `ort` dependency and runtime/model
installer. Earlier two-Mac Seatbelt inference at `6009d63c` is separately
recorded in `CROSS_PLATFORM_WHISPER_MACOS_REAL_CPU_SEATBELT_2026-10-22.md`;
it is not a post-removal real-corpus replay.

## Residual-reference review

`python3 scripts/audit_whisper_migration_references.py --output ...` scanned
905 tracked paths at `c3d32fc2`, matching 176 files across all inventory tokens.
The compact exhaustive path census is `whisper-closeout-c3d32fc2/reference-paths.json`.
The sole skipped gitlink is pinned `third_party/whisper.cpp`; provenance passed.
An additional `git grep -ni moonshine` over every tracked non-documentation
path was reviewed with these dispositions:

- `app/state.rs`: the two retired strings exist only in persisted-settings migration.
- Settings, routing, ASR type, wake-settings, command, conversation and frontend
  test files: migration compatibility or assertions rejecting retired values,
  cloud escalation and obsolete UI. They do not instantiate Moonshine.
- App/native workflows and policy scripts: negative assertions and named
  compatibility regression tests. The Local LLM P13 workflow invokes old
  preparation scripts only inside its fixed historical baseline checkout.
- `prepare_macos_notices.sh` and `.gitignore`: remove/ignore stale generated
  Moonshine staging. User model data is not deleted or reinterpreted.
- Reference inventory: intentionally searches for retired names.
- README/privacy/current guides: explain migration or explicitly historical evidence.
- `AGENTS.md`: retained under the owner's earlier instruction not to modify an
  existing file; old architecture descriptions are superseded by current code.
  `memory.md` and dated plans/evidence remain historical handoffs.

No production Moonshine runtime, factory, installer, enum variant, native
manifest, model download or bundle framework remains. Unrelated ONNX ownership
for KittenTTS and wake-word assets remains intact.

## Work in this loop

- Updated the npm lockfile within existing version ranges: Vite 8.3.5,
  Vitest 4.1.12 and compatible transitive fixes. Audit falls from 12 findings
  (8 high/4 moderate) to **7 (5 high/2 moderate)**; production remains clean.
- Remaining findings are the Tailwind 3 dependency chain (`braces`, `chokidar`,
  `micromatch`, `fast-glob`, `postcss-selector-parser`, `postcss-nested`,
  `tailwindcss`). `braces` has no patched release:
  [GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm).
  Removing this chain requires a separately qualified Tailwind migration;
  no audit suppression, forced dependency override or framework migration is
  claimed here. WSP-600's complete audited-dependency gate stays open.
- Extended bundle verification to reject bundled dylibs/frameworks and inspect
  every Mach-O helper for architecture, deployment floor and developer load
  paths. Added real-Mach-O negative fixtures to the two-architecture app smoke.
  These changes require fresh macOS execution; older passes do not qualify them.
- Updated current Whisper/voice documentation and separated completed
  implementation checks from physical and release acceptance below.

## Local verification of this loop

On Linux, with base `c3d32fc2` plus this loop's working changes,
`VITEST_MAX_WORKERS=2 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run check:all`
passed: 195 frontend tests, Rustfmt, all-target/all-feature Clippy, 1,082 Rust
unit/integration tests (4 ignored artifact/live-API tests), frontend build and
generated-backend-contract drift/31-shape checks. Script unit tests: 44 passed;
repository Python tests: 29 passed. Provenance, both packaging policies,
macOS workflow policy, generated-tree hygiene and license collection passed
(14 npm, 386 Cargo, 30 declaration-only entries). Shell syntax, ShellCheck at
warning severity, actionlint grammar and diff whitespace checks passed.

The host's default Python is 3.10 and cannot import `tomllib`: initial packaging
commands failed for that environment reason. They passed using the installed
Python 3.11; no checker was weakened. Production audit passed; the all-dependency
audit still fails with 7 findings. Native macOS negative fixtures cannot run on
this Linux host and remain pending their fresh remote run.

## Assistance required before final acceptance

1. **Apple credentials:** preflight run `39895042680` explicitly reports all
   five secrets missing: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. Configure them in GitHub
   Actions; do not place secret values in a document, issue or chat.
2. **Representative devices and human observation:** complete WSP-700 and
   `docs/ELSV_SYSTEM_TTS_PHYSICAL_AUDITION_2026-10-15.md` on Apple Silicon,
   Intel and Linux. Use `scripts/run_p13_macos_release_acceptance.sh` for its
   release protocol. Record exact build/OS, input/output device, selected
   providers/voice, expected versus observed behavior and artifact identity.
   CI synthesis does not prove audible playback or bubble completion.
3. **Intel performance:** the recorded real-model RTF is 1.249–1.267, above
   real time. Sustained/thermal representative-device evidence and an explicit
   owner release decision are still required; no acceptance or deferral is
   inferred from authorization to remove Moonshine.
4. **Release license review:** the inventory passes but flags 30 packages with
   declaration-only evidence for distribution review. Signing, notarization,
   Gatekeeper and clean-machine install/upgrade/rollback remain open.

Further autonomous work remains too: qualify the new bundle checks, replay
current-source manual native/real-CPU acceptance, investigate sustained Intel
behavior and remove the remaining vulnerable development dependency chain.
This record is a checkpoint awaiting assistance, not final migration completion.
