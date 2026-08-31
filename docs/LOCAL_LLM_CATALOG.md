# Local LLM catalog

The V1 local model catalog is app-owned and uses immutable artifact identities. Model weights are **not** committed to Git and are **not** downloaded by ordinary CI or application startup.

## SmolLM2 360M Instruct Q4_K_M

- App model ID: `smollm2-360m-instruct-q4-k-m`
- Source repository: `bartowski/SmolLM2-360M-Instruct-GGUF`
- Pinned revision: `ab928a97ee49f3a015f35194879f68211291d6ca`
- Artifact: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- Exact expected bytes: `270590880`
- SHA-256: `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`
- License: Apache-2.0
- Runtime template hint: SmolLM2 chat template

## Qwen3 0.6B Q4_K_M

- App model ID: `qwen3-0-6b-instruct-q4-k-m`
- Source repository: `bartowski/Qwen_Qwen3-0.6B-GGUF`
- Pinned revision: `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`
- Artifact: `Qwen_Qwen3-0.6B-Q4_K_M.gguf`
- Exact expected bytes: `484220320`
- SHA-256: `9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`
- License: Apache-2.0
- Runtime template hint: Qwen3 non-thinking mode

## Integrity policy

The pinned values above are the exact immutable LFS artifact identities used by the production installer. The installer downloads only after explicit user action, streams into a unique staging file, bounds bytes against the catalog value, verifies exact size and SHA-256, and only then promotes the artifact into the revision-scoped install directory.

Independent full-file rehashing and real CPU inference remain part of the explicit real-model acceptance phase. Ordinary unit CI does not fetch these third-party model weights.
