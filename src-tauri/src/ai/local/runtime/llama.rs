use super::manager::RuntimeEngine;
use super::types::{
    LocalRuntimeError, LocalRuntimeGenerateRequest, LocalRuntimeGeneration, RuntimeModelSpec,
};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::TokenToStringError;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

const PROMPT_BATCH_TOKENS: usize = 512;

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

    fn token_piece_bytes(
        model: &LlamaModel,
        token: LlamaToken,
    ) -> Result<Vec<u8>, LocalRuntimeError> {
        match model.token_to_piece_bytes(token, 16, true, None) {
            Ok(bytes) => Ok(bytes),
            Err(TokenToStringError::InsufficientBufferSpace(required)) if required < 0 => {
                let required = required
                    .checked_neg()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(LocalRuntimeError::output_decode)?;
                model
                    .token_to_piece_bytes(token, required, true, None)
                    .map_err(|_| LocalRuntimeError::output_decode())
            }
            Err(_) => Err(LocalRuntimeError::output_decode()),
        }
    }

    fn generate(
        &self,
        spec: &RuntimeModelSpec,
        request: &LocalRuntimeGenerateRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(LocalRuntimeError::model_not_loaded)?;
        let context_size =
            NonZeroU32::new(spec.context_size).ok_or_else(LocalRuntimeError::invalid_request)?;
        let thread_count =
            i32::try_from(spec.thread_count).map_err(|_| LocalRuntimeError::invalid_request())?;
        let context_params = LlamaContextParams::default()
            .with_n_ctx(Some(context_size))
            .with_n_threads(thread_count)
            .with_n_threads_batch(thread_count)
            .with_offload_kqv(false)
            .with_op_offload(false);

        let started = Instant::now();
        let mut context = model
            .new_context(&self.backend, context_params)
            .map_err(|_| LocalRuntimeError::context_creation())?;
        if cancellation.is_cancelled() {
            return Err(LocalRuntimeError::cancelled());
        }

        let prompt_tokens = model
            .str_to_token(&request.prompt, AddBos::Always)
            .map_err(|_| LocalRuntimeError::tokenization())?;
        if prompt_tokens.is_empty() {
            return Err(LocalRuntimeError::tokenization());
        }
        let prompt_token_count =
            u32::try_from(prompt_tokens.len()).map_err(|_| LocalRuntimeError::prompt_too_long())?;
        if prompt_token_count
            .checked_add(request.max_output_tokens)
            .is_none_or(|required| required > spec.context_size)
        {
            return Err(LocalRuntimeError::prompt_too_long());
        }

        let batch_capacity = PROMPT_BATCH_TOKENS.min(prompt_tokens.len()).max(1);
        let mut batch = LlamaBatch::new(batch_capacity, 1);
        for (chunk_index, chunk) in prompt_tokens.chunks(PROMPT_BATCH_TOKENS).enumerate() {
            if cancellation.is_cancelled() {
                return Err(LocalRuntimeError::cancelled());
            }
            batch.clear();
            let chunk_start = chunk_index
                .checked_mul(PROMPT_BATCH_TOKENS)
                .ok_or_else(LocalRuntimeError::prompt_too_long)?;
            for (offset, token) in chunk.iter().enumerate() {
                let absolute = chunk_start
                    .checked_add(offset)
                    .ok_or_else(LocalRuntimeError::prompt_too_long)?;
                let position =
                    i32::try_from(absolute).map_err(|_| LocalRuntimeError::prompt_too_long())?;
                let logits = absolute + 1 == prompt_tokens.len();
                batch
                    .add(*token, position, &[0], logits)
                    .map_err(|_| LocalRuntimeError::decode())?;
            }
            context
                .decode(&mut batch)
                .map_err(|_| LocalRuntimeError::decode())?;
        }

        let mut sampler = if request.temperature <= 0.0 {
            LlamaSampler::greedy()
        } else {
            LlamaSampler::chain_simple([
                LlamaSampler::temp(request.temperature),
                LlamaSampler::top_p(0.95, 1),
                LlamaSampler::dist(request.seed),
            ])
        };
        let mut output_bytes = Vec::new();
        let mut output_tokens = 0_u32;

        while output_tokens < request.max_output_tokens {
            if cancellation.is_cancelled() {
                return Err(LocalRuntimeError::cancelled());
            }
            let logits_index = batch
                .n_tokens()
                .checked_sub(1)
                .ok_or_else(LocalRuntimeError::decode)?;
            let token = sampler.sample(&context, logits_index);
            if model.is_eog_token(token) {
                break;
            }

            output_bytes.extend_from_slice(&Self::token_piece_bytes(model, token)?);
            output_tokens += 1;
            if output_tokens >= request.max_output_tokens {
                break;
            }

            batch.clear();
            let position = prompt_token_count
                .checked_add(output_tokens)
                .and_then(|position| position.checked_sub(1))
                .and_then(|position| i32::try_from(position).ok())
                .ok_or_else(LocalRuntimeError::decode)?;
            batch
                .add(token, position, &[0], true)
                .map_err(|_| LocalRuntimeError::decode())?;
            context
                .decode(&mut batch)
                .map_err(|_| LocalRuntimeError::decode())?;
        }

        let output =
            String::from_utf8(output_bytes).map_err(|_| LocalRuntimeError::output_decode())?;
        let duration = started.elapsed();
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        let tokens_per_second = if output_tokens == 0 || duration.is_zero() {
            None
        } else {
            Some(output_tokens as f32 / duration.as_secs_f32())
        };
        Ok(LocalRuntimeGeneration {
            text: output,
            prompt_tokens: prompt_token_count,
            output_tokens,
            duration_ms,
            tokens_per_second,
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

    fn generate(
        &mut self,
        spec: &RuntimeModelSpec,
        request: &LocalRuntimeGenerateRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        LlamaEngine::generate(self, spec, request, cancellation)
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
