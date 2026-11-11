//! `CTaskSimpleCarDrive::ProcessPed` and `ProcessArmBopping` (batch `mvpk-ai-3`): the in-vehicle ped control and the
//! arm-out-of-the-window bopping. Ported from the 1.0 US bytes (gta-reversed `TaskSimpleCarDrive.cpp` as a reading aid; the bytes win).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVehicle::ClearWindowOpenFlag(uint8)` (0x6D30B0, `ret 4`)
    fn vehicle_clear_window_open_flag(v: GPtr<u8>, door: u32) = thiscall 0x6D30B0 => unported;
    /// `CVehicle::SetWindowOpenFlag(uint8)` (0x6D3080, `ret 4`)
    fn vehicle_set_window_open_flag(v: GPtr<u8>, door: u32) = thiscall 0x6D3080 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(const CVehicle*, const CPed*)` (0x64F110)
    fn car_enter_exit_compute_target_door_to_exit(v: GPtr<u8>, ped: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2250, `ret 4`)
    fn damage_get_door_status_component(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2250 => unported;
    /// `CCarEnterExit::CarHasDoorToClose(const CVehicle*, int32 door)` (0x64EE10)
    fn car_enter_exit_car_has_door_to_close(v: GPtr<u8>, door: u32) -> u8 = cdecl 0x64EE10 => unported;
    /// `CVehicle::CanPedLeanOut(CPed*)` (0x6D5CF0, `ret 4`)
    fn vehicle_can_ped_lean_out(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D5CF0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CTaskSimpleCarDrive::StartBopping(CPed*)` (0x642760, `ret 4`)
    fn car_drive_start_bopping(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x642760 => unported;
    /// `RtQuatRotate(RtQuat*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7EB7C0)
    fn rt_quat_rotate(q: GPtr<u8>, axis: u32, angle: f32, op: u32) -> GPtr<u8> = cdecl 0x7EB7C0 => unported;
}

/// The bopping switch value (0xC81324, float): above 0 every window closes and nothing bops
const BOPPING_BLOCK: usize = 0xC81324;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFOS: usize = 0xA9B0C8;
/// The vehicle anim group data (0xC1CDC8, 0x94 bytes per group; flags +0)
const VEHICLE_ANIM_GROUPS: usize = 0xC1CDC8;
/// 0.4 / 0.44 (low-rider groups, flag bit 3) and 0.39 / 0.46 (others): the seat height window for bopping
const K_BOP_LOW_MIN: usize = 0x858EE8;
const K_BOP_LOW_MAX: usize = 0x86EBF4;
const K_BOP_MIN: usize = 0x86EBF0;
const K_BOP_MAX: usize = 0x86EBEC;
/// 64.0 (0x859A44): the music level below which the arm turns
const K_BOP_LEVEL: usize = 0x859A44;
/// 0.75 (0x858F34) and 45.0 (0x858CB4): the arm angle from the beat
const K_BOP_BEAT_MIN: usize = 0x858F34;
const K_BOP_ANGLE_SCALE: usize = 0x858CB4;
/// The rotation axis (0x86EBE0)
const BOP_AXIS: u32 = 0x86EBE0;
/// 0.0 (0x858B50)
const K_ZERO: usize = 0x858B50;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;

/// `CTaskSimpleCarDrive::ProcessArmBopping(CPed*, bool enable, float level)` (0x642AE0, thiscall, `ret 0xC`). x87
/// compares.
///
/// Players: nothing. The switch at 0xC81324 above 0 → the vehicle's (+0x58C) windows 10, 8, 11, 9 close and `enable`
/// is cleared; else the ped's seat (driver +0x460 → 10, +0x464 → 8, +0x468 → 11, +0x46C → 9) opens its window. The arm
/// anim is 0xA3 for the seats +0x464 / +0x46C, else 0xA2; `assoc = RpAnimBlendClumpGetAssociation(clump, anim)`.
///
/// - Not `enable`: an assoc gets blend delta -4.0; flag +0x5C bit 1 cleared.
/// - Not bopping yet (flag bit 1 clear): `GetRandomNumberInRange(0, 1000)` above 995 and either the vehicle's +0x594 is
///   non-zero, or (the exit door `ComputeTargetDoorToExit` has status 0, `!CarHasDoorToClose` and `CanPedLeanOut`) and the
///   model's seat offset (+0x5C → +0x78) is not zero, the vehicle +0x429 bit 3 is clear and its height above the model's
///   +0x30 point (z) lies within the anim group's window (flags +0 bit 3: [0.4, 0.44], else [0.39, 0.46]) →
///   `BlendAnimation(clump, 0, anim, 4.0)`; the task's +0x20 at -1 → `StartBopping(ped)` and +0x48 = +0x20, else +0x48 =
///   now; flag bit 1 set.
/// - Bopping: no assoc → flag bit 1 cleared. `level` below 64 → `RtQuatRotate(frame +0x10 of +0x49C (anim 0xA2) or
///   +0x4A0, axis 0x86EBE0, f32(-(+0x2C below 0.75 ? 0 : (+0x2C - 0.75) * 45)), 1)` and ped +0x470 |= 0x4000. More than
///   5000 ms since +0x48 and `GetRandomNumberInRange(0, 1000)` above 995 → assoc blend delta -4.0, flag bit 1 cleared.
///
/// The original keeps the anim id in its own `ped` argument slot and clears `enable` in its own slot (both popped by
/// `ret 0xC`).
#[hook(addr = 0x642ae0, conv = thiscall, name = "CTaskSimpleCarDrive::ProcessArmBopping", expect = "83 EC 18 56 8B 74 24 20",
       unverified = "mvpk-ai-3", route = "L2", hazards = "stack-residue,heap-writes", recs = "driving",
       note = "argument slots overwritten (see doc); rotates the ped's arm frame quaternion")]
pub fn car_drive_process_arm_bopping(this: GPtr<u8>, ped: GPtr<u8>, enable: u8, level: f32) {
    if ped_is_player(ped) != 0 {
        return;
    }
    let mut enable = enable;
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if !x(g::<f32>(BOPPING_BLOCK).read()).fcom(x(g::<f32>(K_ZERO).read())).test_any(0x41) {
        for door in [10, 8, 11, 9] {
            vehicle_clear_window_open_flag(rd::<GPtr<u8>>(ped, 0x58C), door);
        }
        enable = 0;
    } else {
        let door = if rd::<GPtr<u8>>(veh, 0x460) == ped {
            Some(10)
        } else if rd::<GPtr<u8>>(veh, 0x464) == ped {
            Some(8)
        } else if rd::<GPtr<u8>>(veh, 0x468) == ped {
            Some(11)
        } else if rd::<GPtr<u8>>(veh, 0x46C) == ped {
            Some(9)
        } else {
            None
        };
        if let Some(d) = door {
            vehicle_set_window_open_flag(veh, d);
        }
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let anim = if rd::<GPtr<u8>>(veh, 0x464) == ped || rd::<GPtr<u8>>(veh, 0x46C) == ped { 0xA3 } else { 0xA2 };
    let assoc = clump_get_association(rd::<u32>(ped, 0x18), anim);
    if enable == 0 {
        if !assoc.is_null() {
            wr::<u32>(assoc, 0x1C, 0xC080_0000);
        }
        wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) & 0xFD);
        return;
    }
    if rd::<u8>(this, 0x5C) & 2 != 0 {
        if assoc.is_null() {
            wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) & 0xFD);
            return;
        }
        if !x(level).fcom(x(g::<f32>(K_BOP_LEVEL).read())).test_parity(5) {
            let beat = rd::<f32>(this, 0x2C);
            let min = g::<f32>(K_BOP_BEAT_MIN).read();
            let angle = if x(beat).fcom(x(min)).c0() {
                x(g::<f32>(K_ZERO).read())
            } else {
                x(beat).fsub_m(min).fmul_m(g::<f32>(K_BOP_ANGLE_SCALE).read())
            };
            let frame = rd::<GPtr<u8>>(ped, if anim == 0xA2 { 0x49C } else { 0x4A0 });
            rt_quat_rotate(rd::<GPtr<u8>>(frame, 0x10), BOP_AXIS, angle.fchs().to_f32(), 1);
            wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x4000);
        }
        if g::<u32>(TIMER_TIME_MS).read().wrapping_sub(rd::<u32>(this, 0x48)) > 5000
            && general_random_int_in_range(0, 1000) > 0x3E3
        {
            wr::<u32>(assoc, 0x1C, 0xC080_0000);
            wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) & 0xFD);
        }
        return;
    }
    if general_random_int_in_range(0, 1000) <= 0x3E3 {
        return;
    }
    let v = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(v, 0x594) == 0 {
        let door = car_enter_exit_compute_target_door_to_exit(v, ped);
        let can = damage_get_door_status_component(rd::<GPtr<u8>>(this, 8).byte_add(0x5A0), door) == 0
            && car_enter_exit_car_has_door_to_close(rd::<GPtr<u8>>(this, 8), door) == 0
            && vehicle_can_ped_lean_out(rd::<GPtr<u8>>(this, 8), ped) != 0;
        let v = rd::<GPtr<u8>>(this, 8);
        let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFOS + 4 * rd::<i16>(v, 0x22) as isize as usize).read();
        let data = rd::<GPtr<u8>>(mi, 0x5C);
        let seat = [rd::<f32>(data, 0x78), rd::<f32>(data, 0x7C), rd::<f32>(data, 0x80)];
        let z = x(g::<f32>(K_ZERO).read());
        if seat.iter().all(|s| !x(*s).fcom(z).test_parity(0x44)) {
            return;
        }
        if rd::<u8>(v, 0x429) & 8 != 0 {
            return;
        }
        let h = x(seat[2]).fsub_m(rd::<f32>(data, 0x38));
        let group = usize::from(rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE));
        let low = (GPtr::<u32>::from_addr(VEHICLE_ANIM_GROUPS + group * 0x94).read() >> 3) & 1 != 0;
        let (lo, hi) = if low { (K_BOP_LOW_MIN, K_BOP_LOW_MAX) } else { (K_BOP_MIN, K_BOP_MAX) };
        if !h.fcom(x(g::<f32>(lo).read())).test_parity(5) {
            return;
        }
        if !h.fcom(x(g::<f32>(hi).read())).test_any(0x41) || !can {
            return;
        }
    }
    anim_blend_animation(rd::<u32>(ped, 0x18), 0, anim, 0x4080_0000);
    if rd::<i32>(this, 0x20) == -1 {
        car_drive_start_bopping(this, ped);
        wr::<u32>(this, 0x48, rd::<u32>(this, 0x20));
    } else {
        wr::<u32>(this, 0x48, g::<u32>(TIMER_TIME_MS).read());
    }
    wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) | 2);
}

orig! {
    /// `CAudioEngine::HasRadioRetuneJustStarted()` (0x506FE0)
    fn audio_has_radio_retune_just_started(ae: GPtr<u8>) -> u8 = thiscall 0x506FE0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CUpsideDownCarCheck::IsCarUpsideDown(const CVehicle*)` (0x463830, `ret 4`)
    fn upside_down_is_car_upside_down(c: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x463830 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0, `ret 4`)
    fn membership_is_leader(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CEventCarUpsideDown::CEventCarUpsideDown(CVehicle*)` (0x4B1CC0, `ret 4`)
    fn event_car_upside_down_ctor(e: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1CC0 => unported;
    /// `CEventCarUpsideDown::~CEventCarUpsideDown()` (0x4B1D50)
    fn event_car_upside_down_dtor(e: GPtr<u8>) = thiscall 0x4B1D50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn vehicle_is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `CEventCopCarBeingStolen::CEventCopCarBeingStolen(CPed*, CVehicle*)` (0x4B1740, `ret 8`)
    fn event_cop_car_being_stolen_ctor(e: GPtr<u8>, ped: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1740 => unported;
    /// `CEventCopCarBeingStolen::~CEventCopCarBeingStolen()` (0x4B17F0)
    fn event_cop_car_being_stolen_dtor(e: GPtr<u8>) = thiscall 0x4B17F0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CTaskSimpleGangDriveBy::CTaskSimpleGangDriveBy(CEntity*, const CVector*, float, int8, int8, bool)` (0x6217D0,
    /// `ret 0x18`; the bool goes as a whole dword)
    fn gang_drive_by_ctor(t: GPtr<u8>, target: GPtr<u8>, pos: u32, range: u32, freq: u32, style: u32, rhs: u32) -> GPtr<u8> = thiscall 0x6217D0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, idx: u32, t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CDoor::IsClosed()` (0x6F4800)
    fn door_is_closed(d: GPtr<u8>) -> u8 = thiscall 0x6F4800 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0; int16 in AX)
    fn pad_get_accelerate(pad: GPtr<u8>) -> u32 = thiscall 0x5403F0 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80; int16 in AX)
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> u32 = thiscall 0x53FB80 => unported;
    /// `CPad::GetBrake()` (0x540080; int16 in AX)
    fn pad_get_brake(pad: GPtr<u8>) -> u32 = thiscall 0x540080 => unported;
    /// `CVehicleAnimGroupData::GetGroupForAnim(int32 group, int32 anim)` (0x639FC0)
    fn vehicle_anim_group_for_anim(group: u32, anim: u32) -> u32 = cdecl 0x639FC0 => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_vehicle_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, time: u32) = thiscall vslot 0x70 => unported;
    /// `CVehicle::IsDoorMissing(eDoors)` (vtable +0x98)
    fn v_vehicle_is_door_missing(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// `CVehicle::ProcessDrivingAnims(CPed*, bool)` (vtable +0xB8; the bool goes as a whole dword)
    fn v_vehicle_process_driving_anims(v: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall vslot 0xB8 => unported;
    /// `CVehicle::GetRideAnimData()` (vtable +0xBC)
    fn v_vehicle_get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230, `ret 4`)
    fn damage_get_door_status(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2230 => unported;
    /// `CVehicle::SetGettingOutFlags(uint8)` (0x6D3020, `ret 4`)
    fn vehicle_set_getting_out_flags(v: GPtr<u8>, f: u32) = thiscall 0x6D3020 => unported;
    /// 0x642700 (`CTaskSimpleCarDrive`, `ret 4`): starts the door-close after getting out (this, ped)
    fn car_drive_642700(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x642700 => unported;
    /// `CTaskSimpleCarDrive::ProcessBopping(CPed*, bool)` (0x642E70, `ret 8`)
    fn car_drive_process_bopping(t: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x642E70 => unported;
    /// `CVehicle::IsPassenger(const CPed*)` (0x6D1BD0, `ret 4`)
    fn vehicle_is_passenger(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn game_logic_is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPed::GetEntityThatThisPedIsHolding()` (0x5E02E0)
    fn ped_get_held_entity(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, blend: i32) = thiscall 0x618280 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskSimpleHoldEntity::PlayAnim(AnimationId, AssocGroupId)` (0x691720, `ret 8`)
    fn hold_entity_play_anim(t: GPtr<u8>, anim: u32, group: u32) = thiscall 0x691720 => unported;
    /// `CTaskSimpleCarDrive::TriggerIK(CPed*)` (0x63C500, `ret 4`)
    fn car_drive_trigger_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63C500 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(s: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, delay: u32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexCarDriveWander::CTaskComplexCarDriveWander(CVehicle*, int32 style, float speed)` (0x63CB10, `ret 0xC`)
    fn car_drive_wander_ctor(t: GPtr<u8>, v: GPtr<u8>, style: u32, speed: u32) -> GPtr<u8> = thiscall 0x63CB10 => unported;
}

/// `AudioEngine` (0xB6BC90)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// The script upside-down car checker (0xA4892C)
const UPSIDE_DOWN_CARS: usize = 0xA4892C;
/// `CTimer::ms_fTimeStep` (0xB7CB5C), `m_FrameCounter` (0xB7CB4C)
const TIMER_TIME_STEP: usize = 0xB7CB5C;
const TIMER_FRAME_COUNTER: usize = 0xB7CB4C;
/// 0.02 (0x858B38) and 1000.0 (0x858C4C): milliseconds per time step
const K_MS_A: usize = 0x858B38;
const K_MS_B: usize = 0x858C4C;
/// The drive-by cheat/switch byte (0x969179)
const DRIVE_BY_SWITCH: usize = 0x969179;
/// `TheCamera.m_nActiveCam` (0xB6F081), `m_aCams[0].m_nMode` (0xB6F1A8; 0x238 per cam)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
const CAMERA_CAM0_MODE: usize = 0xB6F1A8;
/// The held model the passenger phone anim plays for (0x8CD72C, u16)
const HELD_PHONE_MODEL: usize = 0x8CD72C;
/// `g_ikChainMan` (0xC15448)
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// 0.7 (0x858CB0) and 0.1 (0x858B1C): the driver speed limits of the passenger comments
const K_FAST: usize = 0x858CB0;
const K_SLOW: usize = 0x858B1C;

/// The 2D length of the vehicle's move speed (+0x44, +0x48): `sqrt(x² + y²)` (register).
fn move_speed_2d(v: GPtr<u8>) -> X87 {
    let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
    x(sx).fmul_m(sx).fadd(x(sy).fmul_m(sy)).fsqrt()
}

/// `fild` of an int16 compared with 0: not equal (`test ah, 0x44` / `jp`).
fn axis_nonzero(v: u32) -> bool {
    X87::from_i32(i32::from(v as i16)).fcom(x(g::<f32>(K_ZERO).read())).test_parity(0x44)
}

/// `CTaskSimpleCarDrive::ProcessPed(CPed*)` (0x644470, thiscall, `ret 4`). Returns true (AL = 1) when the ped is not in
/// the task's vehicle; the EAX upper bytes are whatever the last call left (the port returns AL zero-extended).
///
/// `retune = AudioEngine.HasRadioRetuneJustStarted()`. Flag +0x5C bit 2, the ped in a vehicle (+0x46C bit 8, +0x58C)
/// other than +8 → +8 re-referenced to it. No vehicle +8 or not in one → in-vehicle flag cleared, an IK look aborted
/// (250 ms), true.
///
/// - The script upside-down check (0xA4892C) on +8: another ped than player -1 with player -1 in that vehicle →
///   `Say(0x22)`; unless in a group not led by the ped, +0x4C += `_ftol(time step * 0.02 * 1000)` and above 2000 → a
///   stack `CEventCarUpsideDown(+8)` into the ped's event group. Not upside down → +0x4C = 0.
/// - A player in a law-enforcement vehicle, the timer +0x50 unset or out of time → timer (now, 2000) and a stack
///   `CEventCopCarBeingStolen(ped, +8)` into the global event group.
/// - Bikes (+0x590 == 9), +0x594 == 2 or model 0x213 → ped +0x478 |= 0x100000.
/// - A player driver: the drive-by switch 0x969179, a weapon (+0x610, ammo +0x61C > 0) and `CanPedLeanOut` → a new
///   `CTaskSimpleGangDriveBy(NULL, NULL, 100, 100, 8, false)` by a stack `CEventScriptCommand(3, task)`. A vehicle
///   (model not 0x23A) of type 6 whose door +0x608 is open: no accelerate/steer/brake input → `ProcessOpenDoor(ped, 10,
///   GetGroupForAnim(group, 0x182), 0x182, 1.0)`. Else a car (+0x590 == 0) with door 2 status 1, not missing: the anim
///   0x182 playing, or the vehicle's +0x487 / +0x486 bit 0 → `ProcessOpenDoor(.., anim group of the assoc's +0x2C,
///   0x182, assoc time +0x20)`; else without input → `SetGettingOutFlags(1)`, flag +0x5C |= 0x20, 0x642700(ped), false;
///   then `ProcessBopping(ped, false)`. Other vehicles: vtable +0xB8 `(ped, retune)` and `ProcessBopping(ped, true)`.
/// - Others: the driver as above; a passenger: once (flags 0x18 clear) the ride anim data's group anim 0xCD (blend
///   8.0) and flag 0x10; `ProcessBopping(ped, true)`; a co-op player passenger with a weapon who can lean out in camera
///   mode 0x31 → the drive-by task (right-hand side unless in seat +0x468) by script command event.
/// - Holding the model at 0x8CD72C, not looking and `GetRandomNumberInRange(0, 60) == 15` with an active hold task
///   (0x133) → `PlayAnim(passenger ? 0xAA : 0xA9, 0)`. Anim 0xA9 playing → abort the IK look and +0x20 = -1; else
///   `TriggerIK(ped)`.
/// - A random ped (+0x484 == 1) without a group as front passenger of a driverless vehicle: after 4 s (timer +0x14) a
///   sequence of `CTaskComplexLeaveCar(+8, 0, 0, true, false)` and `CTaskComplexCarDriveWander(+8, 0, 10)` by script
///   command; otherwise the timer flag +0x1C is cleared.
/// - Every 0x4000 frames (by the ped's +0x20 seed) a passenger of a player driver comments on the speed (`Say` 0x20 above
///   0.7, 0x27 below 0.1, then 0x29). False.
///
/// The original passes `retune` and the drive-by side as bytes over stack dwords (upper bytes uninitialised for the
/// latter).
#[hook(addr = 0x644470, conv = thiscall, name = "CTaskSimpleCarDrive::ProcessPed", expect = "6A FF 68 7F 23 84 00",
       unverified = "mvpk-ai-3", route = "L2", hazards = "ax-al-return,uninit-locals,heap-writes,x87-transcendental", recs = "driving",
       note = "stack events (CEventCarUpsideDown, CEventCopCarBeingStolen, CEventScriptCommand) live on the Rust stack; the passenger drive-by side byte goes over an uninitialised dword; FSQRT of the speed")]
pub fn car_drive_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let retune = audio_has_radio_retune_just_started(GPtr::from_addr(AUDIO_ENGINE));
    let in_vehicle = |p: GPtr<u8>| rd::<u32>(p, 0x46C) & 0x100 != 0;
    if rd::<u8>(this, 0x5C) & 4 != 0 && in_vehicle(ped) {
        let pv = rd::<GPtr<u8>>(ped, 0x58C);
        if !pv.is_null() && pv != rd::<GPtr<u8>>(this, 8) {
            let slot = this.byte_add(8);
            if !rd::<GPtr<u8>>(this, 8).is_null() {
                entity_clean_up_old_reference(rd::<GPtr<u8>>(this, 8), slot);
            }
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<GPtr<u8>>(this, 8, v);
            if !v.is_null() {
                entity_register_reference(v, slot);
            }
        }
    }
    if rd::<GPtr<u8>>(this, 8).is_null() || !in_vehicle(ped) {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !0x100);
        let ik = GPtr::from_addr(IK_CHAIN_MANAGER);
        if ik_is_looking(ik, ped) != 0 {
            ik_abort_look_at(ik, ped, 250);
        }
        return 1;
    }
    let mut ev = [0u32; 0x10];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    let grp = ped_groups_get_peds_group(ped);
    if upside_down_is_car_upside_down(GPtr::from_addr(UPSIDE_DOWN_CARS), rd::<GPtr<u8>>(this, 8)) != 0 {
        if ped != world_find_player_ped(-1)
            && in_vehicle(world_find_player_ped(-1))
            && rd::<GPtr<u8>>(world_find_player_ped(-1), 0x58C) == rd::<GPtr<u8>>(this, 8)
        {
            ped_say(ped, 0x22, 0, 0x3F80_0000, 0, 0, 0);
        }
        if grp.is_null() || membership_is_leader(grp.byte_add(8), ped) != 0 {
            let add = x(g::<f32>(TIMER_TIME_STEP).read()).fmul_m(g::<f32>(K_MS_A).read()).fmul_m(g::<f32>(K_MS_B).read()).ftol() as u32;
            let t = rd::<u32>(this, 0x4C).wrapping_add(add);
            wr::<u32>(this, 0x4C, t);
            if t > 2000 {
                event_car_upside_down_ctor(ev_p, rd::<GPtr<u8>>(this, 8));
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
                event_car_upside_down_dtor(ev_p);
            }
        }
    } else {
        wr::<u32>(this, 0x4C, 0);
    }
    if ped_is_player(ped) != 0 && vehicle_is_law_enforcement_vehicle(rd::<GPtr<u8>>(this, 8)) != 0
        && (rd::<u8>(this, 0x58) == 0 || task_timer_is_out_of_time(this.byte_add(0x50)) != 0)
    {
        wr::<u32>(this, 0x50, g::<u32>(TIMER_TIME_MS).read());
        wr::<u32>(this, 0x54, 2000);
        wr::<u8>(this, 0x58, 1);
        event_cop_car_being_stolen_ctor(ev_p, ped, rd::<GPtr<u8>>(this, 8));
        event_group_add(get_event_global_group(), ev_p, 0);
        event_cop_car_being_stolen_dtor(ev_p);
    }
    let v = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(v, 0x590) == 9 || rd::<u32>(v, 0x594) == 2 || rd::<u16>(v, 0x22) == 0x213 {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x10_0000);
    }
    let drive_by = |rhs: u32| {
        let t = task_new(0x44);
        let t = if t.is_null() { t } else { gang_drive_by_ctor(t, GPtr::NULL, 0, 0x42C8_0000, 100, 8, rhs) };
        let mut e = [0u32; 0x10];
        let e_p: GPtr<u8> = GPtr::from_raw(e.as_mut_ptr()).cast();
        event_script_command_ctor(e_p, 3, t, 0);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e_p, 0);
        event_script_command_dtor(e_p);
    };
    let can_drive_by = || {
        rd::<u32>(ped, 0x610) != 0 && rd::<i32>(ped, 0x61C) > 0 && vehicle_can_ped_lean_out(rd::<GPtr<u8>>(this, 8), ped) != 0
    };
    let driver_anims = |ped: GPtr<u8>| {
        v_vehicle_process_driving_anims(rd::<GPtr<u8>>(this, 8), ped, u32::from(retune));
        car_drive_process_bopping(this, ped, 1);
    };
    if ped_is_player(ped) != 0 && rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x460) == ped {
        let pad = player_get_pad(ped);
        if g::<u8>(DRIVE_BY_SWITCH).read() != 0 && can_drive_by() {
            drive_by(0);
        }
        let any_input = |pad: GPtr<u8>| {
            !pad.is_null()
                && (axis_nonzero(pad_get_accelerate(pad)) || axis_nonzero(pad_get_steering_left_right(pad)) || axis_nonzero(pad_get_brake(pad)))
        };
        let pv = rd::<GPtr<u8>>(ped, 0x58C);
        if rd::<u16>(pv, 0x22) != 0x23A && rd::<u32>(pv, 0x590) == 6 && door_is_closed(pv.byte_add(0x608)) == 0 {
            if !any_input(pad) {
                let group = u32::from(rd::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x384), 0xDE));
                let g = vehicle_anim_group_for_anim(group, 0x182);
                v_vehicle_process_open_door(rd::<GPtr<u8>>(ped, 0x58C), ped, 10, g, 0x182, 0x3F80_0000);
            }
        } else if rd::<u32>(pv, 0x590) != 0 || damage_get_door_status(pv.byte_add(0x5A0), 2) != 1 {
            driver_anims(ped);
        } else if v_vehicle_is_door_missing(rd::<GPtr<u8>>(ped, 0x58C), 2) == 0 {
            let assoc = clump_get_association(rd::<u32>(ped, 0x18), 0x182);
            let pv = rd::<GPtr<u8>>(ped, 0x58C);
            let held = rd::<u8>(pv, 0x487) & 1 != 0 || rd::<u8>(pv, 0x486) & 1 != 0;
            if !held && assoc.is_null() && !any_input(pad) {
                vehicle_set_getting_out_flags(rd::<GPtr<u8>>(ped, 0x58C), 1);
                wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) | 0x20);
                car_drive_642700(this, ped);
                return 0;
            }
            if !assoc.is_null() {
                let group = u32::from(rd::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x384), 0xDE));
                let g = vehicle_anim_group_for_anim(group, rd::<i16>(assoc, 0x2C) as i32 as u32);
                let time = rd::<u32>(assoc, 0x20);
                v_vehicle_process_open_door(rd::<GPtr<u8>>(ped, 0x58C), ped, 10, g, 0x182, time);
            }
            car_drive_process_bopping(this, ped, 0);
        }
    } else if rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x460) == ped {
        driver_anims(ped);
    } else if vehicle_is_passenger(rd::<GPtr<u8>>(this, 8), ped) != 0 {
        if rd::<u8>(this, 0x5C) & 0x18 == 0 {
            let data = v_vehicle_get_ride_anim_data(rd::<GPtr<u8>>(this, 8));
            if !data.is_null() {
                let d = v_vehicle_get_ride_anim_data(rd::<GPtr<u8>>(this, 8));
                anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(d, 0), 0xCD, 0x4100_0000);
            }
            wr::<u8>(this, 0x5C, rd::<u8>(this, 0x5C) | 0x10);
        }
        car_drive_process_bopping(this, ped, 1);
        if ped_is_player(ped) != 0 && game_logic_is_coop_game_going_on() != 0 && can_drive_by() && {
            let active = usize::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
            g::<u16>(CAMERA_CAM0_MODE + active * 0x238).read() == 0x31
        } {
            let rhs = u32::from(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x468) != ped);
            drive_by(rhs);
        }
    }
    let held = ped_get_held_entity(ped);
    let ik = GPtr::from_addr(IK_CHAIN_MANAGER);
    if !held.is_null()
        && i32::from(rd::<i16>(held, 0x22)) == i32::from(g::<u16>(HELD_PHONE_MODEL).read())
        && ik_is_looking(ik, ped) == 0
        && general_random_int_in_range(0, 60) == 15
    {
        let t = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x133);
        if !t.is_null() {
            let anim = if vehicle_is_passenger(rd::<GPtr<u8>>(this, 8), ped) != 0 { 0xAA } else { 0xA9 };
            hold_entity_play_anim(t, anim, 0);
        }
    }
    if !clump_get_association(rd::<u32>(ped, 0x18), 0xA9).is_null() {
        if ik_is_looking(ik, ped) != 0 {
            ik_abort_look_at(ik, ped, 250);
        }
        if rd::<i32>(this, 0x20) != -1 {
            wr::<i32>(this, 0x20, -1);
        }
    } else {
        car_drive_trigger_ik(this, ped);
    }
    if rd::<u8>(ped, 0x484) == 1 && ped_groups_get_peds_group(ped).is_null() {
        let v = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(v, 0x460) == 0 && rd::<GPtr<u8>>(v, 0x464) == ped {
            if rd::<u8>(this, 0x1C) == 0 {
                wr::<u32>(this, 0x14, g::<u32>(TIMER_TIME_MS).read());
                wr::<u32>(this, 0x18, 4000);
                wr::<u8>(this, 0x1C, 1);
            }
            if task_timer_is_out_of_time(this.byte_add(0x14)) != 0 {
                let s = task_new(0x40);
                let seq = if s.is_null() { s } else { sequence_ctor(s) };
                let t = task_new(0x34);
                let t = if t.is_null() { t } else { leave_car_ctor(t, rd::<GPtr<u8>>(this, 8), 0, 0, 1, 0) };
                sequence_add_task(seq, t);
                let w = task_new(0x24);
                let w = if w.is_null() { w } else { car_drive_wander_ctor(w, rd::<GPtr<u8>>(this, 8), 0, 0x4120_0000) };
                sequence_add_task(seq, w);
                let mut e = [0u32; 0x10];
                let e_p: GPtr<u8> = GPtr::from_raw(e.as_mut_ptr()).cast();
                event_script_command_ctor(e_p, 3, seq, 0);
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e_p, 0);
                event_script_command_dtor(e_p);
            }
        } else {
            wr::<u8>(this, 0x1C, 0);
        }
    }
    if (u32::from(rd::<u16>(ped, 0x20)).wrapping_add(g::<u32>(TIMER_FRAME_COUNTER).read())) & 0x3FFF == 0 {
        let driver = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x460);
        if !driver.is_null() && driver != ped && ped_is_player(driver) != 0 {
            if !move_speed_2d(rd::<GPtr<u8>>(this, 8)).fcom(x(g::<f32>(K_FAST).read())).test_any(0x41) {
                ped_say(ped, 0x20, 0, 0x3F80_0000, 0, 0, 0);
            }
            if !move_speed_2d(rd::<GPtr<u8>>(this, 8)).fcom(x(g::<f32>(K_SLOW).read())).test_parity(5) {
                ped_say(ped, 0x27, 0, 0x3F80_0000, 0, 0, 0);
            }
            ped_say(ped, 0x29, 0, 0x3F80_0000, 0, 0, 0);
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_CAR_DRIVE_PROCESS_ARM_BOPPING, HOOK_CAR_DRIVE_PROCESS_PED];
