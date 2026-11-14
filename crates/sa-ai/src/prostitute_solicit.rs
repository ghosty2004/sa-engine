//! `CTaskComplexProstituteSolicit` (PEDHIT: reached by the wander scan of a ped near the player's car, then her
//! task's sub-tasks): `IsTaskValid` and `CreateSubTask`. Ported from the 1.0 US bytes (`IsTaskValid` 0x661BB0 with
//! the displaced fragment 0x40160B, which only loads `ped+0x46C`; `CreateSubTask` 0x666360 leaves through the
//! fragment 0x401BAF, which only reloads the SEH link).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn vehicle_get_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::IsUpsideDown()` (0x6D1D90)
    fn vehicle_is_upside_down(v: GPtr<u8>) -> u8 = thiscall 0x6D1D90 => unported;
    /// `CVehicle::IsOnItsSide()` (0x6D1DD0)
    fn vehicle_is_on_its_side(v: GPtr<u8>) -> u8 = thiscall 0x6D1DD0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, cdecl, returns out)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32, int32, bool, bool)` (0x63B8C0)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32, bool)` (0x640340)
    fn enter_car_as_passenger_ctor(t: GPtr<u8>, v: GPtr<u8>, seat: u32, b: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexCarDrive::CTaskComplexCarDrive(CVehicle*)` (0x63C9D0)
    fn car_drive_ctor(t: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x63C9D0 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord` (entity) constructor (0x66B890, `ret 0xC`)
    fn turn_ctor_entity(t: GPtr<u8>, e: GPtr<u8>, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CMatrix::Invert(const CMatrix& in, CMatrix& out)` (0x59B920, cdecl)
    fn matrix_invert_into(inp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CCarEnterExit::GetPositionToOpenCarDoor(CVector* out, CVehicle*, int32 door)` (0x64E740, cdecl)
    fn position_to_open_car_door(out: GPtr<u8>, v: GPtr<u8>, door: u32) -> GPtr<u8> = cdecl 0x64E740 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, returns out)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>` constructor (0x661DC0, `ret 0x20`)
    #[allow(clippy::too_many_arguments)]
    fn seek_entity_xy_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32, b7: u32, b8: u32) -> GPtr<u8> = thiscall 0x661DC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// The position an entity's placement gives (`m_matrix ? &m_matrix->pos : &m_placement.pos`).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexProstituteSolicit::IsTaskValid(CPed* prostitute, CPed* client)` (0x661BB0, cdecl; AL): true only
/// when `client` is the player (`FindPlayerPed(-1)`, non-null) in a vehicle (+0x46C bit 8, +0x58C), without
/// player-data flag +0x474 bit 6, whose player data's +0xA4 is unset or `prostitute`; the vehicle is a car
/// (`GetVehicleAppearance() == 1`), neither upside down nor on its side; the client's simplest active task is
/// 0x2C5 (the active task is not null-checked); the client drives it (+0x460); the prostitute is not in a vehicle
/// and its +0x484 byte is 0, or she is in this one and it is 1; the vehicle's +0x488 byte is non-zero and its
/// handling flags +0xCC bit 9 are clear; `|client - prostitute|² <= 100.0f` (0x86F840; NaN passes); no mission
/// and no co-op game.
#[hook(addr = 0x661BB0, conv = cdecl, name = "CTaskComplexProstituteSolicit::IsTaskValid",
       expect = "83 EC 0C 56 57 6A FF",
       unverified = "pedhit-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn prostitute_solicit_is_task_valid(prostitute: GPtr<u8>, client: GPtr<u8>) -> u8 {
    if find_player_ped(-1) != client || client.is_null() {
        return 0;
    }
    if rd::<u32>(client, 0x46C) & 0x100 == 0 {
        return 0;
    }
    let veh = rd::<GPtr<u8>>(client, 0x58C);
    if veh.is_null() || rd::<u8>(client, 0x474) & 0x40 != 0 {
        return 0;
    }
    let target = rd::<GPtr<u8>>(rd::<GPtr<u8>>(client, 0x480), 0xA4);
    if !target.is_null() && target != prostitute {
        return 0;
    }
    if vehicle_get_appearance(veh) != 1
        || vehicle_is_upside_down(rd::<GPtr<u8>>(client, 0x58C)) != 0
        || vehicle_is_on_its_side(rd::<GPtr<u8>>(client, 0x58C)) != 0
    {
        return 0;
    }
    let t = tm_get_simplest_active_task(rd::<GPtr<u8>>(client, 0x47C).byte_add(4));
    if task_get_type(t) != 0x2C5 {
        return 0;
    }
    let veh = rd::<GPtr<u8>>(client, 0x58C);
    if rd::<GPtr<u8>>(veh, 0x460) != client {
        return 0;
    }
    let pv = rd::<GPtr<u8>>(prostitute, 0x58C);
    if pv.is_null() {
        if rd::<u8>(veh, 0x484) != 0 {
            return 0;
        }
    } else if pv != veh || rd::<u8>(pv, 0x484) != 1 {
        return 0;
    }
    if rd::<u8>(veh, 0x488) == 0 || (rd::<u32>(rd::<GPtr<u8>>(veh, 0x384), 0xCC) >> 9) & 1 != 0 {
        return 0;
    }
    let mut d = [0f32; 3];
    vector_sub(GPtr::from_raw(d.as_mut_ptr()).cast(), pos(client), pos(prostitute));
    let sq = X87::from_f64_exact(vector_magnitude_sqr(GPtr::from_raw(d.as_mut_ptr()).cast()));
    if !sq.fcom(x(100.0)).test_any(0x41) {
        return 0;
    }
    if is_player_on_a_mission() != 0 || is_coop_game_going_on() != 0 {
        return 0;
    }
    1
}

/// `(x² + y²) + z²` of `pos - p` with `pos` read from memory and the x difference stored as f32 first
/// (`CreateSubTask`'s inline door distances).
fn dist_sq(pos: GPtr<u8>, p: &[f32; 3], store_x: bool) -> X87 {
    let dz = x(rd::<f32>(pos, 8)).fsub_m(p[2]);
    let dy = x(rd::<f32>(pos, 4)).fsub_m(p[1]);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(p[0]);
    let xx = if store_x {
        let s = dx.to_f32();
        x(s).fmul_m(s)
    } else {
        dx.fmul(dx)
    };
    xx.fadd(dy.fmul(dy)).fadd(dz.fmul(dz))
}

/// `CTaskComplexProstituteSolicit::CreateSubTask(int32 taskType, CPed* ped)` (0x666360, thiscall, `ret 8`; the
/// client at +0xC, its vehicle at client +0x58C):
/// - 0x2C0: `CTaskComplexLeaveCar(vehicle, 0, 0, true, false)` (0x34 bytes);
/// - 0x2BC: `CTaskComplexEnterCarAsPassenger(vehicle, 8, false)` (0x50);
/// - 0xCB: `CTaskSimpleStandStill(5000, false, false, 8.0f)` (0x20);
/// - 0x2D5: flags word +0x2C = (w & ~2) | 1, then `CTaskComplexCarDrive(vehicle)` (0x24);
/// - 0x398: `CTaskComplexTurnToFaceEntityOrCoord(client, [0x86FC7C], [0x86FC80])` (0x28);
/// - 0x38B: the passenger (door 10) or driver (door 8) door position (`GetPositionToOpenCarDoor`), whichever is
///   nearer the ped (door 8 unless door 8 is strictly farther: `(dx² + dy²) + dz²`, door 10's x difference stored
///   as f32), turned into the vehicle's frame by the inverse matrix; then
///   `CTaskComplexSeekEntity<XYOffset>(vehicle, 50000, 1000, [0x86FC2C], [0x86FC28], [0x86FC30], false, false)` (0x58)
///   with that offset at +0x44 (written even when the allocation failed, as the original).
/// Any other type, or a failed allocation (except the last case): null.
#[hook(addr = 0x666360, conv = thiscall, name = "CTaskComplexProstituteSolicit::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the CMatrix local's unwritten bytes are 0 here; the SEH frame is not reproduced")]
pub fn prostitute_solicit_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x58C);
    let alloc = task_new;
    match ty {
        0x2C0 => {
            let t = alloc(0x34);
            if t.is_null() { t } else { leave_car_ctor(t, veh(), 0, 0, 1, 0) }
        }
        0x2BC => {
            let t = alloc(0x50);
            if t.is_null() { t } else { enter_car_as_passenger_ctor(t, veh(), 8, 0) }
        }
        0xCB => {
            let t = alloc(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 5000, 0, 0, 8.0f32.to_bits()) }
        }
        0x2D5 => {
            wr::<u16>(this, 0x2C, (rd::<u16>(this, 0x2C) & 0xFFFD) | 1);
            let t = alloc(0x24);
            if t.is_null() { t } else { car_drive_ctor(t, veh()) }
        }
        0x398 => {
            let t = alloc(0x28);
            if t.is_null() {
                t
            } else {
                turn_ctor_entity(t, rd::<GPtr<u8>>(this, 0xC), g::<u32>(0x86_FC7C).read(), g::<u32>(0x86_FC80).read())
            }
        }
        0x38B => {
            let mut m = [0u32; 18];
            let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
            matrix_invert_into(rd::<GPtr<u8>>(veh(), 0x14), mp);
            let (mut p10, mut p8) = ([0f32; 3], [0f32; 3]);
            position_to_open_car_door(GPtr::from_raw(p10.as_mut_ptr()).cast(), veh(), 10);
            position_to_open_car_door(GPtr::from_raw(p8.as_mut_ptr()).cast(), veh(), 8);
            let s10 = dist_sq(pos(ped), &p10, true);
            let s8 = dist_sq(pos(ped), &p8, false);
            let mut door = if !s8.fcom(s10).test_any(0x41) { p10 } else { p8 };
            let mut off = [0f32; 3];
            matrix_mul_vec(GPtr::from_raw(off.as_mut_ptr()).cast(), mp, GPtr::from_raw(door.as_mut_ptr()).cast());
            let mut t = alloc(0x58);
            if !t.is_null() {
                t = seek_entity_xy_ctor(
                    t,
                    veh(),
                    50000,
                    1000,
                    g::<u32>(0x86_FC2C).read(),
                    g::<u32>(0x86_FC28).read(),
                    g::<u32>(0x86_FC30).read(),
                    0,
                    0,
                );
            }
            for (i, v) in off.iter().enumerate() {
                wr::<f32>(t, 0x44 + 4 * i as isize, *v);
            }
            matrix_dtor(mp);
            t
        }
        _ => GPtr::NULL,
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PROSTITUTE_SOLICIT_IS_TASK_VALID, HOOK_PROSTITUTE_SOLICIT_CREATE_SUB_TASK];
