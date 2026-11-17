//! FRONTIER-2 turn 10, lane A (batch `frontier2-a14`): small WORLD ports from the 1.0 US bytes (gta-reversed read as
//! an aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use sa_core::bindings::{eh_vector_dtor, list_dtor};

/// `InteriorManager_c::~InteriorManager_c()` (0x598580, thiscall): the member lists and arrays in reverse order: the
/// lists at +0x4298 and +0x428C, the 8 interior groups at +0x3CAC (0xBC each, `InteriorGroup_c::~InteriorGroup_c`
/// 0x597FF0), the list at +0x3CA0, the 8 interiors at +0 (0x794 each, `Interior_c::~Interior_c` 0x591360).
#[hook(addr = 0x598580, conv = thiscall, name = "InteriorManager_c::~InteriorManager_c", this = "InteriorManager_c",
       expect = "6A FF 68 8D D1 83 00",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes",
       note = "only at exit (the global g_interiorMan's atexit destructor)")]
pub fn interior_manager_dtor(this: GPtr<u8>) {
    list_dtor(this.byte_add(0x4298));
    list_dtor(this.byte_add(0x428C));
    eh_vector_dtor(this.byte_add(0x3CAC), 0xBC, 8, 0x597FF0);
    list_dtor(this.byte_add(0x3CA0));
    eh_vector_dtor(this, 0x794, 8, 0x591360);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_MANAGER_DTOR];
