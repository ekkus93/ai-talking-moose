# Local LLM P3/P4 implementation notes

This tranche implements the authoritative local-model catalog and the model lifecycle/storage boundary. It does not yet load GGUF files with llama.cpp.

## Storage layout

Production model root:

`<app-data>/models/llm/`

Installed artifact layout:

`<model-id>/<revision>/<artifact.gguf>`

A verified install marker is stored beside the artifact. Temporary downloads live under the model root's `.staging` directory so final promotion can use a same-filesystem atomic rename.

## Safety properties

- Catalog model IDs and filenames are validated as single safe path components.
- Download URLs must be HTTPS and contain a pinned 40-hex revision.
- Downloads happen only through an explicit install command.
- A declared Content-Length, when present, must exactly match the catalog size.
- Streaming aborts if downloaded bytes exceed the catalog size.
- Exact file size and SHA-256 are verified before promotion.
- Cancellation removes the staging artifact.
- Stale staging artifacts are removed during installer initialization.
- Duplicate installs of one model are rejected as busy.
- Delete never follows a model-directory symlink.
- Deleting the selected model preserves the selection and reports it as not installed; it never silently selects another local model or a cloud provider.
- Runtime load/delete coordination will share this ownership boundary when P5 adds llama.cpp. Until then no local GGUF can be loaded by the application.

## Acceptance boundary

P3/P4 unit/CI acceptance uses injected transport and small deterministic files. Third-party GGUF weights remain excluded from ordinary CI. Independent full-artifact rehashing and CPU generation are intentionally deferred to the real-model acceptance phase after P5/P6.
