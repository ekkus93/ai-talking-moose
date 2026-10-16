use std::path::{Path, PathBuf};
use std::process::Command;

const MOONSHINE_DYLIB: &str = "libmoonshine.dylib";
const ONNXRUNTIME_DYLIB: &str = "libonnxruntime.1.23.2.dylib";
const UNKNOWN_BUILD_COMMIT: &str = "unknown";

const WHISPER_LIB_DIR: &str = "TALKING_MOOSE_WHISPER_LIB_DIR";
const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";
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

// The pinned whisper.cpp source builds for Linux and macOS without requiring
// shared libwhisper libraries at application runtime. macOS uses a separate
// target-specific CMake directory so ARM and Intel archives cannot be mixed.
// The Moonshine path remains intact until Mac real-model acceptance is proven.
fn explicit_whisper_lib_dir() -> Option<PathBuf> {
    let lib_dir = std::env::var(WHISPER_LIB_DIR).ok()?;
    let lib_dir = lib_dir.trim();
    if lib_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(lib_dir))
    }
}

fn target_is_macos() -> bool {
    matches!(
        std::env::var("TARGET").as_deref(),
        Ok("aarch64-apple-darwin" | "x86_64-apple-darwin")
    )
}

fn whisper_target_supported() -> bool {
    std::env::var("TARGET")
        .map(|target| target.contains("linux") || target_is_macos())
        .unwrap_or(false)
}

fn require_whisper_native() -> bool {
    // An explicit CI/acceptance requirement always wins. Standard production
    // release builds on supported targets must not silently ship a Whisper
    // selector that can only return RuntimeUnavailable when native linking
    // fails. Debug builds without an initialized submodule may still fail
    // closed so developer checkouts remain usable.
    std::env::var("TALKING_MOOSE_REQUIRE_WHISPER_NATIVE").is_ok_and(|value| value == "1")
        || (whisper_target_supported()
            && std::env::var("PROFILE").is_ok_and(|profile| profile == "release"))
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
    if target_is_macos() {
        // The static whisper.cpp/ggml implementation contains C++ objects.
        println!("cargo:rustc-link-lib=dylib=c++");
    }
    println!("cargo:rustc-cfg=whisper_native_linked");
}

fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

fn explicit_whisper_build_jobs() -> Option<usize> {
    let raw = std::env::var(WHISPER_BUILD_JOBS).ok()?;
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    let jobs: usize = value
        .parse()
        .unwrap_or_else(|_| panic!("{WHISPER_BUILD_JOBS} must be a positive integer"));
    if jobs == 0 {
        panic!("{WHISPER_BUILD_JOBS} must be greater than zero");
    }
    Some(jobs)
}

fn whisper_build_jobs() -> usize {
    explicit_whisper_build_jobs().unwrap_or_else(cpu_count)
}

fn emit_rerun_tree(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            pending.push(entry.path());
        }
    }
}

fn emit_whisper_native_rerun_paths(whisper_src: &Path) {
    for path in [
        whisper_src.join("CMakeLists.txt"),
        whisper_src.join("cmake"),
        whisper_src.join("include"),
        whisper_src.join("src"),
        whisper_src.join("ggml").join("CMakeLists.txt"),
        whisper_src.join("ggml").join("cmake"),
        whisper_src.join("ggml").join("include"),
        whisper_src.join("ggml").join("src"),
    ] {
        emit_rerun_tree(&path);
    }
}

fn build_whisper_from_source() {
    if !whisper_target_supported() {
        return;
    }

    let Some(root) = repository_root() else {
        whisper_build_failure("repository root unavailable");
        return;
    };

    let whisper_src = root.join("third_party").join("whisper.cpp");
    if !whisper_src.join("CMakeLists.txt").is_file() {
        whisper_build_failure("pinned whisper.cpp submodule is not checked out");
        return;
    }

    let target = std::env::var("TARGET").expect("Cargo supplies TARGET");
    let build_dir = if target_is_macos() {
        root.join("build").join(format!("whisper-{target}"))
    } else {
        root.join("build").join("whisper")
    };
    if std::fs::create_dir_all(&build_dir).is_err() {
        whisper_build_failure("could not create native build directory");
        return;
    }

    emit_whisper_native_rerun_paths(&whisper_src);

    let mut configure = Command::new("cmake");
    configure
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
        ]);
    if target_is_macos() {
        let arch = if target.starts_with("aarch64-") {
            "arm64"
        } else {
            "x86_64"
        };
        let deployment_target =
            std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.4".to_string());
        configure
            .arg(format!("-DCMAKE_OSX_ARCHITECTURES={arch}"))
            .arg(format!("-DCMAKE_OSX_DEPLOYMENT_TARGET={deployment_target}"))
            // Real-model CPU acceptance precedes optional Metal acceleration.
            .args([
                "-DGGML_METAL=OFF",
                "-DGGML_ACCELERATE=OFF",
                "-DGGML_BLAS=OFF",
            ]);
    }

    let configure = configure.output();
    let Ok(configure) = configure else {
        whisper_build_failure("CMake configuration could not start");
        return;
    };
    if !configure.status.success() {
        whisper_build_failure("CMake configuration failed");
        return;
    }

    let threads = whisper_build_jobs().to_string();
    let make = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .args(["--config", "Release", "--target", "whisper", "--parallel"])
        .arg(&threads)
        .status();
    let Ok(make) = make else {
        whisper_build_failure("native build tool could not start");
        return;
    };
    if !make.success() {
        whisper_build_failure("native build failed");
        return;
    }

    if has_whisper_runtime(&build_dir) {
        emit_whisper_link(&build_dir);
    } else {
        whisper_build_failure("required static archives were not produced");
    }
}

fn whisper_build_failure(reason: &str) {
    if require_whisper_native() {
        panic!("Required Whisper native build failed: {reason}");
    }
    println!("cargo:warning=Whisper native build {reason}; local Whisper ASR is unavailable.");
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
    println!("cargo:rustc-check-cfg=cfg(whisper_native_linked)");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_MOONSHINE_LIB_DIR");

    println!("cargo:rerun-if-env-changed={WHISPER_LIB_DIR}");
    println!("cargo:rerun-if-env-changed={WHISPER_BUILD_JOBS}");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_REQUIRE_WHISPER_NATIVE");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
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

    // Preserve the pre-cutover fail-closed behavior for source checkouts
    // without submodules. Mac native acceptance sets REQUIRE=1, and must fail
    // instead of treating a missing Whisper runtime as a passing build.
    if let Some(build_dir) = explicit_whisper_lib_dir() {
        // This escape hatch is for debug/test ABI qualification only. A
        // production release must build the pinned whisper.cpp submodule
        // rather than silently linking developer-supplied static archives.
        if std::env::var("PROFILE").as_deref() != Ok("debug") {
            panic!(
                "{WHISPER_LIB_DIR} is only allowed in debug/test builds; release builds must compile pinned whisper.cpp source"
            );
        }
        if has_whisper_runtime(&build_dir) {
            emit_whisper_link(&build_dir);
        } else {
            whisper_build_failure("explicit Whisper static archives are missing");
        }
    } else {
        build_whisper_from_source();
    }

    tauri_build::build()
}
