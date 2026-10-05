use std::path::{Path, PathBuf};
use std::process::Command;

const MOONSHINE_DYLIB: &str = "libmoonshine.dylib";
const ONNXRUNTIME_DYLIB: &str = "libonnxruntime.1.23.2.dylib";
const UNKNOWN_BUILD_COMMIT: &str = "unknown";

const WHISPER_LIB_DIR: &str = "TALKING_MOOSE_WHISPER_LIB_DIR";
const WHISPER_STATIC_LIBS: [&str; 4] = ["whisper", "ggml", "ggml-cpu", "ggml-base"];

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
    // An explicit release/acceptance override is authoritative. Otherwise prefer
    // the checkout that Cargo is actually compiling. On pull_request events,
    // GitHub's ambient GITHUB_SHA can name a synthetic merge commit even when a
    // workflow deliberately checks out the exact PR head for provenance.
    commit_from_env("TALKING_MOOSE_BUILD_COMMIT")
        .or_else(git_commit)
        .or_else(|| commit_from_env("GITHUB_SHA"))
        .unwrap_or_else(|| UNKNOWN_BUILD_COMMIT.to_string())
}

// Whisper source build (Linux only). Builds whisper.cpp and ggml into
// <root>/build/whisper and links the resulting static libraries. macOS and
// non-Linux targets fail closed for Whisper until proven.
fn explicit_whisper_lib_dir() -> Option<PathBuf> {
    let lib_dir = std::env::var(WHISPER_LIB_DIR).ok()?;
    let lib_dir = lib_dir.trim();
    if lib_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(lib_dir))
    }
}

fn target_is_linux() -> bool {
    match std::env::var_os("TARGET") {
        Some(target) => target.to_string_lossy().contains("linux"),
        None => false,
    }
}

fn has_whisper_runtime(build_dir: &Path) -> bool {
    build_dir.join("src").join("libwhisper.a").is_file()
        && build_dir
            .join("ggml")
            .join("src")
            .join("libggml.a")
            .is_file()
        && build_dir
            .join("ggml")
            .join("src")
            .join("libggml-base.a")
            .is_file()
        && build_dir
            .join("ggml")
            .join("src")
            .join("libggml-cpu.a")
            .is_file()
}

fn emit_whisper_link(build_dir: &Path) {
    println!("cargo:rustc-link-search=native={}/src", build_dir.display());
    println!(
        "cargo:rustc-link-search=native={}/ggml/src",
        build_dir.display()
    );
    for lib in WHISPER_STATIC_LIBS {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    println!("cargo:rustc-cfg=whisper_native_linked");
}

fn cpu_count() -> usize {
    std::fs::read_to_string("/proc/nproc")
        .ok()
        .and_then(|content| content.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(4)
}

fn build_whisper_from_source() {
    if !target_is_linux() {
        return;
    }

    let Some(root) = repository_root() else {
        return;
    };

    let whisper_src = root.join("third_party").join("whisper.cpp");
    if !whisper_src.join("CMakeLists.txt").is_file() {
        return;
    }

    let build_dir = root.join("build").join("whisper");
    if std::fs::create_dir_all(&build_dir).is_err() {
        return;
    }

    println!(
        "cargo:rerun-if-changed={}",
        whisper_src.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        whisper_src.join("src").join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        whisper_src.join("ggml").join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        whisper_src.join("src").join("whisper.cpp").display()
    );

    let configure = Command::new("cmake")
        .current_dir(&build_dir)
        .arg("-S")
        .arg(&whisper_src)
        .args([
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DWHISPER_BUILD_IS_DEV=OFF",
            "-DWHISPER_BUILD_TESTS=OFF",
            "-DWHISPER_BUILD_EXAMPLES=OFF",
            "-DWHISPER_BUILD_SERVER=OFF",
            "-DWHISPER_ALL_WARNINGS=OFF",
            "-DCMAKE_C_FLAGS=-fPIC",
            "-DCMAKE_CXX_FLAGS=-fPIC",
        ])
        .output();
    if let Ok(configure) = configure {
        if !configure.status.success() {
            return;
        }
    }

    let threads = cpu_count().to_string();
    let make = Command::new("make")
        .current_dir(&build_dir)
        .arg("-j")
        .arg(&threads)
        .arg("whisper")
        .status();
    if let Ok(make) = make {
        if !make.success() {
            return;
        }
    }

    if has_whisper_runtime(&build_dir) {
        emit_whisper_link(&build_dir);
    }
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
    println!("cargo:rustc-check-cfg=cfg(whisper_native_linked)");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_MOONSHINE_LIB_DIR");
    println!("cargo:rerun-if-env-changed={WHISPER_LIB_DIR}");
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

    // Whisper source build (Linux) and escape hatch. Fails closed when the
    // native runtime is absent or on an unsupported target.
    if let Some(build_dir) = explicit_whisper_lib_dir() {
        if has_whisper_runtime(&build_dir) {
            emit_whisper_link(&build_dir);
        }
    } else {
        build_whisper_from_source();
    }

    tauri_build::build()
}
