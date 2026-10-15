//! macOS System Voices acceptance. Synthesizes through AVSpeechSynthesizer, checks the
//! PCM is real speech-shaped audio, exercises cancellation and the missing-voice path,
//! and writes WAV files plus a JSON report for human audition. macOS only.
//!
//! usage: system_tts_acceptance <output-dir>

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("system_tts_acceptance runs only on macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
mod mac {
    use serde_json::{json, Value};
    use std::path::Path;
    use std::time::{Duration, Instant};
    use talking_moose_lib::ai::system_tts;
    use talking_moose_lib::ai::types::{AudioStreamData, ProviderErrorKind, TtsRequest};

    const PHRASE: &str = "Hello! I am the Talking Moose, speaking with a macOS system voice.";

    fn request(voice: Option<&str>, rate: f32) -> TtsRequest {
        TtsRequest {
            text: PHRASE.into(),
            voice_name: voice.map(str::to_string),
            speaking_rate: Some(rate),
            pitch: None,
        }
    }

    fn write_wav(path: &Path, audio: &AudioStreamData) -> std::io::Result<()> {
        let data_len = audio.pcm_bytes.len() as u32;
        let mut out = Vec::with_capacity(44 + audio.pcm_bytes.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&audio.sample_rate.to_le_bytes());
        out.extend_from_slice(&(audio.sample_rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(&audio.pcm_bytes);
        std::fs::write(path, out)
    }

    fn stats(audio: &AudioStreamData) -> Value {
        let samples: Vec<i16> = audio
            .pcm_bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        let rms = (samples.iter().map(|&s| (s as f64).powi(2)).sum::<f64>()
            / samples.len().max(1) as f64)
            .sqrt();
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
        json!({
            "sample_rate_hz": audio.sample_rate,
            "samples": samples.len(),
            "duration_ms": samples.len() as u64 * 1000 / audio.sample_rate.max(1) as u64,
            "rms": rms,
            "peak": peak,
        })
    }

    fn check(name: &str, ok: bool, failures: &mut Vec<String>) {
        println!("{} {name}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            failures.push(name.to_string());
        }
    }

    async fn run(out_dir: &Path) -> (Value, Vec<String>) {
        let mut failures = Vec::new();
        let synthesizer = system_tts::synthesizer();
        let voices = system_tts::installed_voices();
        check(
            "at least one installed voice",
            !voices.is_empty(),
            &mut failures,
        );
        let mut report = json!({ "installed_voice_count": voices.len() });

        // 1. System Default, then the first installed voice, each at normal rate.
        let mut cases: Vec<(String, Option<String>)> = vec![("system_default".into(), None)];
        if let Some(first) = voices.first() {
            cases.push((
                format!("voice_{}", first.display_name),
                Some(first.id.clone()),
            ));
        }
        let mut synthesis = Vec::new();
        for (label, voice) in &cases {
            let started = Instant::now();
            let result = synthesizer.synthesize(request(voice.as_deref(), 1.0)).await;
            match result {
                Ok(audio) => {
                    let s = stats(&audio);
                    let duration_ms = s["duration_ms"].as_u64().unwrap_or(0);
                    check(
                        &format!("{label}: non-silent audio"),
                        s["rms"].as_f64().unwrap_or(0.0) > 50.0
                            && s["peak"].as_u64().unwrap_or(0) > 500,
                        &mut failures,
                    );
                    check(
                        &format!("{label}: plausible duration"),
                        (1_000..=10_000).contains(&duration_ms),
                        &mut failures,
                    );
                    check(
                        &format!("{label}: plausible sample rate"),
                        (8_000..=96_000).contains(&audio.sample_rate),
                        &mut failures,
                    );
                    let safe: String = label
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect();
                    let _ = write_wav(&out_dir.join(format!("{safe}.wav")), &audio);
                    synthesis.push(json!({
                        "case": label, "elapsed_ms": started.elapsed().as_millis() as u64, "audio": s
                    }));
                }
                Err(error) => {
                    check(
                        &format!("{label}: synthesize ({:?})", error.kind),
                        false,
                        &mut failures,
                    );
                    synthesis.push(json!({ "case": label, "error": format!("{:?}", error.kind) }));
                }
            }
        }
        report["synthesis"] = Value::Array(synthesis);

        // 2. Faster rate must not take longer than normal rate.
        let normal = synthesizer.synthesize(request(None, 1.0)).await;
        let fast = synthesizer.synthesize(request(None, 2.0)).await;
        if let (Ok(normal), Ok(fast)) = (&normal, &fast) {
            let (n, f) = (stats(normal), stats(fast));
            check(
                "rate 2.0 is shorter than rate 1.0",
                f["duration_ms"].as_u64() < n["duration_ms"].as_u64(),
                &mut failures,
            );
            report["rate_ms"] = json!({ "normal": n["duration_ms"], "fast": f["duration_ms"] });
        } else {
            check("rate comparison synthesized", false, &mut failures);
        }

        // 3. A missing saved voice fails closed (Model) and never falls back.
        let missing = synthesizer
            .synthesize(request(Some("com.apple.voice.does.not.exist"), 1.0))
            .await;
        check(
            "missing voice fails closed with Model error",
            matches!(&missing, Err(e) if e.kind == ProviderErrorKind::Model),
            &mut failures,
        );

        // 4. Dropping a request mid-synthesis must stop cleanly and not poison the next one.
        let cancelled = tokio::time::timeout(
            Duration::from_millis(150),
            synthesizer.synthesize(request(None, 1.0)),
        )
        .await;
        println!("cancellation probe resolved early: {}", cancelled.is_ok());
        let after = synthesizer.synthesize(request(None, 1.0)).await;
        check(
            "synthesis succeeds after a cancelled request",
            after.is_ok(),
            &mut failures,
        );

        (report, failures)
    }

    pub fn main() {
        let out_dir = std::env::args()
            .nth(1)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                eprintln!("usage: system_tts_acceptance <output-dir>");
                std::process::exit(2);
            });
        std::fs::create_dir_all(&out_dir).expect("create output dir");
        // AVSpeechSynthesizer is driven on the main dispatch queue, as in the app's event
        // loop. Do the work on a worker thread and let this thread serve the main queue.
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("runtime");
            let (mut report, failures) = runtime.block_on(run(&out_dir));
            report["failures"] = json!(failures);
            report["passed"] = json!(failures.is_empty());
            let _ = std::fs::write(
                out_dir.join("report.json"),
                serde_json::to_string_pretty(&report).unwrap(),
            );
            std::process::exit(if failures.is_empty() { 0 } else { 1 });
        });
        extern "C" {
            fn dispatch_main() -> !;
        }
        // SAFETY: called once on the main thread; never returns, the worker exits the process.
        unsafe { dispatch_main() }
    }
}

#[cfg(target_os = "macos")]
fn main() {
    mac::main();
}
