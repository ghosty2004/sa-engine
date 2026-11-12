//! Two destructors on the walking path (batch `std11-ai-1`): `CEventStuckInAir::~CEventStuckInAir` (0x4B1520) and the
//! complete-object destructor of `CTaskComplexWanderProstitute` (0x6726F0). Ported from the 1.0 US bytes; unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskComplex` base destructor (0x61A3D0, thiscall)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => crate::mvpb_ai::task_complex_dtor_61a3d0;
}

/// `CEventStuckInAir::~CEventStuckInAir` (0x4B1520, thiscall): vtable 0x85B508; with the ped (+0xC) set,
/// `ped->CleanUpOldReference(&this->ped)`; then the `CEvent` vtable 0x85AB38. The SEH frame of the original has no
/// handler work (state -1 after the body).
#[hook(addr = 0x4B1520, conv = thiscall, name = "FUN_004b1520", expect = "6A FF 68 D8 A0 83 00",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_stuck_in_air_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B508);
    let ped = rd::<GPtr<u8>>(this, 0xC);
    if !ped.is_null() {
        entity_clean_up_old_reference(ped, this.byte_add(0xC));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CTaskComplexWanderProstitute` complete-object destructor (0x6726F0, thiscall): vtable 0x86FE84, then the
/// `CTaskComplex` base destructor (0x61A3D0, tail jump).
#[hook(addr = 0x6726F0, conv = thiscall, name = "FUN_006726f0", expect = "C7 01 84 FE 86 00",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_complex_wander_prostitute_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FE84);
    task_complex_dtor(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_STUCK_IN_AIR_DTOR, HOOK_TASK_COMPLEX_WANDER_PROSTITUTE_DTOR];
