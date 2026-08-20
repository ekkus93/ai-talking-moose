use crate::asr::AsrEvent;
use std::collections::HashSet;

/// Provider-neutral streaming transcript update consumed by the utterance state machine.
///
/// Adapters map their native stable segment/line identifier into `segment_id`. The state
/// machine never infers utterance identity from transcript text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamingTranscriptUpdate {
    Partial { segment_id: u64, text: String },
    Final { segment_id: u64, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveUtterance {
    segment_id: u64,
    partial_text: Option<String>,
}

/// Converts stable streaming line updates into provider-neutral ASR events.
///
/// At most one utterance is active. A new segment ID closes any older partial-only
/// utterance without fabricating a final transcript. Once a segment is closed or
/// finalized, later stale updates for that ID are ignored for the remainder of this
/// state-machine instance.
#[derive(Debug, Default)]
pub(crate) struct TranscriptStateMachine {
    active: Option<ActiveUtterance>,
    closed_segments: HashSet<u64>,
}

impl TranscriptStateMachine {
    pub(crate) fn apply(&mut self, update: StreamingTranscriptUpdate) -> Vec<AsrEvent> {
        let (segment_id, text, is_final) = match update {
            StreamingTranscriptUpdate::Partial { segment_id, text } => (segment_id, text, false),
            StreamingTranscriptUpdate::Final { segment_id, text } => (segment_id, text, true),
        };

        if text.trim().is_empty() || self.closed_segments.contains(&segment_id) {
            return Vec::new();
        }

        let mut events = Vec::new();
        self.activate_segment(segment_id, &mut events);

        if is_final {
            events.push(AsrEvent::FinalTranscript { text });
            events.push(AsrEvent::SpeechEnded { monotonic_ms: None });
            self.active = None;
            self.closed_segments.insert(segment_id);
            return events;
        }

        let Some(active) = self.active.as_mut() else {
            return events;
        };
        if active.partial_text.as_deref() == Some(text.as_str()) {
            return events;
        }
        active.partial_text = Some(text.clone());
        events.push(AsrEvent::PartialTranscript { text });
        events
    }

    fn activate_segment(&mut self, segment_id: u64, events: &mut Vec<AsrEvent>) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.segment_id == segment_id)
        {
            return;
        }

        if let Some(previous) = self.active.take() {
            self.closed_segments.insert(previous.segment_id);
            events.push(AsrEvent::SpeechEnded { monotonic_ms: None });
        }

        self.active = Some(ActiveUtterance {
            segment_id,
            partial_text: None,
        });
        events.push(AsrEvent::SpeechStarted { monotonic_ms: None });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partial(segment_id: u64, text: &str) -> StreamingTranscriptUpdate {
        StreamingTranscriptUpdate::Partial {
            segment_id,
            text: text.to_string(),
        }
    }

    fn final_update(segment_id: u64, text: &str) -> StreamingTranscriptUpdate {
        StreamingTranscriptUpdate::Final {
            segment_id,
            text: text.to_string(),
        }
    }

    #[test]
    fn first_partial_starts_speech_and_later_partial_replaces_it() {
        let mut state = TranscriptStateMachine::default();

        assert_eq!(
            state.apply(partial(7, "hel")),
            vec![
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::PartialTranscript {
                    text: "hel".to_string(),
                },
            ]
        );
        assert_eq!(
            state.apply(partial(7, "hello")),
            vec![AsrEvent::PartialTranscript {
                text: "hello".to_string(),
            }]
        );
    }

    #[test]
    fn unchanged_partial_is_suppressed() {
        let mut state = TranscriptStateMachine::default();
        let _ = state.apply(partial(9, "same"));

        assert!(state.apply(partial(9, "same")).is_empty());
    }

    #[test]
    fn partial_then_final_emits_final_once_and_links_speech_end() {
        let mut state = TranscriptStateMachine::default();
        let _ = state.apply(partial(42, "hello"));

        assert_eq!(
            state.apply(final_update(42, "hello there")),
            vec![
                AsrEvent::FinalTranscript {
                    text: "hello there".to_string(),
                },
                AsrEvent::SpeechEnded { monotonic_ms: None },
            ]
        );
        assert!(state.apply(final_update(42, "hello there")).is_empty());
        assert!(state
            .apply(final_update(42, "changed after final"))
            .is_empty());
    }

    #[test]
    fn final_without_partial_has_complete_speech_lifecycle() {
        let mut state = TranscriptStateMachine::default();

        assert_eq!(
            state.apply(final_update(5, "done")),
            vec![
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::FinalTranscript {
                    text: "done".to_string(),
                },
                AsrEvent::SpeechEnded { monotonic_ms: None },
            ]
        );
    }

    #[test]
    fn new_segment_abandons_old_partial_without_fabricating_final() {
        let mut state = TranscriptStateMachine::default();
        let _ = state.apply(partial(1, "unfinished"));

        assert_eq!(
            state.apply(partial(2, "new")),
            vec![
                AsrEvent::SpeechEnded { monotonic_ms: None },
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::PartialTranscript {
                    text: "new".to_string(),
                },
            ]
        );
        assert!(state.apply(final_update(1, "stale final")).is_empty());
    }

    #[test]
    fn late_partial_after_final_is_suppressed() {
        let mut state = TranscriptStateMachine::default();
        let _ = state.apply(final_update(11, "complete"));

        assert!(state.apply(partial(11, "late partial")).is_empty());
    }

    #[test]
    fn blank_updates_do_not_create_speech_or_transcript_events() {
        let mut state = TranscriptStateMachine::default();

        assert!(state.apply(partial(1, "   ")).is_empty());
        assert!(state.apply(final_update(1, "\t")).is_empty());
    }

    #[test]
    fn only_final_events_are_eligible_for_retention() {
        let mut state = TranscriptStateMachine::default();
        let mut events = state.apply(partial(4, "private partial"));
        events.extend(state.apply(final_update(4, "retained final")));

        let retained: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                AsrEvent::FinalTranscript { text } => Some(text.as_str()),
                AsrEvent::SpeechStarted { .. }
                | AsrEvent::PartialTranscript { .. }
                | AsrEvent::SpeechEnded { .. }
                | AsrEvent::Error { .. } => None,
            })
            .collect();
        assert_eq!(retained, vec!["retained final"]);
    }
}
