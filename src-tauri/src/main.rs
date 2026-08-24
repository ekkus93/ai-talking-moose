// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().any(|arg| arg == "--build-info") {
        println!("build-commit={}", env!("TALKING_MOOSE_BUILD_COMMIT"));
        println!("version={}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if std::env::args_os().any(|arg| arg == "--moonshine-native-smoke-test") {
        match talking_moose_lib::moonshine_native_smoke_check() {
            Ok(version) => {
                println!("moonshine-runtime-version={version}");
                return;
            }
            Err(error) => {
                eprintln!("moonshine-native-smoke-error={error}");
                std::process::exit(2);
            }
        }
    }

    talking_moose_lib::run();
}
