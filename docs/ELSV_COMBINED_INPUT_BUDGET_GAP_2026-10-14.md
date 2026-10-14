# ELSV combined input size acceptance gap

Observed on master ed91763eab3a0904354625bea68ab7d01f303398 (2026-10-14).

The OpenAI Responses and loopback text adapters each bound the prompt and optional system instruction separately to 64 KiB. A request containing a 40,000-byte prompt and a 40,000-byte system instruction therefore passes both individual checks even though its combined input exceeds 64 KiB.

Required remediation:
- Bound the sum of prompt bytes and system-instruction bytes using saturating arithmetic before any network I/O in both adapters.
- Preserve the existing per-field limits and sanitized Setup error.
- Add deterministic tests for individually valid fields whose combined size exceeds the cap.
- Run Rust formatting and exact-head CI; do not count this as ELSV-200/300 completion until full acceptance.

The privacy guide also contains two outdated statements that Google text selection itself starts Gemini Live. Live routing is determined by explicit Gemini Live audio ASR; Google text with local ASR uses non-Live Talk.
