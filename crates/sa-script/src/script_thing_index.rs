//! Script handle lookups (batch `mvpl-script-1`): `CTheScripts::GetActualScriptThingIndex` and the two vehicle model
//! block lists. Ported from the 1.0 US bytes (gta-reversed `TheScripts.cpp` as a reading aid; the bytes win).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0; creates the instance on first use)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
}

fn rd8(a: u32) -> u8 {
    GPtr::<u8>::from_addr(a as usize).read()
}
fn rd16(a: u32) -> u16 {
    GPtr::<u16>::from_addr(a as usize).read()
}

/// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0, cdecl). `ref == -1` → -1. Else `idx =
/// ref & 0xFFFF`, `id = ref >> 16` (unsigned); the slot is live when its id word equals `id` and its active byte is
/// set, then `idx` is returned, else -1. Per type (jump table 0x483B08; above 8 → -1), `(active byte, id word)`:
/// 0 spheres `0xA91268 + idx*0x18` (+0, +2); 1 effect systems `0xA44110 + idx*8` (+0, +2); 2 search lights
/// `0xA94D68 + idx*0x7C` (+0, +4); 3 checkpoints `0xA44070 + idx*8` (+0, +2); 4 sequence tasks `0xA43F68 + idx*4`
/// (+0, +2); 5 fires `gFireManager 0xB71F80 + idx*0x28` (byte +0 bit 1, word +2); 6 scripted 2D effects (byte
/// `0xC3A1A0 + idx`, word `0xC3A120 + idx*2`); 7 decision makers (`GetInstance()` before each read; word `0xC0AFF4 +
/// idx*2`, byte `0xC0B01C + idx`); 8 ped groups (word `0xC098D0 + idx*2`, byte `0xC098E0 + idx`).
#[hook(addr = 0x4839a0, conv = cdecl, name = "CTheScripts::GetActualScriptThingIndex", expect = "57 8B 7C 24 08",
       unverified = "mvpl-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 {
    if r == -1 {
        return -1;
    }
    let idx = (r as u32) & 0xFFFF;
    let id = u32::from(((r as u32) >> 16) as u16);
    let live = match ty as u8 {
        0 => {
            let e = 0xA91268 + idx * 0x18;
            u32::from(rd16(e + 2)) == id && rd8(e) != 0
        }
        1 => {
            let e = 0xA44110 + idx * 8;
            u32::from(rd16(e + 2)) == id && rd8(e) != 0
        }
        2 => {
            let e = 0xA94D68 + idx * 0x7C;
            u32::from(rd16(e + 4)) == id && rd8(e) != 0
        }
        3 => {
            let e = 0xA44070 + idx * 8;
            u32::from(rd16(e + 2)) == id && rd8(e) != 0
        }
        4 => {
            let e = 0xA43F68 + idx * 4;
            u32::from(rd16(e + 2)) == id && rd8(e) != 0
        }
        5 => {
            let e = 0xB71F80 + idx * 0x28;
            u32::from(rd16(e + 2)) == id && rd8(e) & 2 != 0
        }
        6 => u32::from(rd16(0xC3A120 + idx * 2)) == id && rd8(0xC3A1A0 + idx) != 0,
        7 => {
            decision_maker_types_get_instance();
            if u32::from(rd16(0xC0AFF4 + idx * 2)) != id {
                false
            } else {
                decision_maker_types_get_instance();
                rd8(0xC0B01C + idx) != 0
            }
        }
        8 => u32::from(rd16(0xC098D0 + idx * 2)) == id && rd8(0xC098E0 + idx) != 0,
        _ => false,
    };
    if live { idx as i32 } else { -1 }
}

/// `CTheScripts::HasCarModelBeenSuppressed(int32 model)` (0x46A810, cdecl): `model` in the 40 dwords of
/// `SuppressedVehicleModels` (0xA44940) → AL = 1, else 0. AL only: the upper EAX bytes are the caller's.
#[hook(addr = 0x46a810, conv = cdecl, name = "CTheScripts::HasCarModelBeenSuppressed", expect = "8B 4C 24 04 32 C0",
       unverified = "mvpl-script-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "AL result; the original leaves the caller's upper EAX bytes")]
pub fn has_car_model_been_suppressed(model: i32) -> u8 {
    u8::from((0..40u32).any(|i| GPtr::<i32>::from_addr((0xA44940 + 4 * i) as usize).read() == model))
}

/// `CTheScripts::HasVehicleModelBeenBlockedByScript(int32 model)` (0x46A890, cdecl): `model` in the 20 dwords of
/// `VehicleModelsBlockedByScript` (0xA448F0) → AL = 1, else 0. AL only: the upper EAX bytes are the caller's.
#[hook(addr = 0x46a890, conv = cdecl, name = "CTheScripts::HasVehicleModelBeenBlockedByScript",
       expect = "8B 4C 24 04 32 C0", unverified = "mvpl-script-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk", note = "AL result; the original leaves the caller's upper EAX bytes")]
pub fn has_vehicle_model_been_blocked_by_script(model: i32) -> u8 {
    u8::from((0..20u32).any(|i| GPtr::<i32>::from_addr((0xA448F0 + 4 * i) as usize).read() == model))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_ACTUAL_SCRIPT_THING_INDEX,
    HOOK_HAS_CAR_MODEL_BEEN_SUPPRESSED,
    HOOK_HAS_VEHICLE_MODEL_BEEN_BLOCKED_BY_SCRIPT,
];
