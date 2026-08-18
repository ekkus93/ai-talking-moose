use crate::character::personality::CharacterConfig;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build_system_instruction(
        config: &CharacterConfig,
        memories: &[String],
        desktop_context: Option<&str>,
        is_ambient: bool,
    ) -> String {
        let mut prompt = String::with_capacity(1024);

        // Identity and Core Persona
        prompt.push_str(&format!(
            "IDENTITY:\nYou are {}, a classic retro 1980s/1990s Macintosh style {}.\n\
            You literally live inside the user's computer desktop window. You are not a cloud AI bot, you are a cartoon character.\n\n",
            config.identity.name, config.identity.species
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
            "- Verbosity: {:.2}\n\n",
            config.personality.verbosity
        ));

        // Rules
        prompt.push_str("CORE RULES:\n");
        for rule in &config.rules {
            prompt.push_str(&format!("- {}\n", rule));
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

        // Memories
        if !memories.is_empty() {
            prompt.push_str("\nUSER MEMORIES (Facts you know about the user):\n");
            for mem in memories {
                prompt.push_str(&format!("* {}\n", mem));
            }
        }

        // Desktop Context
        if let Some(ctx) = desktop_context {
            prompt.push_str(&format!(
                "\nLOCAL DESKTOP OBSERVATION (Strictly verified facts from the OS):\n{}\n\
                Note: Do not claim to see what is on the screen beyond this verified observation.\n",
                ctx
            ));
        }

        prompt
    }

    pub fn build_ambient_prompt(
        config: &CharacterConfig,
        event_summary: &str,
        memories: &[String],
    ) -> String {
        let system = Self::build_system_instruction(config, memories, Some(event_summary), true);
        format!(
            "{}\n\nTASK:\nThe following local event just occurred on the user's computer: \"{}\".\n\
            Make a single short, witty, or mildly sarcastic observation about this event in character. Maximum 2 sentences. No greeting, no assistant fluff.",
            system, event_summary
        )
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
    fn test_ambient_prompt() {
        let cfg = CharacterConfig::default();
        let prompt =
            PromptBuilder::build_ambient_prompt(&cfg, "Switched apps 8 times in 30 seconds", &[]);
        assert!(prompt.contains("Switched apps 8 times"));
        assert!(prompt.contains("AMBIENT MODE"));
    }
}
