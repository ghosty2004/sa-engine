//! `CTaskComplexPolicePursuit` (the on-foot cop chasing a wanted player): pursuit bookkeeping with `CWanted`,
//! weapon choice, sub-task creation and control (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over
//! gta-reversed). Every call to another catalogued function goes through `orig!` at the original address.
//!
//! `CTaskComplexPolicePursuit` (0x18 bytes): +8 sub-task, +0xC flags (bit 0 "in pursuit", bit 1 "no-police zone
//! seen", bit 2 cleared when no player is pursued), +0x10 the cop, +0x14 the pursued player ped (registered
//! reference).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CWorld::Players[0].m_pPed` / `CWorld::Players[1].m_pPed`
const PLAYER0_PED: usize = 0xB7CD98;
const PLAYER1_PED: usize = 0xB7CF28;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFOS: usize = 0xA9B0C8;
/// The pursuit range² (0x863A3C)
const K_PURSUIT_RANGE_SQ: usize = 0x863A3C;
/// The player vehicle speed² · distance² above which a cop does not join (0x8599D0)
const K_SPEED_DIST: usize = 0x8599D0;
/// 0.0
const K_ZERO: usize = 0x858B50;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230, cdecl)
    fn find_player_wanted(i: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::RemovePursuitCop(CCopPed*)` (0x562C10, `ret 4`)
    fn wanted_remove_pursuit_cop(w: GPtr<u8>, cop: GPtr<u8>) = thiscall 0x562C10 => unported;
    /// `CWanted::RemoveExcessPursuitCops` (0x562C40)
    fn wanted_remove_excess_pursuit_cops(w: GPtr<u8>) = thiscall 0x562C40 => unported;
    /// `CWanted::IsInPursuit(CCopPed*)` (0x562330, `ret 4`; AL)
    fn wanted_is_in_pursuit(w: GPtr<u8>, cop: GPtr<u8>) -> u32 = thiscall 0x562330 => unported;
    /// `CWanted::SetPursuitCop(CCopPed*)` (0x563060, `ret 4`; AL)
    fn wanted_set_pursuit_cop(w: GPtr<u8>, cop: GPtr<u8>) -> u8 = thiscall 0x563060 => unported;
    /// `CCullZones::NoPolice` (0x72DD50, AL)
    fn cull_zones_no_police() -> u8 = cdecl 0x72DD50 => unported;
    /// `CPed::DoWeHaveWeaponAvailable(eWeaponType)` (0x5DF300, `ret 4`)
    fn ped_do_we_have_weapon_available(p: GPtr<u8>, w: u32) -> u8 = thiscall 0x5DF300 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280, `ret 4`)
    fn ped_set_current_weapon(p: GPtr<u8>, w: u32) = thiscall 0x5E6280 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60, cdecl)
    fn weapon_info_get(w: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTask::operator new` is `task_new` (common); `FUN_0048df30`: the constructor of the 0x20-byte task the
    /// pursuit creates for type 0x1A5 and after reporting a vehicle to steal
    fn task_1a5_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DF30 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32 ms, bool looped, bool useAnimIdle, float blend)`
    /// (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::CTaskComplexSeekEntity(CEntity*, int32 time,
    /// int32 scanInterval, float radius, float moveStateRadius, float, bool, bool)` (0x46AC10, `ret 0x20`)
    #[allow(clippy::too_many_arguments)]
    fn seek_entity_standard_ctor(t: GPtr<u8>, e: GPtr<u8>, time: u32, scan: u32, radius: u32, r2: u32, r3: u32, b1: u32, b2: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
    /// `CTaskComplexArrestPed::CTaskComplexArrestPed(CPed*)` (0x68B990, `ret 4`)
    fn arrest_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68B990 => unported;
    /// `CTaskComplexPolicePursuit::SetPursuit(CPed*)` (0x68BBD0, `ret 4`)
    fn o_set_pursuit(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x68BBD0 => unported;
    /// `CTaskComplexPolicePursuit::ClearPursuit(CCopPed*)` (0x68BD90, `ret 4`)
    fn o_clear_pursuit(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x68BD90 => unported;
    /// `CTaskComplexPolicePursuit::PersistPursuit(CCopPed*)` (0x68BDC0, `ret 4`)
    fn o_persist_pursuit(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x68BDC0 => unported;
    /// `CTaskComplexPolicePursuit::SetWeapon(CPed*)` (0x68BAD0, `ret 4`)
    fn o_set_weapon(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x68BAD0 => unported;
    /// `CTaskComplexPolicePursuit::CreateSubTask(int32 type, CPed*)` (0x68D910, `ret 8`)
    fn o_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68D910 => unported;
    /// `CVector::Difference(const CVector& a, const CVector& b)` (0x40FE00, `ret 8`): `this = a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x40FE00 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CEventVehicleToSteal::CEventVehicleToSteal(CVehicle*)` (0x4AF670, `ret 4`)
    fn event_vehicle_to_steal_ctor(e: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF670 => unported;
    /// `CEventVehicleToSteal::~CEventVehicleToSteal` (`FUN_004af700`)
    fn event_vehicle_to_steal_dtor(e: GPtr<u8>) = thiscall 0x4AF700 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// The entity's position: the matrix translation (+0x30 of the matrix at +0x14) or the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The weapon type of the ped's current weapon slot (`m_aWeapons[m_nActiveWeaponSlot].m_eWeaponType`).
fn current_weapon_type(ped: GPtr<u8>) -> u32 {
    let slot = rd::<u8>(ped, 0x718) as i8 as isize;
    rd::<u32>(ped, slot * 0x1C + 0x5A0)
}

/// `(dz·dz + dx·dx) + dy·dy` of `player - ped` (each difference an x87 `fld; fsub` of the f32 coordinates), stored
/// to an f32 local as the original does.
fn dist_sq(player: GPtr<u8>, ped: GPtr<u8>) -> f32 {
    let a = entity_pos(player);
    let b = entity_pos(ped);
    let d = |o: isize| x(rd::<f32>(a, o)).fsub_m(rd::<f32>(b, o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).to_f32()
}

/// The in-vehicle gate of `SetPursuit`: true (the player is not taken) when the player is in a vehicle (+0x46C bit
/// 0x100) whose `(vx·vx + vy·vy) + vz·vz` of the move speed (+0x44) times the distance² is not below [0x8599D0]
/// (or unordered).
fn too_fast(player: GPtr<u8>, d: f32) -> bool {
    if rd::<u32>(player, 0x46C) & 0x100 == 0 {
        return false;
    }
    let v = rd::<GPtr<u8>>(player, 0x58C);
    let (vx, vy, vz) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)), x(rd::<f32>(v, 0x4C)));
    let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fmul_m(d);
    s.fcom(x(g::<f32>(K_SPEED_DIST).read())).test_parity(0x05)
}

/// `CTaskComplexPolicePursuit::SetPursuit(CPed*)` (0x68BBD0, `ret 4`): picks the nearest player (player 1 only when
/// its distance² is below [0x863A3C], player 2 only when below the best so far, both skipped when in a fast
/// vehicle), drops the old reference at +0x14, and when `CWanted::SetPursuitCop` accepts the cop stores and
/// registers the player there (AL = 1); otherwise +0x14 = null and AL = 0.
#[hook(addr = 0x68BBD0, conv = thiscall, name = "CTaskComplexPolicePursuit::SetPursuit", expect = "8B 15 98 CD B7 00",
       unverified = "ai7-7", route = "L2", hazards = "ax-al-return,heap-writes", recs = "police-gang",
       note = "x87 distance gates; registers an entity reference")]
pub fn police_pursuit_set_pursuit(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let mut best = x(g::<f32>(K_PURSUIT_RANGE_SQ).read());
    let mut chosen = GPtr::<u8>::NULL;
    let p0 = g::<GPtr<u8>>(PLAYER0_PED).read();
    if !p0.is_null() {
        let d = dist_sq(p0, ped);
        if !x(d).fcom(x(g::<f32>(K_PURSUIT_RANGE_SQ).read())).test_parity(0x05) && !too_fast(p0, d) {
            chosen = p0;
            best = x(d);
        }
    }
    let p1 = g::<GPtr<u8>>(PLAYER1_PED).read();
    if !p1.is_null() {
        let d = dist_sq(p1, ped);
        if !x(d).fcom(best).test_parity(0x05) && !too_fast(p1, d) {
            chosen = p1;
        }
    }
    let slot = this.byte_add(0x14);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    let wanted = find_player_wanted(-1);
    if !chosen.is_null() && wanted_set_pursuit_cop(wanted, ped) != 0 {
        wr::<GPtr<u8>>(slot, 0, chosen);
        entity_register_reference(chosen, slot);
        return 1;
    }
    wr::<u32>(slot, 0, 0);
    0
}

/// `CTaskComplexPolicePursuit::ClearPursuit(CCopPed*)` (0x68BD90, `ret 4`; `this` unused): with a player ped,
/// removes the cop from the player's pursuit list (tail jump to `CWanted::RemovePursuitCop`).
#[hook(addr = 0x68BD90, conv = thiscall, name = "CTaskComplexPolicePursuit::ClearPursuit", expect = "6A FF E8 79 24 EE FF",
       unverified = "ai7-7", route = "L2", hazards = "heap-writes", recs = "police-gang")]
pub fn police_pursuit_clear_pursuit(_this: GPtr<u8>, cop: GPtr<u8>) {
    if !find_player_ped(-1).is_null() {
        wanted_remove_pursuit_cop(find_player_wanted(-1), cop);
    }
}

/// `CTaskComplexPolicePursuit::PersistPursuit(CCopPed*)` (0x68BDC0, `ret 4`): a dead cop (health +0x540 not above
/// 0.0) leaves the pursuit; in a no-police zone a cop not yet in pursuit marks +0xC bits 0|1 and clears it when its
/// flag 0x800 (+0x46C) is set; leaving such a zone clears bits 0|1 and the pursuit; with wanted level 0 a cop in
/// pursuit (bit 0, not bit 1) clears both bits and the pursuit. Always then `RemoveExcessPursuitCops` and returns
/// `IsInPursuit(cop)` (its whole EAX).
#[hook(addr = 0x68BDC0, conv = thiscall, name = "CTaskComplexPolicePursuit::PersistPursuit", expect = "53 56 57 6A FF",
       unverified = "ai7-7", route = "L2", hazards = "heap-writes", recs = "police-gang")]
pub fn police_pursuit_persist_pursuit(this: GPtr<u8>, cop: GPtr<u8>) -> u32 {
    let wanted = find_player_wanted(-1);
    // `test ah, 0x41; jnp`: health below or equal to 0.0 leaves the pursuit
    let dead = !x(rd::<f32>(cop, 0x540)).fcom(x(g::<f32>(K_ZERO).read())).test_parity(0x41);
    'done: {
        let mut leave = dead;
        if !dead {
            let mut clear_with: Option<u8> = None;
            let no_police = cull_zones_no_police() != 0;
            let flags = rd::<u8>(this, 0xC);
            if no_police && flags & 1 == 0 {
                if rd::<u32>(cop, 0x46C) & 0x800 == 0 {
                    break 'done;
                }
                clear_with = Some(flags | 3);
            } else if rd::<u8>(this, 0xC) & 2 != 0 && cull_zones_no_police() == 0 {
                wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFC);
                leave = true;
            } else {
                if rd::<u32>(wanted, 0x2C) != 0 {
                    break 'done;
                }
                let f = rd::<u8>(this, 0xC);
                if f & 1 == 0 || f & 2 != 0 {
                    break 'done;
                }
                clear_with = Some(f & 0xFC);
            }
            if let Some(f) = clear_with {
                wr::<u8>(this, 0xC, f);
                o_clear_pursuit(this, cop);
                break 'done;
            }
        }
        if leave && !find_player_ped(-1).is_null() {
            wanted_remove_pursuit_cop(find_player_wanted(-1), cop);
        }
    }
    wanted_remove_excess_pursuit_cops(wanted);
    wanted_is_in_pursuit(wanted, cop)
}

/// `CTaskComplexPolicePursuit::SetWeapon(CPed*)` (0x68BAD0, `ret 4`; `this` unused): wanted level above 1 keeps an
/// armed cop's weapon; level 1 (player on foot, cop not in state 0x31) gives the nightstick (3) unless the player's
/// weapon fire type is 1 or 2 or the cop has none; otherwise the shotgun-style fallback: weapon 0x19 if the cop has
/// it, else 0x16. Level 0, or level 1 with the player in a vehicle (+0x568) or the cop in state 0x31: nothing.
#[hook(addr = 0x68BAD0, conv = thiscall, name = "CTaskComplexPolicePursuit::SetWeapon", expect = "56 6A FF E8 58 27 EE FF",
       unverified = "ai7-7", route = "L2", hazards = "heap-writes", recs = "police-gang")]
pub fn police_pursuit_set_weapon(_this: GPtr<u8>, ped: GPtr<u8>) {
    let level = rd::<i32>(find_player_wanted(-1), 0x2C);
    let fallback = if level > 1 {
        if current_weapon_type(ped) != 0 {
            return;
        }
        true
    } else {
        if level != 1 {
            return;
        }
        if rd::<u32>(find_player_ped(-1), 0x568) != 0 || rd::<u32>(ped, 0x530) == 0x31 {
            return;
        }
        if ped_do_we_have_weapon_available(ped, 3) == 0 {
            true
        } else {
            let p = find_player_ped(-1);
            if rd::<u32>(weapon_info_get(current_weapon_type(p), 1), 0) == 1 {
                true
            } else {
                let p = find_player_ped(-1);
                rd::<u32>(weapon_info_get(current_weapon_type(p), 1), 0) == 2
            }
        }
    };
    if !fallback {
        ped_set_current_weapon(ped, 3);
    } else if ped_do_we_have_weapon_available(ped, 0x19) != 0 {
        ped_set_current_weapon(ped, 0x19);
    } else {
        ped_set_current_weapon(ped, 0x16);
    }
}

/// `CTaskComplexPolicePursuit::CreateSubTask(int32 type, CPed*)` (0x68D910, `ret 8`; entry `nop; jmp` to the island
/// 0x401DE4, which only loads `fs:[0]` for the SEH frame): 0x1A5 → the 0x20-byte task of `FUN_0048df30`; 0xCB →
/// `CTaskSimpleStandStill(0, false, false, 8.0)`; 0x38B → `CTaskComplexSeekEntity<Standard>(ped's vehicle +0x58C,
/// 50000, 1000, its model's col radius + [0x858624], [0x859E30], [0x859E34], true, true)`; 0x44D (with a pursued
/// ped at +0x14) → `CTaskComplexArrestPed(+0x14)`; anything else (0x516 included) or a failed allocation → null.
/// Not hookable: protection code reads the patched bytes (0x406A63 `sbb edx, [0x68D911]`); phase B only.
#[sa_core::native(addr = 0x68D910, conv = thiscall, name = "CTaskComplexPolicePursuit::CreateSubTask")]
pub fn police_pursuit_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x1A5 => {
            let t = task_new(0x20);
            if t.is_null() { t } else { task_1a5_ctor(t) }
        }
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 0, 0, 0, 0x4100_0000) }
        }
        0x38B => {
            let t = task_new(0x4C);
            if t.is_null() {
                return t;
            }
            let veh = rd::<GPtr<u8>>(ped, 0x58C);
            let mi = g::<GPtr<u8>>(MODEL_INFOS + rd::<u16>(veh, 0x22) as i16 as isize as usize * 4).read();
            let col = rd::<GPtr<u8>>(mi, 0x14);
            let radius = x(rd::<f32>(col, 0x24)).fadd_m(g::<f32>(0x858624).read()).to_f32();
            seek_entity_standard_ctor(t, veh, 0xC350, 0x3E8, radius.to_bits(), g::<u32>(0x859E30).read(),
                                      g::<u32>(0x859E34).read(), 1, 1)
        }
        0x44D => {
            if rd::<u32>(this, 0x14) == 0 {
                return GPtr::NULL;
            }
            let t = task_new(0x24);
            if t.is_null() { t } else { arrest_ped_ctor(t, rd::<GPtr<u8>>(this, 0x14)) }
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexPolicePursuit::CreateFirstSubTask(CPed*)` (0x6908E0, `ret 4`): stores the cop (+0x10); with a
/// pursuit (`SetPursuit`) the arrest task 0x44D, otherwise clears flag bit 2 (+0xC) and asks for 0x516 (null).
#[hook(addr = 0x6908E0, conv = thiscall, name = "CTaskComplexPolicePursuit::CreateFirstSubTask", expect = "56 57 8B 7C 24 0C",
       unverified = "ai7-7", route = "L2", hazards = "heap-writes", recs = "police-gang")]
pub fn police_pursuit_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<GPtr<u8>>(this, 0x10, ped);
    if o_set_pursuit(this, ped) != 0 {
        return o_create_sub_task(this, 0x44D, ped);
    }
    wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFB);
    o_create_sub_task(this, 0x516, ped)
}

/// `CTaskComplexPolicePursuit::ControlSubTask(CPed*)` (0x690920, `ret 4`): unless the cop is still in pursuit
/// (`PersistPursuit`): a cop flagged in pursuit draws weapon 0x16 once (sets +0x46C bit 0x200000) and goes for the
/// arrest (0x44D); while arresting, a cop in a vehicle gives up (0x516), one without a vehicle (+0x58C) stands
/// (0xCB), with wanted level 0 reports its vehicle as one to steal, else seeks its vehicle when it is nearer than
/// [0x858FE8] (0x38B) or stands. `SetWeapon` runs when the sub-task or the choice is 0x44D. A new task replaces the
/// sub-task only when it agrees to abort (`MakeAbortable(ped, 1, null)`).
#[hook(addr = 0x690920, conv = thiscall, name = "CTaskComplexPolicePursuit::ControlSubTask", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "ai7-7", route = "L2", hazards = "heap-writes", recs = "police-gang",
       note = "a stack CEventVehicleToSteal (Rust local) is added to the cop's event group")]
pub fn police_pursuit_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut choice: i32 = 0xC8;
    let wanted = find_player_wanted(-1);
    if o_persist_pursuit(this, ped) == 0 {
        let mut decided = false;
        if rd::<u8>(this, 0xC) & 1 != 0 {
            let armed = rd::<u32>(ped, 0x46C) & 0x20_0000 != 0 && current_weapon_type(ped) == 0x16;
            if !armed {
                ped_set_current_weapon(ped, 0x16);
                wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x20_0000);
                choice = 0x44D;
                decided = true;
            }
        }
        if !decided && v_task_type(rd::<GPtr<u8>>(this, 8)) == 0x44D {
            choice = if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                0x516
            } else {
                let veh = rd::<GPtr<u8>>(ped, 0x58C);
                if veh.is_null() {
                    0xCB
                } else if rd::<u32>(wanted, 0x2C) == 0 {
                    0x2BD
                } else {
                    let mut v = [0f32; 3];
                    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
                    vector_difference(vp, entity_pos(veh), entity_pos(ped));
                    let m = X87::from_f64_exact(vector_magnitude_sqr(vp));
                    if m.fcom(x(g::<f32>(0x858FE8).read())).test_parity(0x05) { 0xCB } else { 0x38B }
                }
            };
        }
    }
    if v_task_type(rd::<GPtr<u8>>(this, 8)) == 0x44D || choice == 0x44D {
        o_set_weapon(this, ped);
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if choice == 0xC8 || v_make_abortable(sub, ped, 1, 0) == 0 {
        return sub;
    }
    if choice != 0x2BD {
        return o_create_sub_task(this, choice, ped);
    }
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_vehicle_to_steal_ctor(e, rd::<GPtr<u8>>(ped, 0x58C));
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    let t = task_new(0x20);
    let r = if t.is_null() { t } else { task_1a5_ctor(t) };
    event_vehicle_to_steal_dtor(e);
    r
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_POLICE_PURSUIT_SET_PURSUIT,
    HOOK_POLICE_PURSUIT_CLEAR_PURSUIT,
    HOOK_POLICE_PURSUIT_PERSIST_PURSUIT,
    HOOK_POLICE_PURSUIT_SET_WEAPON,
    HOOK_POLICE_PURSUIT_CREATE_FIRST_SUB_TASK,
    HOOK_POLICE_PURSUIT_CONTROL_SUB_TASK,
];

/// The `#[native]` registrations of this module (entries phase A cannot hook).
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_POLICE_PURSUIT_CREATE_SUB_TASK];
