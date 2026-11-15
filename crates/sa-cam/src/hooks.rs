//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_pop::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    cam, camcol, camctl, camera, camfx, cammodes, camproc, camtarget, camtrans, curves, handshaker, idlecam,
    splines, statics,
};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    camera::HOOKS,
    camctl::HOOKS,
    camfx::HOOKS,
    cam::HOOKS,
    handshaker::HOOKS,
    idlecam::HOOKS,
    curves::HOOKS,
    statics::HOOKS,
    camcol::HOOKS,
    camtarget::HOOKS,
    splines::HOOKS,
    camproc::HOOKS,
    camtrans::HOOKS,
    cammodes::HOOKS,
    crate::followcar::HOOKS,
    crate::dwcams::HOOKS,
    crate::arrestcam::HOOKS,
    crate::twoplayer::HOOKS,
    crate::followped::HOOKS,
    crate::aimweapon::HOOKS,
    crate::avoidgeom::HOOKS,
    crate::firstperson::HOOKS,
    crate::rocket::HOOKS,
    crate::m16::HOOKS,
    crate::pedonpc::HOOKS,
    crate::pedmouse::HOOKS,
    crate::camcontrol::HOOKS,
    crate::obbe::HOOKS,
    crate::camera_process::HOOKS,
    crate::generated::HOOKS,
    // WRITER-NATIVE
    crate::idle_cam_slerp::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    camera::GLOBALS,
    camctl::GLOBALS,
    camfx::GLOBALS,
    cam::GLOBALS,
    handshaker::GLOBALS,
    idlecam::GLOBALS,
    curves::GLOBALS,
    camcol::GLOBALS,
    camtarget::GLOBALS,
    splines::GLOBALS,
    camproc::GLOBALS,
    camtrans::GLOBALS,
    cammodes::GLOBALS,
    crate::followcar::GLOBALS,
    crate::dwcams::GLOBALS,
    crate::arrestcam::GLOBALS,
    crate::followped::GLOBALS,
    crate::aimweapon::GLOBALS,
    crate::avoidgeom::GLOBALS,
    crate::firstperson::GLOBALS,
    crate::rocket::GLOBALS,
    crate::pedonpc::GLOBALS,
    crate::pedmouse::GLOBALS,
    crate::camcontrol::GLOBALS,
    crate::obbe::GLOBALS,
];

use sa_core::hooks::total;

use sa_core::hooks::concat;

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use sa_testkit::{count_hooks, exe_size};
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;
    
    use std::vec::Vec;

    /// Phase B/C storage starts with the original image's initial values (checked when the exe is
    /// available: `$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`).
    #[test]
    fn global_initial_values_match_the_exe() {
        sa_testkit::check_global_initial_values(GLOBALS);
    }

    /// No global of this crate overlaps another one of this crate or one of sa-core's / sa-stream's /
    /// sa-world's / sa-pop's.
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let others: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_pop::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(others.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against the other crates.
    #[test]
    fn list_is_complete_and_unique() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(
            count_hooks(&dir),
            COUNT,
            "a #[hook] is missing from a module's HOOKS"
        );
        let addrs: BTreeSet<u32> = ALL.iter().map(|h| h.addr).collect();
        assert_eq!(addrs.len(), COUNT);
        for h in sa_core::hooks::ALL
            .iter()
            .chain(sa_stream::hooks::ALL.iter())
            .chain(sa_world::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
        {
            assert!(!addrs.contains(&h.addr), "{} hooked by another crate too", h.name);
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
