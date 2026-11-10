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
pub mod coverage;
pub mod emu;
pub mod fidelity;
pub mod generate;
pub mod hosted;
pub mod l2;
pub mod l3;
pub mod list_specs;
pub mod math_specs;
pub mod mspec;
pub mod native;
pub mod pe;
pub mod runner;
pub mod scan;
pub mod specs;
pub mod specs_core;
pub mod specs_coll_data;
pub mod specs_coll_prim;
pub mod specs_coll_surface;
pub mod specs_world_p1;
pub mod specs_phys;
pub mod specs_core_util;
pub mod specs_stream_a;
pub mod specs_stream_b;
pub mod specs_stream_c;
pub mod specs_stream_m;
pub mod split;

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

static HOOK_PRESERVE: OnceLock<std::collections::HashMap<u32, u8>> = OnceLock::new();

/// The `#[hook(preserve = ...)]` registers of the port hooked at `addr` (sa-core, sa-stream and sa-world
/// registrations; `None` when no port is hooked there).
pub fn hook_preserve(addr: u32) -> Option<u8> {
    HOOK_PRESERVE
        .get_or_init(|| {
            sa_core::hooks::ALL
                .iter()
                .chain(sa_stream::hooks::ALL.iter())
                .chain(sa_world::hooks::ALL.iter())
                .map(|r| (r.addr, r.preserve))
                .collect()
        })
        .get(&addr)
        .copied()
}

static HOST_GLOBALS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialises spec ports that run a hooked entry on sa-core's / sa-stream's Rust-owned globals
/// (phase B storage, one copy per process): hold the guard from filling the globals to reading the
/// results, so specs running on other test threads cannot interleave.
pub fn host_globals() -> std::sync::MutexGuard<'static, ()> {
    HOST_GLOBALS.lock().unwrap_or_else(|e| e.into_inner())
}
