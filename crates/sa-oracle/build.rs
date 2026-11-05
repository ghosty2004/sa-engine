//! The bundled Unicorn (QEMU 5.0.1 TCG) uses 16-byte atomics (`__atomic_compare_exchange_16`), which
//! GCC provides in libatomic. sa-oracle is host-only: for Windows targets the crate is empty.
fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        println!("cargo:rustc-link-lib=atomic");
    }
}
