//! PORT-VEH-3 (batch `veh3-3`): the bus door timer (`SetBusDoorTimer`, `ProcessAutoBusDoors`),
//! `StopNitroEffect`, the special plate handler list and `CBike::SetRemoveAnimFlags`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, xi};

orig! {
    /// virtual `CVehicle::IsDoorMissing(eDoors)` (vtable +0x98)
    fn v_is_door_missing(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// virtual `CVehicle::IsDoorClosed(eDoors)` (vtable +0x90)
    fn v_is_door_closed(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x90 => unported;
    /// virtual `CVehicle::OpenDoor(CPed*, int32, eDoors, float, bool)` (vtable +0x6C)
    fn v_open_door(v: GPtr<u8>, ped: u32, comp: u32, door: u32, ratio: f32, sound: u32) = thiscall vslot 0x6C => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32)` (0x4D6A70)
    fn clump_get_first_association(clump: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*, uint32)` (0x4D6AD0)
    fn get_next_association(assoc: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6AD0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`.
fn now() -> u32 {
    g::<u32>(0xB7_CB84).read()
}

/// `CAutomobile::SetBusDoorTimer(uint32 time, bool retain)` (0x6A3860): start = now (500 ms earlier
/// with `retain`), end = start + max(time, 1000).
#[hook(addr = 0x6A3860, conv = thiscall, name = "CAutomobile::SetBusDoorTimer", this = "CAutomobile",
       expect = "8B 44 24 04 3D E8 03 00 00",
       unverified = "veh3-3", route = "L2", hazards = "none")]
pub fn automobile_set_bus_door_timer(this: GPtr<u8>, time: u32, retain: u8) {
    let t = if time < 1000 { 1000 } else { time };
    let mut s = now();
    if retain != 0 {
        s = s.wrapping_sub(500);
    }
    wr::<u32>(this, 0x874, s);
    wr::<u32>(this, 0x870, s.wrapping_add(t));
}

/// One bus door (`door` 2/3, component 10/8, lock bit in +0x486): open by the remaining time
/// (`1 - (now - end + 500) * 0.002`, unsigned) unless it is already closed (then the end time is
/// restarted and the ratio is 0).
fn bus_door(this: GPtr<u8>, door: u32, comp: u32, lock: u8) {
    if v_is_door_missing(this, door) != 0 || rd::<u8>(this, 0x486) & lock != 0 {
        return;
    }
    let ratio = if v_is_door_closed(this, door) == 0 {
        let d = now().wrapping_sub(rd::<u32>(this, 0x870)).wrapping_add(500);
        let mut f = xi(d as i32);
        if (d as i32) < 0 {
            f = f.fadd_m(4_294_967_296.0);
        }
        f.fmul_m(f32::from_bits(0x3B03_126F)).fsubr_m(1.0).to_f32()
    } else {
        wr::<u32>(this, 0x870, now());
        0.0
    };
    v_open_door(this, 0, comp, door, ratio, 1);
}

/// `CAutomobile::ProcessAutoBusDoors()` (0x6A38A0): during the last 500 ms before the end time
/// (+0x870) both bus doors close progressively; after it, once, both are shut (ratio 0) and the
/// timer cleared.
#[hook(addr = 0x6A38A0, conv = thiscall, name = "CAutomobile::ProcessAutoBusDoors", this = "CAutomobile",
       expect = "51 56 8B F1 8B 86 70 08 00 00",
       unverified = "veh3-3", route = "L2", hazards = "none")]
pub fn automobile_process_auto_bus_doors(this: GPtr<u8>) {
    let end = rd::<u32>(this, 0x870);
    if end > now() {
        if end == 0 || now() <= end.wrapping_sub(500) {
            return;
        }
        bus_door(this, 2, 0xA, 1);
        bus_door(this, 3, 0x8, 4);
        return;
    }
    if rd::<u32>(this, 0x874) == 0 {
        return;
    }
    if v_is_door_missing(this, 2) == 0 && rd::<u8>(this, 0x486) & 1 == 0 {
        v_open_door(this, 0, 0xA, 2, 0.0, 1);
    }
    if v_is_door_missing(this, 3) == 0 && rd::<u8>(this, 0x486) & 4 == 0 {
        v_open_door(this, 0, 0x8, 3, 0.0, 1);
    }
    wr::<u32>(this, 0x874, 0);
    wr::<u32>(this, 0x870, 0);
}

/// `CAutomobile::StopNitroEffect()` (0x6A3E60): kills and clears both exhaust fx systems
/// (+0x978, +0x97C).
#[hook(addr = 0x6A3E60, conv = thiscall, name = "CAutomobile::StopNitroEffect", this = "CAutomobile",
       expect = "56 8B F1 8B 8E 78 09 00 00",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes")]
pub fn automobile_stop_nitro_effect(this: GPtr<u8>) {
    for o in [0x978, 0x97C] {
        let fx = rd::<GPtr<u8>>(this, o);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(this, o, 0);
        }
    }
}

/// `CSpecialPlateHandler::Init()` (0x6F2D10): count (+0xF0) 0, the 15 entries (0x10 each) get
/// car id -1 and an empty text.
#[hook(addr = 0x6F2D10, conv = thiscall, name = "CSpecialPlateHandler::Init", this = "CSpecialPlateHandler",
       expect = "8B C1 33 D2 89 90 F0 00 00 00",
       unverified = "veh3-3", route = "L2", hazards = "none")]
pub fn special_plate_handler_init(this: GPtr<u8>) {
    wr::<u32>(this, 0xF0, 0);
    for k in 0..15isize {
        wr::<u32>(this, 0x10 * k, 0xFFFF_FFFF);
        wr::<u8>(this, 0x10 * k + 4, 0);
    }
}

/// `CSpecialPlateHandler::Remove(int32 index)` (0x6F2DD0): for a valid index with entries, the entry
/// is cleared and, unless it is the last one, replaced by the last entry; the count drops.
#[hook(addr = 0x6F2DD0, conv = thiscall, name = "CSpecialPlateHandler::Remove", this = "CSpecialPlateHandler",
       expect = "56 8B 74 24 08",
       unverified = "veh3-3", route = "L2", hazards = "none")]
pub fn special_plate_handler_remove(this: GPtr<u8>, index: i32) {
    if index < 0 || rd::<u32>(this, 0xF0) == 0 {
        return;
    }
    let e = this.byte_add(index as isize * 0x10);
    wr::<u32>(e, 0, 0xFFFF_FFFF);
    wr::<u8>(e, 4, 0);
    let n = rd::<i32>(this, 0xF0);
    if index < n - 1 {
        let last = this.byte_add(n as isize * 0x10 - 0x10);
        for k in 0..4isize {
            wr::<u32>(e, 4 * k, rd::<u32>(last, 4 * k));
        }
    }
    wr::<i32>(this, 0xF0, rd::<i32>(this, 0xF0).wrapping_sub(1));
}

/// `CBike::SetRemoveAnimFlags(CPed*)` (0x6B5F50): for a ped (entity type +0x36 & 7 == 3) every blend
/// association with flag 0x1000 gets +0x2E bit 2 (delete when finished).
#[hook(addr = 0x6B5F50, conv = thiscall, name = "CBike::SetRemoveAnimFlags", this = "CBike",
       expect = "8B 44 24 04 8A 48 36",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       note = "writes the ped's blend associations (heap)")]
pub fn bike_set_remove_anim_flags(_this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(ped, 0x36) & 7 != 3 {
        return;
    }
    let mut a = clump_get_first_association(rd::<GPtr<u8>>(ped, 0x18), 0x1000);
    while !a.is_null() {
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        a = get_next_association(a, 0x1000);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_SET_BUS_DOOR_TIMER,
    HOOK_AUTOMOBILE_PROCESS_AUTO_BUS_DOORS,
    HOOK_AUTOMOBILE_STOP_NITRO_EFFECT,
    HOOK_SPECIAL_PLATE_HANDLER_INIT,
    HOOK_SPECIAL_PLATE_HANDLER_REMOVE,
    HOOK_BIKE_SET_REMOVE_ANIM_FLAGS,
];
