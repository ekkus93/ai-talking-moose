use super::types::LocalRuntimeError;
use crate::ai::local::catalog::LocalModelTemplateHint;

const THINK_START: &str = "<think>";
const THINK_END: &str = "</think>";
const QWEN3_NON_THINKING_PREFILL: &str = "<think>\n\n</think>\n\n";

/// Apply the generation-prompt portion of model-family policy after llama.cpp has rendered and
/// validated the GGUF's embedded chat template.
///
/// `llama-cpp-2` 0.1.154 does not expose arbitrary Jinja template kwargs. Qwen3's documented
/// `enable_thinking=false` template behavior is an empty reasoning block immediately after the
/// assistant opening tag, so the runtime applies that exact prefill here. Control tokens remain
/// isolated inside the local runtime rather than leaking into application call sites.
pub(super) fn apply_generation_prompt_policy(
    template_hint: LocalModelTemplateHint,
    mut rendered: String,
) -> String {
    if matches!(template_hint, LocalModelTemplateHint::Qwen3NonThinking) {
        rendered.push_str(QWEN3_NON_THINKING_PREFILL);
    }
    rendered
}

/// Remove any Qwen reasoning trace before a generation result can leave the native runtime.
///
/// The normal non-thinking path contains no reasoning tags in generated output because the prompt
/// already contains the empty reasoning block. This parser is defense in depth for a model that
/// nevertheless emits a reasoning block. Ambiguous, unterminated, mixed visible/reasoning, or
/// reasoning-only output fails closed instead of exposing potentially hidden reasoning to callers.
pub(super) fn sanitize_generated_output(
    template_hint: LocalModelTemplateHint,
    output: String,
) -> Result<String, LocalRuntimeError> {
    if !matches!(template_hint, LocalModelTemplateHint::Qwen3NonThinking) {
        return Ok(output);
    }

    sanitize_qwen3_output(output)
}

fn sanitize_qwen3_output(output: String) -> Result<String, LocalRuntimeError> {
    let Some(start) = output.find(THINK_START) else {
        return if output.contains(THINK_END) {
            Err(LocalRuntimeError::output_decode())
        } else {
            Ok(output)
        };
    };

    if !output[..start].trim().is_empty() {
        return Err(LocalRuntimeError::output_decode());
    }

    let reasoning_start = start + THINK_START.len();
    let Some(relative_end) = output[reasoning_start..].find(THINK_END) else {
        return Err(LocalRuntimeError::output_decode());
    };
    let reasoning_end = reasoning_start + relative_end;
    let answer_start = reasoning_end + THINK_END.len();
    let answer = &output[answer_start..];

    if answer.contains(THINK_START) || answer.contains(THINK_END) || answer.trim().is_empty() {
        return Err(LocalRuntimeError::output_decode());
    }

    Ok(answer
        .trim_start_matches(['\r', '\n'])
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::super::types::LocalRuntimeErrorKind;
    use super::*;

    #[test]
    fn qwen_nonthinking_prompt_prefills_an_empty_reasoning_block() {
        assert_eq!(
            apply_generation_prompt_policy(
                LocalModelTemplateHint::Qwen3NonThinking,
                "<|im_start|>assistant\n".to_string(),
            ),
            "<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
    }

    #[test]
    fn non_qwen_prompt_is_unchanged() {
        assert_eq!(
            apply_generation_prompt_policy(
                LocalModelTemplateHint::SmolLm2,
                "<|im_start|>assistant\n".to_string(),
            ),
            "<|im_start|>assistant\n"
        );
    }

    #[test]
    fn ordinary_qwen_answer_passes_through_without_reasoning_markers() {
        assert_eq!(
            sanitize_generated_output(
                LocalModelTemplateHint::Qwen3NonThinking,
                "A concise visible answer.".to_string(),
            )
            .unwrap(),
            "A concise visible answer."
        );
    }

    #[test]
    fn qwen_reasoning_block_is_removed_before_runtime_result() {
        assert_eq!(
            sanitize_generated_output(
                LocalModelTemplateHint::Qwen3NonThinking,
                "<think>private reasoning must never escape</think>\n\nVisible answer.".to_string(),
            )
            .unwrap(),
            "Visible answer."
        );
    }

    #[test]
    fn malformed_or_ambiguous_qwen_reasoning_fails_closed() {
        for output in [
            "<think>unterminated private reasoning",
            "</think>Visible answer.",
            "Visible prefix.<think>private reasoning</think>Visible answer.",
            "<think>private reasoning</think>",
            "<think>one</think>Visible<think>two</think>",
        ] {
            let error = sanitize_generated_output(
                LocalModelTemplateHint::Qwen3NonThinking,
                output.to_string(),
            )
            .unwrap_err();
            assert_eq!(error.kind, LocalRuntimeErrorKind::OutputDecode, "{output}");
        }
    }

    #[test]
    fn smollm_output_is_not_subject_to_qwen_reasoning_parser() {
        let text = "Literal <think> markup is ordinary SmolLM text.";
        assert_eq!(
            sanitize_generated_output(LocalModelTemplateHint::SmolLm2, text.to_string(),).unwrap(),
            text
        );
    }
}
