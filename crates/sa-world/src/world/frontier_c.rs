//! Small world helpers reached through the original by ported code (FRONTIER-2 lane C). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

/// The female ped-type test (stub 0x43A6A0 → .HOODLUM 0x15697D0, cdecl; unnamed): EAX = 1 for ped types 22
/// (prostitute) and 5 (civilian female), else 0.
#[hook(addr = 0x43A6A0, conv = cdecl, name = "thunk_FUN_015697d0", expect = "E9 2B F1 12 01",
       unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn is_female_ped_type(ty: u32) -> u32 {
    u32::from(ty == 0x16 || ty == 5)
}

/// `WaterCreatureManager_c::~WaterCreatureManager_c()` (0x6E3F20, thiscall; `__dtor_g_waterCreatureMan`'s target): the
/// `List_c`s at +0x1A0C and +0x1A00 (0x4A8DE0), then the 0x80 creatures of 0x34 (0x6E3E50) through
/// `eh_vector_destructor_iterator`. The SEH frame is not modelled (FRONTIER-2 turn 2).
#[hook(addr = 0x6E3F20, conv = thiscall, name = "WaterCreatureManager_c::~WaterCreatureManager_c",
       expect = "6A FF 68 84 88 84 00", unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn water_creature_manager_dtor(this: GPtr<u8>) {
    list_dtor(this.byte_add(0x1A0C));
    list_dtor(this.byte_add(0x1A00));
    eh_vector_dtor(this, 0x34, 0x80, 0x6E3E50);
}

/// `CMatrixLinkList::~CMatrixLinkList()` (0x54EB70, thiscall; `__dtor_gMatrixList`'s target): the six sentinel links'
/// `CMatrix` destructors (0x59ACD0), +0x1A4, +0x150, +0xFC, +0xA8, +0x54, +0 in that order. The SEH frame is not
/// modelled (FRONTIER-2 turn 2).
#[hook(addr = 0x54EB70, conv = thiscall, name = "CMatrixLinkList::~CMatrixLinkList", expect = "6A FF 68 FD CA 83 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn matrix_link_list_dtor(this: GPtr<u8>) {
    for off in [0x1A4, 0x150, 0xFC, 0xA8, 0x54, 0] {
        matrix_dtor(this.byte_add(off));
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_FEMALE_PED_TYPE, HOOK_WATER_CREATURE_MANAGER_DTOR, HOOK_MATRIX_LINK_LIST_DTOR];

use sa_core::mem::GPtr;
use sa_core::orig;
use super::common::{g, rd, wr, xi};
use crate::bindings::{find_player_ped, find_player_vehicle, matrix_dtor};
use sa_core::bindings::{eh_vector_dtor, list_dtor};

orig! {
    /// virtual slot +0xC8 (`CVehicle::Fix()`)
    fn v_fix(v: GPtr<u8>) = thiscall vslot 0xC8 => unported;
}

/// `CCheat::HealthCheat()` (0x438D60, cdecl): the player ped's health (+0x540) = player 0's max health (byte 0xB7CEE7,
/// `fild`); in a vehicle (every access through `FindPlayerVehicle(-1, false)` again, as the original): health (+0x4C0)
/// = 1000.0; an automobile (type +0x590 == 0) gets +0x8E4 = 0, a bike (type 9) +0x7BC = 0, and either is `Fix()`ed
/// (vtable +0xC8, tail call); other vehicle types stop after the health. Not in the catalogue (the tail of
/// `CCheat::MoneyArmourHealthCheat`'s extent, CAT-12): a phase B native in turn 1, a hook since it is a catalogue row.
#[sa_core::hook(addr = 0x438D60, conv = cdecl, name = "CCheat::HealthCheat", expect = "51 0F B6 05 E7 CE",
       unverified = "frontier2-c8", route = "L2", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn health_cheat() {
    let h = xi(i32::from(g::<u8>(0xB7CEE7).read())).to_f32();
    wr::<f32>(find_player_ped(-1), 0x540, h);
    if find_player_vehicle(-1, 0).is_null() {
        return;
    }
    wr::<u32>(find_player_vehicle(-1, 0), 0x4C0, 0x447A_0000);
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 0 && rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 9 {
        return;
    }
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) == 0 {
        wr::<u32>(find_player_vehicle(-1, 0), 0x8E4, 0);
    } else {
        wr::<u32>(find_player_vehicle(-1, 0), 0x7BC, 0);
    }
    v_fix(find_player_vehicle(-1, 0));
}

/// The water sync stamp (0x6E76E0, cdecl; gta-reversed `CWaterLevel::SyncWater`): `[0xC228A4] =
/// CTimer::m_snTimeInMilliseconds`. Not in the catalogue (CAT-12), so a phase B native.
#[sa_core::native(addr = 0x6E76E0, conv = cdecl, name = "CWaterLevel::SyncWater")]
pub fn sync_water() {
    g::<u32>(0xC228A4).write(g::<u32>(0xB7CB84).read());
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_SYNC_WATER];

/// The lane C turn-1 natives that became hooks once their bodies were catalogue rows (FRONTIER-2 turn 3).
pub const HOOKS3: &[HookReg] = &[HOOK_HEALTH_CHEAT];
