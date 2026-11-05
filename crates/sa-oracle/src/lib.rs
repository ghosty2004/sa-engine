//! Host-side oracles for the 1:1 rebuild (PLAN.md §5).
//!
//! L1 (this crate so far): run original functions of `gta_sa.exe` 1.0 US under Unicorn ([`emu`]) or on
//! the real CPU under Wine ([`native`]), and compare them with the Rust ports with a deterministic
//! property runner ([`runner`]) over the function specs in [`specs`]. [`fidelity`] measures where Unicorn's
//! x87 differs from the real CPU; functions that depend on such instructions carry an explicit route to
//! L2. See `docs/re/l1-harness.md`.
//!
//! Host only: for Windows targets (the i686 workspace build) the crate is empty.
#![cfg(not(windows))]

pub mod call;
pub mod catalog;
pub mod emu;
pub mod fidelity;
pub mod generate;
pub mod l2;
pub mod list_specs;
pub mod math_specs;
pub mod mspec;
pub mod native;
pub mod pe;
pub mod runner;
pub mod scan;
pub mod specs;
pub mod specs_core;
pub mod specs_core_util;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

/// The workspace root (two levels above this crate).
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

static CATALOG: OnceLock<Option<catalog::Catalog>> = OnceLock::new();

/// The catalogue, loaded once (None when `catalog/procedures.csv` is missing).
pub fn catalog_cached() -> Option<&'static catalog::Catalog> {
    CATALOG.get_or_init(|| catalog::Catalog::load().ok()).as_ref()
}

static IMAGE: OnceLock<Option<Arc<pe::Image>>> = OnceLock::new();

/// The exe image, loaded once (None when the exe is not found).
pub fn image_cached() -> Option<Arc<pe::Image>> {
    IMAGE
        .get_or_init(|| {
            pe::find_exe()
                .and_then(|p| pe::Image::load(&p).ok())
                .map(Arc::new)
        })
        .clone()
}
