//! PORT-VEH-3 (batch `veh3-1`): tow hitch / tow bar positions, the trailer's tow link break and
//! the trailer / monster truck constructors. Ported from the 1.0 US bytes; gta-reversed as a reading
//! aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{model_info, rd, wr, x};

orig! {
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): hidden return pointer first
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn audio_add_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CAutomobile::CAutomobile(int32, eVehicleCreatedBy, bool)` (0x6B0A90)
    fn automobile_ctor(v: GPtr<u8>, mi: u32, created_by: u32, setup_susp: u8) -> GPtr<u8> = thiscall 0x6B0A90 => crate::mvpd_v7::automobile_constructor;
    /// `CMonsterTruck::SetupSuspensionLines()` (0x6C7FB0)
    fn monster_truck_setup_suspension_lines(v: GPtr<u8>) = thiscall 0x6C7FB0 => unported;
    /// `CTrailer::SetupSuspensionLines()` (0x6CF1A0)
    fn trailer_setup_suspension_lines(v: GPtr<u8>) = thiscall 0x6CF1A0 => unported;
}

/// `out = (0, y, 0.5 - +0x898)` transformed by the vehicle's matrix (`CMatrix * CVector` into a
/// temporary, then copied).
fn hitch(this: GPtr<u8>, out: GPtr<u8>, y: f32) {
    wr::<u32>(out, 0, 0);
    wr::<f32>(out, 4, y);
    wr::<f32>(out, 8, x(0.5).fsub_m(rd::<f32>(this, 0x898)).to_f32());
    let mut tmp = [0u32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast(), rd::<GPtr<u8>>(this, 0x14), out);
    for k in 0..3 {
        wr::<u32>(out, 4 * k, rd::<u32>(r, 4 * k));
    }
}

/// The colmodel bounding box of the vehicle's model (`ms_modelInfoPtrs[mi] + 0x14`).
fn col_box(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(this, 0x22))), 0x14)
}

/// `CAutomobile::GetTowHitchPos(CVector&, bool, CVehicle*)` (0x6AF1D0): false without the flag;
/// else the point `(0, bbox.max.y - 0.5, 0.5 - height above road)` in world space, true.
#[hook(addr = 0x6AF1D0, conv = thiscall, name = "CAutomobile::GetTowHitchPos", this = "CAutomobile",
       expect = "8A 44 24 08 83 EC 0C",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn automobile_get_tow_hitch_pos(this: GPtr<u8>, out: GPtr<u8>, check: u8, _other: u32) -> u8 {
    if check == 0 {
        return 0;
    }
    let y = x(rd::<f32>(col_box(this), 0x10)).fsub_m(0.5).to_f32();
    hitch(this, out, y);
    1
}

/// `CTrailer::GetTowBarPos(CVector&, bool, CVehicle*)` (0x6CFD60): without the flag only models
/// 606/607 (the baggage trailers); the point `(0, bbox.min.y + 0.05, 0.5 - height above road)` in
/// world space, true.
#[hook(addr = 0x6CFD60, conv = thiscall, name = "CTrailer::GetTowBarPos", this = "CTrailer",
       expect = "8A 44 24 08 83 EC 0C",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn trailer_get_tow_bar_pos(this: GPtr<u8>, out: GPtr<u8>, check: u8, _other: u32) -> u8 {
    let mi = rd::<u16>(this, 0x22);
    if check == 0 && mi != 0x25E && mi != 0x25F {
        return 0;
    }
    let y = x(rd::<f32>(col_box(this), 0x4)).fsub_m(f32::from_bits(0xBD4C_CCCD)).to_f32();
    hitch(this, out, y);
    1
}

/// `CTrailer::BreakTowLink()` (0x6CEFB0): audio event 0x72 on the tractor, both references cleared
/// (the tractor's +0x4C8 and this +0x4C4); a trailer in creation state 10/11 goes to 4 and returns
/// true.
#[hook(addr = 0x6CEFB0, conv = thiscall, name = "CTrailer::BreakTowLink", this = "CTrailer",
       expect = "56 57 8B F9 8B 87 C4 04 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "CleanUpOldReference edits the entities' reference lists")]
pub fn trailer_break_tow_link(this: GPtr<u8>) -> u8 {
    let tractor = rd::<GPtr<u8>>(this, 0x4C4);
    if !tractor.is_null() {
        audio_add_event(tractor.byte_add(0x138), 0x72, 0.0);
    }
    let tractor = rd::<GPtr<u8>>(this, 0x4C4);
    if !tractor.is_null() {
        let t = rd::<GPtr<u8>>(tractor, 0x4C8);
        if !t.is_null() {
            clean_up_old_reference(t, tractor.byte_add(0x4C8));
            wr::<u32>(rd::<GPtr<u8>>(this, 0x4C4), 0x4C8, 0);
        }
        clean_up_old_reference(rd::<GPtr<u8>>(this, 0x4C4), this.byte_add(0x4C4));
        wr::<u32>(this, 0x4C4, 0);
    }
    let s = rd::<u8>(this, 0x36);
    let st = s >> 3;
    if st != 10 && st != 11 {
        return 0;
    }
    wr::<u8>(this, 0x36, (s & 7) | 0x20);
    1
}

/// `CMonsterTruck::CMonsterTruck(int32, eVehicleCreatedBy)` (0x6C8D60): the automobile constructor
/// (no suspension setup), vtable 0x8717D8, the four +0x988 ratios 1.0, SetupSuspensionLines,
/// +0x868 bit 7, vehicle sub-type +0x594 = 1.
/// The entry is `push -1; jmp 0x401443` (a protection fragment that decodes and pushes the SEH handler);
/// 0x409B96 and 0x1562E04 read the jmp bytes at 0x6C8D62 (keycells readers).
#[hook(addr = 0x6C8D60, conv = thiscall, name = "CMonsterTruck::Constructor", this = "CMonsterTruck",
       expect = "6A FF E9 DC 86 D3 FF",
       unverified = "veh4-1", route = "L2", hazards = "entry-protection-read,heap-writes",
       note = "constructs into pool memory; the SEH frame and the entry's protection detour are not reproduced")]
pub fn monster_truck_constructor(this: GPtr<u8>, mi: u32, created_by: u32) -> GPtr<u8> {
    automobile_ctor(this, mi, created_by, 0);
    wr::<u32>(this, 0, 0x0087_17D8);
    for k in 0..4 {
        wr::<u32>(this, 0x988 + 4 * k, 0x3F80_0000);
    }
    monster_truck_setup_suspension_lines(this);
    wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) | 0x80);
    wr::<u32>(this, 0x594, 1);
    this
}

/// `CTrailer::CTrailer(int32, eVehicleCreatedBy)` (0x6D03A0): the automobile constructor (no
/// suspension setup), +0x9E0/+0x9E4/+0x9EC/+0x9F0 = 1.0, vtable 0x871C28, sub-type 11, the baggage
/// trailers (606/607) +0x9EC = -1000, SetupSuspensionLines, creation state 4.
#[hook(addr = 0x6D03A0, conv = thiscall, name = "CTrailer::Constructor", this = "CTrailer",
       expect = "6A FF E9 59 10 D3 FF",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "constructs into pool memory; the SEH frame and the entry's protection detour are not reproduced")]
pub fn trailer_constructor(this: GPtr<u8>, mi: u32, created_by: u32) -> GPtr<u8> {
    automobile_ctor(this, mi, created_by, 0);
    for o in [0x9E0, 0x9E4, 0x9EC, 0x9F0] {
        wr::<u32>(this, o, 0x3F80_0000);
    }
    wr::<u32>(this, 0, 0x0087_1C28);
    wr::<u32>(this, 0x594, 0xB);
    let m = rd::<u16>(this, 0x22);
    if m == 0x25E || m == 0x25F {
        wr::<u32>(this, 0x9EC, 0xC47A_0000);
    }
    trailer_setup_suspension_lines(this);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x20);
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_GET_TOW_HITCH_POS,
    HOOK_TRAILER_GET_TOW_BAR_POS,
    HOOK_TRAILER_BREAK_TOW_LINK,
    HOOK_TRAILER_CONSTRUCTOR,
    HOOK_MONSTER_TRUCK_CONSTRUCTOR,
];
