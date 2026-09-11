use super::runtime::LocalTtsInferenceOutput;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind};
use crate::audio::resample::AudioResampler;

fn invalid_audio_output() -> ProviderError {
    ProviderError {
        kind: ProviderErrorKind::Internal,
        message: "Local TTS produced invalid audio output.".to_string(),
        retryable: false,
    }
}

/// Adapt Kitten's mono floating-point PCM into the existing standalone speech boundary.
///
/// The runtime sample rate is preserved. Physical-device resampling, volume, queue limits,
/// diagnostics, and mouth animation remain owned by `AudioPlayback` after this boundary.
pub(crate) fn inference_output_to_audio_stream_data(
    output: LocalTtsInferenceOutput,
) -> Result<AudioStreamData, ProviderError> {
    if output.sample_rate_hz == 0
        || output.samples.is_empty()
        || output.samples.iter().any(|sample| !sample.is_finite())
    {
        return Err(invalid_audio_output());
    }

    let pcm_samples = AudioResampler::f32_to_i16(&output.samples);
    if pcm_samples.iter().all(|sample| *sample == 0) {
        return Err(invalid_audio_output());
    }

    Ok(AudioStreamData {
        pcm_bytes: AudioResampler::i16_to_bytes(&pcm_samples),
        sample_rate: output.sample_rate_hz,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::playback::{AudioPlayback, MAX_QUEUED_PLAYBACK_SECONDS};

    #[test]
    fn kitten_output_preserves_source_rate_and_encodes_saturated_i16_le_mono() {
        let audio = inference_output_to_audio_stream_data(LocalTtsInferenceOutput {
            samples: vec![-2.0, -1.0, -0.5, 0.5, 1.0, 2.0],
            sample_rate_hz: 24_000,
        })
        .unwrap();

        assert_eq!(audio.sample_rate, 24_000);
        assert_eq!(audio.pcm_bytes.len(), 12);
        let samples = audio
            .pcm_bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| i16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();
        assert_eq!(
            samples,
            vec![
                i16::MIN + 1,
                i16::MIN + 1,
                -16_384,
                16_384,
                i16::MAX,
                i16::MAX,
            ]
        );
    }

    #[test]
    fn malformed_or_zero_playable_kitten_output_fails_closed() {
        for output in [
            LocalTtsInferenceOutput {
                samples: vec![],
                sample_rate_hz: 24_000,
            },
            LocalTtsInferenceOutput {
                samples: vec![0.25],
                sample_rate_hz: 0,
            },
            LocalTtsInferenceOutput {
                samples: vec![f32::NAN],
                sample_rate_hz: 24_000,
            },
            LocalTtsInferenceOutput {
                samples: vec![f32::INFINITY],
                sample_rate_hz: 24_000,
            },
            LocalTtsInferenceOutput {
                samples: vec![0.0, 0.0],
                sample_rate_hz: 24_000,
            },
        ] {
            let error = inference_output_to_audio_stream_data(output).unwrap_err();
            assert_eq!(error.kind, ProviderErrorKind::Internal);
            assert!(!error.retryable);
            assert!(!error.message.contains("pcm"));
        }
    }

    #[test]
    fn adapted_kitten_audio_uses_existing_bounded_playback_queue_and_diagnostics() {
        let source_samples = 24_000 * (MAX_QUEUED_PLAYBACK_SECONDS + 2);
        let audio = inference_output_to_audio_stream_data(LocalTtsInferenceOutput {
            samples: vec![0.25; source_samples],
            sample_rate_hz: 24_000,
        })
        .unwrap();

        let playback = AudioPlayback::new_mock();
        playback.start(None).unwrap();
        let report = playback
            .enqueue_pcm_bytes(&audio.pcm_bytes, audio.sample_rate)
            .unwrap();

        assert_eq!(report.queued_samples, playback.max_queued_samples());
        assert!(report.dropped_samples > 0);
        let diagnostics = playback.diagnostics();
        assert_eq!(diagnostics.sample_rate_hz, Some(24_000));
        assert_eq!(diagnostics.channels, Some(1));
        assert_eq!(
            diagnostics.queue_depth_samples,
            playback.max_queued_samples()
        );
        assert_eq!(
            diagnostics.dropped_samples,
            u64::try_from(report.dropped_samples).unwrap()
        );
    }
}
