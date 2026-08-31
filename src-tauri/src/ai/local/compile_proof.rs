//! Minimal llama.cpp binding compile/API proof for LLM-003.
//!
//! This module is test-only on purpose. P5 owns the production runtime manager and must not be
//! started until this exact binding and CPU-only parameter surface compile on every supported CI
//! architecture.

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::model::params::LlamaModelParams;
use std::num::NonZeroU32;

#[test]
fn model_params_explicitly_request_zero_gpu_layers() {
    let params = LlamaModelParams::default().with_n_gpu_layers(0);
    assert_eq!(params.n_gpu_layers(), 0);
}

#[test]
fn context_params_explicitly_disable_gpu_offload() {
    let defaults = LlamaContextParams::default();
    assert!(
        defaults.offload_kqv(),
        "the pinned llama.cpp binding currently defaults KQV offload on; policy must override it"
    );

    let params = defaults
        .with_n_ctx(NonZeroU32::new(4096))
        .with_n_threads(4)
        .with_n_threads_batch(4)
        .with_offload_kqv(false)
        .with_op_offload(false);

    assert_eq!(params.n_ctx(), NonZeroU32::new(4096));
    assert_eq!(params.n_threads(), 4);
    assert_eq!(params.n_threads_batch(), 4);
    assert!(!params.offload_kqv());
    assert!(!params.op_offload());
}
