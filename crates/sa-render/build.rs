// Phase A (ADR-001) = feature `in-process` on a 32-bit x86 Windows target: game state lives at the
// original's fixed addresses and calls to original functions go through those addresses.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let feature = std::env::var_os("CARGO_FEATURE_IN_PROCESS").is_some();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if feature && arch == "x86" && os == "windows" {
        println!("cargo:rustc-cfg=sa_phase_a");
    }
}
