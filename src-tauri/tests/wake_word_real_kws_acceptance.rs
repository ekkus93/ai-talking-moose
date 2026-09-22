use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use talking_moose_lib::app::wake_word_engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine,
};

const SAMPLE_RATE_HZ: u32 = 16_000;
const FEED_CHUNK_SAMPLES: usize = 1_600;

#[derive(Debug, Deserialize)]
struct Corpus {
    policy: CorpusPolicy,
    acceptance_criteria: AcceptanceCriteria,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct CorpusPolicy {
    sample_rate_hz: u32,
    channels: u16,
    sample_format: String,
}

#[derive(Debug, Deserialize)]
struct AcceptanceCriteria {
    criteria_status: String,
    positive_recall_minimum: Option<f64>,
    negative_false_accepts_maximum: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    path: String,
    expected_detection: bool,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must have a repository parent")
        .to_path_buf()
}

fn required_env_path(name: &str) -> PathBuf {
    let value = std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    let path = PathBuf::from(value);
    assert!(path.is_dir(), "{name} must name an existing directory");
    path
}

fn read_pcm16_mono_wav(path: &Path) -> Vec<i16> {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    assert!(bytes.len() >= 12, "{} is too short to be WAV", path.display());
    assert_eq!(&bytes[0..4], b"RIFF", "{} is not RIFF", path.display());
    assert_eq!(&bytes[8..12], b"WAVE", "{} is not WAVE", path.display());

    let mut offset = 12usize;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.checked_add(size).expect("WAV chunk overflow");
        assert!(end <= bytes.len(), "{} has truncated WAV chunk", path.display());
        if id == b"fmt " {
            assert!(size >= 16, "{} has short fmt chunk", path.display());
            format = Some((
                u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (size & 1);
    }

    let (audio_format, channels, sample_rate, bits_per_sample) = format.expect("WAV fmt chunk missing");
    assert_eq!(audio_format, 1, "fixture must be integer PCM");
    assert_eq!(channels, 1, "fixture must be mono");
    assert_eq!(sample_rate, SAMPLE_RATE_HZ, "fixture must be 16 kHz");
    assert_eq!(bits_per_sample, 16, "fixture must be PCM16");
    let data = data.expect("WAV data chunk missing");
    assert!(!data.is_empty(), "fixture audio must not be empty");
    assert_eq!(data.len() % 2, 0, "PCM16 byte count must be even");
    data.chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
        .collect()
}

fn fixture_detected(session: &mut NativeKwsSession, samples: &[i16]) -> bool {
    let mut detected = false;
    for chunk in samples.chunks(FEED_CHUNK_SAMPLES) {
        if session
            .accept_pcm16_mono(SAMPLE_RATE_HZ, chunk)
            .expect("real KWS inference must succeed")
            .is_some()
        {
            detected = true;
            break;
        }
    }
    session.reset_stream().expect("KWS stream reset must succeed");
    detected
}

#[test]
#[ignore = "requires prepared pinned native artifacts and real redistributable Wake Word corpus"]
fn real_pinned_kws_corpus_acceptance() {
    let root = repo_root();
    let corpus: Corpus = serde_json::from_slice(
        &fs::read(root.join("docs/wake-word-corpus.json")).expect("corpus manifest must be readable"),
    )
    .expect("corpus manifest must be valid JSON");

    assert_eq!(corpus.policy.sample_rate_hz, SAMPLE_RATE_HZ);
    assert_eq!(corpus.policy.channels, 1);
    assert_eq!(corpus.policy.sample_format, "pcm_s16le");
    assert_eq!(corpus.acceptance_criteria.criteria_status, "calibrated");
    let minimum_recall = corpus
        .acceptance_criteria
        .positive_recall_minimum
        .expect("positive recall threshold must be calibrated");
    let maximum_false_accepts = corpus
        .acceptance_criteria
        .negative_false_accepts_maximum
        .expect("negative false-accept threshold must be calibrated");
    assert!(!corpus.fixtures.is_empty(), "real corpus must not be empty");

    let mut session = NativeKwsSession::new(NativeKwsSessionPaths {
        model_dir: required_env_path("WAKE_WORD_MODEL_DIR"),
        runtime_dir: required_env_path("WAKE_WORD_RUNTIME_DIR"),
    })
    .expect("pinned native KWS session must initialize");

    let mut positives = 0u64;
    let mut positive_hits = 0u64;
    let mut negatives = 0u64;
    let mut false_accepts = 0u64;
    for fixture in &corpus.fixtures {
        let path = root.join(&fixture.path);
        let samples = read_pcm16_mono_wav(&path);
        let detected = fixture_detected(&mut session, &samples);
        if fixture.expected_detection {
            positives += 1;
            positive_hits += u64::from(detected);
        } else {
            negatives += 1;
            false_accepts += u64::from(detected);
        }
        println!("fixture={} expected={} detected={detected}", fixture.id, fixture.expected_detection);
    }

    assert!(positives > 0, "corpus must contain positive fixtures");
    assert!(negatives > 0, "corpus must contain negative fixtures");
    let recall = positive_hits as f64 / positives as f64;
    println!(
        "wake_kws_acceptance positives={positives} hits={positive_hits} recall={recall:.6} negatives={negatives} false_accepts={false_accepts}"
    );
    assert!(recall >= minimum_recall, "positive recall {recall:.6} is below {minimum_recall:.6}");
    assert!(
        false_accepts <= maximum_false_accepts,
        "negative false accepts {false_accepts} exceed {maximum_false_accepts}"
    );
    session.shutdown().expect("KWS shutdown must succeed");
}
