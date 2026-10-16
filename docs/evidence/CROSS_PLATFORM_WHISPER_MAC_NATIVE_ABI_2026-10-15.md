# macOS native Whisper ABI acceptance

At commit `f5d88ed3b4a1e0ca50edfa2b346e87c478a50e6a`, workflow run `38827895555` succeeded on arm64 (job `116739553617`) and x86_64 (job `116739553795`). Each job checked the pinned whisper.cpp source, compiled the C ABI layout probe, statically linked the acceptance executable and invoked the native runtime probe. This does not prove real-model inference, a packaged app, or microphone acceptance.

WSP-200 remains open. Keep Moonshine and its macOS default until genuine pinned-model transcription passes on both architectures.
