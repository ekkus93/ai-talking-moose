use std::path::PathBuf;
use talking_moose_lib::ai::local::acceptance::{generate_for_acceptance, install_for_acceptance};

fn usage() -> &'static str {
    "usage: local_llm_acceptance <install|generate> <model-id> <model-root> <report-json> [--require-network-denied]"
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 4 || args.len() > 5 {
        eprintln!("{}", usage());
        std::process::exit(2);
    }

    let phase = &args[0];
    let model_id = &args[1];
    let model_root = PathBuf::from(&args[2]);
    let report_path = PathBuf::from(&args[3]);
    let require_network_denied = match args.get(4).map(String::as_str) {
        None => false,
        Some("--require-network-denied") => true,
        Some(_) => {
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };

    let result = match phase.as_str() {
        "install" => {
            if require_network_denied {
                Err("install phase cannot require a denied network boundary".to_string())
            } else {
                install_for_acceptance(model_id, &model_root, &report_path)
                    .await
                    .and_then(|report| {
                        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
                    })
            }
        }
        "generate" => {
            generate_for_acceptance(model_id, &model_root, &report_path, require_network_denied)
                .await
                .and_then(|report| {
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
                })
        }
        _ => Err(usage().to_string()),
    };

    match result {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("Local LLM acceptance failed: {error}");
            std::process::exit(1);
        }
    }
}
