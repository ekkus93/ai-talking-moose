fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
    println!("cargo:rerun-if-env-changed=TALKING_MOOSE_MOONSHINE_LIB_DIR");

    if let Ok(lib_dir) = std::env::var("TALKING_MOOSE_MOONSHINE_LIB_DIR") {
        if !lib_dir.trim().is_empty() {
            println!("cargo:rustc-link-search=native={lib_dir}");
            println!("cargo:rustc-link-lib=dylib=moonshine");
            println!("cargo:rustc-cfg=moonshine_native_linked");
        }
    }

    tauri_build::build()
}
