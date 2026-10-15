//! macOS AVSpeechSynthesizer buffer synthesis. Audio is captured through
//! `writeUtterance:toBufferCallback:` and converted to bounded mono PCM for the
//! existing Rust playback path; the OS never plays it directly.
use crate::ai::system_tts_pcm::SystemPcmAccumulator;
use crate::ai::system_tts_request::validate_system_tts_request;
use crate::ai::system_tts_selection::{resolve_installed_system_voice, SYSTEM_DEFAULT_VOICE_ID};
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::tts_catalog::TtsVoiceDescriptor;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind, TtsRequest};
use async_trait::async_trait;
use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_avf_audio::{
    AVAudioBuffer, AVAudioCommonFormat, AVAudioPCMBuffer, AVSpeechBoundary, AVSpeechSynthesisVoice,
    AVSpeechSynthesisVoiceQuality, AVSpeechSynthesizer, AVSpeechUtterance,
};
use objc2_foundation::NSString;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// No first buffer within this window means the OS voice never started.
const FIRST_BUFFER_TIMEOUT: Duration = Duration::from_secs(15);
/// Silence between buffers longer than this means the stream stalled.
const BUFFER_GAP_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_FRAMES_PER_BUFFER: usize = 1 << 20;

pub(super) fn installed_voices() -> Vec<TtsVoiceDescriptor> {
    // SAFETY: class method with no preconditions; safe from any thread.
    let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
    voices
        .iter()
        .map(|voice| {
            // SAFETY: property getters on a live AVSpeechSynthesisVoice.
            let (identifier, name, language, quality) = unsafe {
                (
                    voice.identifier().to_string(),
                    voice.name().to_string(),
                    voice.language().to_string(),
                    voice.quality(),
                )
            };
            let quality = if quality == AVSpeechSynthesisVoiceQuality::Premium {
                "Premium"
            } else if quality == AVSpeechSynthesisVoiceQuality::Enhanced {
                "Enhanced"
            } else {
                "Standard"
            };
            TtsVoiceDescriptor {
                id: identifier,
                display_name: name,
                style: Some(format!("{language} · {quality}")),
            }
        })
        .collect()
}

/// Owned, thread-transferable copy of one native buffer.
enum Planar {
    F32(Vec<Vec<f32>>),
    I16(Vec<Vec<i16>>),
    I32(Vec<Vec<i32>>),
}

enum Event {
    Chunk {
        sample_rate: f64,
        data: Planar,
    },
    /// A zero-length buffer, which AVSpeechSynthesizer sends after the last audio.
    Done,
    Failed(ProviderErrorKind),
}

/// The main-queue-confined synthesizer, kept so a dropped request can stop it.
struct MainQueueSynthesizer(Mutex<Option<Retained<AVSpeechSynthesizer>>>);

// SAFETY: the Retained value is only created, used and released on the main
// dispatch queue (see `StopOnDrop` and `start_on_main`); the mutex only moves it.
unsafe impl Send for MainQueueSynthesizer {}
unsafe impl Sync for MainQueueSynthesizer {}

struct StopOnDrop(Arc<MainQueueSynthesizer>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let synthesizer = self.0.clone();
        DispatchQueue::main().exec_async(move || {
            if let Some(active) = synthesizer.0.lock().unwrap().take() {
                // SAFETY: running on the main queue, where the synthesizer lives.
                unsafe { active.stopSpeakingAtBoundary(AVSpeechBoundary::Immediate) };
            }
        });
    }
}

fn copy_channels<T: Copy>(
    data: *mut NonNull<T>,
    channels: usize,
    frames: usize,
    interleaved: bool,
) -> Vec<Vec<T>> {
    // SAFETY (callers): `data` is the buffer's channel-data array, valid for
    // `channels` pointers when planar or one pointer when interleaved, each
    // addressing at least `frames` (x `channels` if interleaved) samples.
    unsafe {
        if interleaved && channels > 1 {
            let all = std::slice::from_raw_parts((*data).as_ptr(), frames * channels);
            (0..channels)
                .map(|c| all.iter().skip(c).step_by(channels).copied().collect())
                .collect()
        } else {
            (0..channels)
                .map(|c| std::slice::from_raw_parts((*data.add(c)).as_ptr(), frames).to_vec())
                .collect()
        }
    }
}

fn classify(buffer: NonNull<AVAudioBuffer>) -> Event {
    // SAFETY: AVFoundation passes a live buffer for the duration of the callback.
    let buffer: &AVAudioBuffer = unsafe { buffer.as_ref() };
    let any: &AnyObject = buffer.as_ref();
    let Some(pcm) = any.downcast_ref::<AVAudioPCMBuffer>() else {
        return Event::Failed(ProviderErrorKind::Protocol);
    };
    // SAFETY: property getters on a live AVAudioPCMBuffer and its format.
    unsafe {
        let frames = pcm.frameLength() as usize;
        if frames == 0 {
            return Event::Done;
        }
        if frames > MAX_FRAMES_PER_BUFFER {
            return Event::Failed(ProviderErrorKind::Protocol);
        }
        let format = pcm.format();
        let channels = format.channelCount() as usize;
        if channels == 0 || channels > 8 {
            return Event::Failed(ProviderErrorKind::Protocol);
        }
        let interleaved = format.isInterleaved();
        let sample_rate = format.sampleRate();
        let data = match format.commonFormat() {
            AVAudioCommonFormat::PCMFormatFloat32 => {
                let ptr = pcm.floatChannelData();
                if ptr.is_null() {
                    return Event::Failed(ProviderErrorKind::Protocol);
                }
                Planar::F32(copy_channels(ptr, channels, frames, interleaved))
            }
            AVAudioCommonFormat::PCMFormatInt16 => {
                let ptr = pcm.int16ChannelData();
                if ptr.is_null() {
                    return Event::Failed(ProviderErrorKind::Protocol);
                }
                Planar::I16(copy_channels(ptr, channels, frames, interleaved))
            }
            AVAudioCommonFormat::PCMFormatInt32 => {
                let ptr = pcm.int32ChannelData();
                if ptr.is_null() {
                    return Event::Failed(ProviderErrorKind::Protocol);
                }
                Planar::I32(copy_channels(ptr, channels, frames, interleaved))
            }
            _ => return Event::Failed(ProviderErrorKind::Protocol),
        };
        Event::Chunk { sample_rate, data }
    }
}

fn append(accumulator: &mut SystemPcmAccumulator, data: &Planar) -> Result<(), ProviderError> {
    match data {
        Planar::F32(channels) => {
            let refs: Vec<&[f32]> = channels.iter().map(Vec::as_slice).collect();
            accumulator.append_planar_f32(&refs)
        }
        Planar::I16(channels) => {
            let refs: Vec<&[i16]> = channels.iter().map(Vec::as_slice).collect();
            accumulator.append_planar_i16(&refs)
        }
        Planar::I32(channels) => {
            let refs: Vec<&[i32]> = channels.iter().map(Vec::as_slice).collect();
            accumulator.append_planar_i32(&refs)
        }
    }
}

/// Create the synthesizer on the main queue, start `writeUtterance`, and route
/// every buffer to `events`. The block keeps the synthesizer alive via `slot`.
fn start_on_main(
    text: String,
    voice_id: Option<String>,
    av_rate: f32,
    slot: Arc<MainQueueSynthesizer>,
    events: mpsc::UnboundedSender<Event>,
) {
    DispatchQueue::main().exec_async(move || {
        // SAFETY: all AVFoundation objects below are created and used on the main queue.
        unsafe {
            let utterance =
                AVSpeechUtterance::speechUtteranceWithString(&NSString::from_str(&text));
            if let Some(id) = voice_id {
                match AVSpeechSynthesisVoice::voiceWithIdentifier(&NSString::from_str(&id)) {
                    Some(voice) => utterance.setVoice(Some(&voice)),
                    None => {
                        let _ = events.send(Event::Failed(ProviderErrorKind::Model));
                        return;
                    }
                }
            }
            utterance.setRate(av_rate);
            let synthesizer = AVSpeechSynthesizer::new();
            let sender = events.clone();
            let callback = RcBlock::new(move |buffer: NonNull<AVAudioBuffer>| {
                let _ = sender.send(classify(buffer));
            });
            *slot.0.lock().unwrap() = Some(synthesizer.clone());
            synthesizer.writeUtterance_toBufferCallback(&utterance, RcBlock::as_ptr(&callback));
        }
    });
}

pub(super) struct MacSystemSpeechSynthesizer;

#[async_trait]
impl SpeechSynthesizer for MacSystemSpeechSynthesizer {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        let installed = super::installed_voices();
        validate_system_tts_request(&request, &installed)?;
        let saved = request
            .voice_name
            .as_deref()
            .unwrap_or(SYSTEM_DEFAULT_VOICE_ID);
        let voice_id = resolve_installed_system_voice(saved, &installed)?.map(|v| v.id.clone());
        let av_rate = super::av_speech_rate(request.speaking_rate.unwrap_or(1.0));

        let (sender, mut events) = mpsc::unbounded_channel();
        let slot = Arc::new(MainQueueSynthesizer(Mutex::new(None)));
        // Dropping this future (cancellation, replacement) stops native synthesis.
        let _stop = StopOnDrop(slot.clone());
        start_on_main(request.text, voice_id, av_rate, slot, sender);

        let mut accumulator: Option<SystemPcmAccumulator> = None;
        let mut wait = FIRST_BUFFER_TIMEOUT;
        loop {
            let event = tokio::time::timeout(wait, events.recv())
                .await
                .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?
                .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Protocol))?;
            wait = BUFFER_GAP_TIMEOUT;
            match event {
                Event::Failed(kind) => return Err(ProviderError::from_kind(kind)),
                Event::Done => break,
                Event::Chunk { sample_rate, data } => {
                    if accumulator.is_none() {
                        if !(sample_rate.is_finite() && sample_rate >= 1.0) {
                            return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
                        }
                        accumulator = Some(SystemPcmAccumulator::new(sample_rate.round() as u32)?);
                    }
                    append(accumulator.as_mut().expect("initialized above"), &data)?;
                }
            }
        }
        // No audio at all is reported by `finish` as a protocol error, never as silence.
        accumulator
            .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Protocol))?
            .finish()
    }
}
