use std::env;
use std::fs;
use std::path::PathBuf;
use talking_moose_lib::app::wake_word_acceptance::run_real_kws_acceptance;

fn value_after(args: &[String], flag: &str) -> Result<PathBuf, String> {
    let index = args
        .iter()
        .position(|arg| arg == flag)
        .ok_or_else(|| format!("missing required argument {flag}"))?;
    args.get(index + 1)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("wake-word acceptance error: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let model_dir = value_after(&args, "--model-dir")?;
    let runtime_dir = value_after(&args, "--runtime-dir")?;
    let corpus_dir = value_after(&args, "--corpus-dir")?;
    let index = value_after(&args, "--index")?;
    let output = value_after(&args, "--output")?;

    let report = run_real_kws_acceptance(&model_dir, &runtime_dir, &corpus_dir, &index)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|_| "failed to serialize Wake Word acceptance report".to_string())?;
    fs::write(&output, format!("{json}\n"))
        .map_err(|_| "failed to write Wake Word acceptance report".to_string())?;
    println!("{json}");
    if !report.passed {
        return Err("real KWS corpus acceptance criteria were not met".to_string());
    }
    Ok(())
}
