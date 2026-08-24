use std::path::{Path, PathBuf};
use std::process::Command;

const MOONSHINE_DYLIB: &str = "libmoonshine.dylib";
const ONNXRUNTIME_DYLIB: &str = "libonnxruntime.1.23.2.dylib";
const UNKNOWN_BUILD_COMMIT: &str = "unknown";

fn explicit_library_dir() -> Option<PathBuf> {
    let lib_dir = std::env::var("TALKING_MOOSE_MOONSHINE_LIB_DIR").ok()?;
    let lib_dir = lib_dir.trim();
    if lib_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(lib_dir))
    }
}

fn packaged_macos_library_dir() -> Option<PathBuf> {
    let target = std::env::var("TARGET").ok()?;
    if !target.ends_with("-apple-darwin") {
        return None;
    }
    Some(PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?).join("native/macos"))
}

fn has_packaged_macos_runtime(lib_dir: &Path) -> bool {
    lib_dir.join(MOONSHINE_DYLIB).is_file() && lib_dir.join(ONNXRUNTIME_DYLIB).is_file()
}

fn emit_moonshine_link(lib_dir: &Path) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=moonshine");
    println!("cargo:rustc-cfg=moonshine_native_linked");
}

fn normalize_commit(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(value.to_ascii_lowercase())
    } else {
        None
    }
}

fn commit_from_env(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    Some(
        normalize_commit(&value)
            .unwrap_or_else(|| panic!("{name} must contain a full 40-character Git SHA")),
    )
}

fn repository_root() -> Option<PathBuf> {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?)
        .parent()
        .map(Path::to_path_buf)
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn git_commit() -> Option<String> {
    let repo_root = repository_root()?;
    normalize_commit(&git_output(&repo_root, &["rev-parse", "HEAD"])?)
}

fn emit_git_rerun_paths() {
    let Some(repo_root) = repository_root() else {
        return;
    };
    let Some(git_dir) = git_output(&repo_root, &["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    let git_dir = PathBuf::from(git_dir.trim());
    let head_path = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head_path.display());

    let Ok(head) = std::fs::read_to_string(&head_path) else {
        return;
    };
    let Some(reference) = head.trim().strip_prefix("ref: ") else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join(reference).display()
    );
}

fn build_commit() -> String {
    commit_from_env("TALKING_MOOSE_BUILD_COMMIT")
        .or_else(|| commit_from_env("GITHUB_SHA"))
        .or_else(git_commit)
        .unwrap_or_else(|| UNKNOWN_BUILD_COMMIT.to_string())
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_MOONSHINE_LIB_DIR");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=native/macos/{MOONSHINE_DYLIB}");
    println!("cargo:rerun-if-changed=native/macos/{ONNXRUNTIME_DYLIB}");
    emit_git_rerun_paths();
    println!(
        "cargo:rustc-env=TALKING_MOOSE_BUILD_COMMIT={}",
        build_commit()
    );

    if let Some(lib_dir) = explicit_library_dir() {
        // Deliberate development/benchmark escape hatch retained from the
        // pre-packaging implementation. Production macOS bundles do not set it.
        emit_moonshine_link(&lib_dir);
    } else if let Some(lib_dir) = packaged_macos_library_dir() {
        if has_packaged_macos_runtime(&lib_dir) {
            emit_moonshine_link(&lib_dir);
        }
    }

    tauri_build::build()
}
