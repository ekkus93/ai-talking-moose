# Local LLM Model Catalog Licenses

This document records license/source metadata for **downloadable model artifacts**.
It is intentionally separate from the application dependency inventory because the
GGUF weights are **not bundled or redistributed** in Talking Moose AI. A user must
explicitly choose **Download & Verify** before the installer retrieves a pinned
artifact.

| Catalog ID | Upstream model | Pinned GGUF revision | Artifact | License | Distribution in app |
| --- | --- | --- | --- | --- | --- |
| `smollm2-360m-instruct-q4-k-m` | Hugging Face SmolLM2-360M-Instruct | `ab928a97ee49f3a015f35194879f68211291d6ca` | `SmolLM2-360M-Instruct-Q4_K_M.gguf` | `Apache-2.0` | Not bundled; explicit verified download |
| `qwen3-0-6b-instruct-q4-k-m` | Qwen3-0.6B | `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db` | `Qwen_Qwen3-0.6B-Q4_K_M.gguf` | `Apache-2.0` | Not bundled; explicit verified download |

The authoritative executable catalog remains
`src-tauri/src/ai/local/catalog.rs`, including source URLs, exact byte counts, and
SHA-256 digests. Catalog changes must keep this document synchronized and must
re-run real-model acceptance before a new model is treated as supported.

This file does not replace upstream license terms. It records the license metadata
verified for the pinned catalog entries and the application's distribution policy.
