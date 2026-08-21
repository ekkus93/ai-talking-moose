# Moonshine Model Installation Policy

Talking Moose installs Moonshine model weights only after an explicit user action. Model payloads are fetched over HTTPS from the pinned `download.moonshine.ai` model directory, streamed into a per-attempt `.partial` staging directory, and verified against the immutable manifest before promotion.

Interrupted, cancelled, or failed attempts are discarded. V1 deliberately does **not** resume partial model files: an explicit retry starts from the beginning against the same pinned revision and SHA-256 manifest. This keeps retry behavior deterministic and prevents partially verified bytes from crossing attempts. A previously installed model is left in place until its replacement has downloaded, verified, and can be atomically promoted.

The model downloader is restricted to the seven declared streaming-model artifacts and must never install the Moonshine native runtime or executable code. Native runtime packaging is a separate application-build responsibility.
