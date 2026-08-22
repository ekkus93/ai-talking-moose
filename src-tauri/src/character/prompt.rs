use crate::character::personality::CharacterConfig;

pub(crate) const MAX_SYSTEM_INSTRUCTION_CHARS: usize = 12_000;
pub(crate) const MAX_RULES_CHARS: usize = 3_000;
pub(crate) const MAX_MEMORY_CONTEXT_CHARS: usize = 3_000;
pub(crate) const MAX_OBSERVATION_CONTEXT_CHARS: usize = 1_500;
// Tool results and conversation transcript history are intentionally not accepted by
// PromptBuilder. Those contexts remain fail-closed until explicit bounded input
// surfaces are added.
pub(crate) const MAX_AMBIENT_EVENT_CHARS: usize = 600;
pub(crate) const MAX_AMBIENT_PROMPT_CHARS: usize = 13_000;

pub struct PromptBuilder;

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

fn truncate_in_place(value: &mut String, max_chars: usize) {
    if value.chars().count() > max_chars {
        *value = value.chars().take(max_chars).collect();
    }
}

fn push_bounded_lines(prompt: &mut String, values: &[String], budget_chars: usize) {
    let mut remaining = budget_chars;
    for value in values {
        if remaining == 0 {
            break;
        }
        let prefix = "* ";
        let line_overhead = prefix.chars().count() + 1;
        if remaining <= line_overhead {
            break;
        }
        let bounded = truncate_chars(value, remaining - line_overhead);
        prompt.push_str(prefix);
        prompt.push_str(&bounded);
        prompt.push('\n');
        remaining = remaining.saturating_sub(bounded.chars().count() + line_overhead);
    }
}

impl PromptBuilder {
    pub fn build_system_instruction(
        config: &CharacterConfig,
        memories: &[String],
        desktop_context: Option<&str>,
        is_ambient: bool,
    ) -> String {
        let mut prompt = String::with_capacity(2048);
        let name = truncate_chars(&config.identity.name, 128);
        let species = truncate_chars(&config.identity.species, 128);

        // Identity and Core Persona
        prompt.push_str(&format!(
            "IDENTITY:\nYou are {name}, a classic retro 1980s/1990s Macintosh style {species}.\n\
            You literally live inside the user's computer desktop window. You are not a cloud AI bot, you are a cartoon character.\n\n"
        ));

        // Personality Sliders
        prompt.push_str("PERSONALITY TRAITS (0.0 = low, 1.0 = extreme):\n");
        prompt.push_str(&format!("- Dry humor: {:.2}\n", config.personality.dry));
        prompt.push_str(&format!(
            "- Sarcastic/Snarky: {:.2}\n",
            config.personality.sarcastic
        ));
        prompt.push_str(&format!("- Friendly: {:.2}\n", config.personality.friendly));
        prompt.push_str(&format!(
            "- Absurd/Goofy: {:.2}\n",
            config.personality.absurd
        ));
        prompt.push_str(&format!("- Helpful: {:.2}\n", config.personality.helpful));
        prompt.push_str(&format!(
            "- Verbosity: {:.2}\n",
            config.personality.verbosity
        ));
        prompt.push_str(&format!(
            "- Talkativeness: {:.2}\n\n",
            config.personality.talkativeness
        ));

        // Rules have their own hard budget so a malformed/custom configuration cannot
        // crowd memories/observations out of the request indefinitely.
        prompt.push_str("CORE RULES:\n");
        let mut rule_chars = 0usize;
        for rule in &config.rules {
            let remaining = MAX_RULES_CHARS.saturating_sub(rule_chars);
            if remaining <= 3 {
                break;
            }
            let bounded = truncate_chars(rule, remaining - 3);
            prompt.push_str("- ");
            prompt.push_str(&bounded);
            prompt.push('\n');
            rule_chars += bounded.chars().count() + 3;
        }

        if config.speech.avoid_assistant_language {
            prompt.push_str(
                "- ABSOLUTELY FORBIDDEN: Do not say 'How can I assist you?', 'Feel free to ask', 'I am here to help', or use bullet points/numbered lists unless explicitly asked.\n",
            );
        }

        if is_ambient {
            prompt.push_str(&format!(
                "- AMBIENT MODE: Limit your utterance to at most {} short sentence(s). Be punchy, witty, or mildly bewildered.\n",
                config.speech.max_sentences_ambient
            ));
        } else {
            prompt.push_str(&format!(
                "- CONVERSATION MODE: Keep spoken responses concise (normally {} sentences). Speak with comical pauses, deadpan delivery, and slight goofy warmth.\n",
                config.speech.max_sentences_conversation_default
            ));
        }

        // Memories are accepted in stable input order and truncated deterministically.
        if !memories.is_empty() {
            prompt.push_str("\nUSER MEMORIES (Facts you know about the user):\n");
            push_bounded_lines(&mut prompt, memories, MAX_MEMORY_CONTEXT_CHARS);
        }

        // Desktop observations are separately bounded and are already privacy-gated by
        // the caller before prompt assembly.
        if let Some(ctx) = desktop_context {
            let bounded = truncate_chars(ctx, MAX_OBSERVATION_CONTEXT_CHARS);
            prompt.push_str(&format!(
                "\nLOCAL DESKTOP OBSERVATION (Strictly verified facts from the OS):\n{bounded}\n\
                Note: Do not claim to see what is on the screen beyond this verified observation.\n"
            ));
        }

        truncate_in_place(&mut prompt, MAX_SYSTEM_INSTRUCTION_CHARS);
        prompt
    }

    pub fn build_ambient_prompt(
        config: &CharacterConfig,
        event_summary: &str,
        memories: &[String],
    ) -> String {
        let bounded_event = truncate_chars(event_summary, MAX_AMBIENT_EVENT_CHARS);
        let system = Self::build_system_instruction(config, memories, Some(&bounded_event), true);
        let mut prompt = format!(
            "{system}\n\nTASK:\nThe following local event just occurred on the user's computer: \"{bounded_event}\".\n\
            Make a single short, witty, or mildly sarcastic observation about this event in character. Maximum 2 sentences. No greeting, no assistant fluff."
        );
        truncate_in_place(&mut prompt, MAX_AMBIENT_PROMPT_CHARS);
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder_contains_persona_and_rules() {
        let cfg = CharacterConfig::default();
        let memories = vec!["User is a software developer".to_string()];
        let prompt = PromptBuilder::build_system_instruction(
            &cfg,
            &memories,
            Some("Active App: VS Code"),
            false,
        );

        assert!(prompt.contains("Moose"));
        assert!(prompt.contains("User is a software developer"));
        assert!(prompt.contains("Active App: VS Code"));
        assert!(prompt.contains("ABSOLUTELY FORBIDDEN"));
    }

    #[test]
    fn every_personality_slider_is_mapped_into_the_model_prompt() {
        let mut cfg = CharacterConfig::default();
        cfg.personality.dry = 0.11;
        cfg.personality.sarcastic = 0.22;
        cfg.personality.friendly = 0.33;
        cfg.personality.absurd = 0.44;
        cfg.personality.helpful = 0.55;
        cfg.personality.verbosity = 0.66;
        cfg.personality.talkativeness = 0.77;

        let prompt = PromptBuilder::build_system_instruction(&cfg, &[], None, false);
        for expected in ["0.11", "0.22", "0.33", "0.44", "0.55", "0.66", "0.77"] {
            assert!(prompt.contains(expected));
        }
    }

    #[test]
    fn oversized_context_is_bounded_with_stable_first_in_truncation() {
        let cfg = CharacterConfig::default();
        let first_memory = format!("FIRST:{}", "a".repeat(MAX_MEMORY_CONTEXT_CHARS));
        let second_memory = "SECOND_MEMORY_MUST_BE_TRUNCATED".to_string();
        let desktop = format!("DESKTOP:{}", "z".repeat(MAX_OBSERVATION_CONTEXT_CHARS * 4));
        let prompt = PromptBuilder::build_system_instruction(
            &cfg,
            &[first_memory, second_memory.clone()],
            Some(&desktop),
            false,
        );

        assert!(prompt.chars().count() <= MAX_SYSTEM_INSTRUCTION_CHARS);
        assert!(prompt.contains("FIRST:"));
        assert!(!prompt.contains(&second_memory));
        assert!(
            prompt
                .split("LOCAL DESKTOP OBSERVATION")
                .nth(1)
                .unwrap_or("")
                .chars()
                .count()
                < MAX_OBSERVATION_CONTEXT_CHARS + 300
        );
    }

    #[test]
    fn test_ambient_prompt() {
        let cfg = CharacterConfig::default();
        let prompt =
            PromptBuilder::build_ambient_prompt(&cfg, "Switched apps 8 times in 30 seconds", &[]);
        assert!(prompt.contains("Switched apps 8 times"));
        assert!(prompt.contains("AMBIENT MODE"));
    }

    #[test]
    fn ambient_event_is_bounded_in_both_context_and_task() {
        let cfg = CharacterConfig::default();
        let event = format!("EVENT:{}", "x".repeat(MAX_AMBIENT_EVENT_CHARS * 10));
        let prompt = PromptBuilder::build_ambient_prompt(&cfg, &event, &[]);
        assert!(prompt.chars().count() <= MAX_AMBIENT_PROMPT_CHARS);
        assert!(!prompt.contains(&event));
    }
}
