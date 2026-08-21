use std::path::{Path, PathBuf};

const MOONSHINE_DYLIB: &str = "libmoonshine.dylib";
const ONNXRUNTIME_DYLIB: &str = "libonnxruntime.1.23.2.dylib";

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

fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_MOONSHINE_LIB_DIR");
    println!("cargo:rerun-if-changed=native/macos/{MOONSHINE_DYLIB}");
    println!("cargo:rerun-if-changed=native/macos/{ONNXRUNTIME_DYLIB}");

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
