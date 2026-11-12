//! Event destructors and one event constructor that SA:MP executes but the catalogue left unnamed (UNK-game rows of
//! `docs/samp/native-unported.csv`, batch `samp-unk-1`). Names proposed from the vtables they store (the
//! `data-tables` vtable names of gta-reversed) and their callers; see `docs/samp/native-unported-names.csv`.
//! Ported from the 1.0 US bytes. The SEH frame each destructor sets up has no other effect. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEventGunShot::~CEventGunShot` (0x4AC720), the tail of `CEventGunShotWhizzedBy`'s destructor
    fn event_gun_shot_dtor(this: GPtr<u8>) = thiscall 0x4AC720 => event_gun_shot_dtor_4ac720;
}

/// `CEvent`'s vtable, stored last by every event destructor.
const CEVENT_VTABLE: u32 = 0x85_AB38;

/// The entity reference at `off`: when set, `CleanUpOldReference(&this[off])` on it.
fn release_ref(this: GPtr<u8>, off: isize) {
    let e = rd::<GPtr<u8>>(this, off);
    if !e.is_null() {
        clean_up_old_reference(e, this.byte_add(off));
    }
}

/// The shared body: vtable `vt`, release the references at `offs` in order, then `CEvent`'s vtable.
fn ref_event_dtor(this: GPtr<u8>, vt: u32, offs: &[isize]) {
    wr::<u32>(this, 0, vt);
    for &o in offs {
        release_ref(this, o);
    }
    wr::<u32>(this, 0, CEVENT_VTABLE);
}

/// `FUN_004ac720` (0x4AC720), proposed `CEventGunShot::~CEventGunShot` (thiscall): vtable 0x85ABE0
/// (`CEventGunShot`), releases the firing entity at +0x14, then `CEvent`'s vtable 0x85AB38. Also the tail of
/// `CEventGunShotWhizzedBy`'s destructor 0x4B2070.
#[hook(addr = 0x4AC720, conv = thiscall, name = "FUN_004ac720", expect = "6A FF 68 68 99 83 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "anim-input")]
pub fn event_gun_shot_dtor_4ac720(this: GPtr<u8>) {
    ref_event_dtor(this, 0x85_ABE0, &[0x14]);
}

/// `FUN_004b13c0` (0x4B13C0), proposed `CEventInWater::~CEventInWater` (thiscall): only stores `CEvent`'s
/// vtable; called by the deleting destructor of vtable 0x85B4C8 and by `CPed::ProcessBuoyancy`'s stack event.
#[hook(addr = 0x4B13C0, conv = thiscall, name = "FUN_004b13c0", expect = "C7 01 38 AB 85 00 C3",
       unverified = "samp-unk-1", route = "L2", hazards = "preserved-regs", preserve = "ecx")]
pub fn event_in_water_dtor_4b13c0(this: GPtr<u8>) {
    wr::<u32>(this, 0, CEVENT_VTABLE);
}

/// `FUN_004b17f0` (0x4B17F0), proposed `CEventCopCarBeingStolen::~CEventCopCarBeingStolen` (thiscall): vtable
/// 0x85B548, releases +0x0C (the thief) then +0x10 (the vehicle), then `CEvent`'s vtable.
#[hook(addr = 0x4B17F0, conv = thiscall, name = "FUN_004b17f0", expect = "6A FF 68 18 A1 83 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes")]
pub fn event_cop_car_being_stolen_dtor_4b17f0(this: GPtr<u8>) {
    ref_event_dtor(this, 0x85_B548, &[0x0C, 0x10]);
}

/// `FUN_004b1f10` (0x4B1F10), proposed `CEventFireNearby::CEventFireNearby(const CVector& pos)` (thiscall,
/// `ret 4`): the `CEventEditableResponse` base fields (+4 = 0, byte +8 = 0, byte +0xC = 1, word +0xE = 200,
/// word +0x10 = -1), vtable 0x85B6E8, the position copied to +0x14..+0x1C. Returns `this`.
#[hook(addr = 0x4B1F10, conv = thiscall, name = "FUN_004b1f10", expect = "8B 54 24 04 8B C1",
       unverified = "samp-unk-1", route = "L2", hazards = "none")]
pub fn event_fire_nearby_ctor_4b1f10(this: GPtr<u8>, pos: GPtr<u32>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<u32>(this, 0, 0x85_B6E8);
    for i in 0..3 {
        wr::<u32>(this, 0x14 + 4 * i as isize, pos.wrapping_index(i).read());
    }
    this
}

/// `FUN_004b2070` (0x4B2070), proposed `CEventGunShotWhizzedBy::~CEventGunShotWhizzedBy` (thiscall): vtable
/// 0x85B730, then tail-jumps to `CEventGunShot`'s destructor 0x4AC720.
#[hook(addr = 0x4B2070, conv = thiscall, name = "FUN_004b2070", expect = "C7 01 30 B7 85 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes")]
pub fn event_gun_shot_whizzed_by_dtor_4b2070(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_B730);
    event_gun_shot_dtor(this);
}

/// `FUN_004b2210` (0x4B2210), proposed `CEventAreaCodes::~CEventAreaCodes` (thiscall): vtable 0x85B7C0,
/// releases the ped at +0x0C, then `CEvent`'s vtable.
#[hook(addr = 0x4B2210, conv = thiscall, name = "FUN_004b2210", expect = "6A FF 68 58 A2 83 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes")]
pub fn event_area_codes_dtor_4b2210(this: GPtr<u8>) {
    ref_event_dtor(this, 0x85_B7C0, &[0x0C]);
}

/// `FUN_004b26a0` (0x4B26A0), proposed `CEventDanger::~CEventDanger` (thiscall): vtable 0x85B848, releases the
/// entity at +0x14, then `CEvent`'s vtable.
#[hook(addr = 0x4B26A0, conv = thiscall, name = "FUN_004b26a0", expect = "6A FF 68 D8 A2 83 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes")]
pub fn event_danger_dtor_4b26a0(this: GPtr<u8>) {
    ref_event_dtor(this, 0x85_B848, &[0x14]);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_GUN_SHOT_DTOR_4AC720,
    HOOK_EVENT_IN_WATER_DTOR_4B13C0,
    HOOK_EVENT_COP_CAR_BEING_STOLEN_DTOR_4B17F0,
    HOOK_EVENT_FIRE_NEARBY_CTOR_4B1F10,
    HOOK_EVENT_GUN_SHOT_WHIZZED_BY_DTOR_4B2070,
    HOOK_EVENT_AREA_CODES_DTOR_4B2210,
    HOOK_EVENT_DANGER_DTOR_4B26A0,
];
