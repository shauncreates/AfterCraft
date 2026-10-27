//! Native builds exercise the portable store through the rlib. The browser ABI is wasm-only.
//! Disable MinGW's implicit DLL exports: exporting the entire engine exceeds PE's 65,535 limit.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        return;
    }
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let linker = std::env::var("RUSTC_LINKER").unwrap_or_default();
    let direct = flags.contains("linker-flavor=ld") || linker.ends_with("ld.lld.exe") || linker.ends_with("ld.lld");
    let option = if direct { "--exclude-all-symbols" } else { "-Wl,--exclude-all-symbols" };
    println!("cargo:rustc-cdylib-link-arg={option}");
}
