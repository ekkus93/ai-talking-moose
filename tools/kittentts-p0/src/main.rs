mod model;
mod npz;
mod tokenize;

use anyhow::{bail, Context, Result};
use model::KittenModel;
use piper_plus_g2p::english::EnglishPhonemizer;
use piper_plus_g2p::Phonemizer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokenize::ipa_to_ids;

const SAMPLE_RATE_HZ: u32 = 24_000;
const BENCHMARK_THREADS: [usize; 3] = [1, 2, 4];
const CANONICAL_THREADS: usize = 2;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: u32,
    model_family: String,
    cases: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
struct CorpusCase {
    id: String,
    category: String,
    text: String,
    normalized_text: String,
    synthesize: bool,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    id: String,
    category: String,
    text: String,
    normalized_text: String,
    candidate_ipa: String,
    candidate_g2p_token_count: usize,
    candidate_model_token_ids: Vec<i64>,
    synthesized: bool,
    synthesis_ms: Option<f64>,
    audio_samples: Option<usize>,
    audio_seconds: Option<f64>,
    rtf: Option<f64>,
    finite_audio: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ThreadBenchmark {
    threads: usize,
    cold_load_ms: f64,
    synthesized_cases: usize,
    max_rtf: Option<f64>,
    mean_rtf: Option<f64>,
    max_rss_mib_after_load: Option<f64>,
    max_rss_mib_after_synthesis: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ProbeReport {
    schema_version: u32,
    model_family: String,
    voice: String,
    voice_key: String,
    sample_rate_hz: u32,
    cmudict_path: String,
    model_path: String,
    voices_path: String,
    canonical_threads: usize,
    cold_load_ms: f64,
    max_rss_mib_after_load: Option<f64>,
    max_rss_mib_after_synthesis: Option<f64>,
    thread_benchmarks: Vec<ThreadBenchmark>,
    cases: Vec<CaseResult>,
}

fn max_rss_mib() -> Option<f64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage initializes the supplied rusage structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: getrusage returned success above.
    let usage = unsafe { usage.assume_init() };
    let raw = usage.ru_maxrss as f64;
    #[cfg(target_os = "macos")]
    {
        Some(raw / (1024.0 * 1024.0))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(raw / 1024.0)
    }
}

fn voice_aliases() -> HashMap<&'static str, &'static str> {
    [
        ("Bella", "expr-voice-2-f"),
        ("Jasper", "expr-voice-2-m"),
        ("Luna", "expr-voice-3-f"),
        ("Bruno", "expr-voice-3-m"),
        ("Rosie", "expr-voice-4-f"),
        ("Hugo", "expr-voice-4-m"),
        ("Kiki", "expr-voice-5-f"),
        ("Leo", "expr-voice-5-m"),
    ]
    .into_iter()
    .collect()
}

fn require_file(path: &Path, label: &str) -> Result<()> {
    if !path.is_file() {
        bail!("{label} is not a regular file: {}", path.display());
    }
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf, PathBuf, PathBuf, String)> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 || args.len() > 6 {
        bail!(
            "usage: {} <corpus.json> <model.onnx> <voices.npz> <report.json> [voice]",
            args.first().map_or("talking-moose-kittentts-p0", String::as_str)
        );
    }
    Ok((
        PathBuf::from(&args[1]),
        PathBuf::from(&args[2]),
        PathBuf::from(&args[3]),
        PathBuf::from(&args[4]),
        args.get(5).cloned().unwrap_or_else(|| "Jasper".to_string()),
    ))
}

fn main() -> Result<()> {
    let (corpus_path, model_path, voices_path, report_path, voice) = parse_args()?;
    require_file(&corpus_path, "corpus")?;
    require_file(&model_path, "model")?;
    require_file(&voices_path, "voices")?;

    let cmudict_path = env::var("CMUDICT_PATH").context("CMUDICT_PATH must be set")?;
    require_file(Path::new(&cmudict_path), "CMU dictionary")?;

    let corpus: Corpus = serde_json::from_str(
        &fs::read_to_string(&corpus_path)
            .with_context(|| format!("failed to read {}", corpus_path.display()))?,
    )
    .context("failed to decode reference corpus")?;
    if corpus.schema_version != 1 {
        bail!("unsupported corpus schema version {}", corpus.schema_version);
    }

    let voice_key = voice_aliases()
        .get(voice.as_str())
        .copied()
        .with_context(|| format!("unknown Kitten voice alias {voice:?}"))?
        .to_string();

    let phonemizer = EnglishPhonemizer::new().context("failed to initialize English G2P")?;
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in corpus.cases {
        let (candidate_tokens, _prosody) = phonemizer
            .phonemize_with_prosody(&case.normalized_text)
            .with_context(|| format!("G2P failed for corpus case {}", case.id))?;
        let candidate_ipa = candidate_tokens.concat();
        let candidate_model_token_ids = ipa_to_ids(&candidate_ipa);
        if case.synthesize && (candidate_ipa.trim().is_empty() || candidate_model_token_ids.len() <= 3)
        {
            bail!("candidate G2P produced no model input for synthesis case {}", case.id);
        }

        results.push(CaseResult {
            id: case.id,
            category: case.category,
            text: case.text,
            normalized_text: case.normalized_text,
            candidate_ipa,
            candidate_g2p_token_count: candidate_tokens.len(),
            candidate_model_token_ids,
            synthesized: case.synthesize,
            synthesis_ms: None,
            audio_samples: None,
            audio_seconds: None,
            rtf: None,
            finite_audio: None,
        });
    }

    let mut thread_benchmarks = Vec::new();
    let mut canonical_cold_load_ms = None;
    let mut canonical_rss_after_load = None;
    let mut canonical_rss_after_synthesis = None;

    for threads in BENCHMARK_THREADS {
        let load_started = Instant::now();
        let model = KittenModel::load(&model_path, &voices_path, threads)
            .with_context(|| format!("failed to load Kitten ONNX runtime with {threads} threads"))?;
        let cold_load_ms = load_started.elapsed().as_secs_f64() * 1000.0;
        let rss_after_load = max_rss_mib();

        let mut rtfs = Vec::new();
        let mut synthesized_cases = 0usize;
        for case in &mut results {
            if !case.synthesized {
                continue;
            }
            let started = Instant::now();
            let audio = model
                .generate_from_ipa(
                    &case.candidate_ipa,
                    &voice_key,
                    1.0,
                    case.normalized_text.len(),
                )
                .with_context(|| {
                    format!(
                        "synthesis failed for corpus case {} with {threads} threads",
                        case.id
                    )
                })?;
            let elapsed = started.elapsed().as_secs_f64();
            let duration = audio.len() as f64 / SAMPLE_RATE_HZ as f64;
            let finite = audio.iter().all(|sample| sample.is_finite());
            if audio.is_empty() || !finite || duration <= 0.0 {
                bail!(
                    "invalid audio for corpus case {} with {threads} threads",
                    case.id
                );
            }
            let rtf = elapsed / duration;
            rtfs.push(rtf);
            synthesized_cases += 1;

            if threads == CANONICAL_THREADS {
                case.synthesis_ms = Some(elapsed * 1000.0);
                case.audio_samples = Some(audio.len());
                case.audio_seconds = Some(duration);
                case.rtf = Some(rtf);
                case.finite_audio = Some(finite);
            }
        }
        let rss_after_synthesis = max_rss_mib();
        let max_rtf = rtfs.iter().copied().reduce(f64::max);
        let mean_rtf = (!rtfs.is_empty()).then(|| rtfs.iter().sum::<f64>() / rtfs.len() as f64);

        if threads == CANONICAL_THREADS {
            canonical_cold_load_ms = Some(cold_load_ms);
            canonical_rss_after_load = rss_after_load;
            canonical_rss_after_synthesis = rss_after_synthesis;
        }

        thread_benchmarks.push(ThreadBenchmark {
            threads,
            cold_load_ms,
            synthesized_cases,
            max_rtf,
            mean_rtf,
            max_rss_mib_after_load: rss_after_load,
            max_rss_mib_after_synthesis: rss_after_synthesis,
        });
    }

    let report = ProbeReport {
        schema_version: 2,
        model_family: corpus.model_family,
        voice,
        voice_key,
        sample_rate_hz: SAMPLE_RATE_HZ,
        cmudict_path,
        model_path: model_path.display().to_string(),
        voices_path: voices_path.display().to_string(),
        canonical_threads: CANONICAL_THREADS,
        cold_load_ms: canonical_cold_load_ms.context("canonical thread benchmark missing")?,
        max_rss_mib_after_load: canonical_rss_after_load,
        max_rss_mib_after_synthesis: canonical_rss_after_synthesis,
        thread_benchmarks,
        cases: results,
    };

    fs::write(&report_path, serde_json::to_string_pretty(&report)? + "\n")
        .with_context(|| format!("failed to write {}", report_path.display()))?;
    Ok(())
}
