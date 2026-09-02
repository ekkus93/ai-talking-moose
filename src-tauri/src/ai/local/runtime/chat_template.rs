use super::reasoning::apply_generation_prompt_policy;
use super::types::{LocalRuntimeError, LocalRuntimeGenerateRequest};
use crate::ai::local::catalog::LocalModelTemplateHint;
use llama_cpp_2::model::LlamaModel;

const SMOLLM2_DEFAULT_SYSTEM_INSTRUCTION: &str =
    "You are a helpful AI assistant named SmolLM, trained by Hugging Face";
const TEMPLATE_FIXTURE_SYSTEM: &str = "system fixture sentinel";
const TEMPLATE_FIXTURE_USER: &str = "user fixture sentinel";

fn append_chatml_message(rendered: &mut String, role: &str, content: &str) {
    rendered.push_str("<|im_start|>");
    rendered.push_str(role);
    rendered.push('\n');
    rendered.push_str(content);
    rendered.push_str("<|im_end|>\n");
}

fn render_family_chat_prompt(
    template_hint: LocalModelTemplateHint,
    system_instruction: Option<&str>,
    user_prompt: &str,
) -> String {
    let mut rendered = String::new();
    match template_hint {
        LocalModelTemplateHint::SmolLm2 => append_chatml_message(
            &mut rendered,
            "system",
            system_instruction.unwrap_or(SMOLLM2_DEFAULT_SYSTEM_INSTRUCTION),
        ),
        LocalModelTemplateHint::Qwen3NonThinking => {
            if let Some(system_instruction) = system_instruction {
                append_chatml_message(&mut rendered, "system", system_instruction);
            }
        }
    }
    append_chatml_message(&mut rendered, "user", user_prompt);
    rendered.push_str("<|im_start|>assistant\n");
    rendered
}

fn validate_embedded_template_source(
    template_hint: LocalModelTemplateHint,
    source: &str,
) -> Result<(), LocalRuntimeError> {
    let required_fragments: &[&str] = match template_hint {
        LocalModelTemplateHint::SmolLm2 => &[
            "<|im_start|>",
            "<|im_end|>",
            "add_generation_prompt",
            "messages[0]",
            "system",
            "assistant",
            SMOLLM2_DEFAULT_SYSTEM_INSTRUCTION,
        ],
        LocalModelTemplateHint::Qwen3NonThinking => &[
            "<|im_start|>",
            "<|im_end|>",
            "add_generation_prompt",
            "assistant",
            "enable_thinking",
            "<think>",
            "</think>",
        ],
    };

    if required_fragments
        .iter()
        .all(|fragment| source.contains(fragment))
    {
        Ok(())
    } else {
        Err(LocalRuntimeError::chat_template())
    }
}

pub(super) fn validate_model_chat_template(
    model: &LlamaModel,
    template_hint: LocalModelTemplateHint,
) -> Result<(), LocalRuntimeError> {
    let template = model
        .chat_template(None)
        .map_err(|_| LocalRuntimeError::chat_template())?;
    let source = template
        .to_str()
        .map_err(|_| LocalRuntimeError::chat_template())?;
    validate_embedded_template_source(template_hint, source)
}

pub(super) fn render_chat_prompt(
    model: &LlamaModel,
    template_hint: LocalModelTemplateHint,
    request: &LocalRuntimeGenerateRequest,
) -> Result<String, LocalRuntimeError> {
    // The pinned llama.cpp template API intentionally supports only a subset of Jinja semantics.
    // Both catalog families use semantics outside that subset: SmolLM2 conditionally injects its
    // default system instruction and Qwen3 uses `enable_thinking`. Validate the GGUF's embedded
    // family invariants, then render the only message shape this runtime supports deterministically
    // instead of silently accepting llama.cpp's lossy generic-template interpretation.
    validate_model_chat_template(model, template_hint)?;
    let rendered = render_family_chat_prompt(
        template_hint,
        request.system_instruction.as_deref(),
        &request.prompt,
    );
    Ok(apply_generation_prompt_policy(template_hint, rendered))
}

#[cfg(test)]
mod tests {
    use super::super::types::LocalRuntimeErrorKind;
    use super::*;

    const SMOLLM2_CANONICAL_TEMPLATE: &str = "{% for message in messages %}{% if loop.first and messages[0]['role'] != 'system' %}{{ '<|im_start|>system\nYou are a helpful AI assistant named SmolLM, trained by Hugging Face<|im_end|>\n' }}{% endif %}{{'<|im_start|>' + message['role'] + '\n' + message['content'] + '<|im_end|>' + '\n'}}{% endfor %}{% if add_generation_prompt %}{{ '<|im_start|>assistant\n' }}{% endif %}";
    const QWEN3_CANONICAL_TEMPLATE_FRAGMENT: &str = "{% if messages[0].role == 'system' %}{{ '<|im_start|>system\n' + messages[0].content + '<|im_end|>\n' }}{% endif %}{% for message in messages %}{{ '<|im_start|>' + message.role + '\n' + message.content + '<|im_end|>\n' }}{% endfor %}{% if add_generation_prompt %}{{ '<|im_start|>assistant\n' }}{% if enable_thinking is defined and enable_thinking is false %}{{ '<think>\n\n</think>\n\n' }}{% endif %}{% endif %}";

    #[test]
    fn smollm2_family_fixtures_match_canonical_template_behavior() {
        assert_eq!(
            render_family_chat_prompt(
                LocalModelTemplateHint::SmolLm2,
                None,
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nYou are a helpful AI assistant named SmolLM, trained by Hugging Face<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
        assert_eq!(
            render_family_chat_prompt(
                LocalModelTemplateHint::SmolLm2,
                Some(TEMPLATE_FIXTURE_SYSTEM),
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nsystem fixture sentinel<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn qwen3_family_fixtures_match_canonical_template_behavior() {
        // The family renderer supplies the base chat framing. The runtime reasoning policy adds
        // Qwen's explicit non-thinking generation prefill after this base shape is constructed.
        assert_eq!(
            render_family_chat_prompt(
                LocalModelTemplateHint::Qwen3NonThinking,
                None,
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
        assert_eq!(
            render_family_chat_prompt(
                LocalModelTemplateHint::Qwen3NonThinking,
                Some(TEMPLATE_FIXTURE_SYSTEM),
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nsystem fixture sentinel<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn canonical_embedded_family_templates_are_accepted() {
        validate_embedded_template_source(
            LocalModelTemplateHint::SmolLm2,
            SMOLLM2_CANONICAL_TEMPLATE,
        )
        .unwrap();
        validate_embedded_template_source(
            LocalModelTemplateHint::Qwen3NonThinking,
            QWEN3_CANONICAL_TEMPLATE_FRAGMENT,
        )
        .unwrap();
    }

    #[test]
    fn generic_chatml_is_not_an_implicit_family_fallback() {
        let generic_chatml = "{% for message in messages %}{{ '<|im_start|>' + message.role + '\n' + message.content + '<|im_end|>\n' }}{% endfor %}{% if add_generation_prompt %}{{ '<|im_start|>assistant\n' }}{% endif %}";

        for template_hint in [
            LocalModelTemplateHint::SmolLm2,
            LocalModelTemplateHint::Qwen3NonThinking,
        ] {
            let error = validate_embedded_template_source(template_hint, generic_chatml).unwrap_err();
            assert_eq!(error.kind, LocalRuntimeErrorKind::ChatTemplate);
        }
    }

    #[test]
    fn missing_family_invariant_fails_closed() {
        let smol_without_default_system = SMOLLM2_CANONICAL_TEMPLATE.replace(
            SMOLLM2_DEFAULT_SYSTEM_INSTRUCTION,
            "different default system instruction",
        );
        let error = validate_embedded_template_source(
            LocalModelTemplateHint::SmolLm2,
            &smol_without_default_system,
        )
        .unwrap_err();
        assert_eq!(error.kind, LocalRuntimeErrorKind::ChatTemplate);

        let qwen_without_non_thinking_switch =
            QWEN3_CANONICAL_TEMPLATE_FRAGMENT.replace("enable_thinking", "thinking_mode");
        let error = validate_embedded_template_source(
            LocalModelTemplateHint::Qwen3NonThinking,
            &qwen_without_non_thinking_switch,
        )
        .unwrap_err();
        assert_eq!(error.kind, LocalRuntimeErrorKind::ChatTemplate);
    }
}
