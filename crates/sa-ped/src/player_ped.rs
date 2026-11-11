//! `CPlayerPed` small helpers (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `PlayerPed.cpp` and `docs/re/specs/PED/CPlayerPed*.md` as reading aids).
//!
//! `CPlayerPed` adds +0x79C (3rd-person mouse target) and +0x7A0 to `CPed`; `CPlayerPedData` (+0x480):
//! +0 wanted, +0x18 sprint energy, +0x1C button sprint time, +0x20 chosen weapon slot, +0x2C, +0x34 flags,
//! +0x38 player group, +0x3C adrenaline end time, +0x44 breath, +0x50 move direction, +0x54, +0x70.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// `CPlayerPed` group members' "never leave" flags saved by `MakePlayerGroupDisappear`
    pub static GROUP_MEMBER_FLAGS @ 0xC0BC08: [u8; 7] = [0; 7];
    /// button-sprint threshold (`GetButtonSprintResults`)
    pub static SPRINT_THRESHOLD @ 0x8D2458: f32 = 5.0;
    /// per-sprint-type parameters (3 x 8 floats; `GetButtonSprintResults` reads the first of each)
    pub static SPRINT_PARAMS @ 0x8D247C: [f32; 24] = [
        0.3, 4.0, 0.7, 0.2, 5.0, 10.0, 0.0, 0.4,
        1.0, 4.0, 0.7, 0.2, 5.0, 10.0, 1.0, 0.3,
        0.3, 4.0, 0.7, 0.2, 5.0, 10.0, 0.0, 0.0,
    ];
    /// `GetWeaponRadiusOnScreen` tuning: shotgun factor, accuracy factor, range numerator
    pub static RADIUS_SHOTGUN @ 0x8D24E0: f32 = 1.0;
    pub static RADIUS_ACC @ 0x8D24E4: f32 = 0.5;
    pub static RADIUS_RANGE @ 0x8D24E8: f32 = 15.0;
}

/// `CCamera TheCamera`
const THE_CAMERA: usize = 0xB6F028;

orig! {
    /// `CWanted::SetWantedLevel` (0x562470)
    pub fn wanted_set_wanted_level(w: GPtr<u8>, l: i32) = thiscall 0x562470 => crate::wanted::set_wanted_level;
    /// `CWanted::SetWantedLevelNoDrop` (0x562570)
    pub fn wanted_set_wanted_level_no_drop(w: GPtr<u8>, l: i32) = thiscall 0x562570 => crate::wanted::set_wanted_level_no_drop;
    /// `CWanted::CheatWantedLevel` (0x562540)
    pub fn wanted_cheat_wanted_level(w: GPtr<u8>, l: i32) = thiscall 0x562540 => crate::wanted::cheat_wanted_level;
    /// `CCamera::ClearPlayerWeaponMode` (0x50AB10)
    pub fn camera_clear_player_weapon_mode(cam: GPtr<u8>) = thiscall 0x50AB10 => unported;
    /// `CWeaponEffects::ClearCrossHair(int32)` (0x742C60)
    pub fn clear_cross_hair(id: i32) = cdecl 0x742C60 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0)
    pub fn group_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    pub fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CWeapon::CanBeUsedFor2Player()` (0x73DEF0)
    pub fn weapon_can_be_used_for_2_player(w: GPtr<u8>) -> u8 = thiscall 0x73DEF0 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    pub fn are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0)
    pub fn get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `std::max<float>`-like instance (0x420800): `a` when `a > b` (ordered), else `b`
    pub fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => max_f_port;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// scalar deleting destructor (vtable slot 0)
    pub fn entity_vdelete(e: GPtr<u8>, flag: i32) = thiscall vslot 0x0 => unported;
    /// `CPhysical::GetLightingTotal` (0x544850)
    pub fn get_lighting_total(e: GPtr<u8>) -> f64 = thiscall 0x544850 => unported;
    /// `CPlayerPed::TellGroupToStartFollowingPlayer(bool, bool, bool)` (0x60A1D0)
    pub fn tell_group_to_start_following_player(p: GPtr<u8>, a: u32, b: u32, c: u32) = thiscall 0x60A1D0 => unported;
    /// `CPlayerPed::MakeChangesForNewWeapon(eWeaponType)` (0x60B460)
    pub fn make_changes_for_new_weapon(p: GPtr<u8>, ty: i32) = thiscall 0x60B460 => unported;
    /// `CPlayerPed::PlayerWantsToAttack` (0x60CC50)
    pub fn player_wants_to_attack(p: GPtr<u8>) = thiscall 0x60CC50 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPed::IsPedInControl` (0x5E3960)
    pub fn ped_is_in_control(p: GPtr<u8>) -> u8 = thiscall 0x5E3960 => crate::ped::is_ped_in_control;
    /// `CTask::GetTaskType` (vtable slot 0x10)
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    pub fn get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650)
    pub fn get_event_of_type(eg: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60)
    pub fn ped_get_weapon_skill(p: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => crate::ped::get_weapon_skill_type;
    /// `CPed::GetWeaponSkill()` (0x5E6580)
    pub fn ped_get_weapon_skill_current(p: GPtr<u8>) -> u8 = thiscall 0x5E6580 => crate::ped::get_weapon_skill_current;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    pub fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571A00 => unported;
}

pub fn max_f_port(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_any(0x41) { f64::from(b) } else { f64::from(a) }
}

fn pdata(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x480)
}

fn wanted_of(p: GPtr<u8>) -> GPtr<u8> {
    let d = pdata(p);
    if d.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(d, 0) }
}

/// `CPlayerPed::GetWantedLevel` (stub 0x41BE60 → 0x156D300): `m_pPlayerData->m_pWanted->m_nWantedLevel`;
/// without player data it reads address 0x2C (crash, as the original).
#[hook(addr = 0x41BE60, conv = thiscall, name = "CPlayerPed::GetWantedLevel", expect = "E9 9B 14 15 01", this = "CPlayerPed")]
pub fn get_wanted_level(this: GPtr<u8>) -> i32 {
    rd::<i32>(wanted_of(this), 0x2C)
}

/// `CPlayerPed::SetWantedLevel(int32)` (0x609F10): tail call on the wanted (null without player data).
#[hook(addr = 0x609F10, conv = thiscall, name = "CPlayerPed::SetWantedLevel", expect = "8B 81 80 04 00 00", this = "CPlayerPed")]
pub fn set_wanted_level(this: GPtr<u8>, l: i32) {
    wanted_set_wanted_level(wanted_of(this), l);
}

/// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
#[hook(addr = 0x609F30, conv = thiscall, name = "CPlayerPed::SetWantedLevelNoDrop", expect = "8B 81 80 04 00 00", this = "CPlayerPed")]
pub fn set_wanted_level_no_drop(this: GPtr<u8>, l: i32) {
    wanted_set_wanted_level_no_drop(wanted_of(this), l);
}

/// `CPlayerPed::CheatWantedLevel(int32)` (0x609F50)
#[hook(addr = 0x609F50, conv = thiscall, name = "CPlayerPed::CheatWantedLevel", expect = "8B 81 80 04 00 00", this = "CPlayerPed")]
pub fn cheat_wanted_level(this: GPtr<u8>, l: i32) {
    wanted_cheat_wanted_level(wanted_of(this), l);
}

/// `CPlayerPed::PedCanBeTargettedVehicleWise(CPed*)` (0x609D90, cdecl, AL): on foot, or in a vehicle of
/// type 9 or with +0x42D bit 2.
#[hook(addr = 0x609D90, conv = cdecl, name = "CPlayerPed::PedCanBeTargettedVehicleWise", expect = "8B 44 24 04 8B 88 6C 04 00 00")]
pub fn ped_can_be_targetted_vehicle_wise(ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        return 1;
    }
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    if v.is_null() {
        return 0;
    }
    (rd::<i32>(v, 0x590) == 9 || rd::<u8>(v, 0x42D) & 4 != 0) as u8
}

/// `CPlayerPed::Busted` (0x609EF0): wanted chaos level = 0 (writes address 0 without player data).
#[hook(addr = 0x609EF0, conv = thiscall, name = "CPlayerPed::Busted", expect = "8B 81 80 04 00 00", this = "CPlayerPed")]
pub fn busted(this: GPtr<u8>) {
    wr::<u32>(wanted_of(this), 0, 0);
}

/// `CPlayerPed::GetPlayerInfoForThisPlayerPed` (0x609FF0): `&CWorld::Players[i]` whose ped is `this`, else
/// null.
#[hook(addr = 0x609FF0, conv = thiscall, name = "CPlayerPed::GetPlayerInfoForThisPlayerPed", expect = "33 C0 BA 98 CD B7 00", this = "CPlayerPed")]
pub fn get_player_info_for_this_player_ped(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..2 {
        let p = g::<u8>(PLAYERS + i * PLAYER_INFO_SIZE);
        if rd::<GPtr<u8>>(p, 0) == this {
            return p;
        }
    }
    GPtr::NULL
}

/// `CPlayerPed::AnnoyPlayerPed(bool)` (0x60A040, `ret 4`): temper (+0x25 of the ped stat, signed) +1 below
/// 0x34; with the flag, +1 below 0x37, else reset to 0x2E.
#[hook(addr = 0x60A040, conv = thiscall, name = "CPlayerPed::AnnoyPlayerPed", expect = "8B 89 9C 05 00 00", this = "CPlayerPed")]
pub fn annoy_player_ped(this: GPtr<u8>, b: u32) {
    let st = rd::<GPtr<u8>>(this, 0x59C);
    let t = rd::<i8>(st, 0x25);
    if t < 0x34 {
        wr::<i8>(st, 0x25, t.wrapping_add(1));
    } else if b as u8 != 0 {
        if t < 0x37 {
            wr::<i8>(st, 0x25, t.wrapping_add(1));
        } else {
            wr::<i8>(st, 0x25, 0x2E);
        }
    }
}

/// `CPlayerPed::ClearAdrenaline` (0x60A070): with adrenaline active (+0x34 bit 1) and an end time, clears
/// it and resets the time scale to 1.0.
#[hook(addr = 0x60A070, conv = thiscall, name = "CPlayerPed::ClearAdrenaline", expect = "8B 81 80 04 00 00", this = "CPlayerPed")]
pub fn clear_adrenaline(this: GPtr<u8>) {
    let d = pdata(this);
    if rd::<u8>(d, 0x34) & 2 != 0 && rd::<u32>(d, 0x3C) != 0 {
        wr::<u32>(d, 0x3C, 0);
        sa_core::timer::TIME_SCALE.set(1.0);
    }
}

/// `CPlayerPed::GetButtonSprintResults(eSprintType)` (0x60A820, `ret 4`, ST0): above the threshold
/// ([0x8D2458]): `max(time / threshold - 1, 0) * mult[type] + 1`; otherwise 1.0 for a positive time, else 0.
// Not registered: caller 0x60B430 reads EDX after the call (needs `preserve = "edx"`), and a preserving
// hook needs an L1 spec comparing EDX, which the hosted L1 harness cannot run for a function that reads
// a pointer field (`this->m_pPlayerData`, 64-bit host). Register once the harness supports it.
pub fn get_button_sprint_results(this: GPtr<u8>, ty: i32) -> X87 {
    let d = pdata(this);
    let e = rd::<f32>(d, 0x1C);
    let thr = SPRINT_THRESHOLD.get();
    if !x(e).fcom(x(thr)).test_any(0x41) {
        let mut v = x(e).fdiv_m(thr).fsub_m(1.0);
        if !x(0.0).fcom(v).test_any(0x41) {
            v = x(0.0);
        }
        let m = SPRINT_PARAMS.ptr().cast::<u8>().byte_add((ty as isize).wrapping_shl(5)).cast::<f32>().read();
        return v.fmul_m(m).fadd_m(1.0);
    }
    if !x(e).fcom(x(0.0)).test_any(0x41) { x(1.0) } else { x(0.0) }
}

/// `CPlayerPed::GetPadFromPlayer` (0x609560): `CPad::GetPad(0/1)` for player types 0/1, else null.
#[hook(addr = 0x609560, conv = thiscall, name = "CPlayerPed::GetPadFromPlayer", expect = "8B 81 98 05 00 00", this = "CPlayerPed")]
pub fn get_pad_from_player(this: GPtr<u8>) -> GPtr<u8> {
    match rd::<i32>(this, 0x598) {
        0 => get_pad(0),
        1 => get_pad(1),
        _ => GPtr::NULL,
    }
}

/// `CPlayerPed::CanPlayerStartMission` (0x609590, AL)
#[hook(addr = 0x609590, conv = thiscall, name = "CPlayerPed::CanPlayerStartMission", expect = "A0 B0 A8 96 00", this = "CPlayerPed")]
pub fn can_player_start_mission(this: GPtr<u8>) -> u8 {
    if g::<u8>(0x96A8B0).read() != 0 || is_coop_game_going_on() != 0 {
        return 0;
    }
    if ped_is_in_control(this) == 0 && rd::<i32>(this, 0x530) != 0x32 {
        return 0;
    }
    let intel = rd::<GPtr<u8>>(this, 0x47C);
    if rd::<u32>(intel, 4) != 0 || rd::<u32>(intel, 0xC) != 0 {
        return 0;
    }
    let t = rd::<GPtr<u8>>(intel, 0x10);
    if !t.is_null() && task_get_type(t) != 0x2C5 {
        return 0;
    }
    if !get_task_secondary(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), 0).is_null() {
        return 0;
    }
    if crate::common::ped_is_alive(this) as u8 == 0 {
        return 0;
    }
    get_event_of_type(rd::<GPtr<u8>>(this, 0x47C).byte_add(0x68), 0x20).is_null() as u8
}

/// `CPlayerPed::ClearWeaponTarget` (0x609C80): players drop the weapon target (+0x71C, reference cleaned),
/// clear the camera weapon mode and their cross hair.
#[hook(addr = 0x609C80, conv = thiscall, name = "CPlayerPed::ClearWeaponTarget", expect = "57 8B F9 90 E9 13 E3 DF FF", this = "CPlayerPed")]
pub fn clear_weapon_target(this: GPtr<u8>) {
    if !matches!(rd::<i32>(this, 0x598), 0 | 1) {
        return;
    }
    let r: GPtr<GPtr<u8>> = this.byte_add(0x71C).cast();
    let e = r.read();
    if !e.is_null() {
        clean_up_old_reference(e, r);
    }
    r.write(GPtr::NULL);
    camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
    clear_cross_hair(rd::<i32>(this, 0x598));
}

/// `CPlayerPed::Clear3rdPersonMouseTarget` (0x609ED0)
#[hook(addr = 0x609ED0, conv = thiscall, name = "CPlayerPed::Clear3rdPersonMouseTarget", expect = "56 8D B1 9C 07 00 00", this = "CPlayerPed")]
pub fn clear_3rd_person_mouse_target(this: GPtr<u8>) {
    let r: GPtr<GPtr<u8>> = this.byte_add(0x79C).cast();
    let e = r.read();
    if !e.is_null() {
        clean_up_old_reference(e, r);
        r.write(GPtr::NULL);
    }
}

/// `CPlayerPed::DoStuffToGoOnFire` (0x60A020): state 0xC clears the camera weapon mode (tail call).
#[hook(addr = 0x60A020, conv = thiscall, name = "CPlayerPed::DoStuffToGoOnFire", expect = "83 B9 30 05 00 00 0C", this = "CPlayerPed")]
pub fn do_stuff_to_go_on_fire(this: GPtr<u8>) {
    if rd::<i32>(this, 0x530) == 0xC {
        camera_clear_player_weapon_mode(g::<u8>(THE_CAMERA));
    }
}

fn group_membership(this: GPtr<u8>) -> GPtr<u8> {
    let grp = rd::<i32>(pdata(this), 0x38);
    g::<u8>(0xC09928).byte_add((grp as isize).wrapping_mul(0x2D4))
}

/// `CPlayerPed::MakePlayerGroupDisappear` (0x60A440): hides the 7 non-mission members (entity flags
/// +0x1C bits 0 and 7 cleared) and saves/sets their +0x474 bit 14.
#[hook(addr = 0x60A440, conv = thiscall, name = "CPlayerPed::MakePlayerGroupDisappear", expect = "56 57 8B F9 33 F6", this = "CPlayerPed")]
pub fn make_player_group_disappear(this: GPtr<u8>) {
    for i in 0..7 {
        let m = group_get_member(group_membership(this), i);
        if m.is_null() || rd::<u8>(m, 0x484) == 2 {
            continue;
        }
        wr::<u32>(m, 0x1C, rd::<u32>(m, 0x1C) & 0xFFFF_FF7E);
        let f = rd::<u32>(m, 0x474);
        GROUP_MEMBER_FLAGS.ptr().cast::<u8>().byte_add(i as isize).write(((f >> 14) & 1) as u8);
        wr::<u32>(m, 0x474, rd::<u32>(m, 0x474) | 0x4000);
    }
}

/// `CPlayerPed::MakePlayerGroupReappear` (0x60A4B0)
#[hook(addr = 0x60A4B0, conv = thiscall, name = "CPlayerPed::MakePlayerGroupReappear", expect = "56 57 8B F9 33 F6", this = "CPlayerPed")]
pub fn make_player_group_reappear(this: GPtr<u8>) {
    for i in 0..7 {
        let m = group_get_member(group_membership(this), i);
        if m.is_null() || rd::<u8>(m, 0x484) == 2 {
            continue;
        }
        let mut f = rd::<u32>(m, 0x1C) | 0x80;
        wr::<u32>(m, 0x1C, f);
        if rd::<u32>(m, 0x46C) & 0x100 == 0 {
            f |= 1;
            wr::<u32>(m, 0x1C, f);
        }
        let s = u32::from(GROUP_MEMBER_FLAGS.ptr().cast::<u8>().byte_add(i as isize).read());
        let v = rd::<u32>(m, 0x474);
        wr::<u32>(m, 0x474, (((s << 14) ^ v) & 0x4000) ^ v);
    }
}

/// `CPlayerPed::DoesPlayerWantNewWeapon(eWeaponType, bool)` (0x609710, `ret 8`, AL)
#[hook(addr = 0x609710, conv = thiscall, name = "CPlayerPed::DoesPlayerWantNewWeapon", expect = "53 55 56 57 8B F9", this = "CPlayerPed")]
pub fn does_player_want_new_weapon(this: GPtr<u8>, ty: i32, in_vehicle: u32) -> u8 {
    match rd::<i32>(this, 0x598) {
        0 => {
            get_pad(0);
        }
        1 => {
            get_pad(1);
        }
        _ => {}
    }
    let slot = rd::<i32>(get_weapon_info(ty, 1), 0x14);
    let cur = rd::<i32>(this, 0x5A0 + (slot as isize).wrapping_mul(0x1C));
    if cur == ty || cur == 0 {
        return 1;
    }
    if in_vehicle as u8 != 0 {
        return 0;
    }
    let active = || i32::from(rd::<i8>(this, 0x718));
    if !get_task_jet_pack(rd::<GPtr<u8>>(this, 0x47C)).is_null() && active() == slot {
        let flag = || {
            let sk = ped_get_weapon_skill(this, cur);
            (rd::<u32>(get_weapon_info(cur, i32::from(sk)), 0x18) >> 1) & 1
        };
        if flag() != 0 && flag() == 0 {
            return 0;
        }
    }
    let s = rd::<i32>(this, 0x530);
    if (s == 0x10 || s == 0x16) && slot == active() {
        return 0;
    }
    1
}

/// `CPlayerPed::PickWeaponAllowedFor2Player` (0x609800): resets the chosen slot (+0x20) to 0 when its
/// weapon cannot be used in 2-player mode.
#[hook(addr = 0x609800, conv = thiscall, name = "CPlayerPed::PickWeaponAllowedFor2Player", expect = "56 8B F1 8B 86 80 04 00 00", this = "CPlayerPed")]
pub fn pick_weapon_allowed_for_2_player(this: GPtr<u8>) {
    let s = i32::from(rd::<i8>(pdata(this), 0x20));
    let r = weapon_can_be_used_for_2_player(this.byte_add(0x5A0 + (s as isize).wrapping_mul(0x1C)));
    if r == 0 {
        wr::<u8>(pdata(this), 0x20, r);
    }
}

/// `CPlayerPed::FindTargetPriority(CEntity*)` (0x609DE0, `ret 4`, ST0): vehicles 0.1; objects 0.1 for
/// object types 2/6, else 0; peds 0.1 (0.25 mission, 0.06 type 8, 0.05 same group), 0.8 with an active
/// task 0x3E8/0x44D, and 1.0 with +0x470 bit 29; other types 0.1.
#[hook(addr = 0x609DE0, conv = thiscall, name = "CPlayerPed::FindTargetPriority", expect = "51 56 8B 74 24 0C", this = "CPlayerPed")]
pub fn find_target_priority(this: GPtr<u8>, e: GPtr<u8>) -> X87 {
    let mut local: f32 = f32::from_bits(0x3DCC_CCCD);
    match rd::<u8>(e, 0x36) & 7 {
        2 => x(f32::from_bits(0x3DCC_CCCD)),
        4 => {
            let t = rd::<u8>(e, 0x13C);
            if t == 2 || t == 6 { x(local) } else { x(0.0) }
        }
        3 => {
            if rd::<u8>(e, 0x484) == 2 {
                local = 0.25;
            }
            if rd::<i32>(e, 0x598) == 8 {
                local = f32::from_bits(0x3D75_C28F);
            }
            if are_in_same_group(this, e) != 0 {
                local = f32::from_bits(0x3D4C_CCCD);
            }
            let tm = || rd::<GPtr<u8>>(e, 0x47C).byte_add(4);
            let v = if !find_active_task_by_type(tm(), 0x3E8).is_null() || !find_active_task_by_type(tm(), 0x44D).is_null() {
                x(f32::from_bits(0x3F4C_CCCD))
            } else {
                x(local)
            };
            if rd::<u32>(e, 0x470) & 0x2000_0000 != 0 { x(1.0) } else { v }
        }
        _ => x(local),
    }
}

/// `CPlayerPed::CanIKReachThisTarget(CVector target, CWeapon*, bool)` (0x609F80, `ret 0x14`, AL): true for
/// weapons with info flag bit 1; else true unless the horizontal distance (x87 sqrt) is below the height
/// of the ped's matrix above the target.
#[hook(addr = 0x609F80, conv = thiscall, name = "CPlayerPed::CanIKReachThisTarget", expect = "8B 44 24 10 56", this = "CPlayerPed")]
pub fn can_ik_reach_this_target(this: GPtr<u8>, tx: f32, ty: f32, tz: f32, w: GPtr<u8>, _b: u32) -> u8 {
    let t = rd::<i32>(w, 0);
    let sk = ped_get_weapon_skill(this, t);
    if (rd::<u32>(get_weapon_info(t, i32::from(sk)), 0x18) >> 1) & 1 != 0 {
        return 1;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let dy = x(ty).fsub_m(rd::<f32>(m, 0x34));
    let dx = x(tx).fsub_m(rd::<f32>(m, 0x30));
    let dz = x(rd::<f32>(m, 0x38)).fsub_m(tz);
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
    if d.fcom(dz).test_parity(5) { 1 } else { 0 }
}

/// `CPlayerPed::ResetSprintEnergy` (0x60A530, entry `push esi; nop; jmp 0x406B3E`)
// Not registered (INF-20): the no-CD protection reads this entry as data (`sbb ebx, [0x60A533]` at 0x4037BC).
pub fn reset_sprint_energy(this: GPtr<u8>) {
    let d = pdata(this);
    wr::<f32>(d, 0x18, X87::from_f64_exact(get_fat_and_muscle_modifier(7)).to_f32());
}

/// `CPlayerPed::HandleSprintEnergy(bool sprint, float rate)` (0x60A550, `ret 8`, AL)
#[hook(addr = 0x60A550, conv = thiscall, name = "CPlayerPed::HandleSprintEnergy", expect = "8A 44 24 04 84 C0", this = "CPlayerPed")]
pub fn handle_sprint_energy(this: GPtr<u8>, sprint: u32, rate: f32) -> u8 {
    if sprint as u8 != 0 {
        let focus = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
        if g::<u8>(0xB7CEE4 + focus * PLAYER_INFO_SIZE).read() != 0 {
            return 1;
        }
        let d = pdata(this);
        if rd::<u8>(d, 0x34) & 2 != 0 {
            return 1;
        }
        if !x(rate).fcom(x(0.0)).test_parity(0x44) {
            return 1;
        }
        let e = rd::<f32>(d, 0x18);
        if x(e).fcom(x(f32::from_bits(0xC316_0000))).test_any(0x41) {
            return 0;
        }
        let v = x(rd::<f32>(d, 0x18)).fsub(x(sa_core::timer::TIME_STEP.get()).fmul_m(rate)).to_f32();
        wr::<f32>(d, 0x18, X87::from_f64_exact(max_f(-150.0, v)).to_f32());
        return 1;
    }
    let m = X87::from_f64_exact(get_fat_and_muscle_modifier(7));
    if m.fcom(x(rd::<f32>(pdata(this), 0x18))).test_any(0x41) {
        return 0;
    }
    let d = pdata(this);
    let v = x(sa_core::timer::TIME_STEP.get()).fmul_m(rate).fmul_m(0.5).fadd_m(rd::<f32>(d, 0x18));
    wr::<f32>(d, 0x18, v.to_f32());
    0
}

/// `CPlayerPed::ResetPlayerBreath` (0x60A8A0)
#[hook(addr = 0x60A8A0, conv = thiscall, name = "CPlayerPed::ResetPlayerBreath", expect = "56 57 8B F1 8B BE 80 04 00 00", this = "CPlayerPed")]
pub fn reset_player_breath(this: GPtr<u8>) {
    let d = pdata(this);
    wr::<f32>(d, 0x44, X87::from_f64_exact(get_fat_and_muscle_modifier(8)).to_f32());
    let d = pdata(this);
    wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FF7F);
}

fn player_slot(id: i32) -> GPtr<GPtr<u8>> {
    g::<GPtr<u8>>(PLAYERS).byte_add((id as isize).wrapping_mul(PLAYER_INFO_SIZE as isize))
}

/// `CPlayerPed::RemovePlayerPed(int32)` (0x6094A0, cdecl): a driving player's vehicle becomes abandoned
/// (status 3, +0x49C = 0, +0x4A0 = 0.1); the ped is removed from the world and deleted.
#[hook(addr = 0x6094A0, conv = cdecl, name = "CPlayerPed::RemovePlayerPed", expect = "56 8B 74 24 08")]
pub fn remove_player_ped(id: i32) {
    let s = player_slot(id);
    let p = s.read();
    if p.is_null() {
        return;
    }
    let v = rd::<GPtr<u8>>(p, 0x58C);
    if !v.is_null() && rd::<GPtr<u8>>(v, 0x460) == p {
        wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
        wr::<u32>(rd::<GPtr<u8>>(p, 0x58C), 0x49C, 0);
        wr::<u32>(rd::<GPtr<u8>>(p, 0x58C), 0x4A0, 0x3DCC_CCCD);
    }
    world_remove(s.read());
    let p = s.read();
    if !p.is_null() {
        entity_vdelete(p, 1);
    }
    s.write(GPtr::NULL);
}

/// `CPlayerPed::DeactivatePlayerPed(int32)` (0x609520): tail call `CWorld::Remove(Players[id].ped)`.
#[hook(addr = 0x609520, conv = cdecl, name = "CPlayerPed::DeactivatePlayerPed", expect = "8B 44 24 04 69 C0 90 01 00 00")]
pub fn deactivate_player_ped(id: i32) {
    world_remove(player_slot(id).read());
}

/// `CPlayerPed::ReactivatePlayerPed(int32)` (0x609540): tail call `CWorld::Add(Players[id].ped)`.
#[hook(addr = 0x609540, conv = cdecl, name = "CPlayerPed::ReactivatePlayerPed", expect = "8B 44 24 04 69 C0 90 01 00 00")]
pub fn reactivate_player_ped(id: i32) {
    world_add(player_slot(id).read());
}

/// `CPlayerPed::IsHidden` (0x609620, EAX): on foot and the lighting total at most 0.05.
#[hook(addr = 0x609620, conv = thiscall, name = "CPlayerPed::IsHidden", expect = "8B 81 6C 04 00 00", this = "CPlayerPed")]
pub fn is_hidden(this: GPtr<u8>) -> u32 {
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        return 0;
    }
    let l = X87::from_f64_exact(get_lighting_total(this));
    (!l.fcom(x(f32::from_bits(0x3D4C_CCCD))).test_parity(0x41)) as u32
}

/// `CPlayerPed::GetWeaponRadiusOnScreen` (0x609CD0, ST0): 0 for melee; `0.5 / accuracy` times 1.0
/// ([0x8D24E0]) for the shotguns (0x19..0x1B), else times `min(15 / range, 1)` and `0.5 * pd+0x2C + 1`
/// (halved while ducking-aim flag +0x46C bit 26); at least 0.2.
#[hook(addr = 0x609CD0, conv = thiscall, name = "CPlayerPed::GetWeaponRadiusOnScreen", expect = "56 8B F1 0F BE 86 18 07 00 00", this = "CPlayerPed")]
pub fn get_weapon_radius_on_screen(this: GPtr<u8>) -> X87 {
    let w = rd::<i32>(this, 0x5A0 + isize::from(rd::<i8>(this, 0x718)) * 0x1C);
    let sk = ped_get_weapon_skill_current(this);
    let info = get_weapon_info(w, i32::from(sk));
    let a = x(0.5).fdiv(x(rd::<f32>(info, 0x38)));
    if rd::<u32>(info, 0) == 0 {
        return x(0.0);
    }
    let mut v = if (0x19..=0x1B).contains(&w) {
        a.fmul_m(RADIUS_SHOTGUN.get())
    } else {
        let mut r = x(RADIUS_RANGE.get()).fdiv(x(rd::<f32>(info, 8)));
        if !x(1.0).fcom(r).test_parity(5) {
            r = x(1.0);
        }
        let t = x(RADIUS_ACC.get()).fmul_m(rd::<f32>(pdata(this), 0x2C)).fadd_m(1.0);
        let mut v = t.fmul(r.fmul(a));
        if rd::<u32>(this, 0x46C) & 0x400_0000 != 0 {
            v = v.fmul_m(0.5);
        }
        v
    };
    if !v.fcom(x(f32::from_bits(0x3E4C_CCCD))).test_parity(5) {
        v = x(f32::from_bits(0x3E4C_CCCD));
    }
    v
}

/// `CPlayerPed::ForceGroupToAlwaysFollow(bool)` (0x60C7C0, `ret 4`)
#[hook(addr = 0x60C7C0, conv = thiscall, name = "CPlayerPed::ForceGroupToAlwaysFollow", expect = "8A 44 24 04 84 C0", this = "CPlayerPed")]
pub fn force_group_to_always_follow(this: GPtr<u8>, b: u32) {
    let d = pdata(this);
    if b as u8 != 0 {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) | 0x200);
        tell_group_to_start_following_player(this, 1, 0, 1);
    } else {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FDFF);
    }
}

/// `CPlayerPed::ForceGroupToNeverFollow(bool)` (0x60C800, `ret 4`)
#[hook(addr = 0x60C800, conv = thiscall, name = "CPlayerPed::ForceGroupToNeverFollow", expect = "8A 44 24 04 84 C0", this = "CPlayerPed")]
pub fn force_group_to_never_follow(this: GPtr<u8>, b: u32) {
    let d = pdata(this);
    if b as u8 != 0 {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) | 0x400);
        tell_group_to_start_following_player(this, 0, 0, 1);
    } else {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_FBFF);
    }
}

/// `CPlayerPed::MakeChangesForNewWeapon(int32 slot)` (0x60D000, `ret 4`): tail call with the slot's weapon
/// type unless -1.
#[hook(addr = 0x60D000, conv = thiscall, name = "CPlayerPed::MakeChangesForNewWeapon-BySlot", expect = "8B 44 24 04 83 F8 FF", this = "CPlayerPed")]
pub fn make_changes_for_new_weapon_by_slot(this: GPtr<u8>, slot: i32) {
    if slot != -1 {
        make_changes_for_new_weapon(this, rd::<i32>(this, 0x5A0 + (slot as isize).wrapping_mul(0x1C)));
    }
}

/// `CPlayerPed::PlayerHasJustAttackedSomeone` (0x60D5A0): `jmp CPlayerPed::PlayerWantsToAttack`.
#[hook(addr = 0x60D5A0, conv = thiscall, name = "CPlayerPed::PlayerHasJustAttackedSomeone", expect = "E9 AB F6 FF FF 90", this = "CPlayerPed")]
pub fn player_has_just_attacked_someone(this: GPtr<u8>) {
    player_wants_to_attack(this);
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_WANTED_LEVEL,
    HOOK_SET_WANTED_LEVEL,
    HOOK_SET_WANTED_LEVEL_NO_DROP,
    HOOK_CHEAT_WANTED_LEVEL,
    HOOK_PED_CAN_BE_TARGETTED_VEHICLE_WISE,
    HOOK_BUSTED,
    HOOK_GET_PLAYER_INFO_FOR_THIS_PLAYER_PED,
    HOOK_ANNOY_PLAYER_PED,
    HOOK_CLEAR_ADRENALINE,
    HOOK_GET_PAD_FROM_PLAYER,
    HOOK_CAN_PLAYER_START_MISSION,
    HOOK_CLEAR_WEAPON_TARGET,
    HOOK_CLEAR_3RD_PERSON_MOUSE_TARGET,
    HOOK_DO_STUFF_TO_GO_ON_FIRE,
    HOOK_MAKE_PLAYER_GROUP_DISAPPEAR,
    HOOK_MAKE_PLAYER_GROUP_REAPPEAR,
    HOOK_DOES_PLAYER_WANT_NEW_WEAPON,
    HOOK_PICK_WEAPON_ALLOWED_FOR_2_PLAYER,
    HOOK_FIND_TARGET_PRIORITY,
    HOOK_CAN_IK_REACH_THIS_TARGET,
    HOOK_HANDLE_SPRINT_ENERGY,
    HOOK_RESET_PLAYER_BREATH,
    HOOK_REMOVE_PLAYER_PED,
    HOOK_DEACTIVATE_PLAYER_PED,
    HOOK_REACTIVATE_PLAYER_PED,
    HOOK_IS_HIDDEN,
    HOOK_GET_WEAPON_RADIUS_ON_SCREEN,
    HOOK_FORCE_GROUP_TO_ALWAYS_FOLLOW,
    HOOK_FORCE_GROUP_TO_NEVER_FOLLOW,
    HOOK_MAKE_CHANGES_FOR_NEW_WEAPON_BY_SLOT,
    HOOK_PLAYER_HAS_JUST_ATTACKED_SOMEONE,
];
