# Moonshine Model Installer Policy

Status: **V1R-ASR-006 implementation contract**
Recorded: 2026-08-19

The Moonshine model installer is an explicit user-action path. Creating the
installer, checking a model path, or verifying an existing installation performs
no network I/O. A model download starts only when the application calls the
installer's `install` operation for a user-selected Tiny or Small model.

## Download policy

- Only the immutable HTTPS URLs from the pinned V1 manifests are accepted.
- HTTPS redirects are allowed only to another HTTPS URL and are bounded to
  three redirects.
- Install requests are serialized process-wide. V1 downloads one model component at a time,
  so model-download concurrency is hard-bounded to one even if more than one installer
  object exists.
- The native Moonshine runtime is never installed by this downloader. V1 accepts
  only the seven pinned model payload filenames (`.ort`, config, and tokenizer)
  from the application manifest.
- The installer performs a free-space preflight on Unix platforms before any
  network request and requires the payload size plus 8 MiB of staging/metadata
  headroom. A probe error fails closed. On platforms where free-space discovery
  is genuinely unavailable, the installer continues with the same strict streamed
  size/integrity limits.

## Staging and integrity

Each revision is installed below a versioned directory:

`<model-root>/<model-id>/<revision>/`

A new download is first written to a unique sibling directory ending in
`.partial`. Every component is verified while it streams to disk:

1. the stream may never exceed the manifest byte count;
2. the final byte count must match exactly;
3. SHA-256 must match the application-owned cryptographic hash;
4. CRC32C must match the pinned upstream secondary checksum;
5. a server `Content-Length`, when present, must match the manifest;
6. each completed staging file is flushed and synchronized before promotion.

The installer writes an application-owned install marker containing the model
ID, revision, expected payload size, and pinned runtime compatibility metadata.
A model is considered installed only when all payloads and that marker verify.

## Promotion and existing installations

A verified existing install is never redownloaded or replaced. If the target
revision exists but is corrupt, the corrupt directory remains in place while a
new candidate is downloaded and verified. Only after the complete staging
directory verifies does the installer rename the old corrupt directory aside
and atomically rename the verified staging directory into the target path on the
same filesystem. If promotion fails, the previous directory is restored when
possible.

A failed download, cancellation, truncation, checksum mismatch, or wrong pinned
revision never promotes staging data and never replaces a known-good model.

## Cancellation, retry, and resume

V1 deliberately has **no transparent retry and no partial-file resume**.
Cancellation is cooperative, including while an install request is queued behind
the process-wide installer lock and again immediately before the promotion commit
point. Graceful failures delete their staging directory immediately. A process crash
can leave a `.partial` directory; the next explicit install removes stale
partial directories for that revision before starting from byte zero.

The user may explicitly retry after a retryable network, disk-space, or
integrity error. This policy avoids treating unverified partial bytes as trusted
resume state and keeps retry behavior visible and bounded.

## Test policy

Ordinary tests use an injected fake transport and temporary directories with
small fixture byte strings. They do not contact `download.moonshine.ai`, do not
download model weights, do not require microphone hardware, and do not require
the native Moonshine runtime. Regression coverage includes successful atomic
promotion, already-installed idempotence, serialized installs, disk-space
failure before network access, cancellation/interruption cleanup, stale partial
cleanup, size/SHA-256/CRC32C failures, revision rejection, and rejection of
native-runtime artifacts.
