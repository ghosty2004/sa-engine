//! `sa-oracle` CLI (host only; for Windows targets the binary is empty).
//!
//! ```text
//! sa-oracle list
//! sa-oracle l1 <fn|all> [--cases N] [--seed S] [--threads T] [--backend auto|unicorn|native]
//!                       [--cw 0x7f] [--mxcsr 0x9fa0] [--no-fixtures]
//! sa-oracle fidelity [--cases N] [--fn-cases N] [--no-functions] [--no-ftz-workaround]
//! sa-oracle mutants [--cases N]          (needs --features mutants)
//! ```

#[cfg(not(windows))]
mod cli;

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    cli::main()
}

#[cfg(windows)]
fn main() {}
