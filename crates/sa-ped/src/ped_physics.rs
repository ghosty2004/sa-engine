//! `CPed` physics/process family (PORT-PED-2, docs/arch/ped-port-2.md): in-car placement, position update.
//! Ported from the 1.0 US bytes; gta-reversed `Entity/Ped/Ped.cpp` was a reading aid (original bugs
//! kept, deviations noted per function).
//!
//! `CVehicle` fields used here: +0x14 matrix, +0x22 model index, +0x384 handling (+0xDE anim group),
//! +0x460 driver, +0x464.. passengers, +0x590 vehicle type (9 = bike), +0x594 sub type,
//! +0x5CC bike lean matrix, +0x64C automobile chassis frame, +0x814 / +0x94C (see the functions).
//! `CMatrix`: right +0, forward +0x10, up +0x20, pos +0x30, attach +0x40, owns +0x44 (0x48 bytes).

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CReplay::Mode` (byte; 1 = playback)
const REPLAY_MODE: usize = 0xA43088;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `HALF_PI` (.rdata 0x858FE4)
const HALF_PI: u32 = 0x3FC9_0FDB;
/// `-HALF_PI` as the bytes push it
const NEG_HALF_PI: u32 = 0xBFC9_0FDB;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `g_surfaceInfos`
const SURFACE_INFOS: usize = 0xB79538;
/// `mod_Buoyancy` (cBuoyancy)
const MOD_BUOYANCY: usize = 0xC1C890;
/// `mod_Buoyancy.m_fWaterLevel`
const WATER_LEVEL: usize = 0xC1C8F4;
/// `g_fx` (Fx_c)
const G_FX: usize = 0xA9AE00;
/// `AudioEngine` (CAudioEngine)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// 0.008 (1/125) as .rdata 0x863984 holds it
const EIGHT_THOUSANDTHS: u32 = 0x3C03_126F;

/// A `CMatrix` local of the original (0x48 bytes; attach pointer +0x40 and owns flag +0x44 are 0 as the
/// original's inlined default constructor leaves them, the rest uninitialised there and 0 here).
type LocalMatrix = [u32; 18];

orig! {
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, `ret 4`)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::Detach()` (0x59ACF0)
    fn matrix_detach(m: GPtr<u8>) = thiscall 0x59ACF0 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40; raw float words)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    fn matrix_rotate_y(m: GPtr<u8>, a: u32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390)
    fn matrix_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B390 => unported;
    /// `CMatrix::SetUnity()` (0x59AE70)
    fn matrix_set_unity(m: GPtr<u8>) = thiscall 0x59AE70 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    fn matrix_mul_assign(dst: GPtr<u8>, rhs: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790, cdecl, returns out)
    fn multiply_3x3(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C790 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150)
    fn bike_calculate_lean_matrix(bike: GPtr<u8>) = thiscall 0x6B7150 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020, `ret 4`)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, h: u32) = thiscall 0x59B020 => unported;
    /// `CPedIntelligence::GetTaskSwim` (0x601070)
    fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CTask::GetTaskType` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVector::operator-=(const CVector&)` (0x406D70, `ret 4`)
    fn vector_sub_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x406D70 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CrossProduct(CVector* out, const CVector&, const CVector&)` (0x59C730, cdecl, returns out)
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, hidden return pointer)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden return pointer)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, hidden return pointer, `ret 0x10`)
    fn physical_get_speed(e: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x542CE0 => unported;
    /// `SurfaceInfos_c::IsSteepSlope(uint32 surface)` (0x55E770, AL)
    fn is_steep_slope(si: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E770 => unported;
    /// `CVector2D::Normalise()` (0x44E480)
    fn vector2d_normalise(v: GPtr<u8>) = thiscall 0x44E480 => unported;
    /// `cBuoyancy::ProcessBuoyancy(CPhysical*, float, CVector* turnPoint, CVector* force)` (0x6C3EF0, AL)
    fn buoyancy_process(b: GPtr<u8>, e: GPtr<u8>, buoyancy: u32, turn: GPtr<u8>, force: GPtr<u8>) -> u8 = thiscall 0x6C3EF0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_mgr_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)`
    /// (0x5674E0, AL)
    fn process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, b6: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CTimeCycle::GetAmbientRed()` (0x560330, ST0)
    fn get_ambient_red() -> f64 = cdecl 0x560330 => unported;
    /// `CTimeCycle::GetAmbientGreen()` (0x560340, ST0)
    fn get_ambient_green() -> f64 = cdecl 0x560340 => unported;
    /// `CTimeCycle::GetAmbientBlue()` (0x560350, ST0)
    fn get_ambient_blue() -> f64 = cdecl 0x560350 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool, CVector*)` (0x6EB690, AL)
    fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u8>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `Fx_c::TriggerWaterSplash(CVector&)` (0x4A1070)
    fn fx_trigger_water_splash(fx: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4A1070 => unported;
    /// `CAudioEngine::ReportWaterSplash(CPhysical*, float, bool)` (0x506F00)
    fn audio_report_water_splash(a: GPtr<u8>, e: GPtr<u8>, volume: u32, force: u32) = thiscall 0x506F00 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`)
    fn apply_move_force(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CPed::GetTransformedBonePosition(CVector&, ePedBones, bool)` (0x5E01C0)
    fn get_transformed_bone_position(ped: GPtr<u8>, v: GPtr<u8>, bone: i32, update: u32) = thiscall 0x5E01C0 => unported;
    /// `CPlayerPed::HandlePlayerBreath(bool, float)` (0x60A8D0)
    fn player_handle_breath(ped: GPtr<u8>, b: u32, f: u32) = thiscall 0x60A8D0 => unported;
    /// `CEventInWater::CEventInWater(float)` (0x4B1370, `ret 4`)
    fn event_in_water_ctor(e: GPtr<u8>, acc: u32) = thiscall 0x4B1370 => unported;
    /// `CEventInWater::~CEventInWater()` (0x4B13C0)
    fn event_in_water_dtor(e: GPtr<u8>) = thiscall 0x4B13C0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedIntelligence::GetTaskClimb()` (0x601180)
    fn get_task_climb(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601180 => unported;
    /// CRT `ceil(double)` (0x823820, ST0)
    fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
    /// `CPedIntelligence::FlushImmediately(bool)` (0x601640)
    fn intel_flush_immediately(intel: GPtr<u8>, b: u32) = thiscall 0x601640 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(c: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    fn clump_get_first_association_flags(c: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*, uint32 flags)` (0x4D6AD0)
    fn get_next_association_flags(a: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6AD0 => unported;
    /// `CPed::ClearAimFlag` (0x5DEF20)
    fn clear_aim_flag(ped: GPtr<u8>) = thiscall 0x5DEF20 => crate::ped::clear_aim_flag;
    /// `CPed::SetMoveAnimSpeed(CAnimBlendAssociation*)` (0x5DEC10)
    fn set_move_anim_speed(ped: GPtr<u8>, a: GPtr<u8>) = thiscall 0x5DEC10 => crate::ped::set_move_anim_speed;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn blend_animation(c: GPtr<u8>, group: i32, id: i32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CPedGroups::IsInPlayersGroup(CPed*)` (0x5F7F10, AL)
    fn is_in_players_group(ped: GPtr<u8>) -> u8 = cdecl 0x5F7F10 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::GetLeader()` (0x5F69A0)
    fn group_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
}

/// `CPlaceable::GetHeading()` as the bytes inline it: `atan2(-fwd.x, fwd.y)` of the matrix (FPATAN, the
/// raw register value) or the placement heading (+0x10).
fn heading(e: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        x(rd::<f32>(e, 0x10))
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    }
}

/// `CPed::SetPedPositionInCar` (0x5DF910): nothing in replay playback. Seat offset from the vehicle model's
/// struct (`GetSeatPositionInVehicle` inlined: driver = front seat, x negated unless the vehicle type is 5
/// or 9, minus `|veh+0x814| * 0.001` in z for sub type 10; passenger 0 = front seat, or the rear seat for
/// bikes/sub type 2; passenger 1 = rear seat with x negated; passenger 2 = rear seat; a ped in no seat gets
/// the front seat). The vehicle matrix (bikes: their lean matrix after `CalculateLeanMatrix`; the combine
/// 0x214: times a chassis-height Y rotation) is moved by `Multiply3x3(matrix, seat)`; anim group 13
/// rotates passengers 1/2 by -/+ pi/2 (1 also +0.6 in y) and offsets their heading. Heading = aimed heading,
/// `SetMatrix`.
///
/// gta-reversed deviations (bytes kept): the combine sets `tempMat.pos.z += chassisZ` after `RotateY`, not
/// `SetTranslateOnly(0, 0, z)` (x/y of the rotated translation stay); passenger 1 of anim group 13 adds
/// 0.6 to `pos.y` only. The vehicle matrix is copied without a null check. Not reproduced: the SEH frame.
#[hook(addr = 0x5DF910, conv = thiscall, name = "CPed::SetPedPositionInCar", expect = "64 A1 00 00 00 00", this = "CPed")]
pub fn set_ped_position_in_car(this: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    let veh = rd::<GPtr<u8>>(this, 0x58C);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(veh, 0x22))).read();
    let mut veh_mat: LocalMatrix = [0; 18];
    let vm = lp(&mut veh_mat).cast::<u8>();
    matrix_copy_ctor(vm, rd::<GPtr<u8>>(veh, 0x14));
    let mut temp_mat: LocalMatrix = [0; 18];
    let tm = lp(&mut temp_mat).cast::<u8>();

    // seat position (inlined GetSeatPositionInVehicle)
    let front = |mi: GPtr<u8>| -> GPtr<u8> {
        let s = rd::<GPtr<u8>>(mi, 0x5C);
        if rd::<i32>(mi, 0x3C) == 5 { s } else { s.byte_add(0x30) }
    };
    let rear = |mi: GPtr<u8>| rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x3C);
    let load = |p: GPtr<u8>| [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
    let mut seat: [f32; 3];
    if this == rd::<GPtr<u8>>(veh, 0x460) {
        seat = load(front(mi));
        let ty = rd::<i32>(veh, 0x590);
        if ty != 5 && ty != 9 {
            seat[0] = x(seat[0]).fchs().to_f32();
        }
        if rd::<i32>(veh, 0x594) == 10 {
            let a = f32::from_bits(rd::<u32>(veh, 0x814) & 0x7FFF_FFFF);
            seat[2] = x(a).fmul_m(f32::from_bits(0x3A83_126F)).fsubr_m(seat[2]).to_f32();
        }
    } else if this == rd::<GPtr<u8>>(veh, 0x464) {
        if rd::<i32>(veh, 0x590) == 9 || rd::<i32>(veh, 0x594) == 2 {
            seat = load(rear(mi));
        } else {
            seat = load(front(mi));
        }
    } else if this == rd::<GPtr<u8>>(veh, 0x468) {
        seat = load(rear(mi));
        seat[0] = x(seat[0]).fchs().to_f32();
    } else if this == rd::<GPtr<u8>>(veh, 0x46C) {
        seat = load(rear(mi));
    } else {
        seat = load(front(mi));
    }

    if rd::<i32>(veh, 0x590) == 9 {
        bike_calculate_lean_matrix(veh);
        matrix_assign(vm, rd::<GPtr<u8>>(this, 0x58C).byte_add(0x5CC));
    } else if rd::<u16>(veh, 0x22) == 0x214 {
        let chassis = rd::<GPtr<u8>>(veh, 0x64C);
        let mut cz = 0.0f32;
        if !chassis.is_null() {
            matrix_attach(tm, chassis.byte_add(0x10), 0);
            cz = rd::<f32>(tm, 0x38);
            matrix_detach(tm);
        }
        matrix_set_translate(tm, 0, 0, x(cz).fchs().to_f32().to_bits());
        matrix_rotate_y(tm, rd::<u32>(rd::<GPtr<u8>>(this, 0x58C), 0x94C));
        wr::<f32>(tm, 0x38, x(rd::<f32>(tm, 0x38)).fadd_m(cz).to_f32());
        matrix_mul_assign(vm, tm);
    }

    let mut out = [0.0f32; 3];
    let r = multiply_3x3(lp(&mut out).cast(), vm, lp(&mut seat).cast());
    let t = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
    for (i, v) in t.iter().enumerate() {
        let o = 0x30 + 4 * i as isize;
        wr::<f32>(vm, o, x(rd::<f32>(vm, o)).fadd_m(*v).to_f32());
    }
    matrix_set_unity(tm);

    let veh = rd::<GPtr<u8>>(this, 0x58C);
    let group = rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE);
    if u32::from(group).wrapping_add(0x58) == 0x65 && this == rd::<GPtr<u8>>(veh, 0x468) {
        wr::<f32>(this, 0x558, heading(veh).fsub_m(f32::from_bits(HALF_PI)).to_f32());
        matrix_set_translate(tm, 0, 0, 0);
        matrix_rotate_z(tm, NEG_HALF_PI);
        wr::<f32>(tm, 0x34, x(rd::<f32>(tm, 0x34)).fadd_m(f32::from_bits(0x3F19_999A)).to_f32());
        matrix_mul_assign(vm, tm);
    } else if u32::from(group).wrapping_add(0x58) == 0x65 && this == rd::<GPtr<u8>>(veh, 0x46C) {
        wr::<f32>(this, 0x558, heading(veh).fadd_m(f32::from_bits(HALF_PI)).to_f32());
        matrix_set_translate(tm, 0, 0, 0);
        matrix_rotate_z(tm, HALF_PI);
        matrix_mul_assign(vm, tm);
    } else {
        wr::<f32>(this, 0x558, heading(veh).to_f32());
    }
    wr::<u32>(this, 0x55C, rd::<u32>(this, 0x558));
    placeable_set_matrix(this, vm);
    matrix_dtor(tm);
    matrix_dtor(vm);
}

/// `CMatrix::SetRotateZOnly` on the matrix, else the placement heading (+0x10) = `h`.
fn set_heading(e: GPtr<u8>, h: u32) {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        wr::<u32>(e, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h);
    }
}

/// `CPed::UpdatePosition` (0x5E1B10): nothing in replay playback. Not standing: only the heading is
/// applied (matrix `SetRotateZOnly` / placement heading), and only while swimming, on a jet pack or with
/// primary task slot +0x10 of type 0x6C. Standing and not attached: heading applied; the wanted move
/// (`m_vecAnimMovingShift` +0x550/+0x554) becomes a speed change `d`:
/// - on a ground entity: its speed at the contact point (`CPhysical::GetSpeed(+0x56C)`; for a non-player
///   ped on a boat instead `turn x (pos - (0,0,1) - boat pos) + boat move - |turn|^2 * that offset * ts`,
///   whose z replaces the ped's move speed z) plus the move minus the ped's move speed; headings += `ts *
///   ground turn.z`;
/// - no ground entity on a steep slope (`IsSteepSlope(+0xBA)`) with a non-zero ground normal (+0x578/
///   +0x57C): normal (normalised 2D) * 0.02 + move, move speed set to (0, 0, -0.001), and the part along
///   the normal removed when it points into it;
/// - else the move minus the ped's move speed.
///
/// `|d|` is then clamped: on a non-attached ground entity that is not (flag 4 without flag 8) of +0x40:
/// `ts * 0.01` for non-vehicles and automobiles, `ts * 0.002` in ped state 0x36, bikes `ts * 0.0002` when
/// their speed² > 0.04; other vehicles compare `|d|` with itself (no clamp). Without ground entity and
/// with flag 0x100000 of +0x474: `ts * 0.01`. Finally move speed xy += d.
///
/// x87: the vector helpers are the original's (orig!); `|d|` is kept both as register and as stored f32,
/// as the bytes compare them. No gta-reversed body (`plugin::Call`).
#[hook(addr = 0x5E1B10, conv = thiscall, name = "CPed::UpdatePosition", expect = "A0 88 30 A4 00", this = "CPed")]
pub fn update_position(this: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    if rd::<u32>(this, 0x46C) & 1 == 0 {
        let intel = rd::<GPtr<u8>>(this, 0x47C);
        if get_task_swim(intel).is_null() && get_task_jet_pack(intel).is_null() {
            let t = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x47C), 0x10);
            if t.is_null() || task_get_type(t) != 0x6C {
                return;
            }
        }
        set_heading(this, rd::<u32>(this, 0x558));
        return;
    }
    if !rd::<GPtr<u8>>(this, 0xFC).is_null() {
        return;
    }
    set_heading(this, rd::<u32>(this, 0x558));
    let ts = g::<f32>(TIME_STEP);
    let ground = rd::<GPtr<u8>>(this, 0x568);
    let (mut dx, mut dy): (f32, f32);
    if !ground.is_null() {
        let mut v: [f32; 3];
        let pt = rd::<i32>(this, 0x598);
        if pt != 0 && pt != 1 && rd::<u8>(ground, 0x36) & 7 == 2 && rd::<i32>(ground, 0x590) == 5 {
            let mut off = vec3(ent_pos(this));
            off[2] = x(off[2]).fsub_m(1.0).to_f32();
            let pv = lp(&mut off).cast::<u8>();
            vector_sub_assign(pv, ent_pos(ground).cast());
            let mut cross = [0.0f32; 3];
            let c = cross_product(lp(&mut cross).cast(), ground.byte_add(0x50), pv);
            let mut sum = [0.0f32; 3];
            let s = vector_add(lp(&mut sum).cast(), c, ground.byte_add(0x44));
            v = [rd::<f32>(s, 0), rd::<f32>(s, 4), rd::<f32>(s, 8)];
            let tsv = ts.read();
            let k = X87::from_f64_exact(magnitude_sqr(rd::<GPtr<u8>>(this, 0x568).byte_add(0x50)))
                .fmul_m(-1.0)
                .to_f32();
            let mut o1 = [0.0f32; 3];
            let r1 = scalar_mul_vector(lp(&mut o1).cast(), k, pv);
            let mut o2 = [0.0f32; 3];
            let r2 = vector_mul_scalar(lp(&mut o2).cast(), r1, tsv);
            vector_add_assign(lp(&mut v).cast(), r2);
            wr::<f32>(this, 0x4C, v[2]);
        } else {
            let p = this.byte_add(0x56C);
            let mut out = [0.0f32; 3];
            let r = physical_get_speed(ground, lp(&mut out).cast(), rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
            v = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        }
        dx = x(v[0]).fadd_m(rd::<f32>(this, 0x550)).fsub_m(rd::<f32>(this, 0x44)).to_f32();
        dy = x(v[1]).fadd_m(rd::<f32>(this, 0x554)).fsub_m(rd::<f32>(this, 0x48)).to_f32();
        let ge = rd::<GPtr<u8>>(this, 0x568);
        for o in [0x558isize, 0x55C] {
            wr::<f32>(this, o, x(ts.read()).fmul_m(rd::<f32>(ge, 0x58)).fadd_m(rd::<f32>(this, o)).to_f32());
        }
    } else {
        let steep = is_steep_slope(GPtr::from_addr(SURFACE_INFOS), u32::from(rd::<u8>(this, 0xBA))) != 0;
        let nz = |o: isize| x(rd::<f32>(this, o)).fcom(X87::zero()).test_parity(0x44);
        if steep && (nz(0x578) || nz(0x57C)) {
            let mut n = [rd::<f32>(this, 0x578), x(rd::<f32>(this, 0x57C)).to_f32()];
            vector2d_normalise(lp(&mut n).cast());
            let c = f32::from_bits(0x3CA3_D70A);
            let ax = x(n[0]).fmul_m(c);
            wr::<f32>(this, 0x44, 0.0);
            wr::<f32>(this, 0x48, 0.0);
            wr::<u32>(this, 0x4C, 0xBA83_126F);
            dx = ax.fadd_m(rd::<f32>(this, 0x550)).to_f32();
            dy = x(n[1]).fmul_m(c).fadd_m(rd::<f32>(this, 0x554)).to_f32();
            let d = x(n[1]).fmul_m(dy).fadd(x(n[0]).fmul_m(dx));
            if !d.fcom(X87::zero()).test_parity(5) {
                dx = x(n[0]).fmul(d).fsubr_m(dx).to_f32();
                dy = d.fmul_m(n[1]).fsubr_m(dy).to_f32();
            }
        } else {
            dx = x(rd::<f32>(this, 0x550)).fsub_m(rd::<f32>(this, 0x44)).to_f32();
            dy = x(rd::<f32>(this, 0x554)).fsub_m(rd::<f32>(this, 0x48)).to_f32();
        }
    }

    let ge = rd::<GPtr<u8>>(this, 0x568);
    let len = || x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
    // (limit, |d| as compared): None = no clamp
    let mut clamp: Option<(X87, f32)> = None;
    let fl = if ge.is_null() { 0 } else { rd::<u32>(ge, 0x40) };
    if !ge.is_null() && rd::<GPtr<u8>>(ge, 0xFC).is_null() && (fl & 4 == 0 || fl & 8 != 0) {
        let l = len();
        let lf = l.to_f32();
        let small = x(ts.read()).fmul_m(f32::from_bits(0x3C23_D70A));
        let limit = if rd::<u8>(ge, 0x36) & 7 != 2 {
            small
        } else if rd::<i32>(this, 0x530) == 0x36 {
            x(ts.read()).fmul_m(f32::from_bits(0x3B03_126F))
        } else {
            let ty = rd::<i32>(ge, 0x590);
            if ty == 9 {
                let m = X87::from_f64_exact(magnitude_sqr(ge.byte_add(0x44)));
                if m.fcom(x(f32::from_bits(0x3D23_D70B))).test_any(0x41) {
                    x(lf)
                } else {
                    x(ts.read()).fmul_m(f32::from_bits(0x3951_B718))
                }
            } else if ty != 0 {
                l
            } else {
                small
            }
        };
        clamp = Some((limit, lf));
    } else if rd::<u32>(this, 0x474) & 0x10_0000 != 0 && ge.is_null() {
        let l = len();
        let limit = x(ts.read()).fmul_m(f32::from_bits(0x3C23_D70A));
        // the register |d| (not the stored f32) is compared and divided here
        if !l.fcom(limit).test_any(0x41) {
            let r = limit.fdiv(l);
            dx = x(dx).fmul(r).to_f32();
            dy = r.fmul_m(dy).to_f32();
        }
    }
    if let Some((limit, lf)) = clamp
        && !x(lf).fcom(limit).test_any(0x41)
    {
        let r = limit.fdiv_m(lf);
        dx = x(dx).fmul(r).to_f32();
        dy = r.fmul_m(dy).to_f32();
    }
    wr::<f32>(this, 0x44, x(dx).fadd_m(rd::<f32>(this, 0x44)).to_f32());
    wr::<f32>(this, 0x48, x(dy).fadd_m(rd::<f32>(this, 0x48)).to_f32());
}

/// `GetSimplestActiveTask(&intel->m_TaskMgr)` of the ped.
fn simplest_task(ped: GPtr<u8>) -> GPtr<u8> {
    task_mgr_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4))
}

/// Adds a `CEventInWater(acceleration)` to the ped's event group (+0x68 of the intelligence) through a
/// local event (constructor 0x4B1370, `CEventGroup::Add(ev, false)`, destructor 0x4B13C0). Bytes 9..11 of
/// the event are uninitialised stack in the original (0 here).
fn add_event_in_water(ped: GPtr<u8>, acc: u32) {
    let mut ev = [0u32; 4];
    let e = lp(&mut ev).cast::<u8>();
    event_in_water_ctor(e, acc);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    event_in_water_dtor(e);
}

/// `bIsStanding = false; bIsDrowning = true` as the bytes write them (+0x46C bit 0, +0x470 bit 19).
fn set_drowning_flags(this: GPtr<u8>) {
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & 0xFFFF_FFFE);
    wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) | 0x8_0000);
}

/// `CPed::ProcessBuoyancy` (0x5E1FA0): nothing in a vehicle. Buoyancy = (1.1, or 1.8 in ped states
/// 0x36/0x37) * mass * 0.008 through `mod_Buoyancy.ProcessBuoyancy`; out of water: touching-water flag
/// (+0x40 bit 27) cleared, the simplest active task's +0x54 = 1000 when swimming. Standing on a boat
/// (contact entity +0x584, not flag 0x20000000) or, for players, a boat with up.z > 0 below (vertical
/// line 3 down): submerged flag (bit 8) cleared (standing: simplest task +0x54 += ts when swimming).
/// Then `CTimeCycle::GetAmbientRed/Green/Blue` (discarded) and one `rand()`; a first touch moving down
/// faster than 0.01 splashes at `pos + speed * ts * 4` on the water level (`TriggerWaterSplash`,
/// `ReportWaterSplash(-100)`); flags set, `ApplyMoveForce`. Drowning (`ts * 0.008 < force.z / mass`, or
/// `pos.z + 0.6 <` water level): standing/drowning flags; players: swimming -> simplest task +0x54 = 0
/// and return, simplest task type 0xFE -> return, else `CEventInWater(force.z / (ts * mass * 0.008))`;
/// other peds `CEventInWater(0.75)`; then speed x/y (and a negative z) times `_CIpow(0.9, ts)`. Not
/// drowning: standing swimmers +0x54 += ts; players with the head bone (offset (0, 0, 0.1)) below the
/// water level `HandlePlayerBreath(true, 1.0)`.
///
/// gta-reversed deviations (bytes kept): the multiplier depends on ped state 0x36/0x37, not `IsAlive`;
/// the task touched is always `GetSimplestActiveTask`; players check the simplest task type 0xFE where
/// gta-reversed calls `GetTaskClimb`; the breath branch is absent there. The vertical line's entity slot
/// holds the line's end z when nothing is hit (only read on a hit), the water level slot the splash y.
/// Not reproduced: the SEH frame.
#[hook(addr = 0x5E1FA0, conv = thiscall, name = "CPed::ProcessBuoyancy", expect = "64 A1 00 00 00 00", this = "CPed")]
pub fn process_buoyancy(this: GPtr<u8>) {
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        return;
    }
    let st = rd::<i32>(this, 0x530);
    let mult = if st == 0x37 || st == 0x36 { f32::from_bits(0x3FE6_6666) } else { f32::from_bits(0x3F8C_CCCD) };
    let k = x(mult).fmul_m(rd::<f32>(this, 0x8C)).fmul_m(f32::from_bits(EIGHT_THOUSANDTHS)).to_f32();
    let mut turn = [0.0f32; 3];
    let mut force = [0.0f32; 3];
    let ok = buoyancy_process(GPtr::from_addr(MOD_BUOYANCY), this, k.to_bits(), lp(&mut turn).cast(), lp(&mut force).cast());
    if ok == 0 {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xF7FF_FFFF);
        if !get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            wr::<f32>(simplest_task(this), 0x54, 1000.0);
        }
        return;
    }
    let ts = g::<f32>(TIME_STEP);
    if rd::<u32>(this, 0x46C) & 1 != 0 {
        let c = rd::<GPtr<u8>>(this, 0x584);
        if !c.is_null() && rd::<u8>(c, 0x36) & 7 == 2 && rd::<i32>(c, 0x590) == 5 && rd::<u32>(c, 0x40) & 0x2000_0000 == 0 {
            wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
            if !get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
                let t = simplest_task(this);
                wr::<f32>(t, 0x54, x(ts.read()).fadd_m(rd::<f32>(t, 0x54)).to_f32());
            }
            return;
        }
    }
    if !rd::<GPtr<u8>>(this, 0x480).is_null() {
        let mut pos = vec3(ent_pos(this));
        let z = x(pos[2]).fsub_m(3.0).to_f32();
        // the entity slot first holds the end z (the bytes reuse it)
        let mut ent: u32 = z.to_bits();
        let mut cp = [0u32; 11];
        let hit = process_vertical_line(
            lp(&mut pos).cast(),
            z.to_bits(),
            lp(&mut cp).cast(),
            lp(&mut ent).cast(),
            0,
            1,
            0,
            0,
            0,
            0,
            0,
        );
        if hit != 0 {
            let e = GPtr::<u8>::from_addr(ent as usize);
            if rd::<u8>(e, 0x36) & 7 == 2
                && rd::<i32>(e, 0x590) == 5
                && rd::<u32>(e, 0x40) & 0x2000_0000 == 0
                && !x(rd::<f32>(get_matrix(e), 0x28)).fcom(X87::zero()).test_any(0x41)
            {
                wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FEFF);
                return;
            }
        }
    }
    let _ = get_ambient_red();
    let _ = get_ambient_green();
    let _ = get_ambient_blue();
    let _ = at_site(0x5E2165, rand);
    if rd::<u32>(this, 0x40) & 0x0800_0000 == 0
        && !x(rd::<f32>(this, 0x4C)).fcom(x(f32::from_bits(0xBC23_D70A))).test_parity(5)
    {
        let t = ts.read();
        let a = x(t).fmul_m(rd::<f32>(this, 0x4C));
        let b = x(t).fmul_m(rd::<f32>(this, 0x48));
        let c = x(t).fmul_m(rd::<f32>(this, 0x44)).to_f32();
        let a4 = a.fmul_m(4.0).to_f32();
        let b4 = b.fmul_m(4.0);
        let c4 = x(c).fmul_m(4.0);
        let p = ent_pos(this);
        let sz = x(a4).fadd_m(p.wrapping_index(2).read()).to_f32();
        let sy = b4.fadd_m(p.wrapping_index(1).read()).to_f32();
        let sx = c4.fadd_m(p.read()).to_f32();
        // the water level out slot holds the splash y first (the bytes reuse it)
        let mut level = sy;
        if get_water_level(sx.to_bits(), sy.to_bits(), sz.to_bits(), lp(&mut level).cast(), 1, 0) != 0 {
            let mut splash = [sx, sy, level];
            fx_trigger_water_splash(GPtr::from_addr(G_FX), lp(&mut splash).cast());
            audio_report_water_splash(GPtr::from_addr(AUDIO_ENGINE), this, 0xC2C8_0000, 1);
        }
    }
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x0800_0100);
    apply_move_force(this, force[0].to_bits(), force[1].to_bits(), force[2].to_bits());

    let q = x(force[2]).fdiv_m(rd::<f32>(this, 0x8C));
    let p = x(ts.read()).fmul_m(f32::from_bits(EIGHT_THOUSANDTHS));
    let wl = g::<f32>(WATER_LEVEL);
    let drowning = !p.fcom(q).test_parity(5)
        || !x(ent_pos(this).wrapping_index(2).read())
            .fadd_m(f32::from_bits(0x3F19_999A))
            .fcom(x(wl.read()))
            .test_parity(5);
    if !drowning {
        if rd::<u32>(this, 0x46C) & 1 != 0 && !get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            let t = simplest_task(this);
            wr::<f32>(t, 0x54, x(ts.read()).fadd_m(rd::<f32>(t, 0x54)).to_f32());
            return;
        }
        if rd::<GPtr<u8>>(this, 0x480).is_null() {
            return;
        }
        let mut head = [0.0f32, 0.0, f32::from_bits(0x3DCC_CCCD)];
        get_transformed_bone_position(this, lp(&mut head).cast(), 5, 0);
        if x(head[2]).fcom(x(wl.read())).test_parity(5) {
            return;
        }
        player_handle_breath(this, 1, 0x3F80_0000);
        return;
    }
    let pt = rd::<i32>(this, 0x598);
    if pt == 0 || pt == 1 {
        let mut done = false;
        if !get_task_swim(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
            wr::<u32>(simplest_task(this), 0x54, 0);
            done = true;
        } else if !simplest_task(this).is_null() && task_get_type(simplest_task(this)) == 0xFE {
            done = true;
        } else {
            let d = x(ts.read())
                .fmul_m(rd::<f32>(this, 0x8C))
                .fmul_m(f32::from_bits(EIGHT_THOUSANDTHS))
                .fdivr_m(force[2]);
            add_event_in_water(this, d.to_f32().to_bits());
        }
        set_drowning_flags(this);
        if done {
            return;
        }
    } else {
        set_drowning_flags(this);
        add_event_in_water(this, 0x3F40_0000);
    }
    let base = X87::from_f64_exact(f64::from_bits(0x3FEC_CCCC_C000_0000));
    let f = sa_world::physical::shared::ci_pow(base, x(ts.read()));
    wr::<f32>(this, 0x44, f.fmul_m(rd::<f32>(this, 0x44)).to_f32());
    wr::<f32>(this, 0x48, f.fmul_m(rd::<f32>(this, 0x48)).to_f32());
    if !x(rd::<f32>(this, 0x4C)).fcom(X87::zero()).test_parity(5) {
        wr::<f32>(this, 0x4C, f.fmul_m(rd::<f32>(this, 0x4C)).to_f32());
    }
}

/// `(e->m_nFlags >> 4) & 1` (the entity's "stuck" bit of +0x1C, as the bytes test it).
fn stuck(e: GPtr<u8>) -> bool {
    (rd::<u32>(e, 0x1C) >> 4) & 1 != 0
}

/// `|v| < 0.001` as `fld; fabs; fcomp [0x858CDC]; test ah, 5; jp` decides it.
fn abs_below_thousandth(v: u32) -> bool {
    !x(f32::from_bits(v & 0x7FFF_FFFF)).fcom(x(f32::from_bits(0x3A83_126F))).test_parity(5)
}

/// `CPed::SpecialEntityPreCollisionStuff(CPhysical* col, bool ignoreStuck, bool& collisionDisabled, bool&
/// colIgnored, bool& unableToMove, bool& stuck)` (0x5E3C30, `ret 0x18`). With `F` = `+0x40 |= 0x1000`:
/// - a vehicle `col` that is the ped's vehicle while +0x470 bit 31: `collisionDisabled`;
/// - either side's ignored-collision entity (+0x128) is the other: `colIgnored`, then `F` unless (+0x470
///   bit 4 set and bit 31 clear);
/// - either is attached (+0xFC) to the other, or both are attached to something: `collisionDisabled`;
/// - `col` with physical flag 0x20: flag 4 or 0x40000000 -> `unableToMove`; else `ignoreStuck` ->
///   `collisionDisabled`; else either stuck -> `stuck`; `F` in every case;
/// - `col` with flags 0xC0: `ignoreStuck` -> `colIgnored`, else either stuck -> `stuck`; `F`;
/// - an object: object flag 0x100 (+0x140) with up.z < 0.66, or model 0x156 lower than the ped:
///   `colIgnored` + `F`; else when its +0x160 info +0x14 > 0 or flag 4 is set and all speed components
///   are below 0.001: `unableToMove`; else stuck -> `unableToMove`;
/// - other entities: models 0x1B9/0x234/0x252 -> `colIgnored` + `F`; stuck -> `unableToMove`.
///
/// Finally players (ped type 0/1) with `GetTaskClimb` get `F`. No gta-reversed body (`plugin::Call`).
/// The object matrices are dereferenced without null checks, as in the bytes.
#[hook(addr = 0x5E3C30, conv = thiscall, name = "CPed::SpecialEntityPreCollisionStuff", expect = "53 56 8B F1 8B 4C 24 0C", this = "CPed")]
pub fn special_entity_pre_collision_stuff(
    this: GPtr<u8>,
    col: GPtr<u8>,
    ignore_stuck: u32,
    collision_disabled: GPtr<u8>,
    col_ignored: GPtr<u8>,
    unable_to_move: GPtr<u8>,
    stuck_out: GPtr<u8>,
) {
    let set_f = || wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x1000);
    let ty = rd::<u8>(col, 0x36) & 7;
    // `true` = the path through the `F` write (0x5E3E61)
    let or_f = 'b: {
        if ty == 2 && (rd::<i32>(this, 0x470) < 0) && rd::<GPtr<u8>>(this, 0x58C) == col {
            collision_disabled.write(1);
            break 'b false;
        }
        if rd::<GPtr<u8>>(this, 0x128) == col || rd::<GPtr<u8>>(col, 0x128) == this {
            col_ignored.write(1);
            let f = rd::<u32>(this, 0x470);
            break 'b f & 0x10 == 0 || (f as i32) < 0;
        }
        let a = rd::<GPtr<u8>>(this, 0xFC);
        let b = rd::<GPtr<u8>>(col, 0xFC);
        if a == col || b == this || (!a.is_null() && !b.is_null()) {
            collision_disabled.write(1);
            break 'b false;
        }
        let fl = rd::<u32>(col, 0x40);
        if fl & 0x20 != 0 {
            if fl & 4 != 0 || fl & 0x4000_0000 != 0 {
                unable_to_move.write(1);
            } else if ignore_stuck & 0xFF != 0 {
                collision_disabled.write(1);
            } else if stuck(this) || stuck(col) {
                stuck_out.write(1);
            }
            break 'b true;
        }
        if fl & 0xC0 != 0 {
            if ignore_stuck & 0xFF != 0 {
                col_ignored.write(1);
            } else if stuck(this) || stuck(col) {
                stuck_out.write(1);
            }
            break 'b true;
        }
        if ty == 4 {
            if rd::<u32>(col, 0x140) & 0x100 != 0
                && !x(rd::<f32>(rd::<GPtr<u8>>(col, 0x14), 0x28)).fcom(x(f32::from_bits(0x3F28_F5C3))).test_parity(5)
            {
                col_ignored.write(1);
                break 'b true;
            }
            if rd::<u16>(col, 0x22) == 0x156
                && !x(rd::<f32>(rd::<GPtr<u8>>(col, 0x14), 0x38))
                    .fcom(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x38)))
                    .test_parity(5)
            {
                col_ignored.write(1);
                break 'b true;
            }
            let info = rd::<GPtr<u8>>(col, 0x160);
            let positive = !x(rd::<f32>(info, 0x14)).fcom(X87::zero()).test_any(0x41);
            if (positive || fl & 4 != 0)
                && abs_below_thousandth(rd::<u32>(col, 0x44))
                && abs_below_thousandth(rd::<u32>(col, 0x48))
                && abs_below_thousandth(rd::<u32>(col, 0x4C))
            {
                unable_to_move.write(1);
                break 'b false;
            }
            if stuck(col) {
                unable_to_move.write(1);
            }
            break 'b false;
        }
        let m = rd::<u16>(col, 0x22);
        if m == 0x1B9 || m == 0x234 || m == 0x252 {
            col_ignored.write(1);
            break 'b true;
        }
        if stuck(col) {
            unable_to_move.write(1);
        }
        false
    };
    if or_f {
        set_f();
    }
    let pt = rd::<i32>(this, 0x598);
    if (pt == 0 || pt == 1) && !get_task_climb(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
        set_f();
    }
}

/// `ceil` then `_ftol`, low byte (AL), as the bytes return the step count.
fn ceil_steps(v: X87) -> u8 {
    ftol32(X87::from_f64_exact(crt_ceil(v.to_f64()))) as u8
}

/// The steps with a minimum: `ceil(t) < min ? min : ceil((float)t)` (`t` also stored as f32 first).
fn ceil_steps_min(t: X87, min: f32) -> u8 {
    let tf = t.to_f32();
    let c = X87::from_f64_exact(crt_ceil(t.to_f64()));
    if !c.fcom(x(min)).test_parity(5) {
        ftol32(x(min)) as u8
    } else {
        ceil_steps(x(tf))
    }
}

/// `CPed::SpecialEntityCalcCollisionSteps(bool&, bool&)` (0x5E3E90, `ret 8`, AL; both arguments unused):
/// attached -> 1. Non-players with `((x² + y²) + z²) * ts * ts < 0.09` (or NaN) -> 1. Otherwise, with
/// `d = |speed| * ts`: non-players `ceil(d * 5)` and their +0x9C doubled; players on a ground entity
/// (+0x568) `max(4, ceil(d * 6.6666665))`, without one `max(2, ceil(d * 3.3333333))` (the comparison on
/// the register value, the result from the stored f32). `ceil` is the CRT's (0x823820), `_ftol` modelled.
/// AL only: EAX's upper bytes (attached pointer / FPU status / `_ftol` result) are not reproduced; the
/// caller (`CPhysical::ProcessCollision`, `movzx edx, al`) reads AL only. No gta-reversed body.
#[hook(addr = 0x5E3E90, conv = thiscall, name = "CPed::SpecialEntityCalcCollisionSteps", expect = "55 8B EC 83 E4 F8", this = "CPed")]
pub fn special_entity_calc_collision_steps(this: GPtr<u8>, _a: GPtr<u8>, _b: GPtr<u8>) -> u8 {
    if !rd::<GPtr<u8>>(this, 0xFC).is_null() {
        return 1;
    }
    let (sx, sy, sz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
    let m2 = sx.fmul(sx).fadd(sy.fmul(sy)).fadd(sz.fmul(sz));
    let ts = g::<f32>(TIME_STEP);
    let player = !rd::<GPtr<u8>>(this, 0x480).is_null();
    if !player && m2.fmul_m(ts.read()).fmul_m(ts.read()).fcom(x(f32::from_bits(0x3DB8_51EC))).c0() {
        return 1;
    }
    let d = m2.fsqrt().fmul_m(ts.read());
    if !player {
        let r = ceil_steps(d.fmul_m(5.0));
        wr::<f32>(this, 0x9C, x(rd::<f32>(this, 0x9C)).fdouble().to_f32());
        return r;
    }
    if !rd::<GPtr<u8>>(this, 0x568).is_null() {
        ceil_steps_min(d.fmul_m(f32::from_bits(0x40D5_5555)), 4.0)
    } else {
        ceil_steps_min(d.fmul_m(f32::from_bits(0x4055_5555)), 2.0)
    }
}

/// `CPed::Teleport(CVector destination, bool resetRotation)` (0x5E4110, `ret 0x10`; `resetRotation`
/// unused): players, or peds with an active task 0x2C0 (`TASK_COMPLEX_LEAVE_CAR`), are flushed
/// (`FlushImmediately(true)`); `CWorld::Remove`, position set (matrix or placement), `bIsStanding`
/// cleared, the damage entity (+0xDC) reference cleaned up and the field zeroed (gta-reversed leaves it as
/// a todo), `CWorld::Add`, move and turn speed zeroed. Not reproduced: the original zeroes its own
/// argument slots on the stack while building the zero vectors.
#[hook(addr = 0x5E4110, conv = thiscall, name = "CPed::Teleport", expect = "56 8B F1 8B 86 98 05 00 00", this = "CPed")]
pub fn teleport(this: GPtr<u8>, dx: u32, dy: u32, dz: u32, _reset_rotation: u32) {
    let pt = rd::<i32>(this, 0x598);
    let flush = pt == 0
        || pt == 1
        || !crate::ped::find_active_task_by_type(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), 0x2C0).is_null();
    if flush {
        intel_flush_immediately(rd::<GPtr<u8>>(this, 0x47C), 1);
    }
    world_remove(this);
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        wr::<u32>(this, 4, dx);
        wr::<u32>(this, 8, dy);
        wr::<u32>(this, 0xC, dz);
    } else {
        wr::<u32>(m, 0x30, dx);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x34, dy);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x38, dz);
    }
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & 0xFFFF_FFFE);
    let r = at::<GPtr<u8>>(this, 0xDC);
    let e = r.read();
    if !e.is_null() {
        crate::ped::clean_up_old_reference(e, r);
    }
    r.write(GPtr::from_addr(0));
    world_add(this);
    for o in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<u32>(this, o, 0);
    }
}

/// `CPed::SetMoveAnim` (0x5E4A00, virtual): nothing in ped states 0x36/0x37 (dead), ducking (+0x46C bit
/// 26) or attached. Unchanged move state (+0x534 == +0x538): from walk (4) up, and only while the move
/// anim speed (+0x518) is outside [-0.01, 0.01], the walk/run/sprint association (jog uses walk) gets
/// `SetMoveAnimSpeed` unless the task set it (+0x47A bit 0). Changed: state 0 just stores it; otherwise it
/// is stored, walk/run/sprint fade out the partial associations that are neither auto-remove (8) nor
/// 0x400 (blend delta -2, flag 4) and clear the aim and look flags; then `BlendAnimation`: still = idle (3)
/// of the ped's group at 4, turn l/r = 0x89/0x8A of group 0 at 16, walk = 0 at 1, jog = nothing, run = 1
/// at 3 in ped state 9 (else 1), sprint = 2 of group 0x36 when in the player's group with a leader moving
/// at >= run, else of the ped's group, at 1; the result gets `SetMoveAnimSpeed` unless set by task.
///
/// gta-reversed deviations (bytes kept): the move-anim-speed window for the unchanged case; dead = ped
/// states 0x36/0x37 (`IsAlive` inlined).
#[hook(addr = 0x5E4A00, conv = thiscall, name = "CPed::SetMoveAnim", expect = "56 8B F1 8B 86 30 05 00 00", this = "CPed")]
pub fn set_move_anim(this: GPtr<u8>) {
    let st = rd::<i32>(this, 0x530);
    if st == 0x36 || st == 0x37 || rd::<u32>(this, 0x46C) & 0x400_0000 != 0 || !rd::<GPtr<u8>>(this, 0xFC).is_null() {
        return;
    }
    let ms = rd::<i32>(this, 0x534);
    let clump = || rd::<GPtr<u8>>(this, 0x18);
    let update = |a: GPtr<u8>| {
        if !a.is_null() && rd::<u8>(this, 0x47A) & 1 == 0 {
            set_move_anim_speed(this, a);
        }
    };
    if rd::<i32>(this, 0x538) == ms {
        if ms < 4 {
            return;
        }
        let sp = x(rd::<f32>(this, 0x518));
        if sp.fcom(x(f32::from_bits(0x3C23_D70A))).test_any(0x41)
            && sp.fcom(x(f32::from_bits(0xBC23_D70A))).test_parity(5)
        {
            return;
        }
        let id = if ms == 6 { 1 } else if ms == 7 { 2 } else { 0 };
        update(clump_get_association(clump(), id));
        return;
    }
    if ms == 0 {
        wr::<i32>(this, 0x538, 0);
        return;
    }
    let group = rd::<i32>(this, 0x4D4);
    wr::<i32>(this, 0x538, ms);
    if ms == 4 || ms == 6 || ms == 7 {
        let mut a = clump_get_first_association_flags(clump(), 0x10);
        while !a.is_null() {
            let f = rd::<u16>(a, 0x2E);
            if f & 8 == 0 && f & 0x400 == 0 {
                wr::<u32>(a, 0x1C, 0xC000_0000);
                wr::<u16>(a, 0x2E, f | 4);
            }
            a = get_next_association_flags(a, 0x10);
        }
        clear_aim_flag(this);
        crate::ped::clear_look_flag_call(this);
    }
    let r = match rd::<i32>(this, 0x534) {
        1 => blend_animation(clump(), group, 3, 0x4080_0000),
        2 => blend_animation(clump(), 0, 0x89, 0x4180_0000),
        3 => blend_animation(clump(), 0, 0x8A, 0x4180_0000),
        4 => blend_animation(clump(), group, 0, 0x3F80_0000),
        6 => {
            let d = if rd::<i32>(this, 0x530) == 9 { 0x4040_0000 } else { 0x3F80_0000 };
            blend_animation(clump(), group, 1, d)
        }
        7 => {
            let fast_leader = is_in_players_group(this) != 0
                && !group_get_leader(get_peds_group(this).byte_add(8)).is_null()
                && rd::<i32>(group_get_leader(get_peds_group(this).byte_add(8)), 0x534) >= 6;
            blend_animation(clump(), if fast_leader { 0x36 } else { group }, 2, 0x3F80_0000)
        }
        _ => return,
    };
    update(r);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_PED_POSITION_IN_CAR, HOOK_UPDATE_POSITION, HOOK_PROCESS_BUOYANCY,
    HOOK_SPECIAL_ENTITY_PRE_COLLISION_STUFF,
    HOOK_SPECIAL_ENTITY_CALC_COLLISION_STEPS,
    HOOK_TELEPORT,
    HOOK_SET_MOVE_ANIM,
];
