use super::types::{LocalRuntimeError, LocalRuntimeGenerateRequest};
use crate::ai::local::catalog::LocalModelTemplateHint;
use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};

const SMOLLM2_DEFAULT_SYSTEM_INSTRUCTION: &str =
    "You are a helpful AI assistant named SmolLM, trained by Hugging Face";
const TEMPLATE_FIXTURE_SYSTEM: &str = "system fixture sentinel";
const TEMPLATE_FIXTURE_USER: &str = "user fixture sentinel";

fn chat_messages(
    system_instruction: Option<&str>,
    user_prompt: &str,
) -> Result<Vec<LlamaChatMessage>, LocalRuntimeError> {
    let mut messages = Vec::with_capacity(2);
    if let Some(system_instruction) = system_instruction {
        messages.push(
            LlamaChatMessage::new("system".to_string(), system_instruction.to_string())
                .map_err(|_| LocalRuntimeError::chat_template())?,
        );
    }
    messages.push(
        LlamaChatMessage::new("user".to_string(), user_prompt.to_string())
            .map_err(|_| LocalRuntimeError::chat_template())?,
    );
    Ok(messages)
}

fn append_chatml_message(rendered: &mut String, role: &str, content: &str) {
    rendered.push_str("<|im_start|>");
    rendered.push_str(role);
    rendered.push('\n');
    rendered.push_str(content);
    rendered.push_str("<|im_end|>\n");
}

fn expected_rendered_chat_prompt(
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

fn validate_rendered_chat_prompt(
    template_hint: LocalModelTemplateHint,
    system_instruction: Option<&str>,
    user_prompt: &str,
    rendered: &str,
) -> Result<(), LocalRuntimeError> {
    let expected = expected_rendered_chat_prompt(template_hint, system_instruction, user_prompt);
    if rendered == expected {
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

    for system_instruction in [None, Some(TEMPLATE_FIXTURE_SYSTEM)] {
        let messages = chat_messages(system_instruction, TEMPLATE_FIXTURE_USER)?;
        let rendered = model
            .apply_chat_template(&template, &messages, true)
            .map_err(|_| LocalRuntimeError::chat_template())?;
        validate_rendered_chat_prompt(
            template_hint,
            system_instruction,
            TEMPLATE_FIXTURE_USER,
            &rendered,
        )?;
    }
    Ok(())
}

pub(super) fn render_chat_prompt(
    model: &LlamaModel,
    template_hint: LocalModelTemplateHint,
    request: &LocalRuntimeGenerateRequest,
) -> Result<String, LocalRuntimeError> {
    let template = model
        .chat_template(None)
        .map_err(|_| LocalRuntimeError::chat_template())?;
    let messages = chat_messages(request.system_instruction.as_deref(), &request.prompt)?;
    let rendered = model
        .apply_chat_template(&template, &messages, true)
        .map_err(|_| LocalRuntimeError::chat_template())?;
    validate_rendered_chat_prompt(
        template_hint,
        request.system_instruction.as_deref(),
        &request.prompt,
        &rendered,
    )?;
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::LocalRuntimeErrorKind;

    #[test]
    fn smollm2_family_fixtures_match_pinned_template_behavior() {
        assert_eq!(
            expected_rendered_chat_prompt(
                LocalModelTemplateHint::SmolLm2,
                None,
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nYou are a helpful AI assistant named SmolLM, trained by Hugging Face<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
        assert_eq!(
            expected_rendered_chat_prompt(
                LocalModelTemplateHint::SmolLm2,
                Some(TEMPLATE_FIXTURE_SYSTEM),
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nsystem fixture sentinel<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn qwen3_family_fixtures_match_pinned_template_behavior() {
        // LLM-061 validates the base chat framing only. Qwen non-thinking generation behavior is
        // applied and tested separately by LLM-062.
        assert_eq!(
            expected_rendered_chat_prompt(
                LocalModelTemplateHint::Qwen3NonThinking,
                None,
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
        assert_eq!(
            expected_rendered_chat_prompt(
                LocalModelTemplateHint::Qwen3NonThinking,
                Some(TEMPLATE_FIXTURE_SYSTEM),
                TEMPLATE_FIXTURE_USER,
            ),
            "<|im_start|>system\nsystem fixture sentinel<|im_end|>\n<|im_start|>user\nuser fixture sentinel<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn incompatible_family_rendering_fails_closed() {
        let qwen_rendered = expected_rendered_chat_prompt(
            LocalModelTemplateHint::Qwen3NonThinking,
            None,
            TEMPLATE_FIXTURE_USER,
        );
        let error = validate_rendered_chat_prompt(
            LocalModelTemplateHint::SmolLm2,
            None,
            TEMPLATE_FIXTURE_USER,
            &qwen_rendered,
        )
        .unwrap_err();
        assert_eq!(error.kind, LocalRuntimeErrorKind::ChatTemplate);
    }
}
