//! PORT-VEH-3 (batch `veh3-3`, SA:MP-executed rows): `CVehicle::KillPedsGettingInVehicle`. Ported
//! from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, e: GPtr<u8>, dmg: u32, wt: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32, eWeaponType, ePedPieceTypes, uint8, bool, bool)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0)
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0)
    fn damage_calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// `CVehicle::KillPedsGettingInVehicle()` (0x6D82F0): every ped not in a vehicle (+0x46C bit 8) and
/// without entity flag +0x1C bit 0 whose active enter-car task (701 as driver, else 700 as passenger)
/// targets this vehicle gets a 1000-damage weapon-51 (explosion) torso hit from itself: a
/// `CEventDamage` (its response computed when it affects the ped, else marked calculated) added to
/// the ped's event group.
#[hook(addr = 0x6D82F0, conv = thiscall, name = "CVehicle::KillPedsGettingInVehicle", this = "CVehicle",
       expect = "6A FF 68 60 87 84 00",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       recs = "driving",
       note = "the event's last argument is (ped flags >> 8) & 0xFFFFFF01 as the original passes it; the SEH frame is not reproduced")]
pub fn vehicle_kill_peds_getting_in_vehicle(this: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(0xB7_4490).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u8>(ped, 0x1C) & 1 != 0 {
            continue;
        }
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        let mut t = tm_find_active_task_by_type(tm, 0x2BD);
        if t.is_null() {
            t = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x2BC);
        }
        let target = if t.is_null() { GPtr::from_addr(0) } else { rd::<GPtr<u8>>(t, 0xC) };
        if target != this {
            continue;
        }
        let mut calc = [0u32; 5];
        let cp: GPtr<u8> = GPtr::from_raw(&mut calc as *mut [u32; 5]).cast();
        damage_calc_ctor(cp, ped, 0x447A_0000, 0x33, 3, 0);
        let mut ev = [0u32; 0x11];
        let ep: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 0x11]).cast();
        let flag = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
        event_damage_ctor(ep, ped, g::<u32>(0xB7_CB84).read(), 0x33, 3, 0, 0, flag);
        if event_damage_affects_ped(ep, ped) != 0 {
            damage_calc_compute_damage_response(cp, ped, ep.byte_add(0x38), 1);
        } else {
            wr::<u8>(ep, 0x42, 1);
        }
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
        event_damage_dtor(ep);
        damage_calc_dtor(cp);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_KILL_PEDS_GETTING_IN_VEHICLE];
