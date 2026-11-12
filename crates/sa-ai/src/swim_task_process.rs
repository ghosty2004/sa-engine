//! `CTaskSimpleSwim::ProcessPed` and the per-frame helpers it calls (`ProcessControlInput`, `ProcessSwimAnims`,
//! `ProcessSwimmingResistance`, `ProcessEffects`, `ApplyRollAndPitch`), executed by the original under SA:MP
//! 0.3.7 (`catalog/samp-priority.csv`) (PORT-AI-8). Ported from the 1.0 US bytes (the exe wins over
//! gta-reversed). Every call to another catalogued function goes through `orig!` at the original address; the
//! CRT intrinsics `_CIasin` (0x821E70) and `_CIpow` (0x822130) go through `sa_core::orig::x87_unary` /
//! `x87_binary`.
//!
//! Layout used (`CTaskSimpleSwim`, 0x64 bytes): +8 finished blending, +9 anim block ref added, +0xA swim state
//! (u16), +0xC anim id, +0x10 anim speed, +0x14 position, +0x20 ped, +0x24 rotation x (pitch), +0x28 turning
//! rotation y (roll), +0x2C upper torso rotation x, +0x30 aiming rotation, +0x34 state changer, +0x38 climb
//! entity, +0x3C climb position, +0x48 angle, +0x4C surface type, +0x50 random move blend ratio, +0x54 swim stop
//! time, +0x58 time step, +0x5C fx system, +0x60 water splash triggered.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;
use sa_math::x87::Cmp;

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// 0.0f
const K_ZERO: usize = 0x858B50;
/// 1.0f
const K_ONE: usize = 0x858624;
/// -1.0f
const K_MINUS_ONE: usize = 0x858C1C;
/// 0.5f
const K_HALF: usize = 0x858B8C;
/// pi (f32)
const K_PI: usize = 0x858CB8;
/// 2 pi (f32)
const K_TWO_PI: usize = 0x858CBC;
/// -pi (f32)
const K_MINUS_PI: usize = 0x858CC0;
/// 10.0f
const K_TEN: usize = 0x85862C;
/// 0.01f
const K_SMALL: usize = 0x858C58;
/// -0.01f
const K_MINUS_SMALL: usize = 0x86C310;
/// `_CIasin`
const CI_ASIN: usize = 0x821E70;
/// `_CIpow`
const CI_POW: usize = 0x822130;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `g_fxMan`
const FX_MAN: usize = 0xA9AE80;
/// `g_fx` (`Fx_c`)
const G_FX: usize = 0xA9AE00;

orig! {
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `RpAnimBlendClumpSetBlendDeltas(RpClump*, uint32 flags, float delta)` (0x4D67E0, cdecl)
    fn clump_set_blend_deltas(clump: u32, flags: u32, delta: u32) = cdecl 0x4D67E0 => unported;
    /// `CPed::RestoreHeadingRate` (0x5DFD60)
    fn ped_restore_heading_rate(p: GPtr<u8>) = thiscall 0x5DFD60 => unported;
    /// `FxSystem_c::Kill` (0x4AA3F0)
    fn fx_system_kill(f: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskSimpleSwim::ProcessControlAI(CPed*)` (0x689640, `ret 4`)
    fn o_swim_process_control_ai(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x689640 => unported;
    /// `CTaskSimpleSwim::ProcessControlInput(CPlayerPed*)` (0x688A90, `ret 4`)
    fn o_swim_process_control_input(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x688A90 => unported;
    /// `CTaskSimpleSwim::ProcessSwimAnims(CPed*)` (0x6899F0, `ret 4`)
    fn o_swim_process_swim_anims(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6899F0 => unported;
    /// `CTaskSimpleSwim::ProcessSwimmingResistance(CPed*)` (0x68A1D0, `ret 4`)
    fn o_swim_process_swimming_resistance(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x68A1D0 => unported;
    /// `CTaskSimpleSwim::ProcessEffects(CPed*)` (0x68AA70, `ret 4`)
    fn o_swim_process_effects(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x68AA70 => unported;
    /// `CTaskSimpleSwim::DestroyFxSystem` (0x68AA50)
    fn o_swim_destroy_fx_system(t: GPtr<u8>) = thiscall 0x68AA50 => unported;
    /// `CTaskSimpleSwim::CreateFxSystem(CPed*, RwMatrix*)` (0x68A9F0, `ret 8`)
    fn o_swim_create_fx_system(t: GPtr<u8>, ped: GPtr<u8>, m: GPtr<u8>) = thiscall 0x68A9F0 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator+(CVector* out, const CVector* a, const CVector* b)` (0x40FE30): out = a + b
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CPlayerPed::HandlePlayerBreath(bool underWater, float rate)` (0x60A8D0, `ret 8`)
    fn player_handle_player_breath(p: GPtr<u8>, under: u32, rate: u32) = thiscall 0x60A8D0 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, cdecl, ST0)
    fn stats_get_fat_and_muscle_modifier(i: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes,
    /// bool)` (0x4AD3F0, `ret 0x14`)
    fn calc_ctor(c: GPtr<u8>, src: u32, dmg: u32, wt: u32, piece: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator` (0x4AD420)
    fn calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse*, bool)` (0x4B5AC0, `ret 0xC`)
    fn calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>, b: u32) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 dir, bool, bool)`
    /// (0x4AD830, `ret 0x1C`)
    fn damage_ctor(e: GPtr<u8>, src: u32, time: u32, wt: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`)
    fn damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CEventDamage::~CEventDamage` (0x4AD960)
    fn damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPlayerPed::GetPadFromPlayer` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetPedWalkUpDown` (0x53FD30; AX)
    fn pad_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPad::GetPedWalkLeftRight` (0x53FC90; AX)
    fn pad_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CPad::JumpJustDown` (0x540770)
    fn pad_jump_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540770 => unported;
    /// `CPad::WeaponJustDown(CPed*)` (0x540250, `ret 4`)
    fn pad_weapon_just_down(pad: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x540250 => unported;
    /// `CCamera::GetLookDirection` (0x50AE90)
    fn camera_get_look_direction(c: GPtr<u8>) -> i32 = thiscall 0x50AE90 => unported;
    /// `FUN_00420800(float a, float b)` (0x420800, cdecl, ST0): `a > b ? a : b` (`test ah, 0x41`)
    fn max_f(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `FUN_00404330(float a, float b)` (0x404330, cdecl, ST0): `a < b ? a : b` (`test ah, 5; jp`)
    fn min_f(a: u32, b: u32) -> f64 = cdecl 0x404330 => unported;
    /// `CPlayerPed::ControlButtonSprint(eSprintType)` (0x60A610, `ret 4`, ST0)
    fn player_control_button_sprint(p: GPtr<u8>, ty: u32) -> f64 = thiscall 0x60A610 => unported;
    /// `CPlayerPed::GetButtonSprintResults(eSprintType)` (0x60A820, `ret 4`, ST0)
    fn player_get_button_sprint_results(p: GPtr<u8>, ty: u32) -> f64 = thiscall 0x60A820 => unported;
    /// `CPlayerPed::HandleSprintEnergy(bool, float)` (0x60A550, `ret 8`)
    fn player_handle_sprint_energy(p: GPtr<u8>, b: u32, f: u32) = thiscall 0x60A550 => unported;
    /// `CGameLogic::IsPlayerUse2PlayerControls(CPed*)` (0x442020, cdecl)
    fn game_logic_use_2player_controls(p: GPtr<u8>) -> u8 = cdecl 0x442020 => unported;
    /// `CGameLogic::IsPlayerAllowedToGoInThisDirection(CPed*, CVector dir, float)` (0x441E10, cdecl; the vector by
    /// value)
    fn game_logic_allowed_direction(p: GPtr<u8>, x: u32, y: u32, z: u32, d: u32) -> u8 = cdecl 0x441E10 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, cdecl, ST0)
    fn general_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CGeneral::GetAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CEA0, cdecl, ST0)
    fn general_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CEA0 => unported;
    /// `CGeneral::LimitAngle(float)` (0x53CB00, cdecl, ST0)
    fn general_limit_angle(a: u32) -> f64 = cdecl 0x53CB00 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool waves, CVector* normal)` (0x6EB690,
    /// cdecl)
    fn water_get_water_level(x: u32, y: u32, z: u32, out: GPtr<u8>, waves: u32, n: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CAnimManager::GetAnimBlockName(AssocGroupId)` (0x4D3A30, cdecl)
    fn anim_get_anim_block_name(g: u32) -> u32 = cdecl 0x4D3A30 => unported;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940, cdecl)
    fn anim_get_animation_block(name: u32) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0, cdecl)
    fn streaming_request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0, cdecl)
    fn anim_add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CTaskSimpleClimb::TestForClimb(CPed*, CVector& pos, float& angle, uint8& surface, bool)` (0x6803A0, cdecl)
    fn climb_test_for_climb(ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surf: GPtr<u8>, b: u32) -> GPtr<u8> = cdecl 0x6803A0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CAnimBlendAssociation::Start(float)` (0x4CEB70, `ret 4`)
    fn assoc_start(a: GPtr<u8>, t: u32) = thiscall 0x4CEB70 => unported;
    /// `CStats::UpdateStatsWhenSwimming(bool underWater, bool sprinting)` (0x55C990, cdecl)
    fn stats_update_stats_when_swimming(under: u32, sprint: u32) = cdecl 0x55C990 => unported;
    /// `CPhysical::ApplyMoveForce(float x, float y, float z)` (0x4ABBA0, `ret 0xC`)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x4ABBA0 => unported;
    /// `RwMatrixCreate` (0x7F2A50, cdecl)
    fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0, cdecl)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20, cdecl)
    fn rw_matrix_destroy(m: GPtr<u8>) -> u32 = cdecl 0x7F2A20 => unported;
    /// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630, `ret 4`)
    fn fx_system_set_matrix(f: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4AA630 => unported;
    /// `FxSystem_c::SetLocalParticles(uint32)` (0x4AA910, `ret 4`)
    fn fx_system_set_local_particles(f: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `Fx_c::TriggerWaterSplash(CVector&)` (0x4A1070, `ret 4`)
    fn fx_trigger_water_splash(fx: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4A1070 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32 id, float volume, float speed, CPhysical*, uint8, int32, uint32)`
    /// (0x4E2BB0, `ret 0x1C`)
    fn ped_audio_add_audio_event(a: GPtr<u8>, id: u32, vol: u32, speed: u32, e: u32, b: u32, c: u32, d: u32) = thiscall 0x4E2BB0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32 min, int32 max)` (0x407180, cdecl)
    fn general_random_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40, cdecl)
    fn get_anim_hierarchy_from_skin_clump(clump: u32) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120, cdecl)
    fn hanim_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32 id)` (0x7C51A0, cdecl)
    fn hanim_id_get_index(h: GPtr<u8>, id: u32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, float g, float b, float a, float size, float, float life)` (0x4AB290,
    /// `ret 0x1C`)
    fn fx_prt_mult_ctor(m: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d* pos, RwV3d* vel, float time, FxPrtMult_c*, float rot, float bright, float,
    /// uint8)` (0x4AA440, `ret 0x20`)
    fn fx_system_add_particle(f: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, t: u32, m: GPtr<u8>, rot: u32, br: u32, a: u32, b: u32) = thiscall 0x4AA440 => unported;
    /// `FxManager_c::CreateFxSystem(char* name, RwV3d* pos, RwMatrix* parent, bool)` (0x4A9BE0, `ret 0x10`)
    fn fx_manager_create_fx_system(m: GPtr<u8>, name: u32, pos: GPtr<u8>, parent: u32, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::PlayAndKill` (0x4AA3D0)
    fn fx_system_play_and_kill(f: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
    /// `CPedIK::RotateTorso(AnimBlendFrameData*, LimbOrientation&, bool)` (0x5FDDB0, `ret 0xC`)
    fn ped_ik_rotate_torso(ik: GPtr<u8>, frame: u32, orient: GPtr<u8>, b: u32) -> u8 = thiscall 0x5FDDB0 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool owns)` (0x59C050, `ret 8`)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40, `ret 0xC`)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0, `ret 4`)
    fn matrix_rotate_y(m: GPtr<u8>, a: u32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0, `ret 4`)
    fn matrix_rotate_x(m: GPtr<u8>, a: u32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80, `ret 4`)
    fn matrix_mul_assign(m: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A80 => unported;
    /// `CMatrix::UpdateRW` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEntity::UpdateRwFrame` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
}

/// A dword-aligned stack object of `N` dwords, as a game pointer.
fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The f32 constant at `a`.
fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The f32 bits of an x87 value as `fstp dword` stores them.
fn bits(v: X87) -> u32 {
    f32_of(v).to_bits()
}

/// The ped's position: its matrix +0x30, or the placement at +4 without a matrix.
fn pos_of(ped: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) }
}

/// MSVC's clamp of a register value: `fld hi; fcomp st(1)` taken (`hi < v`) gives `hi`, else `fld lo; fcomp
/// st(1)` with `lo > v` gives `lo`, else `v` (a NaN stays).
fn clamp_reg(v: X87, lo: f32, hi: f32) -> X87 {
    if x(hi).fcom(v) == Cmp::Less {
        x(hi)
    } else if !x(lo).fcom(v).test_any(0x41) {
        x(lo)
    } else {
        v
    }
}

/// The angle wrap after `fcom pi`: above pi (`test ah, 0x41; je`) subtracts 2 pi; else below -pi (`test ah, 5;
/// jp` not taken) adds 2 pi. `None` when unchanged (the original pops it then).
fn wrap_pi(v: X87) -> Option<X87> {
    if !v.fcom(x(k(K_PI))).test_any(0x41) {
        Some(v.fsub_m(k(K_TWO_PI)))
    } else if !v.fcom(x(k(K_MINUS_PI))).test_parity(0x05) {
        Some(v.fadd_m(k(K_TWO_PI)))
    } else {
        None
    }
}

/// `fst [m]` of `v` to the f32 at `ped + off`, then the [`wrap_pi`] correction stored over it.
fn store_wrapped(p: GPtr<u8>, off: isize, v: X87) {
    wr::<f32>(p, off, f32_of(v));
    if let Some(w) = wrap_pi(v) {
        wr::<f32>(p, off, f32_of(w));
    }
}

/// `-(rot - cur)` wrapped into [-pi, pi], times 10.0, clamped by `FUN_00420800(-1.0, .)` then
/// `FUN_00404330(1.0, .)` (both through the stack as f32).
fn turn_input(rot: f32, cur: f32) -> X87 {
    let d = x(rot).fsub_m(cur).fchs();
    let d = wrap_pi(d).unwrap_or(d).fmul_m(k(K_TEN));
    let r1 = X87::from_f64_exact(max_f(0xBF80_0000, bits(d)));
    X87::from_f64_exact(min_f(0x3F80_0000, bits(r1)))
}

/// `this +0x24 / +0x28 / +0x30 / +0x2C` decay of the tail of `ProcessControlInput`: a value inside
/// [-0.01, 0.01] (`> 0.01` and `< -0.01` both fail; a NaN too) is zeroed, else multiplied by `pw`.
fn outside_small(v: f32) -> bool {
    x(v).fcom(x(k(K_SMALL))) == Cmp::Greater || x(v).fcom(x(k(K_MINUS_SMALL))) == Cmp::Less
}

/// `CTaskSimpleSwim::ProcessPed(CPed*)` (0x68B1C0, `ret 4`). With a climb entity (+0x38): blends the ped's anim 3
/// of its group (+0x4D4) at 8.0, move states +0x534/+0x538 = 1, kills the fx system, AL = 1. Ends the swim (blends
/// out the swim anim +0xC unless 0xBF: anim 0x80 gets flag byte +0x2E bit 3, others delta -4.0; anim 0x137 / 0x138
/// pick move states 4 / 6 and anim 0 / 1, else anim 3; blend at 4.0; heading rate restored; fx killed; AL = 1) when
/// the stop time +0x54 is above 15.0 or the ped's flag bit 0x100 of +0x46C is set. Otherwise clears flag 0x200 of
/// +0x46C and: a player with a time step (+0x58) outside state 4 runs `ProcessControlAI` and counts the time step
/// down by `ftol(step * 0.02 * 1000.0)` (unsigned, floored at 0); with the distance to +0x14 below 0.5 (stored in
/// the player data's +0x14) it zeroes it, blends anim 3 at 4.0, restores the heading rate, destroys the fx and
/// returns AL = 1 (no fx kill); above 1.0 it is clamped to 1.0. Without a time step it runs `ProcessControlInput`
/// and zeroes +0x58. Then `HandlePlayerBreath(state == 4, rate)` with rate 1.0, or `0x13A`'s speed * blend + 1.0
/// under water (the original writes the rate over its own argument slot and pushes the whole dword of the bool's
/// stack byte); `Say(0x164)` when blended in, not under water and `GetFatAndMuscleModifier(8) * 0.5 > breath
/// (+0x44)`; `SetMoveState(0)`. A non-player runs `ProcessControlAI` and, under water (state 4), adds a drowning
/// `CEventDamage` (weapon 0x35, piece 3, `(+0x46C >> 8) & 0xFFFFFF01` as the in-vehicle flag; the response is
/// computed when it affects the ped, else byte +0x42 = 1) to its event group (+0x47C +0x68). Finally the anims,
/// resistance and effects are processed and AL = 0.
#[hook(addr = 0x68B1C0, conv = thiscall, name = "CTaskSimpleSwim::ProcessPed", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "ai8-1", route = "L2", hazards = "ax-al-return,heap-writes,uninit-locals",
       recs = "air-water", note = "the original overwrites its own CPed* argument slot with the breath rate")]
pub fn swim_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x38) != 0 {
        anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x4100_0000);
        wr::<u32>(ped, 0x534, 1);
        wr::<u32>(ped, 0x538, 1);
        kill_fx(this);
        return 1;
    }
    let ended = !x(rd::<f32>(this, 0x54)).fcom(x(k(0x8D2FC0))).test_any(0x41);
    let fl = rd::<u32>(ped, 0x46C);
    if ended || fl & 0x100 != 0 {
        let anim = rd::<u32>(this, 0xC);
        let a = if anim == 0xBF { GPtr::NULL } else { clump_get_association(rd::<u32>(ped, 0x18), anim) };
        wr::<u32>(ped, 0x538, 1);
        wr::<u32>(ped, 0x534, 1);
        let mut id = 3;
        if !a.is_null() {
            if rd::<u16>(a, 0x2C) == 0x80 {
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 8);
            } else {
                wr::<u32>(a, 0x1C, 0xC080_0000);
            }
            let anim = rd::<u32>(this, 0xC);
            if anim == 0x137 {
                wr::<u32>(ped, 0x538, 4);
                wr::<u32>(ped, 0x534, 4);
                id = 0;
            } else if anim == 0x138 {
                wr::<u32>(ped, 0x538, 6);
                wr::<u32>(ped, 0x534, 6);
                id = 1;
            }
        }
        anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), id, 0x4080_0000);
        ped_restore_heading_rate(ped);
        kill_fx(this);
        return 1;
    }
    wr::<u32>(ped, 0x46C, fl & 0xFFFF_FDFF);
    if ped_is_player(ped) != 0 {
        if rd::<u32>(this, 0x58) != 0 && rd::<u16>(this, 0xA) != 4 {
            o_swim_process_control_ai(this, ped);
            let st = ftol32(x(k(TIME_STEP)).fmul_m(k(0x858B38)).fmul_m(k(0x858C4C)));
            let t = rd::<u32>(this, 0x58);
            wr::<u32>(this, 0x58, t.saturating_sub(st as u32));
            let mut d = [0u32; 3];
            let r = vector_sub(local(&mut d), this.byte_add(0x14), pos_of(ped));
            let (dx, dy) = (x(rd::<f32>(r, 0)), x(rd::<f32>(r, 4)));
            let len = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            wr::<f32>(pd, 0x14, f32_of(len));
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            if !x(rd::<f32>(pd, 0x14)).fcom(x(k(0x870878))).test_parity(0x05) {
                wr::<u32>(pd, 0x14, 0);
                anim_blend_animation(rd::<u32>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, 0x4080_0000);
                ped_restore_heading_rate(ped);
                o_swim_destroy_fx_system(this);
                return 1;
            }
            if !x(rd::<f32>(pd, 0x14)).fcom(X87::from_f64_exact(g::<f64>(0x85A310).read())).test_any(0x41) {
                wr::<u32>(pd, 0x14, 0x3F80_0000);
            }
        } else {
            o_swim_process_control_input(this, ped);
            wr::<u32>(this, 0x58, 0);
        }
        let mut under = 0u32;
        let mut rate = 1.0f32;
        if rd::<u16>(this, 0xA) == 4 {
            under = 1;
            let a = clump_get_association(rd::<u32>(ped, 0x18), 0x13A);
            if !a.is_null() {
                rate = f32_of(x(rd::<f32>(a, 0x24)).fmul_m(rd::<f32>(a, 0x18)).fadd_m(k(K_ONE)));
            }
        }
        player_handle_player_breath(ped, under, rate.to_bits());
        if rd::<u8>(this, 8) != 0 && rd::<u16>(this, 0xA) != 4 {
            let pd = rd::<GPtr<u8>>(ped, 0x480);
            let m = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(8)).fmul_m(k(K_HALF));
            if !m.fcom(x(rd::<f32>(pd, 0x44))).test_any(0x41) {
                ped_say(ped, 0x164, 0, 0x3F80_0000, 0, 0, 0);
            }
        }
        ped_set_move_state(ped, 0);
    } else {
        o_swim_process_control_ai(this, ped);
        if rd::<u16>(this, 0xA) == 4 {
            let mut calc = [0u32; 5];
            calc_ctor(local(&mut calc), 0, g::<u32>(TIME_STEP).read(), 0x35, 3, 0);
            let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
            let mut ev = [0u32; 0x11];
            damage_ctor(local(&mut ev), 0, g::<u32>(TIME_MS).read(), 0x35, 3, 0, 0, in_veh);
            if damage_affects_ped(local(&mut ev), ped) != 0 {
                calc_compute_damage_response(local(&mut calc), ped, local(&mut ev).byte_add(0x38), 1);
            } else {
                wr::<u8>(local(&mut ev), 0x42, 1);
            }
            event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), local(&mut ev), 0);
            damage_dtor(local(&mut ev));
            calc_dtor(local(&mut calc));
        }
    }
    o_swim_process_swim_anims(this, ped);
    o_swim_process_swimming_resistance(this, ped);
    o_swim_process_effects(this, ped);
    0
}

/// Kills and clears the fx system at +0x5C when there is one.
fn kill_fx(this: GPtr<u8>) {
    let fx = rd::<GPtr<u8>>(this, 0x5C);
    if !fx.is_null() {
        fx_system_kill(fx);
        wr::<u32>(this, 0x5C, 0);
    }
}

/// `CTaskSimpleSwim::ProcessControlInput(CPlayerPed*)` (0x688A90, `ret 4`). Before the blend and the anim block
/// (+8, +9) the player data's move speed +0x14 is zeroed. Otherwise reads the pad (walk up/down and left/right
/// times 1/128; their length), switches to state 5 on jump or 3 (move speed 0) on fire below state 2, then by
/// state: 3 zeroes a positive +0x34; 4 steers under water (with the 3rd-person mouse flag 0xB6EC2E from the active
/// camera's front vector: aim heading +0x55C, roll input clamped to [-1, 1] from the heading error, pitch input
/// from `asin(front.z)` (0 unless looking forward) minus +0x24; else from the stick), adding to +0x28 / +0x30 /
/// +0x24 / +0x2C, then clamps +0x24 + step * 0.001 to [-1.396, 1.396], sets a non-positive breath rate +0x18 to
/// 0.1 and calls `ControlButtonSprint(3)`; 0..2 steer on the surface (2-player controls: the stick's camera-
/// relative heading with `IsPlayerAllowedToGoInThisDirection`), accelerate the move speed by at most step * 0.07
/// (decelerate by step * 0.05 without input), turn +0x30 / +0x28 / +0x2C, sprint (state 2) pitches +0x24 by the
/// water slope 1 unit ahead and behind, and the sprint button picks state 2, else 1 above move speed 0.5, else 0.
/// The tail decays +0x28 (and +0x24 outside states 2 / 4) by `pow(0.95, step)`, +0x30 / +0x2C by `pow(0.95 |
/// 0.92, step)` (values inside +-0.01 are zeroed) and calls `HandleSprintEnergy(0, 0.5)` in state 1, `(0, 1.0)`
/// outside state 2. The original keeps its locals in its argument slot and the left/right slot (overwritten with
/// the clamped turn input); the port keeps them in Rust locals.
#[hook(addr = 0x688A90, conv = thiscall, name = "CTaskSimpleSwim::ProcessControlInput", expect = "83 EC 1C 53 55 56",
       unverified = "ai8-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "air-water")]
pub fn swim_process_control_input(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 8) == 0 || rd::<u8>(this, 9) == 0 {
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0);
        return;
    }
    let ts = || k(TIME_STEP);
    let pad = player_get_pad(ped);
    let ud_raw = f32_of(xi(i32::from(pad_walk_up_down(pad))));
    let mut lr = f32_of(xi(i32::from(pad_walk_left_right(pad))).fmul_m(k(0x858B88)));
    let mut ud = f32_of(x(ud_raw).fmul_m(k(0x858B88)));
    let mut mag = f32_of(x(ud).fmul_m(ud).fadd(x(lr).fmul_m(lr)).fsqrt());
    let state = rd::<i16>(this, 0xA);
    if state < 2 {
        if pad_jump_just_down(pad) != 0 {
            wr::<u16>(this, 0xA, 5);
        } else if pad_weapon_just_down(pad, ped) != 0 {
            wr::<u16>(this, 0xA, 3);
            wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0);
        }
    }
    let st = rd::<i16>(this, 0xA) as i32;
    match st {
        3 => {
            if !x(rd::<f32>(this, 0x34)).fcom(x(k(K_ZERO))).test_any(0x41) {
                wr::<u32>(this, 0x34, 0);
            }
        }
        4 => {
            let p = if g::<u8>(0xB6EC2E).read() != 0 {
                let cam = cam_front();
                let (fx, fy) = (cam[0], cam[1]);
                let mut fz = cam[2];
                let (sx, sy) = if camera_get_look_direction(g(THE_CAMERA)) != 3 {
                    fz = 0.0;
                    (x(fx).fmul_m(k(K_MINUS_ONE)), x(fy).fmul_m(k(K_MINUS_ONE)))
                } else {
                    (x(fx), x(fy))
                };
                let aim = sx.fchs().fpatan(sy);
                wr::<f32>(ped, 0x55C, f32_of(aim));
                let d = aim.fsub_m(rd::<f32>(ped, 0x558)).fchs();
                let d = wrap_pi(d).unwrap_or(d).fmul_m(k(K_TEN));
                let r1 = X87::from_f64_exact(max_f(0xBF80_0000, bits(d)));
                lr = f32_of(X87::from_f64_exact(min_f(0x3F80_0000, bits(r1))));
                add_f(this, 0x28, x(ts()).fmul_m(k(0x870888)).fmul_m(lr));
                add_f(this, 0x30, x(ts()).fmul_m(k(0x87088C)).fmul_m(lr));
                let a = X87::from_f64_exact(sa_core::orig::x87_unary(CI_ASIN, f64::from(fz)));
                let v = a.fsub_m(rd::<f32>(this, 0x24)).fmul_m(k(K_TEN));
                let r1 = X87::from_f64_exact(max_f(0xBF80_0000, bits(v)));
                X87::from_f64_exact(min_f(0x3F80_0000, bits(r1)))
            } else {
                if x(mag).fcom(x(k(K_ZERO))).test_any(0x41) {
                    swim_pitch_tail(this, ped);
                    return swim_control_tail(this, ped);
                }
                let inv = x(k(K_ONE)).fdiv_m(mag);
                lr = f32_of(x(lr).fmul(inv));
                let u = inv.fmul_m(ud);
                let rot = x(ts()).fmul_m(k(0x87087C)).fmul_m(lr).fadd_m(rd::<f32>(ped, 0x55C));
                store_wrapped(ped, 0x55C, rot);
                add_f(this, 0x28, x(ts()).fmul_m(k(0x870888)).fmul_m(lr));
                add_f(this, 0x30, x(ts()).fmul_m(k(0x87088C)).fmul_m(lr));
                u
            };
            if x(rd::<f32>(this, 0x34)).fcom(x(k(K_ZERO))) == Cmp::Equal || !p.fcom(x(k(K_ZERO))).test_any(0x41) {
                add_f(this, 0x24, x(ts()).fmul_m(k(0x870884)).fmul(p));
            }
            let t = clamp_reg(x(rd::<f32>(this, 0x28)).fdiv_m(k(0x8708A4)), k(K_MINUS_ONE), k(K_ONE));
            let q = clamp_reg(p.fadd(t.fmul_m(k(0x870890)).fmul_m(lr)), k(K_MINUS_ONE), k(K_ONE));
            add_f(this, 0x2C, x(ts()).fmul_m(k(0x870890)).fmul(q));
            swim_pitch_tail(this, ped);
        }
        0..=2 => swim_surface_input(this, ped, &mut lr, &mut ud, &mut mag),
        _ => {}
    }
    swim_control_tail(this, ped)
}

/// `TheCamera.Cams[ActiveCam].Front` (0xB6F32C + 0x238 * [0xB6F081]).
fn cam_front() -> [f32; 3] {
    let c = GPtr::<u8>::from_addr(usize::from(g::<u8>(0xB6F081).read()) * 0x238 + 0xB6F32C);
    [rd::<f32>(c, 0), rd::<f32>(c, 4), rd::<f32>(c, 8)]
}

/// `fadd [this + off]; fstp [this + off]` of `v`.
fn add_f(p: GPtr<u8>, off: isize, v: X87) {
    wr::<f32>(p, off, f32_of(v.fadd_m(rd::<f32>(p, off))));
}

/// State 4's end (0x688EE0): +0x24 + step * 0.001 clamped to [-1.396, 1.396]; a breath rate (player data +0x18)
/// at or below 0.0 becomes 0.1; `ControlButtonSprint(3)` (its ST0 result dropped).
fn swim_pitch_tail(this: GPtr<u8>, ped: GPtr<u8>) {
    let r = x(k(TIME_STEP)).fmul_m(k(0x8708B4)).fadd_m(rd::<f32>(this, 0x24));
    wr::<f32>(this, 0x24, f32_of(r));
    wr::<f32>(this, 0x24, f32_of(clamp_reg(r, k(0x870944), k(0x870948))));
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !x(rd::<f32>(pd, 0x18)).fcom(x(k(K_ZERO))).test_parity(0x41) {
        wr::<u32>(pd, 0x18, 0x3DCC_CCCD);
    }
    player_control_button_sprint(ped, 3);
}

/// States 0..2 of `ProcessControlInput` (0x688F65..0x689497).
fn swim_surface_input(this: GPtr<u8>, ped: GPtr<u8>, lr: &mut f32, ud: &mut f32, mag: &mut f32) {
    let ts = || k(TIME_STEP);
    let two = game_logic_use_2player_controls(ped) != 0;
    let mut s;
    if two {
        s = x(*lr);
        if !x(*mag).fcom(x(k(K_ZERO))).test_any(0x41) {
            let a0 = X87::from_f64_exact(general_radian_angle_between_points(0, 0, bits(x(*lr).fchs()), ud.to_bits()));
            let a1 = f32_of(a0.fsub_m(k(0xB6F178)));
            let ang = X87::from_f64_exact(general_limit_radian_angle(a1.to_bits()));
            let dir = [f32_of(ang.fsin().fchs()), f32_of(ang.fcos()), 0.0f32];
            let cur = rd::<f32>(ped, 0x558);
            let mut a = ang;
            if !a.fcom(x(cur).fadd_m(k(K_PI))).test_any(0x41) {
                a = a.fsub_m(k(K_TWO_PI));
            } else if !a.fcom(x(cur).fsub_m(k(K_PI))).test_parity(0x05) {
                a = a.fadd_m(k(K_TWO_PI));
            }
            let mut d = a.fsub_m(rd::<f32>(ped, 0x558));
            if !d.fcom(x(k(K_ONE))).test_any(0x41) {
                d = x(k(K_ONE));
            } else if !d.fcom(x(k(K_MINUS_ONE))).test_parity(0x05) {
                d = x(k(K_MINUS_ONE));
            }
            let rot = d.fmul(x(ts()).fmul_m(k(0x870880))).fadd_m(rd::<f32>(ped, 0x558));
            store_wrapped(ped, 0x55C, rot);
            if game_logic_allowed_direction(ped, dir[0].to_bits(), dir[1].to_bits(), dir[2].to_bits(), 0) != 0 {
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let a1 = x(dir[1])
                    .fmul_m(rd::<f32>(m, 4))
                    .fadd(x(dir[0]).fmul_m(rd::<f32>(m, 0)))
                    .fadd(x(rd::<f32>(m, 8)).fmul_m(k(K_ZERO)))
                    .fmul_m(*mag);
                let b = x(dir[1])
                    .fmul_m(rd::<f32>(m, 0x14))
                    .fadd(x(rd::<f32>(m, 0x18)).fmul_m(k(K_ZERO)))
                    .fadd(x(dir[0]).fmul_m(rd::<f32>(m, 0x10)))
                    .fmul_m(*mag)
                    .fchs();
                *ud = f32_of(b);
                s = a1;
            } else {
                s = x(k(K_ZERO));
                *mag = 0.0;
                *ud = 0.0;
            }
        }
    } else {
        s = x(*lr);
    }
    let pd = || rd::<GPtr<u8>>(ped, 0x480);
    if g::<u8>(0xB6EC2E).read() != 0 {
        let c = cam_front();
        let aim = x(c[0]).fchs().fpatan(x(c[1]));
        wr::<f32>(ped, 0x55C, f32_of(aim));
        if camera_get_look_direction(g(THE_CAMERA)) != 3 {
            let r = x(rd::<f32>(ped, 0x55C)).fadd_m(k(K_PI));
            wr::<f32>(ped, 0x55C, f32_of(r));
            if !r.fcom(x(k(K_PI))).test_any(0x41) {
                wr::<f32>(ped, 0x55C, f32_of(r.fsub_m(k(K_TWO_PI))));
            }
        }
        let w = x(*ud).fchs();
        let lim = f32_of(x(ts()).fmul_m(k(0x858CA8)));
        *mag = lim;
        let p = pd();
        let w = if !w.fsub_m(rd::<f32>(p, 0x14)).fcom(x(lim)).test_any(0x41) { x(lim).fadd_m(rd::<f32>(p, 0x14)) } else { w };
        wr::<f32>(p, 0x14, f32_of(w));
        s = turn_input(rd::<f32>(ped, 0x55C), rd::<f32>(ped, 0x558));
    } else if !x(*mag).fcom(x(k(K_ZERO))).test_any(0x41) {
        if !two {
            let rot = x(ts()).fmul_m(k(0x87087C)).fmul(s).fadd_m(rd::<f32>(ped, 0x55C));
            store_wrapped(ped, 0x55C, rot);
            if rd::<u16>(this, 0xA) == 0 {
                *mag = f32_of(x(*ud).fchs());
            } else if !x(*ud).fcom(x(k(K_ZERO))).test_any(0x41) {
                *mag = f32_of(x(*mag).fsub_m(*ud));
            }
            if !x(*mag).fcom(x(k(K_ZERO))).test_parity(0x05) {
                *mag = 0.0;
            }
        }
        let lim = x(ts()).fmul_m(k(0x858CA8));
        let p = pd();
        if !x(*mag).fsub_m(rd::<f32>(p, 0x14)).fcom(lim).test_any(0x41) {
            wr::<f32>(p, 0x14, f32_of(lim.fadd_m(rd::<f32>(p, 0x14))));
        } else {
            wr::<f32>(p, 0x14, *mag);
        }
    } else {
        let p = pd();
        if !x(rd::<f32>(p, 0x14)).fcom(x(k(K_ZERO))).test_any(0x41) {
            let v = x(ts()).fmul_m(k(0x858C28)).fsubr_m(rd::<f32>(p, 0x14));
            let v = if !x(k(K_ZERO)).fcom(v).test_any(0x41) { x(k(K_ZERO)) } else { v };
            wr::<f32>(p, 0x14, f32_of(v));
        }
    }
    let st = rd::<u16>(this, 0xA);
    add_f(this, 0x30, x(ts()).fmul_m(k(0x870898)).fmul(s));
    if st == 2 {
        add_f(this, 0x28, x(ts()).fmul_m(k(0x870894)).fmul(s));
    } else if st == 1 {
        add_f(this, 0x2C, s.fabs().fmul_m(ts()).fmul_m(k(0x87089C)));
    }
    if st == 2 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        let fwd = m.byte_add(0x10);
        let mut ahead = [0u32; 3];
        vector_add(local(&mut ahead), pos, fwd);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        let mut behind = [0u32; 3];
        vector_sub(local(&mut behind), pos, fwd);
        let mut w1 = [0u32; 1];
        let mut w2 = [0u32; 1];
        if water_get_water_level(ahead[0], ahead[1], ahead[2], local(&mut w1), 1, 0) != 0
            && water_get_water_level(behind[0], behind[1], behind[2], local(&mut w2), 1, 0) != 0
        {
            let dz = x(f32::from_bits(w1[0])).fsub_m(f32::from_bits(w2[0]));
            let a = dz.fpatan(X87::from_f64_exact(g::<f64>(0x86F990).read())).fmul_m(k(0x8708A0));
            wr::<f32>(this, 0x24, f32_of(a));
        }
    }
    let sp = X87::from_f64_exact(player_control_button_sprint(ped, 2));
    if !sp.fcom(x(k(K_ONE))).test_any(0x01) {
        wr::<u16>(this, 0xA, 2);
    } else if !x(rd::<f32>(pd(), 0x14)).fcom(x(k(K_HALF))).test_any(0x41) {
        wr::<u16>(this, 0xA, 1);
    } else {
        wr::<u16>(this, 0xA, 0);
    }
}

/// The common end of `ProcessControlInput` (0x6894A0): the decays and the sprint energy.
fn swim_control_tail(this: GPtr<u8>, ped: GPtr<u8>) {
    let st = rd::<u16>(this, 0xA);
    let ts = f64::from(k(TIME_STEP));
    let f1 = if st == 4 || st == 2 { k(0x8708A8) } else { k(0x8708B0) };
    let pw = |f: f32| X87::from_f64_exact(sa_core::orig::x87_binary(CI_POW, f64::from(f), ts));
    if outside_small(rd::<f32>(this, 0x28)) {
        wr::<f32>(this, 0x28, f32_of(pw(f1).fmul_m(rd::<f32>(this, 0x28))));
    } else {
        wr::<u32>(this, 0x28, 0);
    }
    if st != 4 && st != 2 {
        if outside_small(rd::<f32>(this, 0x24)) {
            wr::<f32>(this, 0x24, f32_of(pw(f1).fmul_m(rd::<f32>(this, 0x24))));
        } else {
            wr::<u32>(this, 0x24, 0);
        }
    }
    let f2 = if st == 3 || st == 5 { k(0x8708B0) } else { k(0x8708AC) };
    if outside_small(rd::<f32>(this, 0x30)) || outside_small(rd::<f32>(this, 0x2C)) {
        let p = pw(f2);
        wr::<f32>(this, 0x30, f32_of(p.fmul_m(rd::<f32>(this, 0x30))));
        wr::<f32>(this, 0x2C, f32_of(p.fmul_m(rd::<f32>(this, 0x2C))));
    } else {
        wr::<u32>(this, 0x30, 0);
        wr::<u32>(this, 0x2C, 0);
    }
    if st == 1 {
        player_handle_sprint_energy(ped, 0, 0x3F00_0000);
    } else if st != 2 {
        player_handle_sprint_energy(ped, 0, 0x3F80_0000);
    }
}

/// `CTaskSimpleSwim::ProcessSwimAnims(CPed*)` (0x6899F0, `ret 4`; its tail lives in a fragment at 0x401DEF).
/// Until blended in (+8): blends the idle anim 0xE (group 0) at 8.0 when missing, marks +8 once its blend amount
/// reaches 1.0, sets the blend deltas of the anims with flag 0x10 to -8.0, kills the current weapon's fx system
/// (+0x5B8 + 0x1C * weapon slot +0x718) and, for a player treading water, switches to state 4 when the water level
/// (in the ped's argument slot in the original) minus 2.0 is above the ped. Blended in: re-blends 0xE at 8.0 when
/// it fades out (blend below 1.0, delta <= 0). Requests (model 0x63E7 + block index, flags 8) or references the
/// swim anim block (group 0x47; the player's block pointer [0xB4EA34] +0x58C, else by name) and marks +9. With both
/// flags: per state 5 (climb out: 0x13C then 0x80 once a quarter through, flag 8; back to state 0 at the end or
/// with the water above the ped + 0.5; else start 0x13C with move speed z = 8.0 / mass and `TestForClimb` into
/// +0x38 (referenced)), 3 (dive 0x139, state 4 at its end), 4 (under water 0x13A / 0x13B, sprint start/speed),
/// 1 (0x137 at the player's move speed or +0x10 in (0, 1]), 2 (0x138 at max(1.0, sprint results) or min(1.5, +0x10
/// - 1.0)), 0 (blend out 0x137..0x13C, 0x80 at -1 / -4 / -2 and anim 0xE). A player with move speed above 0.5 or
/// in state 4 updates the swimming stats (`UpdateStatsWhenSwimming(state == 4, state == 2)`).
#[hook(addr = 0x6899F0, conv = thiscall, name = "CTaskSimpleSwim::ProcessSwimAnims", expect = "53 55 56 8B 74 24 10",
       unverified = "ai8-1", route = "L2", hazards = "heap-writes", recs = "air-water",
       note = "the original passes the sete results with the upper EAX/ECX bytes of fnstsw/earlier values")]
pub fn swim_process_swim_anims(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = || rd::<u32>(ped, 0x18);
    let mut a = clump_get_association(clump(), 0xE);
    if rd::<u8>(this, 8) == 0 {
        if a.is_null() {
            a = anim_blend_animation(clump(), 0, 0xE, 0x4100_0000);
        }
        if !a.is_null() && !x(rd::<f32>(a, 0x18)).fcom(x(k(K_ONE))).test_any(0x01) {
            wr::<u8>(this, 8, 1);
        }
        clump_set_blend_deltas(clump(), 0x10, 0xC100_0000);
        let slot = |p: GPtr<u8>| p.byte_add(i32::from(rd::<i8>(p, 0x718)) as isize * 0x1C + 0x5B8);
        let fx = rd::<GPtr<u8>>(slot(ped), 0);
        if !fx.is_null() {
            fx_system_kill(fx);
            wr::<u32>(slot(ped), 0, 0);
        }
        if ped_is_player(ped) != 0 && rd::<u16>(this, 0xA) == 0 {
            let mut w = [0u32; 1];
            let p = pos_of(ped);
            if water_get_water_level(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), local(&mut w), 1, 0) != 0
                && !x(f32::from_bits(w[0])).fsub_m(k(0x858CA0)).fcom(x(rd::<f32>(pos_of(ped), 8))).test_any(0x41)
            {
                wr::<u16>(this, 0xA, 4);
            }
        }
    } else if !a.is_null()
        && !x(rd::<f32>(a, 0x18)).fcom(x(k(K_ONE))).test_parity(0x05)
        && !x(rd::<f32>(a, 0x1C)).fcom(x(k(K_ZERO))).test_parity(0x41)
    {
        anim_blend_animation(clump(), 0, 0xE, 0x4100_0000);
    }
    if rd::<u8>(this, 9) == 0 {
        let mut blk = rd::<GPtr<u8>>(g::<GPtr<u8>>(0xB4EA34).read(), 0x58C);
        if blk.is_null() {
            blk = anim_get_animation_block(anim_get_anim_block_name(0x47));
        }
        let idx = (blk.addr() as u32).wrapping_sub(0xB5D4A0) as i32 >> 5;
        if rd::<u8>(blk, 0x10) != 1 {
            streaming_request_model(idx.wrapping_add(0x63E7), 8);
        } else {
            anim_add_anim_block_ref(idx);
            wr::<u8>(this, 9, 1);
        }
    }
    if rd::<u8>(this, 8) == 0 || rd::<u8>(this, 9) == 0 {
        return;
    }
    let total = |a: GPtr<u8>| rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10);
    match rd::<i16>(this, 0xA) {
        5 => {
            let anim = rd::<u32>(this, 0xC);
            if anim == 0x13C {
                let a = clump_get_association(clump(), 0x13C);
                if a.is_null() {
                    wr::<u16>(this, 0xA, 0);
                } else {
                    let t = x(rd::<f32>(a, 0x28)).fadd_m(rd::<f32>(a, 0x20));
                    if !x(total(a)).fmul_m(k(0x8708BC)).fcom(t).test_parity(0x41) {
                        let n = anim_blend_animation(clump(), 0, 0x80, 0x4100_0000);
                        wr::<u8>(n, 0x2E, rd::<u8>(n, 0x2E) | 8);
                        wr::<u32>(this, 0xC, 0x80);
                    }
                }
            } else if anim == 0x80 {
                let a = clump_get_association(clump(), 0x80);
                if a.is_null() {
                    wr::<u16>(this, 0xA, 0);
                } else {
                    let mut w = [0u32; 1];
                    let p = pos_of(ped);
                    if water_get_water_level(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), local(&mut w), 1, 0) != 0
                        && !x(rd::<f32>(pos_of(ped), 8)).fadd_m(k(K_HALF)).fcom(x(f32::from_bits(w[0]))).test_parity(0x05)
                    {
                        wr::<u16>(this, 0xA, 0);
                    }
                }
            } else {
                anim_blend_animation(clump(), 0x47, 0x13C, 0x4100_0000);
                wr::<u32>(this, 0xC, 0x13C);
                wr::<f32>(ped, 0x4C, f32_of(x(k(0x8708B8)).fdiv_m(rd::<f32>(ped, 0x8C))));
                let e = climb_test_for_climb(ped, this.byte_add(0x3C), this.byte_add(0x48), this.byte_add(0x4C), 1);
                wr::<GPtr<u8>>(this, 0x38, e);
                if !e.is_null() {
                    entity_register_reference(e, this.byte_add(0x38));
                }
            }
        }
        3 => {
            if rd::<u32>(this, 0xC) != 0x139 {
                anim_blend_animation(clump(), 0x47, 0x139, 0x4100_0000);
                wr::<u32>(this, 0xC, 0x139);
            } else {
                let a = clump_get_association(clump(), 0x139);
                if a.is_null() {
                    wr::<u16>(this, 0xA, 0);
                } else if !x(total(a)).fcom(x(rd::<f32>(a, 0x20))).test_parity(0x44) {
                    wr::<u16>(this, 0xA, 4);
                }
            }
        }
        4 => swim_anims_under_water(this, ped),
        1 => {
            if rd::<u32>(this, 0xC) != 0x137 {
                anim_blend_animation(clump(), 0x47, 0x137, 0x4000_0000);
                wr::<u32>(this, 0xC, 0x137);
            } else {
                let a = clump_get_association(clump(), 0x137);
                if a.is_null() {
                    wr::<u16>(this, 0xA, 0);
                } else if ped_is_player(ped) != 0 {
                    wr::<f32>(a, 0x24, f32_of(x(rd::<f32>(rd::<GPtr<u8>>(ped, 0x480), 0x14))));
                } else if !x(rd::<f32>(this, 0x10)).fcom(x(k(K_ZERO))).test_any(0x41)
                    && !x(rd::<f32>(this, 0x10)).fcom(x(k(K_ONE))).test_parity(0x41)
                {
                    wr::<u32>(a, 0x24, rd::<u32>(this, 0x10));
                }
            }
        }
        2 => {
            if rd::<u32>(this, 0xC) != 0x138 {
                anim_blend_animation(clump(), 0x47, 0x138, 0x4000_0000);
                wr::<u32>(this, 0xC, 0x138);
            } else {
                let a = clump_get_association(clump(), 0x138);
                if a.is_null() {
                    wr::<u16>(this, 0xA, 0);
                } else if !rd::<GPtr<u8>>(ped, 0x480).is_null() {
                    let r = X87::from_f64_exact(player_get_button_sprint_results(ped, 2));
                    wr::<f32>(a, 0x24, f32_of(X87::from_f64_exact(max_f(0x3F80_0000, bits(r)))));
                } else if !x(rd::<f32>(this, 0x10)).fcom(x(k(K_ONE))).test_any(0x41) {
                    let v = x(rd::<f32>(this, 0x10)).fsub_m(k(K_ONE));
                    wr::<f32>(a, 0x24, f32_of(X87::from_f64_exact(min_f(0x3FC0_0000, bits(v)))));
                }
            }
        }
        0 if rd::<u32>(this, 0xC) != 0xE => {
            for (id, d) in [
                (0x137, 0xBF80_0000u32),
                (0x138, 0xBF80_0000),
                (0x139, 0xC080_0000),
                (0x13A, 0xC000_0000),
                (0x13B, 0xC000_0000),
                (0x13C, 0xC080_0000),
                (0x80, 0xC080_0000),
            ] {
                let a = clump_get_association(clump(), id);
                if !a.is_null() {
                    wr::<u32>(a, 0x1C, d);
                }
            }
            wr::<u32>(this, 0xC, 0xE);
        }
        _ => {}
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if pd.is_null() {
        return;
    }
    let st = rd::<u16>(this, 0xA);
    if !x(rd::<f32>(pd, 0x14)).fcom(x(k(K_HALF))).test_any(0x41) || st == 4 {
        stats_update_stats_when_swimming(u32::from(st == 4), u32::from(st == 2));
    }
}

/// State 4 of `ProcessSwimAnims` (0x689D8E): under water.
fn swim_anims_under_water(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = || rd::<u32>(ped, 0x18);
    let total = |a: GPtr<u8>| rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10);
    let anim = rd::<u32>(this, 0xC);
    let sprint = (anim == 0x13A || anim == 0x13B)
        && x(rd::<f32>(this, 0x34)).fcom(x(k(K_ZERO))).test_parity(0x05);
    if !sprint {
        let a = clump_get_association(clump(), 0x13A);
        if a.is_null() {
            anim_blend_animation(clump(), 0x47, 0x13A, 0x447A_0000);
            let cur = rd::<u32>(this, 0xC);
            wr::<u32>(this, 0x34, if cur == 0xE || cur == 0xBF { 0xC000_0000 } else { 0xBF80_0000 });
            wr::<u32>(this, 0xC, 0x13A);
            return;
        }
        if x(rd::<f32>(this, 0x34)).fcom(x(k(K_ZERO))).test_parity(0x05)
            || x(rd::<f32>(a, 0x18)).fcom(x(k(0x862CD0))).test_any(0x01)
        {
            return;
        }
        if x(rd::<f32>(this, 0x34)).fcom(x(k(0x858B18))).test_parity(0x41) {
            wr::<f32>(this, 0x24, f32_of(x(k(0x8D2FC4)).fmul_m(k(0x8595EC))));
        } else {
            wr::<u32>(this, 0x24, 0x3FB2_B8C3);
        }
        wr::<u32>(this, 0x34, 0);
        return;
    }
    let faded = |a: GPtr<u8>| {
        a.is_null()
            || !x(rd::<f32>(a, 0x1C)).fcom(x(k(K_ZERO))).test_parity(0x05)
            || !x(rd::<f32>(a, 0x18)).fcom(x(k(K_ZERO))).test_parity(0x44)
    };
    if !rd::<GPtr<u8>>(ped, 0x480).is_null()
        && !X87::from_f64_exact(player_get_button_sprint_results(ped, 3)).fcom(x(k(K_ONE))).test_any(0x01)
    {
        let mut a = clump_get_association(clump(), 0x13A);
        if faded(a) {
            a = anim_blend_animation(clump(), 0x47, 0x13A, 0x4080_0000);
        }
        if !x(total(a)).fcom(x(rd::<f32>(a, 0x20))).test_parity(0x44) {
            assoc_start(a, 0);
            let r = X87::from_f64_exact(player_get_button_sprint_results(ped, 3));
            wr::<f32>(a, 0x24, f32_of(X87::from_f64_exact(max_f(0x3F33_3333, bits(r)))));
        }
        wr::<u32>(this, 0xC, 0x13A);
    } else {
        let a = clump_get_association(clump(), 0x13B);
        if faded(a) {
            anim_blend_animation(clump(), 0x47, 0x13B, 0x4080_0000);
        }
        wr::<u32>(this, 0xC, 0x13B);
    }
}

/// `CTaskSimpleSwim::ProcessSwimmingResistance(CPed*)` (0x68A1D0, `ret 4`). The wanted speed is the ped's anim
/// move shift (+0x4D8 along the matrix right, +0x4DC along forward; under water forward is pitched by +0x24 with
/// `fcos` / `fsin` and z gets + 0.01; diving (state 3) takes z from the dive anim 0x139's progress * -0.1). On the
/// surface (states 0..2) the buoyancy is 0.55 * max(0, 1 - blend(0x137) - blend(0x138)) + 0.4 * blend(0x137) +
/// 0.2 * blend(0x138). State 5 (climbing out) only applies an up force `step * mass * 0.3 * 0.008` while 0x80 /
/// 0x13C plays (and returns). The move speed blends toward the wanted speed by `1 - pow(0.9, step)`. With the water
/// level at the next position: under water rising to the surface (+0x24 > 0.785 near it ends state 4), the pitch
/// rate +0x34 and pitch +0x24 are steered, and a positive buoyancy pulls the move speed z toward `level -
/// (buoyancy + z)` per step, each limited to step * 0.1 and step * 0.02. Below z = -69.0 the ped is put back at
/// -69.0 with a non-negative move speed z.
#[hook(addr = 0x68A1D0, conv = thiscall, name = "CTaskSimpleSwim::ProcessSwimmingResistance", expect = "83 EC 2C 56 8B 74 24 34",
       unverified = "ai8-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "air-water")]
pub fn swim_process_swimming_resistance(this: GPtr<u8>, ped: GPtr<u8>) {
    let ts = || k(TIME_STEP);
    let mut res = [0f32; 3];
    let mut buoy = -1.0f32;
    let st = rd::<i16>(this, 0xA);
    // res = f * right + g * forward (`f` = +0x4D8, `g` = +0x4DC or `cos(pitch) * g`), as the original orders it
    let shift = |res: &mut [f32; 3], fwd_scale: Option<X87>| {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let f = rd::<f32>(ped, 0x4D8);
        let rx = f32_of(x(f).fmul_m(rd::<f32>(m, 0)));
        let ry = x(f).fmul_m(rd::<f32>(m, 4));
        let rz = x(f32_of(x(f).fmul_m(rd::<f32>(m, 8))));
        let gg = rd::<f32>(ped, 0x4DC);
        match fwd_scale {
            None => {
                let gx = x(gg).fmul_m(rd::<f32>(m, 0x10));
                let gy = f32_of(x(gg).fmul_m(rd::<f32>(m, 0x14)));
                let gz = f32_of(x(gg).fmul_m(rd::<f32>(m, 0x18)));
                res[0] = f32_of(gx.fadd_m(rx));
                res[1] = f32_of(x(gy).fadd(ry));
                (rz, x(gz))
            }
            Some(c) => {
                let cx = c.fmul_m(rd::<f32>(m, 0x10));
                let cy = f32_of(c.fmul_m(rd::<f32>(m, 0x14)));
                let cz = f32_of(c.fmul_m(rd::<f32>(m, 0x18)));
                let cxg = cx.fmul_m(gg);
                let cyg = x(cy).fmul_m(gg);
                let czg = f32_of(x(cz).fmul_m(gg));
                res[0] = f32_of(cxg.fadd_m(rx));
                res[1] = f32_of(cyg.fadd(ry));
                (rz, x(czg))
            }
        }
    };
    match st {
        0..=2 => {
            let a1 = clump_get_association(rd::<u32>(ped, 0x18), 0x137);
            let a2 = clump_get_association(rd::<u32>(ped, 0x18), 0x138);
            let (mut s0, mut s1) = if a1.is_null() {
                (x(1.0), x(k(K_ZERO)))
            } else {
                (x(k(K_ONE)).fsub_m(rd::<f32>(a1, 0x18)), x(k(0x8708C4)).fmul_m(rd::<f32>(a1, 0x18)))
            };
            if !a2.is_null() {
                s1 = s1.fadd(x(k(0x8708C8)).fmul_m(rd::<f32>(a2, 0x18)));
                s0 = s0.fsub_m(rd::<f32>(a2, 0x18));
            }
            if !x(k(K_ZERO)).fcom(s0).test_any(0x41) {
                s0 = x(k(K_ZERO));
            }
            buoy = f32_of(s0.fmul_m(k(0x8708C0)).fadd(s1));
            let (rz, gz) = shift(&mut res, None);
            res[2] = f32_of(gz.fadd(rz));
        }
        5 => {
            let mut a = clump_get_association(rd::<u32>(ped, 0x18), 0x80);
            if a.is_null() {
                a = clump_get_association(rd::<u32>(ped, 0x18), 0x13C);
                if a.is_null() {
                    return;
                }
            }
            let total = rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10);
            if x(total).fcom(x(rd::<f32>(a, 0x20))).test_any(0x41) {
                return;
            }
            if x(rd::<f32>(a, 0x18)).fcom(x(k(K_ONE))).test_any(0x01)
                && x(rd::<f32>(a, 0x1C)).fcom(x(k(K_ZERO))).test_any(0x41)
            {
                return;
            }
            let z = x(ts()).fmul_m(rd::<f32>(ped, 0x8C)).fmul_m(k(0x8708DC)).fmul_m(k(0x863984));
            physical_apply_move_force(ped, 0, 0, bits(z));
            return;
        }
        3 => {
            let (rz, gz) = shift(&mut res, None);
            res[2] = f32_of(gz.fadd(rz));
            let a = clump_get_association(rd::<u32>(ped, 0x18), 0x139);
            if !a.is_null() {
                let p = x(rd::<f32>(a, 0x20)).fdiv_m(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
                res[2] = f32_of(p.fmul_m(k(0x858EF4)));
            }
        }
        4 => {
            let c = x(rd::<f32>(this, 0x24)).fcos();
            let (rz, czg) = shift(&mut res, Some(c));
            let sg = x(rd::<f32>(this, 0x24)).fsin().fmul_m(rd::<f32>(ped, 0x4DC));
            res[2] = f32_of(sg.fadd(czg.fadd(rz)).fadd_m(k(0x8708CC)));
        }
        _ => {}
    }
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(CI_POW, f64::from(k(0x8708D0)), f64::from(ts())));
    let q = x(k(K_ONE)).fsub(p);
    let qx = f32_of(x(res[0]).fmul(q));
    let qy = f32_of(x(res[1]).fmul(q));
    let qz = q.fmul_m(res[2]);
    let pmx = p.fmul_m(rd::<f32>(ped, 0x44));
    let pmy = p.fmul_m(rd::<f32>(ped, 0x48));
    let pmz = f32_of(p.fmul_m(rd::<f32>(ped, 0x4C)));
    wr::<f32>(ped, 0x44, f32_of(pmx.fadd_m(qx)));
    wr::<f32>(ped, 0x48, f32_of(pmy.fadd_m(qy)));
    wr::<f32>(ped, 0x4C, f32_of(x(pmz).fadd(qz)));
    let nx = x(ts()).fmul_m(rd::<f32>(ped, 0x44));
    let ny = x(ts()).fmul_m(rd::<f32>(ped, 0x48));
    let p0 = pos_of(ped);
    let nx = f32_of(nx.fadd_m(rd::<f32>(p0, 0)));
    let ny = f32_of(ny.fadd_m(rd::<f32>(p0, 4)));
    let mut w = [0u32; 1];
    if water_get_water_level(nx.to_bits(), ny.to_bits(), rd::<u32>(pos_of(ped), 8), local(&mut w), 1, 0) != 0 {
        let level = f32::from_bits(w[0]);
        if rd::<u16>(this, 0xA) == 4 && x(rd::<f32>(this, 0x34)).fcom(x(k(K_ZERO))).test_parity(0x05) {
            buoy = swim_under_water_pitch(this, ped, level, buoy);
        }
        if !x(buoy).fcom(x(k(K_ZERO))).test_any(0x41) {
            let d = x(level).fsub(x(buoy).fadd_m(rd::<f32>(pos_of(ped), 8)));
            let v = d.fdiv_m(ts());
            let lim = f32_of(x(ts()).fmul_m(k(0x8708D4)));
            let v = clamp_fcom(v, lim);
            let v = v.fsub_m(rd::<f32>(ped, 0x4C));
            let lim2 = f32_of(x(ts()).fmul_m(k(0x8708D8)));
            let v = clamp_fcom(v, lim2);
            wr::<f32>(ped, 0x4C, f32_of(v.fadd_m(rd::<f32>(ped, 0x4C))));
        }
    }
    let p = pos_of(ped);
    if !x(rd::<f32>(p, 8)).fcom(x(k(0x87094C))).test_parity(0x05) {
        wr::<u32>(pos_of(ped), 8, 0xC28A_0000);
        let mz = x(rd::<f32>(ped, 0x4C));
        let mz = if !x(k(K_ZERO)).fcom(mz).test_any(0x41) { x(k(K_ZERO)) } else { mz };
        wr::<f32>(ped, 0x4C, f32_of(mz));
    }
}

/// `fcom lim` (above `lim` gives `lim`), then `-lim` (`fld; fchs; fcomp st(1)`: `-lim > v` gives `-lim`).
fn clamp_fcom(v: X87, lim: f32) -> X87 {
    let v = if !v.fcom(x(lim)).test_any(0x41) { x(lim) } else { v };
    let n = f32_of(x(lim).fchs());
    if !x(n).fcom(v).test_any(0x41) { x(n) } else { v }
}

/// Under water with a non-negative pitch rate (`ProcessSwimmingResistance` 0x68A5D1..0x68A7C9): steers the pitch
/// rate +0x34 and the pitch +0x24 by the water level; may end the dive (state 0) or return a new buoyancy.
fn swim_under_water_pitch(this: GPtr<u8>, ped: GPtr<u8>, level: f32, buoy: f32) -> f32 {
    let ts = || k(TIME_STEP);
    let z = || rd::<f32>(pos_of(ped), 8);
    let above = |v: X87| !v.fcom(x(level)).test_any(0x41);
    let step_pitch = |this: GPtr<u8>| {
        let r = x(ts()).fmul_m(rd::<f32>(this, 0x34)).fadd_m(rd::<f32>(this, 0x24));
        wr::<f32>(this, 0x24, f32_of(r));
        r
    };
    if above(x(k(0x8708E8)).fadd_m(z())) && !x(rd::<f32>(this, 0x24)).fcom(x(k(0x859AB0))).test_any(0x41) {
        wr::<u16>(this, 0xA, 0);
        wr::<u32>(this, 0x34, 0);
        return buoy;
    }
    if !x(rd::<f32>(this, 0x24)).fcom(x(k(K_ZERO))).test_parity(0x05) {
        // pitched down
        let tip = x(rd::<f32>(this, 0x24)).fsin().fsubr_m(z()).fadd_m(k(0x8708E8));
        if above(tip) {
            let r = x(ts()).fmul_m(k(0x8708E0)).fadd_m(rd::<f32>(this, 0x34));
            wr::<f32>(this, 0x34, f32_of(r));
            if !r.fcom(x(k(0x8708E4))).test_any(0x41) {
                wr::<u32>(this, 0x34, g::<u32>(0x8708E4).read());
            }
        } else if !x(rd::<f32>(this, 0x34)).fcom(x(k(0x858CDC))).test_any(0x41) {
            wr::<f32>(this, 0x34, f32_of(x(rd::<f32>(this, 0x34)).fmul_m(k(0x858EF0))));
        } else {
            wr::<u32>(this, 0x34, 0);
        }
        step_pitch(this);
        return buoy;
    }
    if above(x(k(0x8708E8)).fadd_m(z())) {
        let h = f32_of(x(k(0x8708E4)).fmul_m(k(K_HALF)));
        if !x(rd::<f32>(this, 0x34)).fcom(x(h)).test_any(0x41) {
            wr::<f32>(this, 0x34, f32_of(x(rd::<f32>(this, 0x34)).fmul_m(k(0x858EF0))));
        }
        if !x(rd::<f32>(this, 0x34)).fcom(x(h)).test_parity(0x05) {
            let b = x(ts()).fmul_m(k(0x8708E0)).fadd_m(rd::<f32>(this, 0x34));
            wr::<f32>(this, 0x34, f32_of(X87::from_f64_exact(min_f(h.to_bits(), bits(b)))));
        }
        let r = step_pitch(this);
        let d = x(k(0x8708C0)).fsub_m(k(0x8708C8));
        return f32_of(d.fmul(r.fmul_m(k(0x870950))).fmul_m(k(0x858F34)).fadd_m(k(0x8708C8)));
    }
    if !x(rd::<f32>(this, 0x34)).fcom(x(k(0x858CDC))).test_any(0x41) {
        wr::<f32>(this, 0x34, f32_of(x(rd::<f32>(this, 0x34)).fmul_m(k(0x858EF0))));
    } else {
        wr::<u32>(this, 0x34, 0);
    }
    buoy
}

/// `CTaskSimpleSwim::ProcessEffects(CPed*)` (0x68AA70, `ret 4`). The effect position is 0.4 ahead of the ped
/// (its matrix forward, or `(-sin, cos, 0)` of the heading +0x10 without a matrix) at the water level (player data
/// +0x90) or 0.5 above the ped. Treading water (state 0) keeps the ripples fx (created on demand) on a temporary
/// RwMatrix at that position with local particles; other states kill it. Diving (state 3) splashes once (+0x60)
/// with audio event 0x4B; under water (state 4) adds a breath bubble at bone 3 (static `FxPrtMult_c` 0xC19668)
/// with chance `(100 - breath / mod * 100) / 3` % (5 % for others); swimming (1, 2) adds a wake particle rotated by
/// the ped's heading + 180 with audio event 0x4C, and sprinting (2) plays a `water_swim` splash with audio event
/// 0x4A at each of bones 0x18, 0x22, 0x35, 0x2B within 0.05 of the ped's z.
#[hook(addr = 0x68AA70, conv = thiscall, name = "CTaskSimpleSwim::ProcessEffects", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "ai8-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "air-water")]
pub fn swim_process_effects(this: GPtr<u8>, ped: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let (fx, fy, fz) = if !m.is_null() {
        (rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), x(rd::<f32>(m, 0x18)))
    } else {
        let h = x(rd::<f32>(ped, 0x10));
        let s = f32_of(h.fsin());
        (f32_of(x(s).fchs()), f32_of(h.fcos()), x(k(K_ZERO)))
    };
    let z = fz.fmul_m(k(0x858EE8));
    let sy = f32_of(x(fy).fmul_m(k(0x858EE8)));
    let sx = f32_of(x(fx).fmul_m(k(0x858EE8)));
    let p = pos_of(ped);
    let z = z.fadd_m(rd::<f32>(p, 8));
    let py = f32_of(x(sy).fadd_m(rd::<f32>(p, 4)));
    let px = f32_of(x(sx).fadd_m(rd::<f32>(p, 0)));
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    let pz = if !pd.is_null() { rd::<f32>(pd, 0x90) } else { f32_of(z.fadd_m(k(K_HALF))) };
    let mut pos = [px.to_bits(), py.to_bits(), pz.to_bits()];
    if rd::<u16>(this, 0xA) == 0 {
        let rm = rw_matrix_create();
        wr::<u32>(rm, 0x30, pos[0]);
        wr::<u32>(rm, 0x34, pos[1]);
        wr::<u32>(rm, 0x38, pos[2]);
        rw_matrix_update(rm);
        if rd::<u32>(this, 0x5C) == 0 {
            o_swim_create_fx_system(this, ped, rm);
        }
        let f = rd::<GPtr<u8>>(this, 0x5C);
        if !f.is_null() {
            fx_system_set_matrix(f, rm);
            fx_system_set_local_particles(rd::<GPtr<u8>>(this, 0x5C), 1);
        }
        rw_matrix_destroy(rm);
    } else {
        kill_fx(this);
    }
    let st = rd::<u16>(this, 0xA);
    if st != 3 {
        wr::<u8>(this, 0x60, 0);
    }
    let audio = ped.byte_add(0x138);
    match st {
        1 | 2 => {
            let fm = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
            let a = general_angle_between_points(rd::<u32>(fm, 0), rd::<u32>(fm, 4), 0, 0);
            let a = f32_of(X87::from_f64_exact(a));
            let rot = f32_of(X87::from_f64_exact(general_limit_angle(a.to_bits())).fadd_m(k(0x85A994)));
            let mut mult = [0u32; 8];
            fx_prt_mult_ctor(local(&mut mult), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3E4C_CCCD, 0x3ECC_CCCD, 0, 0x3F00_0000);
            let mut vel = [0u32; 3];
            fx_system_add_particle(g::<GPtr<u8>>(0xA9AE34).read(), local(&mut pos), local(&mut vel), 0, local(&mut mult),
                                   rot.to_bits(), 0x3F99_999A, 0x3F19_999A, 0);
            ped_audio_add_audio_event(audio, 0x4C, 0, 0x3F80_0000, 0, 0, 0, 0);
            if rd::<u16>(this, 0xA) == 2 {
                swim_sprint_splashes(ped, audio);
            }
        }
        3 => {
            if rd::<u8>(this, 0x60) == 0 {
                let mut v = pos;
                fx_trigger_water_splash(g(G_FX), local(&mut v));
                wr::<u8>(this, 0x60, 1);
                ped_audio_add_audio_event(audio, 0x4B, 0, 0x3F80_0000, 0, 0, 0, 0);
            }
        }
        4 => {
            let mut chance = 5;
            if ped_is_player(ped) != 0 {
                let pd = rd::<GPtr<u8>>(ped, 0x480);
                let m = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(8));
                let v = m.fdivr_m(rd::<f32>(pd, 0x44)).fmul_m(k(0x858628)).fsubr_m(k(0x858628)).fmul_m(k(0x859040));
                chance = ftol32(v);
            }
            if general_random_in_range(0, 100) >= chance {
                return;
            }
            let h = get_anim_hierarchy_from_skin_clump(rd::<u32>(ped, 0x18));
            let b = hanim_get_matrix_array(h).byte_add(0xF0);
            let mut bp = [rd::<u32>(b, 0), rd::<u32>(b, 4), rd::<u32>(b, 8)];
            if g::<u32>(0xC19684).read() & 1 == 0 {
                g::<u32>(0xC19684).write(g::<u32>(0xC19684).read() | 1);
                fx_prt_mult_ctor(g(0xC19668), 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3E80_0000, 0x3E99_999A, 0, 0x3F00_0000);
            }
            let mut vel = [0u32, 0, 0x4000_0000];
            fx_system_add_particle(g::<GPtr<u8>>(0xA9AE08).read(), local(&mut bp), local(&mut vel), 0, g(0xC19668),
                                   0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
        }
        _ => {}
    }
}

/// The sprint splashes of `ProcessEffects` (0x68AE0D..0x68AFD6): bones 0x18, 0x22, 0x35, 0x2B (their matrix
/// positions, index * 0x40 + 0x30), each within 0.05 (strictly, `test ah, 5; jp`) of the ped's z.
fn swim_sprint_splashes(ped: GPtr<u8>, audio: GPtr<u8>) {
    let h = get_anim_hierarchy_from_skin_clump(rd::<u32>(ped, 0x18));
    let mut bones = [GPtr::<u8>::NULL; 4];
    for (i, id) in [0x18u32, 0x22, 0x35, 0x2B].into_iter().enumerate() {
        let idx = hanim_id_get_index(h, id);
        bones[i] = hanim_get_matrix_array(h).byte_add((idx << 6) as isize + 0x30);
    }
    let pz = rd::<f32>(pos_of(ped), 8);
    for b in bones {
        if !x(rd::<f32>(b, 8)).fsub_m(pz).fabs().fcom(x(k(0x858C28))).test_parity(0x05) {
            let f = fx_manager_create_fx_system(g(FX_MAN), 0x870964, b, 0, 0);
            if !f.is_null() {
                fx_system_play_and_kill(f);
                ped_audio_add_audio_event(audio, 0x4A, 0, 0x3F80_0000, 0, 0, 0, 0);
            }
        }
    }
}

/// `CTaskSimpleSwim::ApplyRollAndPitch(CPed*)` (0x68A8E0, `ret 4`): rotates the torso (`CPedIK::RotateTorso` on
/// the ped's IK +0x50C with the frame data +0x48C and the orientation (+0x30, +0x2C), no forced flag), then
/// multiplies the ped's frame matrix (a stack `CMatrix` attached to the RwFrame's modelling matrix, or to null
/// without an RwObject) by a stack rotation (`SetTranslate(0, 0, 0)`, `RotateY(+0x28)`, `RotateX(+0x24)`; only
/// its attach fields are initialised before, the rest is written by `SetTranslate`), updates the RW matrix and
/// the ped's RwFrame, and destroys both matrices.
#[hook(addr = 0x68A8E0, conv = thiscall, name = "CTaskSimpleSwim::ApplyRollAndPitch", expect = "6A FF 68 53 6C 84 00",
       unverified = "ai8-1", route = "L2", hazards = "heap-writes", recs = "air-water")]
pub fn swim_apply_roll_and_pitch(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut orient = [rd::<u32>(this, 0x30), rd::<u32>(this, 0x2C)];
    ped_ik_rotate_torso(ped.byte_add(0x50C), rd::<u32>(ped, 0x48C), local(&mut orient), 0);
    let rw = rd::<GPtr<u8>>(ped, 0x18);
    let frame_m = if rw.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) };
    let mut mat = [0u32; 0x12];
    matrix_ctor_rw(local(&mut mat), frame_m, 0);
    let mut rot = [0u32; 0x12];
    matrix_set_translate(local(&mut rot), 0, 0, 0);
    matrix_rotate_y(local(&mut rot), rd::<u32>(this, 0x28));
    matrix_rotate_x(local(&mut rot), rd::<u32>(this, 0x24));
    matrix_mul_assign(local(&mut mat), local(&mut rot));
    matrix_update_rw(local(&mut mat));
    entity_update_rw_frame(ped);
    matrix_dtor(local(&mut rot));
    matrix_dtor(local(&mut mat));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SWIM_PROCESS_PED,
    HOOK_SWIM_PROCESS_CONTROL_INPUT,
    HOOK_SWIM_PROCESS_SWIM_ANIMS,
    HOOK_SWIM_PROCESS_SWIMMING_RESISTANCE,
    HOOK_SWIM_PROCESS_EFFECTS,
    HOOK_SWIM_APPLY_ROLL_AND_PITCH,
];
