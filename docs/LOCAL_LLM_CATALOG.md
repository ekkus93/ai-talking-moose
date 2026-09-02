# Local LLM Catalog and Maintenance Guide

The V1 Local model catalog is app-owned and uses immutable artifact identities. Model weights are **not** committed to Git, embedded in application bundles, downloaded by ordinary CI, or downloaded merely because a model/provider is selected.

The authoritative executable catalog is `src-tauri/src/ai/local/catalog.rs`. This document is the human-maintenance companion; `docs/LOCAL_LLM_MODEL_LICENSES.md` records downloadable model license/distribution metadata.

## Supported V1 models

### SmolLM2 360M Instruct Q4_K_M

- App model ID: `smollm2-360m-instruct-q4-k-m`
- Source repository: `bartowski/SmolLM2-360M-Instruct-GGUF`
- Pinned revision: `ab928a97ee49f3a015f35194879f68211291d6ca`
- Artifact: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Exact expected bytes: `270590880`
- SHA-256: `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`
- License: Apache-2.0
- Context limit: 8,192
- Recommended max output: 192 tokens
- Runtime template hint: SmolLM2 chat template
- Product role: recommended/default Local text model

### Qwen3 0.6B Q4_K_M

- App model ID: `qwen3-0-6b-instruct-q4-k-m`
- Source repository: `bartowski/Qwen_Qwen3-0.6B-GGUF`
- Pinned revision: `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`
- Artifact: `Qwen_Qwen3-0.6B-Q4_K_M.gguf`
- Exact expected bytes: `484220320`
- SHA-256: `9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`
- License: Apache-2.0
- Context limit: 32,768
- Recommended max output: 192 tokens
- Runtime template hint: Qwen3 non-thinking mode
- Product role: larger supported alternative

The canonical P12 run accepted both artifacts with real CPU generation. SmolLM2 remains recommended because it had materially lower disk/RAM/CPU cost in that run; that decision is not a claim that it has superior semantic quality. See `LOCAL_LLM_CPU_ACCEPTANCE_20260902.md`.

## Artifact-selection criteria

Do not add a model just because a GGUF exists. A supported entry should have all of the following:

1. **Appropriate model behavior.** Use an instruct/chat model suitable for short visible assistant output. If the family exposes reasoning/thinking modes, the V1 visible-output policy must be explicitly defined and testable.
2. **CPU practicality.** The artifact must be small enough to load and generate acceptably on the supported CPU-first application profile. Prefer conservative quantization such as `Q4_K_M` unless acceptance evidence justifies another choice.
3. **Known chat-template semantics.** The app/runtime must have a fail-closed template policy for the family. Do not add a catalog row and then concatenate guessed control tokens at call sites.
4. **Immutable source identity.** The source URL must be HTTPS and include a pinned 40-hex revision rather than `main`, `latest`, a branch name, or another moving reference.
5. **Single-file safe identity.** App model IDs and filenames must be path-safe single components; filenames must be `.gguf` files owned by the catalog.
6. **Verifiable artifact metadata.** Exact byte count and SHA-256 must be established independently before the entry is considered supported.
7. **License clarity.** Model license/source metadata must be known, compatible with the app's explicit-download policy, and recorded in `LOCAL_LLM_MODEL_LICENSES.md`.
8. **Real acceptance.** The production installer and native runtime must load/generate from the exact pinned artifact before support is declared complete.

## Pinning a revision

Use an immutable upstream commit/revision for the artifact source. For Hugging Face-style URLs, the production URL should have the form:

```text
https://huggingface.co/<repo>/resolve/<40-hex-revision>/<artifact>.gguf?download=true
```

Before committing a candidate:

- verify the revision resolves to the intended upstream repository state;
- verify the artifact filename exists at that revision;
- copy the same revision into the `revision` field;
- ensure `source_url` itself contains the revision;
- never substitute a branch or mutable tag after verification.

`validate_local_model_catalog()` rejects missing/malformed pinned revisions and other structural catalog errors, but that syntactic validation is not a substitute for independently verifying the remote artifact.

## Verifying exact bytes and SHA-256

Download the candidate outside the repository working tree. Do not put GGUFs under Git-controlled application directories.

Example:

```bash
candidate=/tmp/talking-moose-candidate.gguf
curl --fail --location --output "$candidate" '<pinned-artifact-url>'

# Exact bytes
wc -c < "$candidate"

# Linux
sha256sum "$candidate"

# macOS
shasum -a 256 "$candidate"
```

Record the exact integer byte count and lowercase 64-hex SHA-256 in `catalog.rs`. Verify them a second time from a separately retrieved artifact or independent evidence when practical. Do not round the production `expected_bytes` value to a displayed MB size.

The production installer independently enforces both values before atomic promotion. A mismatch is an install failure, not permission to accept a nearby artifact.

## Adding or changing a catalog entry

When adding a model or changing any immutable artifact identity:

1. Update `src-tauri/src/ai/local/catalog.rs`.
2. Add or extend the family/template policy and fixture tests if the model family is new.
3. Keep the default ID valid if `DEFAULT_LOCAL_TEXT_MODEL_ID` changes.
4. Update this document.
5. Update `docs/LOCAL_LLM_MODEL_LICENSES.md` with source, pinned revision, artifact, license, and the fact that the model is not bundled.
6. Regenerate/review the Rust-owned frontend contract if catalog-facing representative data changes:

   ```bash
   npm run generate:frontend-contract
   npm run check:generated-backend-contract
   ```

7. Run ordinary quality/packaging policy gates:

   ```bash
   npm run check:all
   python3 scripts/check_local_llm_packaging_policy.py
   ```

8. Run the manual real-model acceptance workflow for the changed model before describing it as supported.

Changing only display copy is not equivalent to changing the artifact identity, but it still requires ordinary tests/contract review where applicable.

## Real-model acceptance

The canonical workflow is `.github/workflows/local-llm-real-cpu-acceptance.yml` (**Local LLM Real CPU Acceptance**). It is intentionally manual-only and accepts:

- `all`;
- `smollm2-360m-instruct-q4-k-m`;
- `qwen3-0-6b-instruct-q4-k-m`.

For a catalog change, dispatch it on the exact intended branch/SHA and select the affected model (or `all`). The workflow:

1. builds the opt-in acceptance binary;
2. installs through the production installer while network access is available;
3. verifies exact pinned identity/bytes/hash;
4. enters denied-network generation;
5. records CPU/runtime evidence and reload behavior;
6. uploads machine-readable reports.

Do not infer acceptance from a unit test, compile proof, or successful download alone. Review the artifact report and record the exact workflow run/source SHA in the relevant reconciliation document.

If the changed model affects the recommended default, base that decision on measured usability and explicitly state what the acceptance does **not** prove (for example, it is not automatically a semantic-quality benchmark).

## Integrity and distribution policy

The production installer downloads only after explicit user action, streams into a unique staging file, bounds bytes against the catalog value, verifies exact size and SHA-256, and only then promotes the artifact into the revision-scoped install directory.

Ordinary CI, release builds, generated-contract export, and macOS bundles remain model-weight-free. `scripts/check_local_llm_packaging_policy.py`, `.gitignore`, and `scripts/verify_macos_bundle.sh` enforce that boundary. Model licenses are therefore tracked separately from shipped runtime/dependency notices.
