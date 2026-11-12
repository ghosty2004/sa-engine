//! `CEventVehicleDamage` (vtable 0x85B588; batch `attr-4`): the destructor, `AffectsPed`, `IsCriminalEvent` and
//! `ReportCriminalEvent`. Layout: +0x14 the vehicle, +0x18 the attacker (registered references), +0x1C the weapon type.
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CVehicle::IsPassenger(CPed*)` (0x6D1BD0, `ret 4`)
    fn vehicle_is_passenger(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `FindPlayerPed(int)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedType::PoliceDontCareAboutCrimesAgainstPedType(int)` (0x608A40)
    fn police_dont_care(ty: u32) -> u8 = cdecl 0x608A40 => unported;
    /// `FindPlayerWanted(int)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::RegisterCrime(eCrimeType, const CVector&, uint32 ped, bool)` (0x562410, `ret 0x10`)
    fn wanted_register_crime(w: GPtr<u8>, crime: u32, pos: GPtr<u8>, ped: u32, flag: u32) = thiscall 0x562410 => unported;
    /// `CEvent::IsCriminalEvent()` (vtable slot 0x1C)
    fn is_criminal_event_v(e: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;
}

/// `CEventVehicleDamage::~CEventVehicleDamage()` (0x4B1990, thiscall, SEH frame): vtable 0x85B588,
/// `CleanUpOldReference` on the vehicle (+0x14) and the attacker (+0x18) when set, then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4B1990, conv = thiscall, name = "FUN_004b1990", expect = "6A FF 68 58 A1 83 00",
       unverified = "attr-4", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_vehicle_damage_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B588);
    let v = rd::<GPtr<u8>>(this, 0x14);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0x14));
    }
    let a = rd::<GPtr<u8>>(this, 0x18);
    if !a.is_null() {
        entity_clean_up_old_reference(a, this.byte_add(0x18));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CEventVehicleDamage::AffectsPed(CPed*)` (0x4B1A00, thiscall, `ret 4`): false for a dead ped or without an attacker,
/// and for a ped in a car (+0x46C bit 8, vehicle +0x58C) as its passenger while the car has a driver; otherwise true
/// when the damaged vehicle is the ped's vehicle.
#[hook(addr = 0x4B1A00, conv = thiscall, name = "CEventVehicleDamage::AffectsPed", expect = "56 8B 74 24 08 57 8B F9",
       unverified = "attr-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_vehicle_damage_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 || rd::<u32>(this, 0x18) == 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && vehicle_is_passenger(v, ped) != 0 && rd::<u32>(rd::<GPtr<u8>>(ped, 0x58C), 0x460) != 0 {
            return 0;
        }
    }
    let v = rd::<u32>(this, 0x14);
    (v != 0 && rd::<u32>(ped, 0x58C) == v) as u8
}

/// `CEventVehicleDamage::IsCriminalEvent()` (0x4B1A90, thiscall): an attacker ped (type 3) that is the player, or an
/// attacker vehicle (type 2) whose driver is `FindPlayerPed(-1)`.
#[hook(addr = 0x4B1A90, conv = thiscall, name = "CEventVehicleDamage::IsCriminalEvent", expect = "56 8B F1 8B 4E 18 85 C9",
       unverified = "attr-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_vehicle_damage_is_criminal_event(this: GPtr<u8>) -> u8 {
    let a = rd::<GPtr<u8>>(this, 0x18);
    if a.is_null() {
        return 0;
    }
    if rd::<u8>(a, 0x36) & 7 == 3 && ped_is_player(a) != 0 {
        return 1;
    }
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x18), 0x36) & 7 != 2 {
        return 0;
    }
    let p = find_player_ped(-1);
    (rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), 0x460) == p) as u8
}

/// `CEventVehicleDamage::ReportCriminalEvent(CPed*)` (0x4B50B0, thiscall, `ret 4`): for a criminal event with an
/// attacker, `PoliceDontCareAboutCrimesAgainstPedType(ped type +0x598)`; a cop (type 6) registers crime 8 at the
/// attacker's position with the player's wanted. The flag is written into the low byte of the ped argument's stack
/// slot and that whole dword is pushed (the upper three bytes are the ped pointer's), as in the original.
#[hook(addr = 0x4B50B0, conv = thiscall, name = "CEventVehicleDamage::ReportCriminalEvent", expect = "56 8B F1 8B 06 FF 50 1C",
       unverified = "attr-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_vehicle_damage_report_criminal_event(this: GPtr<u8>, ped: GPtr<u8>) {
    if is_criminal_event_v(this) == 0 || rd::<u32>(this, 0x18) == 0 {
        return;
    }
    let flag = police_dont_care(rd::<u32>(ped, 0x598));
    if rd::<u32>(ped, 0x598) != 6 {
        return;
    }
    let a = rd::<GPtr<u8>>(this, 0x18);
    let m = rd::<GPtr<u8>>(a, 0x14);
    let pos = if m.is_null() { a.byte_add(4) } else { m.byte_add(0x30) };
    let slot = (ped.addr() as u32 & 0xFFFF_FF00) | flag as u32;
    wanted_register_crime(find_player_wanted(-1), 8, pos, ped.addr() as u32, slot);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_VEHICLE_DAMAGE_DTOR,
    HOOK_EVENT_VEHICLE_DAMAGE_AFFECTS_PED,
    HOOK_EVENT_VEHICLE_DAMAGE_IS_CRIMINAL_EVENT,
    HOOK_EVENT_VEHICLE_DAMAGE_REPORT_CRIMINAL_EVENT,
];
