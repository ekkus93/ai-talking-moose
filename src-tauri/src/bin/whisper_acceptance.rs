use std::path::PathBuf;

use talking_moose_lib::asr::whisper::acceptance::{
    delete_for_acceptance, install_for_acceptance, transcribe_for_acceptance,
};

fn usage() -> &'static str {
    "usage:\n\
     whisper_acceptance install <model-root> <report-json>\n\
     whisper_acceptance delete <model-root> <report-json>\n\
     whisper_acceptance transcribe <model-root> <corpus-wav> <report-json> [--require-network-denied]"
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", usage());
        std::process::exit(2);
    }

    let phase = &args[0];

    let (model_root, corpus_wav, report_path, require_network_denied) =
        match (phase.as_str(), args.len()) {
            ("install", 3) => (
                PathBuf::from(&args[1]),
                None,
                PathBuf::from(&args[2]),
                false,
            ),
            ("delete", 3) => (
                PathBuf::from(&args[1]),
                None,
                PathBuf::from(&args[2]),
                false,
            ),
            ("transcribe", 4) => (
                PathBuf::from(&args[1]),
                Some(PathBuf::from(&args[2])),
                PathBuf::from(&args[3]),
                false,
            ),
            ("transcribe", 5) if args[4].as_str() == "--require-network-denied" => (
                PathBuf::from(&args[1]),
                Some(PathBuf::from(&args[2])),
                PathBuf::from(&args[3]),
                true,
            ),
            _ => {
                eprintln!("{}", usage());
                std::process::exit(2);
            }
        };

    let result = match phase.as_str() {
        "install" => install_for_acceptance(&model_root, &report_path)
            .await
            .and_then(|report| {
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
            }),
        "transcribe" => transcribe_for_acceptance(
            &model_root,
            corpus_wav
                .as_ref()
                .expect("transcribe phase always provides a corpus wav"),
            &report_path,
            require_network_denied,
        )
        .await
        .and_then(|report| {
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
        }),
        "delete" => delete_for_acceptance(&model_root, &report_path)
            .await
            .and_then(|report| {
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
            }),
        _ => {
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };

    match result {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("Whisper ASR acceptance failed: {error}");
            std::process::exit(1);
        }
    }
}
