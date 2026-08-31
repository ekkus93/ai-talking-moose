use super::{LocalRuntimeError, RuntimeEngine, RuntimeModelSpec};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use std::sync::Arc;

pub(super) struct LlamaEngine {
    backend: LlamaBackend,
    model: Option<Arc<LlamaModel>>,
}

impl LlamaEngine {
    pub(super) fn initialize() -> Result<Self, LocalRuntimeError> {
        let mut backend = LlamaBackend::init().map_err(|_| LocalRuntimeError::initialization())?;
        // llama.cpp may include the model path in native diagnostics. Local runtime failures are
        // surfaced through our sanitized categories instead, so native stderr logging is disabled.
        backend.void_logs();
        Ok(Self {
            backend,
            model: None,
        })
    }
}

impl RuntimeEngine for LlamaEngine {
    fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
        debug_assert!(self.model.is_none());
        let params = LlamaModelParams::default().with_n_gpu_layers(0);
        let model = LlamaModel::load_from_file(&self.backend, &spec.path, &params)
            .map_err(|_| LocalRuntimeError::model_load())?;
        self.model = Some(Arc::new(model));
        Ok(())
    }

    fn unload_model(&mut self) {
        self.model = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn cached_native_objects_can_live_behind_the_runtime_owner() {
        assert_send::<LlamaBackend>();
        assert_sync::<LlamaBackend>();
        assert_send::<LlamaModel>();
        assert_sync::<LlamaModel>();
    }
}
