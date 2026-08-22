fn main() {
    println!("cargo:rustc-check-cfg=cfg(moonshine_native_linked)");
}
