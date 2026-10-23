# Bundled Native Notices

Run `bash scripts/prepare_macos_notices.sh` after initializing the pinned Whisper submodule. It stages the project license, third-party inventory, and upstream Whisper MIT license in `WhisperRuntime/WHISPER_CPP_LICENSE`.

Run `python3 scripts/collect_release_licenses.py` after installing dependencies to generate `Dependencies/DEPENDENCY_LICENSES.md` and its notice payloads. The checked-in `LocalLlmRuntime/` notices cover the statically linked llama.cpp runtime and Rust bindings.

Local TTS ONNX Runtime, model, voices, and dictionary artifacts remain verified user downloads. Their licenses are recorded in `docs/LOCAL_TTS_ASSET_LICENSES.md`; they are not application-bundle resources. Release verification requires the project, Whisper, Local LLM, and dependency notices.
