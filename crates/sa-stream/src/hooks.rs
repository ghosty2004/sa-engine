//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_core::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::modelinfo;

/// Every module's registrations, in this order (one `HOOKS` list per module keeps parallel porting
/// work conflict-free).
pub const MODULES: &[&[HookReg]] = &[
    crate::cdstream::HOOKS,
    crate::img_size::HOOKS,
    crate::object_instance::HOOKS,
    crate::streaming_info::HOOKS,
    crate::streaming::HOOKS,
    crate::colstore::HOOKS,
    crate::iplstore::HOOKS,
    crate::txdstore::HOOKS,
    crate::loaded_car_group::HOOKS,
    crate::name_damage::HOOKS,
    crate::loading_screen::HOOKS,
    crate::rw_gta_stream::HOOKS,
    crate::misc::HOOKS,
    crate::fileloader::HOOKS,
    crate::pool_new_stubs::HOOKS,
    modelinfo::base::HOOKS,
    modelinfo::atomic::HOOKS,
    modelinfo::time::HOOKS,
    modelinfo::weapon::HOOKS,
    modelinfo::clump::HOOKS,
    modelinfo::ped::HOOKS,
    modelinfo::vehicle::HOOKS,
    modelinfo::model_info::HOOKS,
    modelinfo::model_indices::HOOKS,
    crate::generated::HOOKS,
];

/// Every `#[native]` of this crate (phase B entries of ports that cannot be hooked in phase A).
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::cdstream::NATIVE_CD_STREAM_OPEN_2,
    crate::modelinfo::vehicle::NATIVE_UPGRADE_DESC_CTOR,
    crate::loaded_car_group::NATIVE_ADD_MEMBER,
    crate::loaded_car_group::NATIVE_COUNT_MEMBERS,
    crate::cdstream::NATIVE_CD_STREAM_READ,
    crate::modelinfo::clump::NATIVE_GET_BOUNDING_BOX,
    crate::modelinfo::clump::NATIVE_GET_MODEL_TYPE,
    crate::modelinfo::clump::NATIVE_GET_ANIM_FILE_INDEX,
    crate::modelinfo::atomic::NATIVE_AS_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::atomic::NATIVE_GET_MODEL_TYPE,
    crate::modelinfo::atomic::NATIVE_AS_DAMAGE_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::atomic::NATIVE_LOD_GET_MODEL_TYPE,
    crate::modelinfo::atomic::NATIVE_AS_LOD_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::ped::NATIVE_GET_MODEL_TYPE,
    crate::modelinfo::ped::NATIVE_ADD_XTRA_ATOMICS,
    crate::modelinfo::ped::NATIVE_SET_FACE_TEXTURE,
    crate::modelinfo::weapon::NATIVE_GET_MODEL_TYPE,
    crate::modelinfo::time::NATIVE_GET_TIME_ON,
    crate::modelinfo::time::NATIVE_TIME_GET_MODEL_TYPE,
    crate::modelinfo::time::NATIVE_TIME_GET_TIME_INFO,
    crate::modelinfo::time::NATIVE_LOD_TIME_GET_MODEL_TYPE,
    crate::modelinfo::time::NATIVE_LOD_TIME_GET_TIME_INFO,
    crate::modelinfo::base::NATIVE_AS_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::base::NATIVE_AS_DAMAGE_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::base::NATIVE_AS_LOD_ATOMIC_MODEL_INFO_PTR,
    crate::modelinfo::base::NATIVE_GET_TIME_INFO,
    crate::modelinfo::base::NATIVE_SET_ANIM_FILE,
    crate::modelinfo::base::NATIVE_CONVERT_ANIM_FILE_INDEX,
    crate::modelinfo::base::NATIVE_GET_ANIM_FILE_INDEX,
    crate::modelinfo::vehicle::NATIVE_GET_MODEL_TYPE,
    crate::modelinfo::vehicle::NATIVE_DELETE_VEHICLE_COLOUR_TEXTURES,
    crate::streaming::NATIVE_REMOVE_UNUSED_MODELS_IN_LOADED_LIST,
    crate::streaming::NATIVE_LOAD_INITIAL_VEHICLES,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    crate::shared::GLOBALS,
    crate::cdstream::GLOBALS,
    crate::streaming_info::GLOBALS,
    crate::streaming::GLOBALS,
    crate::colstore::GLOBALS,
    crate::iplstore::GLOBALS,
    crate::txdstore::GLOBALS,
    crate::loaded_car_group::GLOBALS,
    crate::loading_screen::GLOBALS,
    crate::rw_gta_stream::GLOBALS,
    crate::misc::GLOBALS,
    crate::fileloader::GLOBALS,
    modelinfo::base::GLOBALS,
    modelinfo::atomic::GLOBALS,
    modelinfo::time::GLOBALS,
    modelinfo::weapon::GLOBALS,
    modelinfo::clump::GLOBALS,
    modelinfo::ped::GLOBALS,
    modelinfo::vehicle::GLOBALS,
    modelinfo::model_info::GLOBALS,
    modelinfo::model_indices::GLOBALS,
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

    /// No global of this crate overlaps another one of this crate or one of sa-core's (a global
    /// declared twice would be two separate statics in phases B/C). Sizes are the exe's (32-bit).
    /// sa-core's globals are not checked against each other here (sa-core has its own test).
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let core: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
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

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against sa-core's.
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
        for h in sa_core::hooks::ALL {
            assert!(!addrs.contains(&h.addr), "{} hooked by sa-core too", h.name);
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
