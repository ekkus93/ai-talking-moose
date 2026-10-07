use std::path::{Path, PathBuf};

pub(super) fn whisper_model_root(db_path: Option<&str>) -> PathBuf {
    let Some(db_path) = db_path else {
        return std::env::temp_dir()
            .join("talking-moose-ai-tests")
            .join("models")
            .join("whisper")
            .join("whisper-small");
    };

    Path::new(db_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("models")
        .join("whisper")
        .join("whisper-small")
}

pub(super) fn moonshine_model_root(db_path: Option<&str>) -> PathBuf {
    let Some(db_path) = db_path else {
        return std::env::temp_dir()
            .join("talking-moose-ai-tests")
            .join("models")
            .join("moonshine");
    };

    Path::new(db_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("models")
        .join("moonshine")
}
