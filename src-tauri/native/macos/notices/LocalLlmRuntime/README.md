# Local LLM native runtime notice

Talking Moose AI links the CPU-only `llama.cpp` / `ggml` runtime through the pinned
Rust crates `llama-cpp-2 = 0.1.154` and `llama-cpp-sys-2 = 0.1.154` from
`utilityai/llama-cpp-rs` release `0.1.154` (`bed81ad4ab1a6c904b11d425608e50f976d8ea62`).
That release pins its `llama.cpp` submodule to
`5f55650a78f92aff4d48d671423e888fac0469ff`. `llama-cpp-sys-2` builds those
vendored native sources; no developer-local system llama.cpp installation is part
of the build contract.

The native llama.cpp/ggml source at that pinned submodule revision is MIT licensed.
`LLAMA_CPP_LICENSE` is bundled with the application as the native-runtime notice.
The Rust binding crates are declared `MIT OR Apache-2.0`; Talking Moose redistributes
them under the MIT option and bundles the upstream `LLAMA_CPP_RS_LICENSE_MIT` text.
The binding crates are also represented by the generated dependency inventory under
`../Dependencies/DEPENDENCY_LICENSES.md` at build/release time.

GGUF model weights are not part of this notice set and are not bundled with the
application. Catalog model source/license information is maintained separately in
`docs/LOCAL_LLM_MODEL_LICENSES.md` because those artifacts are downloaded only by
an explicit user install action.
