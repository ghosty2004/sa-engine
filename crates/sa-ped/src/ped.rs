//! `CPed` accessors and small helpers (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `Ped.cpp` and `docs/re/specs/PED/CPed*.md` as reading aids).
//!
//! `CPed` fields used here: +0x18 RwObject (clump), +0x1C entity flags, +0x22 model index, +0x46C/+0x470/
//! +0x474 ped flags (dwords), +0x47C intelligence, +0x480 player data, +0x484 created by, +0x488 nodes[19],
//! +0x4F8 gun-flash object, +0x4FC goggles object, +0x504/+0x508 gun-flash alpha (i16), +0x518 move speed
//! multiplier, +0x528 anim flags, +0x530 ped state, +0x534 move state, +0x540 health, +0x548 armour,
//! +0x558 current heading, +0x55C aimed heading, +0x560 heading change rate, +0x58C vehicle, +0x598 ped
//! type, +0x59C ped stats, +0x5A0 weapons[13] (0x1C each: type, state, ammo in clip, total ammo), +0x718
//! active weapon slot, +0x72C weapon skill, +0x738 look-at entity, +0x73C look heading, +0x748 look timer,
//! +0x754 body part to remove, +0x788 cover point.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

const _: () = assert!(sa_core::layouts::size::CPed == 0x79C);

/// `CPedChat` global: the ped in conversation (0x9691C0)
const CONV_PED: usize = 0x9691C0;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00, thiscall on the referenced entity)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571B70 => unported;
    /// `CPedGroupMembership::IsFollower(const CPed*)` (0x5F69E0)
    pub fn group_is_follower(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F69E0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    pub fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskSimpleHoldEntity::CanThrowEntity` (0x691700)
    pub fn can_throw_entity(t: GPtr<u8>) -> u8 = thiscall 0x691700 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    pub fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    pub fn set_ped_decision_maker_type(intel: GPtr<u8>, ty: i32) = thiscall 0x600B50 => unported;
    /// `CPedIntelligence::SetSeeingRange(float)` (0x600BF0)
    pub fn set_seeing_range(intel: GPtr<u8>, r: u32) = thiscall 0x600BF0 => unported;
    /// `CPedIntelligence::SetHearingRange(float)` (0x600BE0)
    pub fn set_hearing_range(intel: GPtr<u8>, r: u32) = thiscall 0x600BE0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetPedWalkLeftRight` (0x53FC90)
    pub fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown` (0x53FD30)
    pub fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0)
    pub fn get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50)
    pub fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CGeneral::GetAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CEA0)
    pub fn get_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CEA0 => unported;
    /// `CWeaponInfo::GetSkillStatIndex(eWeaponType)` (0x743CD0)
    pub fn get_skill_stat_index(ty: i32) -> i32 = cdecl 0x743CD0 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60)
    pub fn get_weapon_skill_type_call(ped: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => get_weapon_skill_type;
    /// `CPed::ClearLookFlag` (0x5E1950)
    pub fn clear_look_flag_call(ped: GPtr<u8>) = thiscall 0x5E1950 => clear_look_flag;
    /// `CPed::TurnBody` (0x5E4000)
    pub fn turn_body_call(ped: GPtr<u8>) -> u8 = thiscall 0x5E4000 => turn_body;
    /// `CAEPedSpeechAudioEntity::GetPedTalking` (0x4E3F50)
    pub fn speech_get_ped_talking(e: GPtr<u8>) -> u8 = thiscall 0x4E3F50 => unported;
    /// `CAEPedSpeechAudioEntity::EnablePedSpeech` (0x4E3F70)
    pub fn speech_enable(e: GPtr<u8>) = thiscall 0x4E3F70 => unported;
    /// `CAEPedSpeechAudioEntity::EnablePedSpeechForScriptSpeech` (0x4E3F90)
    pub fn speech_enable_script(e: GPtr<u8>) = thiscall 0x4E3F90 => unported;
    /// CRT printf (0x821982)
    pub fn debug_printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;
    /// `CLocalisation::ShootLimbs` (0x56D280)
    pub fn shoot_limbs() -> u8 = cdecl 0x56D280 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    pub fn find_task_by_type(intel: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPointLights::RemoveLightsAffectingObject` (0x6FFFE0)
    pub fn remove_lights_affecting_object() = cdecl 0x6FFFE0 => unported;
    /// `Light::SetAmbientColours()` (0x735D30)
    pub fn set_ambient_colours() = cdecl 0x735D30 => unported;
    /// `Light::DeActivateDirectional` (0x735C70)
    pub fn deactivate_directional() = cdecl 0x735C70 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    pub fn clump_get_first_association(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*)` (0x4D6AB0)
    pub fn get_next_association(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6AB0 => unported;
    /// `RwHelper::GetFirstObject(RwFrame*)` (0x7348C0)
    pub fn get_first_object(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `CVehicle::SetComponentAtomicAlpha(RpAtomic*, int32)` (0x6D2960)
    pub fn set_component_atomic_alpha(a: GPtr<u8>, alpha: i32) = cdecl 0x6D2960 => unported;
    /// `CCoverPoint::ReleaseCoverPointForPed(CPed*)` (0x698EF0)
    pub fn release_cover_point_for_ped(cp: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x698EF0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32)` (0x587C60)
    pub fn clear_blip_for_entity(ty: i32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80)
    pub fn set_orientation(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
}

/// The ped pool handle of `ped` (`CPool::GetRef`): `(index << 8) + flags[index]`, index as the
/// original's signed magic-number division by 0x7C4.
pub fn ped_pool_ref(ped: GPtr<u8>) -> i32 {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let base = rd::<GPtr<u8>>(pool, 0);
    let idx = (ped.addr() as u32).wrapping_sub(base.addr() as u32) as i32 / PED_SLOT as i32;
    let f = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read();
    (i32::from(f)).wrapping_add(idx.wrapping_shl(8))
}

fn state(p: GPtr<u8>) -> i32 {
    rd::<i32>(p, 0x530)
}

fn weapon(p: GPtr<u8>, slot: i32) -> GPtr<u8> {
    p.byte_add(0x5A0 + (slot as isize).wrapping_mul(0x1C))
}

/// `CPed::PedIsInvolvedInConversation` (0x43AB90, AL): `this == CPedChat ped`.
#[hook(addr = 0x43AB90, conv = thiscall, name = "CPed::PedIsInvolvedInConversation", expect = "3B 0D C0 91 96 00", this = "CPed")]
pub fn ped_is_involved_in_conversation(this: GPtr<u8>) -> u8 {
    (this == g::<GPtr<u8>>(CONV_PED).read()) as u8
}

/// `CPed::IsWearingGoggles` (0x479D10, EAX)
#[hook(addr = 0x479D10, conv = thiscall, name = "CPed::IsWearingGoggles", expect = "8B 91 FC 04 00 00", this = "CPed")]
pub fn is_wearing_goggles(this: GPtr<u8>) -> u32 {
    (!rd::<GPtr<u8>>(this, 0x4FC).is_null()) as u32
}

/// `CPed::SetStayInSamePlace(bool)` (stub 0x481090 → 0x156D8B0, `ret 4`): flag bit 21 of +0x46C.
#[hook(addr = 0x481090, conv = thiscall, name = "CPed::SetStayInSamePlace", expect = "E9 1B C8 0E 01", this = "CPed")]
pub fn set_stay_in_same_place(this: GPtr<u8>, b: u32) {
    let v = rd::<u32>(this, 0x46C);
    wr::<u32>(this, 0x46C, ((((b & 0xFF) << 21) ^ v) & 0x20_0000) ^ v);
}

/// `CPed::SetPedStats(ePedStats)` (0x5DEBC0, `ret 4`)
#[hook(addr = 0x5DEBC0, conv = thiscall, name = "CPed::SetPedStats", expect = "8B 44 24 04 8B 15 EC BB C0 00", this = "CPed")]
pub fn set_ped_stats(this: GPtr<u8>, stat: i32) {
    let base = crate::ped_type::PED_STATS.get();
    wr::<GPtr<u8>>(this, 0x59C, base.byte_add((stat as isize).wrapping_mul(0x34)));
}

/// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
#[hook(addr = 0x5DEC00, conv = thiscall, name = "CPed::SetMoveState", expect = "8B 44 24 04 89 81 34 05 00 00", this = "CPed")]
pub fn set_move_state(this: GPtr<u8>, s: i32) {
    wr::<i32>(this, 0x534, s);
}

/// `CPed::SetMoveAnimSpeed(CAnimBlendAssociation*)` (0x5DEC10, `ret 4`): assoc speed (+0x24) = the move
/// speed multiplier (+0x518) clamped to [-0.3, 0.3] (the original's compare sequence, NaN passes through)
/// plus 1.0 for mission peds; otherwise plus 1.2 minus `(u16 +0x20 random seed) * 0.4 / 32767`.
#[hook(addr = 0x5DEC10, conv = thiscall, name = "CPed::SetMoveAnimSpeed", expect = "51 D9 05 24 8C 85 00", this = "CPed")]
pub fn set_move_anim_speed(this: GPtr<u8>, assoc: GPtr<u8>) {
    const P3: f32 = f32::from_bits(0x3E99_999A);
    const M3: f32 = f32::from_bits(0xBE99_999A);
    let v = rd::<f32>(this, 0x518);
    let clamped = if x(P3).fcom(x(v)).test_parity(5) && !x(M3).fcom(x(v)).test_any(0x41) {
        M3
    } else if x(P3).fcom(x(v)).test_parity(5) {
        v
    } else {
        P3
    };
    let r = if rd::<u8>(this, 0x484) == 2 {
        x(clamped).fadd_m(1.0)
    } else {
        let seed = xi(i32::from(rd::<u16>(this, 0x20))).fmul_m(f32::from_bits(0x3ECC_CCCD)).fmul_m(f32::from_bits(0x3800_0100));
        x(clamped).fadd_m(f32::from_bits(0x3F99_999A)).fsub(seed)
    };
    wr::<f32>(assoc, 0x24, r.to_f32());
}

/// `CPed::CanUseTorsoWhenLooking` (0x5DED90, AL): not in states 0x32/0x3A and +0x46C bit 26 clear.
#[hook(addr = 0x5DED90, conv = thiscall, name = "CPed::CanUseTorsoWhenLooking", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn can_use_torso_when_looking(this: GPtr<u8>) -> u8 {
    let s = state(this);
    (s != 0x32 && s != 0x3A && rd::<u32>(this, 0x46C) & 0x400_0000 == 0) as u8
}

/// `CPed::ClearAimFlag` (0x5DEF20)
#[hook(addr = 0x5DEF20, conv = thiscall, name = "CPed::ClearAimFlag", expect = "8B 81 6C 04 00 00", this = "CPed")]
pub fn clear_aim_flag(this: GPtr<u8>) {
    let f = rd::<u32>(this, 0x46C);
    if f & 0x10 != 0 {
        wr::<u32>(this, 0x46C, (f & !0x10) | 0x20);
        wr::<u32>(this, 0x528, rd::<u32>(this, 0x528) & !4);
        wr::<u32>(this, 0x748, 0);
    }
    let pd = rd::<GPtr<u8>>(this, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x54, 0);
    }
}

/// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60, `ret 4`): `atan2(-x, y) - heading + pi/4`, raised
/// by 2pi while negative, times 2/pi, `_ftol`, then reduced into 0..3 (`n - 4*((n-4)/4 + 1)` for n > 3).
/// FPATAN through `X87::fpatan` (L2 decides it on the CPU).
#[hook(addr = 0x5DEF60, conv = thiscall, name = "CPed::GetLocalDirection", expect = "8B 44 24 04 D9 00", this = "CPed")]
pub fn get_local_direction(this: GPtr<u8>, v: GPtr<f32>) -> i32 {
    let a = x(v.read()).fchs().fpatan(x(v.wrapping_index(1).read()));
    let mut a = a.fsub_m(rd::<f32>(this, 0x558)).fadd_m(f32::from_bits(0x3F49_0FDB));
    if !a.fcom(x(0.0)).test_parity(5) {
        loop {
            a = a.fadd_m(f32::from_bits(0x40C9_0FDB));
            if a.fcom(x(0.0)).test_parity(5) {
                break;
            }
        }
    }
    let n = ftol32(a.fmul_m(f32::from_bits(0x3F22_F983)));
    if n > 3 {
        let k = ((n.wrapping_sub(4) as u32) >> 2) << 2;
        n.wrapping_add((-4i32).wrapping_sub(k as i32))
    } else {
        n
    }
}

/// `CPed::IsPedShootable` (0x5DEFD0, AL): state <= 0x2E.
#[hook(addr = 0x5DEFD0, conv = thiscall, name = "CPed::IsPedShootable", expect = "83 B9 30 05 00 00 2E", this = "CPed")]
pub fn is_ped_shootable(this: GPtr<u8>) -> u8 {
    (state(this) <= 0x2E) as u8
}

/// `CPed::UseGroundColModel` (0x5DEFE0, AL): states 0x2A, 0x2D, 0x36, 0x37.
#[hook(addr = 0x5DEFE0, conv = thiscall, name = "CPed::UseGroundColModel", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn use_ground_col_model(this: GPtr<u8>) -> u8 {
    matches!(state(this), 0x2A | 0x2D | 0x36 | 0x37) as u8
}

/// `CPed::CanPedReturnToState` (0x5DF000, AL): state <= 0x26 and not 0x16/0x10/0x11/0x1F/0xC/2.
#[hook(addr = 0x5DF000, conv = thiscall, name = "CPed::CanPedReturnToState", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn can_ped_return_to_state(this: GPtr<u8>) -> u8 {
    let s = state(this);
    (s <= 0x26 && !matches!(s, 0x16 | 0x10 | 0x11 | 0x1F | 0xC | 2)) as u8
}

/// `CPed::CanSetPedState` (0x5DF030, AL): not 0x36/0x37/0x3F/0x3B/0x39/0x3C.
#[hook(addr = 0x5DF030, conv = thiscall, name = "CPed::CanSetPedState", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn can_set_ped_state(this: GPtr<u8>) -> u8 {
    !matches!(state(this), 0x36 | 0x37 | 0x3F | 0x3B | 0x39 | 0x3C) as u8
}

/// `CPed::CanBeArrested` (0x5DF060, AL): not 0x36/0x37/0x3F/0x3B/0x3D.
#[hook(addr = 0x5DF060, conv = thiscall, name = "CPed::CanBeArrested", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn can_be_arrested(this: GPtr<u8>) -> u8 {
    !matches!(state(this), 0x36 | 0x37 | 0x3F | 0x3B | 0x3D) as u8
}

/// `CPed::CanStrafeOrMouseControl` (0x5DF090, AL): states 1, 9, 8, 0, 0x16, 0x10, 0x11, 0x29, 0x24.
#[hook(addr = 0x5DF090, conv = thiscall, name = "CPed::CanStrafeOrMouseControl", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn can_strafe_or_mouse_control(this: GPtr<u8>) -> u8 {
    matches!(state(this), 1 | 9 | 8 | 0 | 0x16 | 0x10 | 0x11 | 0x29 | 0x24) as u8
}

/// `CPed::CanBeDeletedEvenInVehicle` (0x5DF150, AL): false for created-by 2 and 3.
#[hook(addr = 0x5DF150, conv = thiscall, name = "CPed::CanBeDeletedEvenInVehicle", expect = "0F B6 81 84 04 00 00", this = "CPed")]
pub fn can_be_deleted_even_in_vehicle(this: GPtr<u8>) -> u8 {
    !matches!(rd::<u8>(this, 0x484), 2 | 3) as u8
}

/// `CPed::SetLookTimer(uint32)` (0x5DF8D0, `ret 4`): when the timer has expired (now > timer, unsigned),
/// timer = now + time.
#[hook(addr = 0x5DF8D0, conv = thiscall, name = "CPed::SetLookTimer", expect = "A1 84 CB B7 00", this = "CPed")]
pub fn set_look_timer(this: GPtr<u8>, t: u32) {
    let now = sa_core::timer::TIME_MS.get();
    if now > rd::<u32>(this, 0x748) {
        wr::<u32>(this, 0x748, now.wrapping_add(t));
    }
}

/// `CPed::IsPlayer` (0x5DF8F0, AL): ped type 0 or 1.
#[hook(addr = 0x5DF8F0, conv = thiscall, name = "CPed::IsPlayer", expect = "8B 81 98 05 00 00", this = "CPed")]
pub fn is_player(this: GPtr<u8>) -> u8 {
    matches!(rd::<i32>(this, 0x598), 0 | 1) as u8
}

/// `CPed::RestoreHeadingRate` (0x5DFD60, entry `nop; jmp 0x403866`): heading rate = stat +0x20.
#[hook(addr = 0x5DFD60, conv = thiscall, name = "CPed::RestoreHeadingRate", expect = "90 E9 00 3B E2 FF", this = "CPed")]
pub fn restore_heading_rate(this: GPtr<u8>) {
    wr::<u32>(this, 0x560, rd::<u32>(rd::<GPtr<u8>>(this, 0x59C), 0x20));
}

/// `CPed::RestoreHeadingRateCB(CAnimBlendAssociation*, void* ped)` (0x5DFD70, cdecl)
#[hook(addr = 0x5DFD70, conv = cdecl, name = "CPed::RestoreHeadingRateCB", expect = "8B 44 24 08 8B 88 9C 05 00 00")]
pub fn restore_heading_rate_cb(_assoc: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u32>(ped, 0x560, rd::<u32>(rd::<GPtr<u8>>(ped, 0x59C), 0x20));
}

/// `CPed::SetRadioStation` (0x5DFD90): a non-player driver sets its vehicle's radio station (+0x1D2) to
/// the ped model's first (+0x38) or second (+0x39) favourite, `rand() < 0x3FFF` picking the first.
#[hook(addr = 0x5DFD90, conv = thiscall, name = "CPed::SetRadioStation", expect = "56 8B F1 0F BF 46 22", this = "CPed")]
pub fn set_radio_station(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (rd::<i16>(this, 0x22) as isize as usize)).read();
    if is_player(this) != 0 {
        return;
    }
    let v = rd::<GPtr<u8>>(this, 0x58C);
    if v.is_null() || rd::<GPtr<u8>>(v, 0x460) != this {
        return;
    }
    let r = at_site(0x5DFDC5, rand);
    let st = if r < 0x3FFF { rd::<u8>(mi, 0x38) } else { rd::<u8>(mi, 0x39) };
    wr::<u8>(rd::<GPtr<u8>>(this, 0x58C), 0x1D2, st);
}

/// `CPed::IsAlive` (0x5E0170, EAX): not dying/dead (0x36/0x37).
#[hook(addr = 0x5E0170, conv = thiscall, name = "CPed::IsAlive", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn is_alive(this: GPtr<u8>) -> u32 {
    !matches!(state(this), 0x36 | 0x37) as u32
}

/// `CPed::ClearLookFlag` (0x5E1950, entry `nop; nop; jmp 0x4096D5`): while the look flag (+0x46C bit 2)
/// is set: clear it and +0x470 bit 11, set bit 3, clear anim flag bit 1 when the torso may be used, look
/// timer = now + 2000 (players) / 4000; loops again only in states 2/3 with the flag still set.
#[hook(addr = 0x5E1950, conv = thiscall, name = "CPed::ClearLookFlag", expect = "90 90 E9 7E 7D E2 FF", this = "CPed")]
pub fn clear_look_flag(this: GPtr<u8>) {
    if rd::<u8>(this, 0x46C) & 4 == 0 {
        return;
    }
    loop {
        wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) & 0xFFFF_F7FF);
        let f = (rd::<u32>(this, 0x46C) & !4) | 8;
        let s = state(this);
        wr::<u32>(this, 0x46C, f);
        if s != 0x32 && s != 0x3A && f & 0x400_0000 == 0 {
            wr::<u32>(this, 0x528, rd::<u32>(this, 0x528) & !2);
        }
        let add = if is_player(this) != 0 { 0x7D0 } else { 0xFA0 };
        wr::<u32>(this, 0x748, sa_core::timer::TIME_MS.get().wrapping_add(add));
        let s = state(this);
        if !(s == 3 || s == 2) || rd::<u8>(this, 0x46C) & 4 == 0 {
            return;
        }
    }
}

/// `CPed::IsPedInControl` (0x5E3960, entry `nop; jmp 0x4012D0`, AL): not falling/in-the-air (+0x46C bits
/// 9/10) and not dying, dead or arrested.
#[hook(addr = 0x5E3960, conv = thiscall, name = "CPed::IsPedInControl", expect = "90 E9 6A D9 E1 FF", this = "CPed")]
pub fn is_ped_in_control(this: GPtr<u8>) -> u8 {
    (rd::<u32>(this, 0x46C) & 0x600 == 0 && !matches!(state(this), 0x36 | 0x37 | 0x3F)) as u8
}

/// `CPed::SetWeaponSkill(eWeaponSkill)` (0x5E3C10, `ret 8`): not for players.
#[hook(addr = 0x5E3C10, conv = thiscall, name = "CPed::SetWeaponSkill", expect = "8B 81 98 05 00 00", this = "CPed")]
pub fn set_weapon_skill(this: GPtr<u8>, _unused: u32, skill: u32) {
    if is_player(this) == 0 {
        wr::<u8>(this, 0x72C, skill as u8);
    }
}

/// `CPed::ClearAll` (0x5E5320): for peds in control, or dead (0x37): set +0x46C bit 13, state and move
/// state 0, clear +0x470 bits 4 and 31, +0x128 = 0.
#[hook(addr = 0x5E5320, conv = thiscall, name = "CPed::ClearAll", expect = "8B 81 6C 04 00 00", this = "CPed")]
pub fn clear_all(this: GPtr<u8>) {
    let f = rd::<u32>(this, 0x46C);
    let in_control = f & 0x600 == 0 && !matches!(state(this), 0x36 | 0x37 | 0x3F);
    if !in_control && state(this) != 0x37 {
        return;
    }
    wr::<u32>(this, 0x46C, f | 0x2000);
    let g2 = rd::<u32>(this, 0x470) & 0x7FFF_FFEF;
    wr::<u32>(this, 0x530, 0);
    wr::<u32>(this, 0x534, 0);
    wr::<u32>(this, 0x470, g2);
    wr::<u32>(this, 0x128, 0);
}

/// `CPed::SetIdle` (0x5E7980): unless state 1/0x15/9: state 0x16 becomes 1; move state 1.
#[hook(addr = 0x5E7980, conv = thiscall, name = "CPed::SetIdle", expect = "8B 81 30 05 00 00", this = "CPed")]
pub fn set_idle(this: GPtr<u8>) {
    let s = state(this);
    if matches!(s, 1 | 0x15 | 9) {
        return;
    }
    if s == 0x16 {
        wr::<i32>(this, 0x530, 1);
    }
    wr::<i32>(this, 0x534, 1);
}

/// `CPed::SetPedAtomicVisibilityCB(RwObject*, void* data)` (0x5F0060, cdecl): hides (flags byte +2 = 0)
/// when `data` is null; returns the object.
#[hook(addr = 0x5F0060, conv = cdecl, name = "CPed::SetPedAtomicVisibilityCB", expect = "8B 44 24 08 85 C0")]
pub fn set_ped_atomic_visibility_cb(obj: GPtr<u8>, data: u32) -> GPtr<u8> {
    if data == 0 {
        wr::<u8>(obj, 2, 0);
    }
    obj
}

fn look_tail(this: GPtr<u8>) {
    let s = state(this);
    if s != 0x32 && s != 0x3A && rd::<u32>(this, 0x46C) & 0x400_0000 == 0 {
        wr::<u32>(this, 0x528, rd::<u32>(this, 0x528) & !2);
    }
}

fn look_ref(this: GPtr<u8>) -> GPtr<GPtr<u8>> {
    this.byte_add(0x738).cast()
}

/// `CPed::SetLookFlag(float heading, bool, bool override)` (0x5DEDC0, `ret 0xC`): when the look timer has
/// expired (unsigned) or `override`: set the look flag, look heading, drop the look-at entity (reference
/// cleaned up), timer 0, and clear anim flag bit 1 when the torso may be used.
#[hook(addr = 0x5DEDC0, conv = thiscall, name = "CPed::SetLookFlag-Heading", expect = "56 8B F1 8B 86 48 07 00 00", this = "CPed")]
pub fn set_look_flag_heading(this: GPtr<u8>, heading: u32, _b: u32, over: u32) {
    if rd::<u32>(this, 0x748) >= sa_core::timer::TIME_MS.get() && over as u8 == 0 {
        return;
    }
    wr::<u32>(this, 0x46C, (rd::<u32>(this, 0x46C) & !8) | 4);
    wr::<u32>(this, 0x73C, heading);
    let e = look_ref(this).read();
    if !e.is_null() {
        clean_up_old_reference(e, look_ref(this));
    }
    look_ref(this).write(GPtr::NULL);
    wr::<u32>(this, 0x748, 0);
    look_tail(this);
}

/// `CPed::SetLookFlag(CEntity*, bool, bool override)` (0x5DEE40, `ret 0xC`): as the heading version with
/// the entity registered as reference and the heading 999999.0.
#[hook(addr = 0x5DEE40, conv = thiscall, name = "CPed::SetLookFlag-Entity", expect = "56 8B F1 8B 86 48 07 00 00", this = "CPed")]
pub fn set_look_flag_entity(this: GPtr<u8>, ent: GPtr<u8>, _b: u32, over: u32) {
    if rd::<u32>(this, 0x748) >= sa_core::timer::TIME_MS.get() && over as u8 == 0 {
        return;
    }
    wr::<u32>(this, 0x46C, (rd::<u32>(this, 0x46C) & !8) | 4);
    let e = look_ref(this).read();
    if !e.is_null() {
        clean_up_old_reference(e, look_ref(this));
    }
    look_ref(this).write(ent);
    register_reference(ent, look_ref(this));
    wr::<u32>(this, 0x73C, 0x4974_23F0);
    wr::<u32>(this, 0x748, 0);
    look_tail(this);
}

/// `CPed::SetAimFlag(CEntity*)` (0x5DEED0, `ret 4`)
#[hook(addr = 0x5DEED0, conv = thiscall, name = "CPed::SetAimFlag-Entity", expect = "56 8B F1 8B 86 6C 04 00 00", this = "CPed")]
pub fn set_aim_flag_entity(this: GPtr<u8>, ent: GPtr<u8>) {
    wr::<u32>(this, 0x46C, (rd::<u32>(this, 0x46C) & !0x20) | 0x10);
    let e = look_ref(this).read();
    if !e.is_null() {
        clean_up_old_reference(e, look_ref(this));
    }
    look_ref(this).write(ent);
    register_reference(ent, look_ref(this));
    wr::<u32>(this, 0x748, 0);
}

/// `CPed::CanBeDeleted` (0x5DF100, AL): not in a vehicle, not a follower of the local player's group,
/// created-by not 2/3.
#[hook(addr = 0x5DF100, conv = thiscall, name = "CPed::CanBeDeleted", expect = "56 8B F1 8B 86 6C 04 00 00", this = "CPed")]
pub fn can_be_deleted(this: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        return 0;
    }
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let grp = rd::<i32>(pd, 0x38);
    let m = g::<u8>(0xC09928).byte_add((grp as isize).wrapping_mul(0x2D4));
    if group_is_follower(m, this) != 0 {
        return 0;
    }
    !matches!(rd::<u8>(this, 0x484), 2 | 3) as u8
}

/// `CPed::GetWeaponSlot(eWeaponType)` (0x5DF200, `ret 4`): `GetWeaponInfo(type, 1)->m_nSlot`.
#[hook(addr = 0x5DF200, conv = thiscall, name = "CPed::GetWeaponSlot", expect = "8B 44 24 04 6A 01", this = "CPed")]
pub fn get_weapon_slot(_this: GPtr<u8>, ty: i32) -> i32 {
    rd::<i32>(get_weapon_info(ty, 1), 0x14)
}

/// `CPed::GrantAmmo(eWeaponType, uint32)` (0x5DF220, `ret 8`): total ammo += amount, capped at 99999
/// (signed compare); an out-of-ammo weapon (state 3) with ammo becomes ready (0).
#[hook(addr = 0x5DF220, conv = thiscall, name = "CPed::GrantAmmo", expect = "8B 44 24 04 56", this = "CPed")]
pub fn grant_ammo(this: GPtr<u8>, ty: i32, amount: i32) {
    let slot = rd::<i32>(get_weapon_info(ty, 1), 0x14);
    if slot == -1 {
        return;
    }
    let w = weapon(this, slot);
    let t = rd::<i32>(w, 0xC).wrapping_add(amount);
    wr::<i32>(w, 0xC, t);
    let t = if t >= 0x1869F { 0x1869F } else { t };
    wr::<i32>(w, 0xC, t);
    if rd::<i32>(w, 4) == 3 && t > 0 {
        wr::<i32>(w, 4, 0);
    }
}

/// `CPed::SetAmmo(eWeaponType, uint32)` (0x5DF290, `ret 8`): total ammo = amount capped at 99999; ammo in
/// clip = min(clip, total) (the original's `min`, not gta-reversed's); out-of-ammo with ammo → ready.
#[hook(addr = 0x5DF290, conv = thiscall, name = "CPed::SetAmmo", expect = "8B 44 24 04 56", this = "CPed")]
pub fn set_ammo(this: GPtr<u8>, ty: i32, amount: i32) {
    let slot = rd::<i32>(get_weapon_info(ty, 1), 0x14);
    if slot == -1 {
        return;
    }
    let w = weapon(this, slot);
    wr::<i32>(w, 0xC, amount);
    let t = if amount >= 0x1869F { 0x1869F } else { amount };
    wr::<i32>(w, 0xC, t);
    let clip = rd::<i32>(w, 8);
    wr::<i32>(w, 8, if t < clip { t } else { clip });
    if rd::<i32>(w, 4) == 3 && t > 0 {
        wr::<i32>(w, 4, 0);
    }
}

/// `CPed::DoWeHaveWeaponAvailable(eWeaponType)` (0x5DF300, `ret 4`, AL)
#[hook(addr = 0x5DF300, conv = thiscall, name = "CPed::DoWeHaveWeaponAvailable", expect = "56 57 8B 7C 24 0C", this = "CPed")]
pub fn do_we_have_weapon_available(this: GPtr<u8>, ty: i32) -> u8 {
    let slot = rd::<i32>(get_weapon_info(ty, 1), 0x14);
    (slot != -1 && rd::<i32>(weapon(this, slot), 0) == ty) as u8
}

/// `CPed::GetBikeRidingSkill` (0x5DF510, ST0): players: `min(stat 0xE5 / 1000, 1.0)` as the original's
/// compare picks it (NaN stays); others: 1.0 for mission peds, else 0.
#[hook(addr = 0x5DF510, conv = thiscall, name = "CPed::GetBikeRidingSkill", expect = "8B 81 80 04 00 00", this = "CPed")]
pub fn get_bike_riding_skill(this: GPtr<u8>) -> X87 {
    if !rd::<GPtr<u8>>(this, 0x480).is_null() {
        let v = X87::from_f64_exact(get_stat_value(0xE5)).fdiv_m(1000.0);
        if x(1.0).fcom(v).test_parity(5) { v } else { x(1.0) }
    } else if rd::<u8>(this, 0x484) == 2 {
        x(1.0)
    } else {
        x(0.0)
    }
}

fn task_manager(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x47C).byte_add(4)
}

/// The first active hold task (types 0x133, 0x134, 0x135), as `GetHoldingTask` searches.
fn holding_task(this: GPtr<u8>) -> GPtr<u8> {
    let t = find_active_task_by_type(task_manager(this), 0x133);
    if !t.is_null() {
        return t;
    }
    let t = find_active_task_by_type(task_manager(this), 0x134);
    if !t.is_null() {
        return t;
    }
    find_active_task_by_type(task_manager(this), 0x135)
}

/// `CPed::GetHoldingTask` (0x5E0290)
#[hook(addr = 0x5E0290, conv = thiscall, name = "CPed::GetHoldingTask", expect = "56 8B F1 8B 86 7C 04 00 00", this = "CPed")]
pub fn get_holding_task(this: GPtr<u8>) -> GPtr<u8> {
    holding_task(this)
}

/// `CPed::GetEntityThatThisPedIsHolding` (0x5E02E0): the hold task's entity (+8), else a pick-up task
/// (0x136)'s entity (+0xC), else null.
#[hook(addr = 0x5E02E0, conv = thiscall, name = "CPed::GetEntityThatThisPedIsHolding", expect = "56 8B F1 8B 86 7C 04 00 00", this = "CPed")]
pub fn get_entity_that_this_ped_is_holding(this: GPtr<u8>) -> GPtr<u8> {
    let t = holding_task(this);
    if !t.is_null() {
        return rd::<GPtr<u8>>(t, 8);
    }
    let t = find_active_task_by_type(task_manager(this), 0x136);
    if !t.is_null() { rd::<GPtr<u8>>(t, 0xC) } else { GPtr::NULL }
}

/// `CPed::CanThrowEntityThatThisPedIsHolding` (0x5E0400, AL): tail call of
/// `CTaskSimpleHoldEntity::CanThrowEntity` on the hold task, false without one.
#[hook(addr = 0x5E0400, conv = thiscall, name = "CPed::CanThrowEntityThatThisPedIsHolding", expect = "56 8B F1 8B 86 7C 04 00 00", this = "CPed")]
pub fn can_throw_entity_that_this_ped_is_holding(this: GPtr<u8>) -> u8 {
    let t = holding_task(this);
    if t.is_null() { 0 } else { can_throw_entity(t) }
}

/// `CPed::IsPlayingHandSignal` (0x5E0460, AL)
#[hook(addr = 0x5E0460, conv = thiscall, name = "CPed::IsPlayingHandSignal", expect = "8B 89 7C 04 00 00", this = "CPed")]
pub fn is_playing_hand_signal(this: GPtr<u8>) -> u8 {
    (!find_active_task_by_type(task_manager(this), 0x1AA).is_null()) as u8
}

/// `CPed::StopPlayingHandSignal` (0x5E0480): `MakeAbortable(this, 1, null)` on the hand-signal task.
#[hook(addr = 0x5E0480, conv = thiscall, name = "CPed::StopPlayingHandSignal", expect = "56 8B F1 8B 86 7C 04 00 00", this = "CPed")]
pub fn stop_playing_hand_signal(this: GPtr<u8>) {
    let t = find_active_task_by_type(task_manager(this), 0x1AA);
    if !t.is_null() {
        task_make_abortable(t, this, 1, 0);
    }
}

/// `CPed::SetPedDefaultDecisionMaker` (0x5E06E0): -2 for players, -1 for mission peds, else the stat's
/// default (+0x32, signed byte).
#[hook(addr = 0x5E06E0, conv = thiscall, name = "CPed::SetPedDefaultDecisionMaker", expect = "8B 81 98 05 00 00", this = "CPed")]
pub fn set_ped_default_decision_maker(this: GPtr<u8>) {
    let intel = rd::<GPtr<u8>>(this, 0x47C);
    let t = if is_player(this) != 0 {
        -2
    } else if rd::<u8>(this, 0x484) == 2 {
        -1
    } else {
        i32::from(rd::<i8>(rd::<GPtr<u8>>(this, 0x59C), 0x32))
    };
    set_ped_decision_maker_type(intel, t);
}

/// `CPed::WorkOutHeadingForMovingFirstPerson(float heading)` (0x5E1A00, `ret 4`, ST0): for players with
/// player data: the pad's walk stick (left/right stored as f32, up/down as f32) gives the move direction
/// (+0x50 of the player data: -pi/2 / pi/2 for pure sideways, else `GetRadianAngleBetweenPoints(0, 0,
/// -lr, ud)`... as the bytes order the arguments); returns `LimitRadianAngle(heading + direction)`; other
/// peds: 0.
#[hook(addr = 0x5E1A00, conv = thiscall, name = "CPed::WorkOutHeadingForMovingFirstPerson", expect = "83 EC 08 56 8B F1", this = "CPed")]
pub fn work_out_heading_for_moving_first_person(this: GPtr<u8>, heading: f32) -> X87 {
    if is_player(this) == 0 || rd::<GPtr<u8>>(this, 0x480).is_null() {
        return x(0.0);
    }
    let lr = xi(i32::from(pad_get_ped_walk_left_right(get_pad(0)))).to_f32();
    let ud = xi(i32::from(pad_get_ped_walk_up_down(get_pad(0)))).to_f32();
    if !x(ud).fcom(x(0.0)).test_parity(0x44) {
        // up/down == 0.0
        if !x(lr).fcom(x(0.0)).test_any(0x41) {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x480), 0x50, 0xBFC9_0FDB);
        } else if !x(lr).fcom(x(0.0)).test_parity(5) {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x480), 0x50, 0x3FC9_0FDB);
        }
    } else {
        let pd = rd::<GPtr<u8>>(this, 0x480);
        let a = get_radian_angle_between_points(0, 0, x(lr).fchs().to_f32().to_bits(), ud.to_bits());
        wr::<f32>(pd, 0x50, X87::from_f64_exact(a).to_f32());
    }
    let pd = rd::<GPtr<u8>>(this, 0x480);
    let s = x(heading).fadd_m(rd::<f32>(pd, 0x50)).to_f32();
    X87::from_f64_exact(limit_radian_angle(s.to_bits()))
}

/// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60, `ret 4`, AL): types outside 0x16..0x20 → 1 (std);
/// players: 2 (pro) when the skill stat reaches the pro level of `GetWeaponInfo(type, 2)` (+0x34 as int),
/// else 1 when it reaches the std level, else 0 (the stat is read twice); others: 3 (special) for a cop
/// (type 6) with weapon 0x16, else +0x72C.
#[hook(addr = 0x5E3B60, conv = thiscall, name = "CPed::GetWeaponSkill-WeaponType", expect = "56 8B 74 24 08", this = "CPed")]
pub fn get_weapon_skill_type(this: GPtr<u8>, ty: i32) -> u8 {
    if !(0x16..=0x20).contains(&ty) {
        return 1;
    }
    let pt = rd::<i32>(this, 0x598);
    if pt == 0 || pt == 1 {
        let idx = get_skill_stat_index(ty);
        let req = rd::<i32>(get_weapon_info(ty, 2), 0x34);
        let stat = X87::from_f64_exact(get_stat_value(idx as u32));
        if !xi(req).fcom(stat).test_parity(0x41) {
            return 2;
        }
        let req = rd::<i32>(get_weapon_info(ty, 1), 0x34);
        let stat = X87::from_f64_exact(get_stat_value(idx as u32));
        return if !xi(req).fcom(stat).test_parity(0x41) { 1 } else { 0 };
    }
    if ty == 0x16 && pt == 6 {
        return 3;
    }
    rd::<u8>(this, 0x72C)
}

/// `CPed::ClearLook` (0x5E3FF0): `jmp CPed::ClearLookFlag`.
#[hook(addr = 0x5E3FF0, conv = thiscall, name = "CPed::ClearLook", expect = "E9 5B D9 FF FF 90", this = "CPed")]
pub fn clear_look(this: GPtr<u8>) {
    clear_look_flag_call(this);
}

/// `CPed::TurnBody` (0x5E4000, AL): look heading from the look-at entity (`GetRadianAngleBetweenPoints(
/// target, self)`), limited; brought within pi of the current heading (+-2pi); the aimed heading +0x55C
/// = it; a difference above 0.05 turns the current heading by 0.2 of it and returns false, else true.
#[hook(addr = 0x5E4000, conv = thiscall, name = "CPed::TurnBody", expect = "51 53 56 8B F1", this = "CPed")]
pub fn turn_body(this: GPtr<u8>) -> u8 {
    const PI: f32 = f32::from_bits(0x4049_0FDB);
    const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
    let e = rd::<GPtr<u8>>(this, 0x738);
    if !e.is_null() {
        let s = ent_pos(this).cast::<u32>();
        let t = ent_pos(e).cast::<u32>();
        let a = get_radian_angle_between_points(t.read(), t.wrapping_index(1).read(), s.read(), s.wrapping_index(1).read());
        wr::<f32>(this, 0x73C, X87::from_f64_exact(a).to_f32());
    }
    let mut f = Fpu::new();
    f.push(X87::from_f64_exact(limit_radian_angle(rd::<u32>(this, 0x73C))));
    f.fld(rd::<f32>(this, 0x558));
    f.fld(PI);
    f.fadd_st(1);
    f.fcomp_st(2);
    if !f.jp(5) {
        f.fxch(1);
        f.fsub_m(TWO_PI);
        f.fxch(1);
    } else {
        f.fld(PI);
        f.fsubr_st(1);
        f.fcomp_st(2);
        if !f.jne(0x41) {
            f.fxch(1);
            f.fadd_m(TWO_PI);
            f.fxch(1);
        }
    }
    f.fld_st(0);
    f.fsub_st(2);
    f.fld_st(2);
    wr::<f32>(this, 0x55C, f.fstp());
    f.fcom_m(0.0);
    f.fld_st(0);
    if !f.jp(5) {
        f.fchs();
    }
    f.fcomp_m(f32::from_bits(0x3D4C_CCCD));
    if !f.jne(0x41) {
        f.fmul_m(f32::from_bits(0x3E4C_CCCD));
        f.fsubp(1);
        wr::<f32>(this, 0x558, f.fstp());
        wr::<f32>(this, 0x73C, f.fstp());
        0
    } else {
        f.fstp_st(0);
        wr::<f32>(this, 0x558, f.fstp());
        wr::<f32>(this, 0x73C, f.fstp());
        1
    }
}

/// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0, `ret 4`): created-by, the default decision maker (players
/// -2, mission peds -1, else the stat's), and for mission peds seeing/hearing range 30 ([0x86C92C]) and,
/// unless a player, intelligence +0xC4/+0xC8 = 0.
#[hook(addr = 0x5E47E0, conv = thiscall, name = "CPed::SetCharCreatedBy", expect = "56 8B F1 8B 86 98 05 00 00", this = "CPed")]
pub fn set_char_created_by(this: GPtr<u8>, by: u32) {
    let by = by as u8;
    wr::<u8>(this, 0x484, by);
    let t = if is_player(this) != 0 {
        -2
    } else if by == 2 {
        -1
    } else {
        i32::from(rd::<i8>(rd::<GPtr<u8>>(this, 0x59C), 0x32))
    };
    set_ped_decision_maker_type(rd::<GPtr<u8>>(this, 0x47C), t);
    if rd::<u8>(this, 0x484) != 2 {
        return;
    }
    // [0x86C92C] (.rdata) = 30.0
    set_seeing_range(rd::<GPtr<u8>>(this, 0x47C), 0x41F0_0000);
    set_hearing_range(rd::<GPtr<u8>>(this, 0x47C), 0x41F0_0000);
    if is_player(this) == 0 {
        let intel = rd::<GPtr<u8>>(this, 0x47C);
        wr::<u32>(intel, 0xC8, 0);
        wr::<u32>(intel, 0xC4, 0);
    }
}

fn can_look(this: GPtr<u8>) -> bool {
    rd::<u32>(this, 0x46C) & 0x600 == 0 && !matches!(state(this), 0x36 | 0x37 | 0x3F)
}

/// `CPed::SetLook(float heading)` (0x5E79B0, `ret 4`): for peds in control: state 3 (look heading), then
/// the look flag as `SetLookFlag` when the look timer has expired (unsigned).
#[hook(addr = 0x5E79B0, conv = thiscall, name = "CPed::SetLook-Heading", expect = "56 8B F1 8B 86 6C 04 00 00", this = "CPed")]
pub fn set_look_heading(this: GPtr<u8>, heading: u32) {
    if !can_look(this) {
        return;
    }
    wr::<i32>(this, 0x530, 3);
    if rd::<u32>(this, 0x748) >= sa_core::timer::TIME_MS.get() {
        return;
    }
    wr::<u32>(this, 0x46C, (rd::<u32>(this, 0x46C) & !8) | 4);
    wr::<u32>(this, 0x73C, heading);
    let e = look_ref(this).read();
    if !e.is_null() {
        clean_up_old_reference(e, look_ref(this));
    }
    look_ref(this).write(GPtr::NULL);
    wr::<u32>(this, 0x748, 0);
    look_tail(this);
}

/// `CPed::SetLook(CEntity*)` (0x5E7A60, `ret 4`): state 2 (look entity) and the entity look flag.
#[hook(addr = 0x5E7A60, conv = thiscall, name = "CPed::SetLook-Entity", expect = "56 8B F1 8B 86 6C 04 00 00", this = "CPed")]
pub fn set_look_entity(this: GPtr<u8>, ent: GPtr<u8>) {
    if !can_look(this) {
        return;
    }
    wr::<i32>(this, 0x530, 2);
    if rd::<u32>(this, 0x748) >= sa_core::timer::TIME_MS.get() {
        return;
    }
    wr::<u32>(this, 0x46C, (rd::<u32>(this, 0x46C) & !8) | 4);
    let e = look_ref(this).read();
    if !e.is_null() {
        clean_up_old_reference(e, look_ref(this));
    }
    look_ref(this).write(ent);
    register_reference(ent, look_ref(this));
    wr::<u32>(this, 0x73C, 0x4974_23F0);
    wr::<u32>(this, 0x748, 0);
    look_tail(this);
}

/// `CPed::Look` (0x5E7B20): `jmp CPed::TurnBody`.
#[hook(addr = 0x5E7B20, conv = thiscall, name = "CPed::Look", expect = "E9 DB C4 FF FF 90", this = "CPed")]
pub fn look(this: GPtr<u8>) -> u8 {
    turn_body_call(this)
}

/// `CPed::GetPedTalking` (0x5EFF50): tail call into the speech entity (+0x294).
#[hook(addr = 0x5EFF50, conv = thiscall, name = "CPed::GetPedTalking", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn get_ped_talking(this: GPtr<u8>) -> u8 {
    speech_get_ped_talking(this.byte_add(0x294))
}

/// `CPed::EnablePedSpeech` (0x5EFF70)
#[hook(addr = 0x5EFF70, conv = thiscall, name = "CPed::EnablePedSpeech", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn enable_ped_speech(this: GPtr<u8>) {
    speech_enable(this.byte_add(0x294));
}

/// `CPed::EnablePedSpeechForScriptSpeech` (0x5EFF90)
#[hook(addr = 0x5EFF90, conv = thiscall, name = "CPed::EnablePedSpeechForScriptSpeech", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn enable_ped_speech_for_script_speech(this: GPtr<u8>) {
    speech_enable_script(this.byte_add(0x294));
}

/// `CPed::RemoveBodyPart(ePedNode, char)` (0x5F0140, `ret 8`): a node without a frame prints "Trying to
/// remove ped component"; with `CLocalisation::ShootLimbs`, flags +0x46C bit 15 and the part (+0x754).
#[hook(addr = 0x5F0140, conv = thiscall, name = "CPed::RemoveBodyPart", expect = "53 8B 5C 24 08", this = "CPed")]
pub fn remove_body_part(this: GPtr<u8>, node: i32, _dir: u32) {
    let n = rd::<GPtr<u8>>(this, 0x488 + (node as isize).wrapping_mul(4));
    if rd::<u32>(n, 0x10) == 0 {
        debug_printf(0x86C65C);
        return;
    }
    if shoot_limbs() != 0 {
        wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) | 0x8000);
        wr::<u8>(this, 0x754, node as u8);
    }
}

/// `CPed::PedCanPickUpPickUp` (0x455560, EAX): player 0's ped has neither task 0x2BD nor 0x640.
#[hook(addr = 0x455560, conv = cdecl, name = "CPed::PedCanPickUpPickUp", expect = "A1 98 CD B7 00")]
pub fn ped_can_pick_up_pick_up() -> u32 {
    let p = || rd::<GPtr<u8>>(g::<GPtr<u8>>(PLAYERS).read(), 0x47C);
    if !find_task_by_type(p(), 0x2BD).is_null() {
        return 0;
    }
    find_task_by_type(p(), 0x640).is_null() as u32
}

/// `CPed::RemoveLighting(bool)` (0x5533B0, `ret 4`)
#[hook(addr = 0x5533B0, conv = thiscall, name = "CPed::RemoveLighting", expect = "F7 41 40 00 00 00 20", this = "CPed")]
pub fn remove_lighting(this: GPtr<u8>, _reset: u32) {
    if rd::<u32>(this, 0x40) & 0x2000_0000 == 0 {
        remove_lights_affecting_object();
    }
    set_ambient_colours();
    deactivate_directional();
}

/// `CPed::StopNonPartialAnims` (0x5DED10): clears flag bit 0 (+0x2E) of every non-partial (bit 4 clear)
/// association.
#[hook(addr = 0x5DED10, conv = thiscall, name = "CPed::StopNonPartialAnims", expect = "8B 41 18 50 E8 C7 28 EF FF", this = "CPed")]
pub fn stop_non_partial_anims(this: GPtr<u8>) {
    let mut a = clump_get_first_association(rd::<GPtr<u8>>(this, 0x18));
    while !a.is_null() {
        let f = rd::<u16>(a, 0x2E);
        if (f as u8 >> 4) & 1 == 0 {
            wr::<u16>(a, 0x2E, f & 0xFFFE);
        }
        a = get_next_association(a);
    }
}

/// `CPed::RestartNonPartialAnims` (0x5DED50): sets flag bit 0 of every non-partial association.
#[hook(addr = 0x5DED50, conv = thiscall, name = "CPed::RestartNonPartialAnims", expect = "8B 41 18 50 E8 87 28 EF FF", this = "CPed")]
pub fn restart_non_partial_anims(this: GPtr<u8>) {
    let mut a = clump_get_first_association(rd::<GPtr<u8>>(this, 0x18));
    while !a.is_null() {
        let f = rd::<u16>(a, 0x2E);
        if (f as u8 >> 4) & 1 == 0 {
            wr::<u16>(a, 0x2E, f | 1);
        }
        a = get_next_association(a);
    }
}

/// `CPed::ResetGunFlashAlpha` (0x5DF4E0)
#[hook(addr = 0x5DF4E0, conv = thiscall, name = "CPed::ResetGunFlashAlpha", expect = "8B 81 F8 04 00 00", this = "CPed")]
pub fn reset_gun_flash_alpha(this: GPtr<u8>) {
    let fr = rd::<GPtr<u8>>(this, 0x4F8);
    if fr.is_null() {
        return;
    }
    let a = get_first_object(fr);
    if a.is_null() {
        return;
    }
    wr::<u8>(a, 2, 0);
    set_component_atomic_alpha(a, 0);
}

/// `CPed::ReleaseCoverPoint` (0x5E0270)
#[hook(addr = 0x5E0270, conv = thiscall, name = "CPed::ReleaseCoverPoint", expect = "56 8B F1 8B 8E 88 07 00 00", this = "CPed")]
pub fn release_cover_point(this: GPtr<u8>) {
    let cp = rd::<GPtr<u8>>(this, 0x788);
    if !cp.is_null() {
        release_cover_point_for_ped(cp, this);
        wr::<u32>(this, 0x788, 0);
    }
}

/// `CPed::SetPedState(ePedState)` (0x5E4500, `ret 4`): dying/dead releases the cover point and clears a
/// script blip (+0x474 bit 13) for the ped's pool handle.
#[hook(addr = 0x5E4500, conv = thiscall, name = "CPed::SetPedState", expect = "53 8B 5C 24 08", this = "CPed")]
pub fn set_ped_state(this: GPtr<u8>, s: i32) {
    if s == 0x37 || s == 0x36 {
        release_cover_point(this);
        if rd::<u32>(this, 0x474) & 0x2000 != 0 {
            clear_blip_for_entity(2, ped_pool_ref(this));
        }
    }
    wr::<i32>(this, 0x530, s);
}

/// `CPed::CalculateNewOrientation` (0x5E52E0): unless the world is paused-for-cutscene byte 0xA43088 == 1,
/// for peds in control: `SetOrientation(0, 0, heading)`.
#[hook(addr = 0x5E52E0, conv = thiscall, name = "CPed::CalculateNewOrientation", expect = "80 3D 88 30 A4 00 01", this = "CPed")]
pub fn calculate_new_orientation(this: GPtr<u8>) {
    if g::<u8>(0xA43088).read() == 1 || !can_look(this) {
        return;
    }
    set_orientation(this, 0, 0, rd::<u32>(this, 0x558));
}

/// `CPed::GetWeaponSkill()` (0x5E6580, AL): of the active slot's weapon type.
#[hook(addr = 0x5E6580, conv = thiscall, name = "CPed::GetWeaponSkill-Current", expect = "0F BE 81 18 07 00 00", this = "CPed")]
pub fn get_weapon_skill_current(this: GPtr<u8>) -> u8 {
    let slot = i32::from(rd::<i8>(this, 0x718));
    get_weapon_skill_type_call(this, rd::<i32>(weapon(this, slot), 0))
}

/// `CPed::CanSeeEntity(CEntity*, float threshold)` (0x5E0730, `ret 8`, AL): the angle to the entity
/// (`GetAngleBetweenPoints(target, self)` in degrees x [0x8595EC]) and the heading, each brought into
/// [0, 2pi), differ (absolute) by less than `threshold` or by more than `2pi - threshold`.
#[hook(addr = 0x5E0730, conv = thiscall, name = "CPed::CanSeeEntity", expect = "53 56 57 8B F9", this = "CPed")]
pub fn can_see_entity(this: GPtr<u8>, ent: GPtr<u8>, thr: f32) -> u8 {
    const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
    let s = ent_pos(this).cast::<u32>();
    let t = ent_pos(ent).cast::<u32>();
    let a = get_angle_between_points(t.read(), t.wrapping_index(1).read(), s.read(), s.wrapping_index(1).read());
    let mut a = X87::from_f64_exact(a).fmul_m(f32::from_bits(0x3C8E_FA35));
    if !a.fcom(x(TWO_PI)).test_any(0x41) {
        a = a.fsub_m(TWO_PI);
    } else if !a.fcom(x(0.0)).test_parity(5) {
        a = a.fadd_m(TWO_PI);
    }
    let mut h = x(rd::<f32>(this, 0x558));
    if !h.fcom(x(TWO_PI)).test_any(0x41) {
        h = h.fsub_m(TWO_PI);
    } else if !h.fcom(x(0.0)).test_parity(5) {
        h = h.fadd_m(TWO_PI);
    }
    let mut d = a.fsub(h);
    if !d.fcom(x(0.0)).test_parity(5) {
        d = d.fchs();
    }
    if !d.fcom(x(thr)).test_parity(5) {
        return 1;
    }
    let lim = x(TWO_PI).fsub_m(thr);
    (!d.fcom(lim).test_any(0x41)) as u8
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_PED_IS_INVOLVED_IN_CONVERSATION,
    HOOK_IS_WEARING_GOGGLES,
    HOOK_SET_STAY_IN_SAME_PLACE,
    HOOK_SET_PED_STATS,
    HOOK_SET_MOVE_STATE,
    HOOK_SET_MOVE_ANIM_SPEED,
    HOOK_CAN_USE_TORSO_WHEN_LOOKING,
    HOOK_CLEAR_AIM_FLAG,
    HOOK_GET_LOCAL_DIRECTION,
    HOOK_IS_PED_SHOOTABLE,
    HOOK_USE_GROUND_COL_MODEL,
    HOOK_CAN_PED_RETURN_TO_STATE,
    HOOK_CAN_SET_PED_STATE,
    HOOK_CAN_BE_ARRESTED,
    HOOK_CAN_STRAFE_OR_MOUSE_CONTROL,
    HOOK_CAN_BE_DELETED_EVEN_IN_VEHICLE,
    HOOK_SET_LOOK_TIMER,
    HOOK_IS_PLAYER,
    HOOK_RESTORE_HEADING_RATE,
    HOOK_RESTORE_HEADING_RATE_CB,
    HOOK_SET_RADIO_STATION,
    HOOK_IS_ALIVE,
    HOOK_CLEAR_LOOK_FLAG,
    HOOK_IS_PED_IN_CONTROL,
    HOOK_SET_WEAPON_SKILL,
    HOOK_CLEAR_ALL,
    HOOK_SET_IDLE,
    HOOK_SET_PED_ATOMIC_VISIBILITY_CB,
    HOOK_SET_LOOK_FLAG_HEADING,
    HOOK_SET_LOOK_FLAG_ENTITY,
    HOOK_SET_AIM_FLAG_ENTITY,
    HOOK_CAN_BE_DELETED,
    HOOK_GET_WEAPON_SLOT,
    HOOK_GRANT_AMMO,
    HOOK_SET_AMMO,
    HOOK_DO_WE_HAVE_WEAPON_AVAILABLE,
    HOOK_GET_BIKE_RIDING_SKILL,
    HOOK_GET_HOLDING_TASK,
    HOOK_GET_ENTITY_THAT_THIS_PED_IS_HOLDING,
    HOOK_CAN_THROW_ENTITY_THAT_THIS_PED_IS_HOLDING,
    HOOK_IS_PLAYING_HAND_SIGNAL,
    HOOK_STOP_PLAYING_HAND_SIGNAL,
    HOOK_SET_PED_DEFAULT_DECISION_MAKER,
    HOOK_WORK_OUT_HEADING_FOR_MOVING_FIRST_PERSON,
    HOOK_GET_WEAPON_SKILL_TYPE,
    HOOK_CLEAR_LOOK,
    HOOK_TURN_BODY,
    HOOK_SET_CHAR_CREATED_BY,
    HOOK_SET_LOOK_HEADING,
    HOOK_SET_LOOK_ENTITY,
    HOOK_LOOK,
    HOOK_GET_PED_TALKING,
    HOOK_ENABLE_PED_SPEECH,
    HOOK_ENABLE_PED_SPEECH_FOR_SCRIPT_SPEECH,
    HOOK_REMOVE_BODY_PART,
    HOOK_PED_CAN_PICK_UP_PICK_UP,
    HOOK_REMOVE_LIGHTING,
    HOOK_STOP_NON_PARTIAL_ANIMS,
    HOOK_RESTART_NON_PARTIAL_ANIMS,
    HOOK_RESET_GUN_FLASH_ALPHA,
    HOOK_RELEASE_COVER_POINT,
    HOOK_SET_PED_STATE,
    HOOK_CALCULATE_NEW_ORIENTATION,
    HOOK_GET_WEAPON_SKILL_CURRENT,
    HOOK_CAN_SEE_ENTITY,
];
