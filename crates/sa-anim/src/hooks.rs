//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after the
//! hooks of `sa_core`, `sa_stream`, `sa_world`, `sa_input`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::assoc;
use crate::bones;
use crate::building;
use crate::clumpdata;
use crate::cutscene;
use crate::group;
use crate::hierarchy;
use crate::ik;
use crate::manager;
use crate::node;
use crate::pedik;
use crate::sequence;
use crate::statics;
use crate::vehgroup;

/// Every module's registrations, in this order (PORT-ANIM-INPUT, ANIM).
pub const MODULES: &[&[HookReg]] = &[
    node::HOOKS,
    sequence::HOOKS,
    hierarchy::HOOKS,
    assoc::HOOKS,
    group::HOOKS,
    clumpdata::HOOKS,
    manager::HOOKS,
    bones::HOOKS,
    ik::HOOKS,
    pedik::HOOKS,
    vehgroup::HOOKS,
    cutscene::HOOKS,
    crate::cutscene_postload::HOOKS,
    building::HOOKS,
    statics::HOOKS,
    crate::mvpb_anim::HOOKS,
    crate::anim_file_load::HOOKS,
    crate::rp_anim_blend::HOOKS,
    crate::clump_update::HOOKS,
    crate::frame_update::HOOKS,
    crate::frame_update_non_skinned::HOOKS,
    crate::ped_ik_point_gun::HOOKS,
    crate::cutscene_update::HOOKS,
    crate::cutscene_delete::HOOKS,
    crate::cutscene_preload::HOOKS,
    crate::generated::HOOKS,
    crate::cutscene_memory::HOOKS,
    crate::quaternion_get_matrix::HOOKS,
];

/// Every `#[native]` registration of this crate (phase B entries of ports that cannot be hooked).
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::rp_anim_blend::NATIVE_RP_ANIM_BLEND_FRAME_GET_NAME,
    crate::manager::NATIVE_GET_ANIMATION_BLOCK,
    crate::rp_anim_blend::NATIVE_RP_ANIM_BLEND_CLUMP_GET_FIRST_ASSOCIATION,
    crate::rp_anim_blend::NATIVE_RP_ANIM_BLEND_CLUMP_IS_INITIALIZED,
    crate::cutscene_memory::NATIVE_CUTSCENE_ADD_CUTSCENE_HEAD,
    crate::cutscene_memory::NATIVE_CUTSCENE_SET_HEAD_ANIM,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    node::GLOBALS,
    sequence::GLOBALS,
    hierarchy::GLOBALS,
    assoc::GLOBALS,
    group::GLOBALS,
    clumpdata::GLOBALS,
    manager::GLOBALS,
    bones::GLOBALS,
    ik::GLOBALS,
    pedik::GLOBALS,
    vehgroup::GLOBALS,
    cutscene::GLOBALS,
    building::GLOBALS,
    statics::GLOBALS,
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

    /// No global of this crate overlaps another one of this crate or one of the other engine crates' (a
    /// global declared twice would be two separate statics in phases B/C). Sizes are the exe's
    /// (32-bit).
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let core: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_input::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(core.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against the other engine
    /// crates'.
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
            .chain(sa_input::hooks::ALL.iter())
        {
            assert!(
                !addrs.contains(&h.addr),
                "{} hooked by another engine crate too",
                h.name
            );
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
