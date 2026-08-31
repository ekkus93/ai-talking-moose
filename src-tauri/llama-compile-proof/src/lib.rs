// Reuse the application's LLM-003 CPU/offload policy proof without building the Tauri crate.
// The normal Rust/macOS jobs still compile llama-cpp-2 as a direct application dependency.
#[path = "../../src/ai/local/compile_proof.rs"]
mod proof;
