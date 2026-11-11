//! PORT-AI-4: falling and landing — `CTaskComplexInAirAndLand`, `CTaskSimpleInAir`, `CTaskSimpleLand`.
//! Ported from the 1.0 US bytes (gta-reversed and `docs/re/specs/AI` as reading aids only).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTaskSimpleInAir::DeleteAnimCB` (passed as the delete callback)
const IN_AIR_DELETE_ANIM_CB: u32 = 0x678E60;
/// `CDefaultAnimCallback::DefaultAnimCB` (0x4D1390, the "no callback" the original installs)
const DEFAULT_ANIM_CB: u32 = 0x4D1390;

orig! {
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTaskSimpleInAir::CTaskSimpleInAir(bool, bool, bool)` (0x678CD0, `ret 0xC`)
    fn in_air_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x678CD0 => unported;
    /// `CTaskSimpleClimb::CTaskSimpleClimb(CEntity*, const CVector&, float, uint8, eClimbHeights, bool)`
    /// (0x67A110, `ret 0x18`)
    fn climb_ctor(t: GPtr<u8>, e: u32, pos: GPtr<u8>, heading: u32, surf: u32, height: u32, force: u32) -> GPtr<u8> = thiscall 0x67A110 => unported;
    /// `CTaskSimpleFall::CTaskSimpleFall(AnimationId, AssocGroupId, int32)` (0x6782C0, `ret 0xC`)
    fn fall_ctor(t: GPtr<u8>, anim: u32, group: u32, time: u32) -> GPtr<u8> = thiscall 0x6782C0 => unported;
    /// `CTaskSimpleGetUp::CTaskSimpleGetUp()` (0x677F50)
    fn get_up_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x677F50 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(cb, data)` (0x4CEBC0, `ret 8`)
    fn assoc_set_delete_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(id, volume, speed, physical, surface, a6, a7)` (0x4E2BB0, `ret 0x1C`)
    fn ped_audio_add_audio_event(a: GPtr<u8>, id: u32, vol: u32, speed: u32, p: u32, s: u32, a6: u32, a7: u32) = thiscall 0x4E2BB0 => unported;
    /// `CPedClothesDesc::GetIsWearingBalaclava()` (0x5A7950)
    fn clothes_is_wearing_balaclava(c: GPtr<u8>) -> u8 = thiscall 0x5A7950 => unported;
    /// `CEventSoundQuiet::CEventSoundQuiet(CEntity*, float, uint32, const CVector&)` (0x5E05B0, `ret 0x10`)
    fn sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, vol: u32, time: u32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, flag: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `FUN_005dea00`: the `CEventSoundQuiet` destructor the original calls on its stack event
    fn sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_get_pad(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90, AX)
    fn pad_get_ped_walk_left_right(p: GPtr<u8>) -> u16 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30, AX)
    fn pad_get_ped_walk_up_down(p: GPtr<u8>) -> u16 = thiscall 0x53FD30 => unported;
    /// `CPed::Say(id, offset, volume, a4, a5, a6)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a4: u32, a5: u32, a6: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CPhysical::ApplyMoveForce(float, float, float)` (0x4ABBA0, `ret 0xC`)
    fn physical_apply_move_force(p: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x4ABBA0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`; the vector by value)
    fn physical_apply_move_force_vec(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, returns `out`)
    fn vector_mul(out: GPtr<u8>, v: GPtr<u8>, f: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CWorld::ProcessVerticalLine(pos, zEnd, colPoint, outEntity, buildings, vehicles, peds, objects, dummies,
    /// seeThrough, camera)` (0x5674E0, AL)
    fn world_process_vertical_line(
        pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, c: u32
    ) -> u8 = cdecl 0x5674E0 => unported;
    /// `CTaskSimpleClimb::TestForClimb(CPed*, CVector&, float&, uint8&, bool)` (0x6803A0)
    fn climb_test_for_climb(ped: GPtr<u8>, pos: GPtr<u8>, heading: GPtr<u8>, surf: GPtr<u8>, launch: u32) -> GPtr<u8> = cdecl 0x6803A0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleGetUp::StartAnim(CPed*)` (0x67C770, `ret 4`)
    fn get_up_start_anim(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x67C770 => unported;
    /// `CTaskSimpleFall::StartAnim(CPed*)` (0x67CA40, `ret 4`)
    fn fall_start_anim(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x67CA40 => unported;
    /// `CTaskSimpleFall::ProcessFall(CPed*)` (0x6784C0, `ret 4`)
    fn fall_process_fall(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6784C0 => unported;
    /// `CTaskSimpleEvasiveStep::CTaskSimpleEvasiveStep(CEntity*)` (0x653160, `ret 4`)
    fn evasive_step_simple_ctor(t: GPtr<u8>, e: u32) -> GPtr<u8> = thiscall 0x653160 => unported;
    /// `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(float, float, float)` (0x667E20, `ret 0xC`)
    fn achieve_heading_ctor(t: GPtr<u8>, h: u32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CEvent::GetEventType()` (vtable +4)
    fn v_get_event_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetEventPriority()` (vtable +8)
    fn v_get_event_priority(e: GPtr<u8>) -> u32 = thiscall vslot 0x8 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`)
    fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    fn clump_get_first_association_flags(clump: u32, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `CPedPlacement::IsPositionClearOfCars(const CPed*)` (0x616A40)
    fn ped_placement_is_position_clear_of_cars(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x616A40 => unported;
    /// `CVehicle::IsVehiclePointerValid(CVehicle*)` (0x6E38F0)
    fn vehicle_is_pointer_valid(v: GPtr<u8>) -> u8 = cdecl 0x6E38F0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> u32 = thiscall 0x411990 => unported;
    /// `CCollision::ProcessColModels(matA, colA, matB, colB, colPoints, lineColPoints, lineDists, doSeparation)`
    /// (0x4185C0)
    fn collision_process_col_models(ma: u32, ca: u32, mb: u32, cb: u32, cp: u32, lp: u32, ld: u32, sep: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)`
    /// (0x4AD3F0, `ret 0x14`)
    fn damage_calc_ctor(c: GPtr<u8>, e: GPtr<u8>, dmg: u32, wt: u32, piece: u32, a5: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8, bool, bool)` (0x4AD830,
    /// `ret 0x1C`)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`)
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse*, bool)` (0x4B5AC0, `ret 0xC`)
    fn damage_calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4B5AC0 => unported;
    /// `FUN_004ad960`: the `CEventDamage` destructor the original calls on its stack event
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPed::SetPedState(ePedState)` (0x5E4500, `ret 4`)
    fn ped_set_ped_state(p: GPtr<u8>, s: u32) = thiscall 0x5E4500 => unported;
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

fn time_ms() -> u32 {
    g::<u32>(TIME_MS).read()
}

/// `fld dword [addr]`
fn gf(addr: usize) -> X87 {
    x(gm(addr))
}

/// A float memory operand (`fmul dword [addr]` ...).
fn gm(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

// ---- constructors and leaves ----------------------------------------------------------------------------

/// `CTaskComplexInAirAndLand::CTaskComplexInAirAndLand(bool usingJumpGlide, bool fallAfterVault)` (0x678C80,
/// `ret 8`; the vtable 0x8704AC is written by the island 0x405FDF): +0xC/+0xD = the bools, +0xE = 0.
#[hook(addr = 0x678C80, conv = thiscall, name = "CTaskComplexInAirAndLand::Constructor", expect = "56 8B F1 E8 28 17 FA FF")]
pub fn in_air_and_land_ctor(this: GPtr<u8>, glide: u8, vault: u8) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u8>(this, 0xC, glide);
    wr::<u32>(this, 0, 0x8704AC);
    wr::<u8>(this, 0xD, vault);
    wr::<u8>(this, 0xE, 0);
    this
}

/// `CTaskSimpleInAir::CTaskSimpleInAir(bool usingJumpGlide, bool fallAfterVault, bool usingFallGlide)` (0x678CD0,
/// `ret 0xC`): vtable 0x8704D8; +0x24 = (+0x24 & 0xF8) | bits 0..2 from the bools; +0x1C/+0x20/+0x28/+0x38 = 0;
/// the timer +0x2C = now, +0x30 = 500, started (+0x34 = 1, +0x35 = 0).
#[hook(addr = 0x678CD0, conv = thiscall, name = "CTaskSimpleInAir::Constructor", expect = "56 8B F1 E8 B8 16 FA FF")]
pub fn in_air_ctor_port(this: GPtr<u8>, glide: u8, vault: u8, fall_glide: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    let old = rd::<u8>(this, 0x24);
    let bits = ((((fall_glide & 1) << 1) | (vault & 1)) << 1) | (old & 0xF8) | (glide & 1);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0, 0x8704D8);
    wr::<u8>(this, 0x24, bits);
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u8>(this, 0x34, 0);
    wr::<u8>(this, 0x35, 0);
    wr::<u32>(this, 0x38, 0);
    wr::<u32>(this, 0x2C, time_ms());
    wr::<u32>(this, 0x30, 0x1F4);
    wr::<u8>(this, 0x34, 1);
    this
}

/// `CTaskSimpleLand::FinishAnimCB(CAnimBlendAssociation*, void* task)` (0x678FA0, cdecl): a task gets +0x10 |= 1
/// (finished) and its anim pointer +8 cleared; an association with anim id 0x77 or 0xA gets blend delta -100.
#[hook(addr = 0x678FA0, conv = cdecl, name = "CTaskSimpleLand::FinishAnimCB", expect = "8B 44 24 08 85 C0")]
pub fn land_finish_anim_cb(assoc: GPtr<u8>, task: GPtr<u8>) {
    if !task.is_null() {
        wr::<u8>(task, 0x10, rd::<u8>(task, 0x10) | 1);
        wr::<u32>(task, 8, 0);
    }
    if !assoc.is_null() {
        let id = rd::<u16>(assoc, 0x2C);
        if id == 0x77 || id == 0xA {
            wr::<u32>(assoc, 0x1C, 0xC2C8_0000);
        }
    }
}

/// The foot-landed tests: anim time (+0x20) >= `c` and time - delta (+0x28) < `c` (ordered).
fn foot_landed(this: GPtr<u8>, c: usize) -> u8 {
    let a = rd::<GPtr<u8>>(this, 8);
    if a.is_null() || x(rd::<f32>(a, 0x20)).fcom(gf(c)).test_any(1) {
        return 0;
    }
    let t = x(rd::<f32>(a, 0x20)).fsub_m(rd::<f32>(a, 0x28));
    (!t.fcom(gf(c)).test_parity(5)) as u8
}

/// `CTaskSimpleLand::RightFootLanded()` (0x678FE0): the anim (+8) crossed time [0x858B1C] this frame.
#[hook(addr = 0x678FE0, conv = thiscall, name = "CTaskSimpleLand::RightFootLanded", expect = "8B 49 08 85 C9")]
pub fn land_right_foot_landed(this: GPtr<u8>) -> u8 {
    foot_landed(this, 0x858B1C)
}

/// `CTaskSimpleLand::LeftFootLanded()` (0x679010): the anim (+8) crossed time [0x858CC4] this frame.
#[hook(addr = 0x679010, conv = thiscall, name = "CTaskSimpleLand::LeftFootLanded", expect = "8B 49 08 85 C9")]
pub fn land_left_foot_landed(this: GPtr<u8>) -> u8 {
    foot_landed(this, 0x858CC4)
}

// ---- CTaskComplexInAirAndLand ---------------------------------------------------------------------------

/// `CTaskComplexInAirAndLand::CreateFirstSubTask(CPed*)` (0x67CC30, `ret 4`): a pool `CTaskSimpleInAir(+0xC != 0,
/// +0xD != 0, false)` (null when the pool is full). The original pushes the two setne bytes inside dwords whose
/// upper bytes are leftover register contents; the constructor reads only the low bytes.
#[hook(addr = 0x67CC30, conv = thiscall, name = "CTaskComplexInAirAndLand::CreateFirstSubTask", expect = "6A FF 68 4B 63 84 00")]
pub fn in_air_and_land_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x3C);
    if t.is_null() {
        return GPtr::NULL;
    }
    let b = (rd::<u8>(this, 0xD) != 0) as u32;
    let a = (rd::<u8>(this, 0xC) != 0) as u32;
    in_air_ctor(t, a, b, 0)
}

/// The inlined `CTaskSimpleLand(AnimationId)` constructor (pool slot 0x14): `CTaskSimple` base, vtable 0x8704FC,
/// +8 = 0, +0xC = anim id, +0x10 = (+0x10 & `and`) | `or`. Null when the pool is full.
fn land_new(id: u32, and: u8, or: u8) -> GPtr<u8> {
    let t = task_new(0x14);
    if t.is_null() {
        return GPtr::NULL;
    }
    task_simple_ctor(t);
    let f = rd::<u8>(t, 0x10);
    wr::<u32>(t, 0, 0x8704FC);
    wr::<u32>(t, 8, 0);
    wr::<u32>(t, 0xC, id);
    wr::<u8>(t, 0x10, (f & and) | or);
    t
}

/// Landing noise (0x67CE7C / 0x67D0E2): `AddAudioEvent(event, 0, 1.0, 0...)` on the ped's audio entity (+0x138);
/// with ped data (+0x480) a stack `CEventSoundQuiet(ped, volume (balaclava: `loud`), -1, (0,0,0))` is added to the
/// global event group and destroyed.
fn landing_noise(ped: GPtr<u8>, event: u32, quiet: u32, loud: u32) {
    ped_audio_add_audio_event(ped.byte_add(0x138), event, 0, 0x3F80_0000, 0, 0, 0, 0);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if pd.is_null() {
        return;
    }
    let vol = if clothes_is_wearing_balaclava(rd::<GPtr<u8>>(pd, 4)) != 0 { loud } else { quiet };
    let mut v = [0u32; 3];
    let mut ev = [0u32; 0x12];
    let ep = buf(&mut ev);
    sound_quiet_ctor(ep, ped, vol, 0xFFFF_FFFF, buf(&mut v));
    event_group_add(get_event_global_group(), ep, 0);
    sound_quiet_dtor(ep);
}

/// `CTaskComplexInAirAndLand::CreateNextSubTask(CPed*)` (0x67CCB0, `ret 4`; the epilogue continues through the
/// island 0x40A1F1). By the sub-task type: 0xCF → a pool `CTaskSimpleGetUp`; 0xF1 (in air) → a land task, see
/// below; anything else → null. In air after a vault (+0xD): the in-air anim (sub +0x1C) gets +0x2E |= 4, blend
/// delta -8 and the default finish callback, and is dropped; land anim 0x1A (flags |4) when ped velocity z
/// (+0x4C) < [0x858EF4], else -1 (flags |6, bit 0 cleared). Otherwise: a fall anim 0x78 → `CTaskSimpleFall(0x1A, 0,
/// 700)` when sub +0x20 < [0x858F0C], else land 0x7B, then landing noise (event 0x3B, volume 60/75); else land
/// 0xA after anim 9, for a player land 0x77 when ped data +0x14 > [0x858CE8], a pad exists and either stick axis
/// differs from [0x858B50], else 0x7A; then landing noise (event 0x3A, volume 55/70).
#[hook(addr = 0x67CCB0, conv = thiscall, name = "CTaskComplexInAirAndLand::CreateNextSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn in_air_and_land_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    match v_get_task_type(sub) {
        0xCF => {
            let t = task_new(0x10);
            if t.is_null() { GPtr::NULL } else { get_up_ctor(t) }
        }
        0xF1 => {
            if rd::<u8>(this, 0xD) != 0 {
                let a = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x1C);
                if !a.is_null() {
                    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
                    wr::<u32>(a, 0x1C, 0xC100_0000);
                    assoc_set_finish_callback(a, DEFAULT_ANIM_CB, 0);
                    wr::<u32>(rd::<GPtr<u8>>(this, 8), 0x1C, 0);
                }
                let vz = x(rd::<f32>(ped, 0x4C));
                if !vz.fcom(gf(0x858EF4)).test_parity(5) {
                    land_new(0x1A, 0xFC, 4)
                } else {
                    land_new(0xFFFF_FFFF, 0xFE, 6)
                }
            } else {
                let s = rd::<GPtr<u8>>(this, 8);
                let a = rd::<GPtr<u8>>(s, 0x1C);
                if !a.is_null() && rd::<u16>(a, 0x2C) == 0x78 {
                    let t = if !x(rd::<f32>(s, 0x20)).fcom(gf(0x858F0C)).test_parity(5) {
                        let t = task_new(0x20);
                        if t.is_null() { GPtr::NULL } else { fall_ctor(t, 0x1A, 0, 0x2BC) }
                    } else {
                        land_new(0x7B, 0xFC, 4)
                    };
                    landing_noise(ped, 0x3B, 0x4270_0000, 0x4296_0000);
                    t
                } else {
                    let t = if ped_is_player(ped) != 0 {
                        let pad = player_get_pad(ped);
                        let a = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x1C);
                        if !a.is_null() && rd::<u16>(a, 0x2C) == 9 {
                            land_new(0xA, 0xFC, 4)
                        } else {
                            let pd = rd::<GPtr<u8>>(ped, 0x480);
                            let moving = !x(rd::<f32>(pd, 0x14)).fcom(gf(0x858CE8)).test_any(0x41)
                                && !pad.is_null()
                                && (x87_axis(pad_get_ped_walk_left_right(pad)).fcom(gf(0x858B50)).test_parity(0x44)
                                    || x87_axis(pad_get_ped_walk_up_down(pad)).fcom(gf(0x858B50)).test_parity(0x44));
                            if moving { land_new(0x77, 0xFC, 4) } else { land_new(0x7A, 0xFC, 4) }
                        }
                    } else {
                        land_new(0x7A, 0xFC, 4)
                    };
                    landing_noise(ped, 0x3A, 0x425C_0000, 0x428C_0000);
                    t
                }
            }
        }
        _ => GPtr::NULL,
    }
}

/// `fild` of a pad axis (`movsx` of AX).
fn x87_axis(v: u16) -> X87 {
    xi(v as i16 as i32)
}

/// `CTaskComplexInAirAndLand::ControlSubTask(CPed*)` (0x67D230, `ret 4`): unless +0xD (vault) or +0xE, an in-air
/// sub-task (0xF1) that found a climb entity (+0x38) is replaced by a pool `CTaskSimpleClimb(entity, sub +8,
/// heading sub +0x14, surface byte sub +0x18, height, false)`; height 2 when [0x8D2F34] - [0x858C24] > sub +0x10 -
/// ped z, else 1 (the original passes the height byte inside a dword whose upper bytes are the task pointer's;
/// the constructor reads the byte). Otherwise the current sub-task.
#[hook(addr = 0x67D230, conv = thiscall, name = "CTaskComplexInAirAndLand::ControlSubTask", expect = "6A FF 68 EB 63 84 00")]
pub fn in_air_and_land_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(this, 0xD) != 0 || sub.is_null() || v_get_task_type(sub) != 0xF1 || rd::<u8>(this, 0xE) != 0 {
        return sub;
    }
    let s = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(s, 0x38) == 0 {
        return sub;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let a = x(rd::<f32>(s, 0x10)).fsub_m(rd::<f32>(pos, 8));
    let b = gf(0x8D2F34).fsub_m(gm(0x858C24));
    let height = if b.fcom(a).test_any(0x41) { 1 } else { 2 };
    let t = task_new(0x30);
    if t.is_null() {
        return GPtr::NULL;
    }
    let e = rd::<u32>(s, 0x38);
    let surf = rd::<u8>(s, 0x18) as u32;
    climb_ctor(t, e, s.byte_add(8), rd::<u32>(s, 0x14), surf, height, 0)
}

// ---- CTaskSimpleInAir::ProcessPed -----------------------------------------------------------------------

/// The fall-anim test (0x680702 / 0x6807B6): an association whose blend amount < [0x858624] and delta <=
/// [0x858B50] (ordered) may be replaced; a null one too.
fn assoc_fading(a: GPtr<u8>) -> bool {
    a.is_null()
        || (!x(rd::<f32>(a, 0x18)).fcom(gf(0x858624)).test_parity(5)
            && !x(rd::<f32>(a, 0x1C)).fcom(gf(0x858B50)).test_parity(0x41))
}

/// The landing end (0x680C0C): with an anim and +0x24 bit 1 the anim gets delta -1000, +0x2E |= 4 and the
/// default finish callback and is dropped; ped +0x46C bit 0x200 cleared; true.
fn in_air_done(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if !a.is_null() && rd::<u8>(this, 0x24) & 2 != 0 {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
        let a = rd::<GPtr<u8>>(this, 0x1C);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x1C), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x1C, 0);
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
    1
}

/// Ground contact (0x680A44): done when the ped stands less than [0x859918] above the line hit (colpoint z), is
/// standing (+0x46C bit 0) or attached (+0xFC); otherwise the task goes on (the climb test at 0x680A7A).
fn in_air_landed(this: GPtr<u8>, ped: GPtr<u8>, hit_z: f32) -> Option<u8> {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let h = x(rd::<f32>(m, 0x38)).fsub_m(hit_z);
    if !h.fcom(gf(0x859918)).test_parity(5) {
        return Some(in_air_done(this, ped));
    }
    if rd::<u32>(ped, 0x46C) & 1 != 0 || rd::<u32>(ped, 0xFC) != 0 {
        return Some(in_air_done(this, ped));
    }
    None
}

/// `CTaskSimpleInAir::ProcessPed(CPed*)` (0x680600, `ret 4`; the climb test continues through the island 0x404880,
/// which loads ped +0x530). Fall shout once the 500 ms timer expired (`Say(0x166 player / 0x15A)` unless state
/// 0x27, when vz·[0x858B40] <= [0x870644]); first frame (no anim +0x1C): ped +0x46C |= 0x200 and the jump-glide
/// (0x76/0x80; the blend result is not stored, as in the original), vault (0x79, with a second delete callback)
/// or fall-glide (0x79) anim; the glide anim 0x80 pushes the ped up (`ApplyMoveForce(0, 0, step·mass·k)`);
/// climbing out of a jump (parent's parent 0xD3, falling-towards-wall dot < k, counter +0x28 < 1000) applies
/// `forward·(k - dot)·mass` and advances the counter by `_ftol(step·[0x858B38]·[0x858C4C])`; otherwise the
/// minimum vz is tracked, a vertical line below the ped (unless already on ground) or the vault counter decides
/// landing; still falling fast without a fall anim → the fall anim 0x78. A jump-glide (+0x24 bit 0) player with no
/// climb entity yet tests for a climb (`TestForClimb`, reference registered on +0x38) or marks the parent jump
/// task (+0xE) when the heading +0x14 < [0x859948]. The original keeps `&this->+0x38` in its own argument slot.
/// Ported, not registered: the first build set ped +0x46C bit 0 (standing) on every landing, which the bytes do
/// only after the slow-fall counter (0x680A3D, the `or` sits just before the 0x680A44 target); the grove walk
/// diverged at frame 257 (`peds`) with it. Fixed after the verification runs; needs an L3 rerun before the hook
/// is registered again.
pub fn in_air_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let mut pos = [rd::<u32>(m, 0x30), f32_of(x(rd::<f32>(m, 0x34))).to_bits(), f32_of(x(rd::<f32>(m, 0x38))).to_bits()];
    let z_end = f32_of(x(rd::<f32>(m, 0x38)).fsub_m(gm(0x858B90)));
    let clump = rd::<u32>(ped, 0x18);
    if rd::<u8>(this, 0x34) != 0 {
        if rd::<u8>(this, 0x35) != 0 {
            wr::<u32>(this, 0x2C, time_ms());
            wr::<u8>(this, 0x35, 0);
        }
        let end = rd::<u32>(this, 0x30).wrapping_add(rd::<u32>(this, 0x2C));
        if time_ms() >= end
            && rd::<u32>(ped, 0x530) != 0x27
            && !x(rd::<f32>(ped, 0x4C)).fmul_m(gm(0x858B40)).fcom(gf(0x870644)).test_parity(0x41)
        {
            let id = if ped_is_player(ped) != 0 { 0x166 } else { 0x15A };
            ped_say(ped, id, 0, 0x3F80_0000, 0, 0, 0);
            wr::<u8>(this, 0x34, 0);
        }
    }
    if rd::<u32>(this, 0x1C) == 0 {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x200);
        let fl = rd::<u8>(this, 0x24);
        if fl & 1 != 0 {
            let mut a = clump_get_association(clump, 0x76);
            wr::<GPtr<u8>>(this, 0x1C, a);
            if a.is_null() {
                a = clump_get_association(rd::<u32>(ped, 0x18), 0x80);
                wr::<GPtr<u8>>(this, 0x1C, a);
            }
            if assoc_fading(rd::<GPtr<u8>>(this, 0x1C)) {
                anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x76, 0x4080_0000);
            }
        } else if fl & 2 != 0 {
            if !x(rd::<f32>(ped, 0x4C)).fcom(gf(0x8703A8)).test_parity(0x41) {
                let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x79, 0x4080_0000);
                wr::<GPtr<u8>>(this, 0x1C, a);
                assoc_set_delete_callback(a, IN_AIR_DELETE_ANIM_CB, this.addr() as u32);
                wr::<u32>(rd::<GPtr<u8>>(this, 0x1C), 0x1C, 0x3F00_0000);
                wr::<u32>(rd::<GPtr<u8>>(this, 0x1C), 0x18, 0x3F80_0000);
            }
        } else {
            if rd::<u32>(this, 0x1C) == 0 {
                let a = clump_get_association(rd::<u32>(ped, 0x18), 0x80);
                wr::<GPtr<u8>>(this, 0x1C, a);
            }
            if assoc_fading(rd::<GPtr<u8>>(this, 0x1C)) {
                let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x79, 0x4080_0000);
                wr::<GPtr<u8>>(this, 0x1C, a);
            }
        }
        let a = rd::<GPtr<u8>>(this, 0x1C);
        if !a.is_null() {
            assoc_set_delete_callback(a, IN_AIR_DELETE_ANIM_CB, this.addr() as u32);
        }
    }
    'body: {
        let a = rd::<GPtr<u8>>(this, 0x1C);
        if !a.is_null() && rd::<u16>(a, 0x2C) == 0x80 && rd::<u8>(ped, 0x46C) & 1 == 0 {
            let cur = x(rd::<f32>(a, 0x20));
            let total = x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10));
            if !total.fcom(cur).test_any(0x41)
                && (!x(rd::<f32>(a, 0x18)).fcom(gf(0x858624)).test_any(1)
                    || !x(rd::<f32>(a, 0x1C)).fcom(gf(0x858B50)).test_any(0x41))
            {
                let f = gf(TIME_STEP).fmul_m(rd::<f32>(ped, 0x8C)).fmul_m(gm(0x8703AC)).fmul_m(gm(0x863984));
                physical_apply_move_force(ped, 0.0, 0.0, f32_of(f));
                break 'body;
            }
        }
        let c0 = gf(0x858B50);
        let vz_up = x(rd::<f32>(ped, 0x4C)).fcom(c0).test_parity(0x41);
        let moving = vz_up
            && (x(rd::<f32>(ped, 0x44)).fcom(c0).test_parity(0x44)
                || x(rd::<f32>(ped, 0x48)).fcom(c0).test_parity(0x44)
                || x(rd::<f32>(ped, 0x4C)).fcom(c0).test_parity(0x44));
        let climb_out = moving
            || (rd::<u8>(this, 0x24) & 2 == 0 && matches!(rd::<u32>(ped, 0x530), 0x36 | 0x37));
        if climb_out {
            if rd::<u8>(this, 0x24) & 2 != 0 {
                break 'body;
            }
            let p = rd::<GPtr<u8>>(this, 4);
            if p.is_null() {
                break 'body;
            }
            let pp = rd::<GPtr<u8>>(p, 4);
            if pp.is_null() || v_get_task_type(pp) != 0xD3 {
                break 'body;
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let fwd = m.byte_add(0x10);
            let dot = f32_of(
                x(rd::<f32>(ped, 0x4C))
                    .fmul_m(rd::<f32>(m, 0x18))
                    .fadd(x(rd::<f32>(ped, 0x48)).fmul_m(rd::<f32>(fwd, 4)))
                    .fadd(x(rd::<f32>(ped, 0x44)).fmul_m(rd::<f32>(fwd, 0))),
            );
            let k = f32_of(gf(0x8703B0).fmul_m(gm(0x8703A4)));
            if x(dot).fcom(x(k)).test_parity(5) || rd::<i32>(this, 0x28) >= 0x3E8 {
                break 'body;
            }
            let f = f32_of(x(k).fsub_m(dot).fmul_m(rd::<f32>(ped, 0x8C)));
            let mut out = [0u32; 3];
            let op = buf(&mut out);
            vector_mul(op, fwd, f);
            physical_apply_move_force_vec(ped, out[0], out[1], out[2]);
            let n = ftol32(gf(TIME_STEP).fmul_m(gm(0x858B38)).fmul_m(gm(0x858C4C)));
            wr::<i32>(this, 0x28, rd::<i32>(this, 0x28).wrapping_add(n));
            break 'body;
        }
        // 0x6809AF
        if !x(rd::<f32>(ped, 0x4C)).fcom(x(rd::<f32>(this, 0x20))).test_parity(5) {
            wr::<u32>(this, 0x20, rd::<u32>(ped, 0x4C));
        }
        let mut cp = [0u32; 0xB];
        let mut hit_ent = [0u32; 1];
        let ground = if rd::<u8>(ped, 0x46C) & 1 != 0 {
            true
        } else {
            world_process_vertical_line(
                buf(&mut pos), z_end.to_bits(), buf(&mut cp), buf(&mut hit_ent), 1, 1, 0, 1, 0, 0, 0,
            ) != 0
        };
        let hit_z = f32::from_bits(cp[2]);
        if rd::<u8>(this, 0x24) & 2 != 0 {
            let vz = x(rd::<f32>(ped, 0x4C));
            if !vz.fcom(gf(0x858B50)).test_parity(0x41) && !x(rd::<f32>(ped, 0x4C)).fcom(gf(0x8D2EFC)).test_any(0x41) {
                let n = rd::<i32>(this, 0x28).wrapping_add(1);
                wr::<i32>(this, 0x28, n);
                if n > g::<i32>(0x8D2EF8).read() {
                    // 0x680A3D: only the slow-fall counter marks the ped standing (+0x46C bit 0)
                    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
                    if let Some(r) = in_air_landed(this, ped, hit_z) {
                        return r;
                    }
                    break 'body;
                }
            } else {
                wr::<u32>(this, 0x28, 0);
            }
        }
        if ground {
            if let Some(r) = in_air_landed(this, ped, hit_z) {
                return r;
            }
            break 'body;
        }
        if rd::<u32>(ped, 0x530) == 0x27 || x(rd::<f32>(ped, 0x4C)).fcom(gf(0x858EF4)).test_parity(5) {
            break 'body;
        }
        let a = rd::<GPtr<u8>>(this, 0x1C);
        if a.is_null() || rd::<u16>(a, 0x2C) == 0x78 {
            break 'body;
        }
        assoc_set_delete_callback(a, DEFAULT_ANIM_CB, 0);
        let n = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x78, 0x4080_0000);
        wr::<GPtr<u8>>(this, 0x1C, n);
        assoc_set_delete_callback(n, IN_AIR_DELETE_ANIM_CB, this.addr() as u32);
    }
    // 0x680A7A
    if rd::<u8>(this, 0x24) & 1 == 0 || rd::<u32>(this, 0x38) != 0 {
        return 0;
    }
    let st = rd::<u32>(ped, 0x530);
    if st == 0x36 || st == 0x37 || ped_is_player(ped) == 0 || g::<u32>(0xB72914).read() != 0 {
        return 0;
    }
    if x(rd::<f32>(ped, 0x4C)).fcom(gf(0x858FDC)).test_any(0x41) {
        return 0;
    }
    let (vy, vx) = (x(rd::<f32>(ped, 0x48)), x(rd::<f32>(ped, 0x44)));
    let sp = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
    if sp.fcom(gf(0x858C28)).test_any(0x41) {
        return 0;
    }
    let mut jump = GPtr::<u8>::NULL;
    let p = rd::<GPtr<u8>>(this, 4);
    if !p.is_null() && v_get_task_type(p) == 0xF0 {
        jump = rd::<GPtr<u8>>(this, 4);
        if !jump.is_null() && rd::<u8>(jump, 0xE) != 0 {
            return 0;
        }
    }
    let r = climb_test_for_climb(ped, this.byte_add(8), this.byte_add(0x14), this.byte_add(0x18), 0);
    wr::<GPtr<u8>>(this, 0x38, r);
    if !r.is_null() {
        entity_register_reference(r, this.byte_add(0x38));
        return 0;
    }
    if !x(rd::<f32>(this, 0x14)).fcom(gf(0x859948)).test_parity(5) && !jump.is_null() {
        wr::<u8>(jump, 0xE, 1);
    }
    0
}

// ---- CTaskSimpleGetUp / CTaskSimpleFall / evasive step --------------------------------------------------

/// `CTaskSimpleGetUp::ProcessPed(CPed*)` (0x67FA80, `ret 4`): ped +0x528 |= 8; finished (+9) → true. Without an anim
/// (+0xC) `StartAnim`; with one the ped's +0x478 bit 3 is cleared. Until the anim passed [0x858F34] of its length
/// (or without an anim) ped +0x474 |= 0x100000. False.
#[hook(addr = 0x67FA80, conv = thiscall, name = "CTaskSimpleGetUp::ProcessPed", expect = "56 57 8B 7C 24 0C")]
pub fn get_up_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
    if rd::<u8>(this, 9) != 0 {
        return 1;
    }
    let has = rd::<u32>(this, 0xC) != 0 || {
        get_up_start_anim(this, ped);
        rd::<u32>(this, 0xC) != 0
    };
    if has {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFF7);
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    let early = a.is_null() || {
        let cur = x(rd::<f32>(a, 0x20));
        let part = x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)).fmul_m(gm(0x858F34));
        !part.fcom(cur).test_any(0x41)
    };
    if early {
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x10_0000);
    }
    0
}

/// `CTaskSimpleGetUp::FinishGetUpAnimCB(CAnimBlendAssociation*, void* task)` (0x678110, cdecl): the task's anim
/// (+0xC, no null check) gets blend delta -1000 and is dropped; +9 (finished) and +8 set.
#[hook(addr = 0x678110, conv = cdecl, name = "CTaskSimpleGetUp::FinishGetUpAnimCB", expect = "8B 44 24 08 8B 50 0C")]
pub fn get_up_finish_anim_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(task, 0xC);
    wr::<u8>(task, 9, 1);
    wr::<u32>(a, 0x1C, 0xC47A_0000);
    wr::<u32>(task, 0xC, 0);
    wr::<u8>(task, 8, 1);
}

/// `CTaskSimpleFall::ProcessPed(CPed*)` (0x67FAF0, `ret 4`): ped +0x528 |= 8, +0x474 |= 0x100000. Landed (+8) on
/// the ground (+0x46C bit 0 or +0x470 bit 0x80000): the down time +0x1C counts down by `_ftol(step·[0x858B38]·
/// [0x858C4C])` (unsigned; true once it ran out, +0x1C = 0); above [0x8D2EF4] a player with a pad whose word
/// +0x10E is 0 (not +0x474 bit 6, state != 0x3F) is cut to [0x8D2EF4] minus one step. Otherwise the fall anim is
/// started when there is none (+0x14) and not landed, and `ProcessFall` runs. False unless done.
#[hook(addr = 0x67FAF0, conv = thiscall, name = "CTaskSimpleFall::ProcessPed", expect = "56 8B 74 24 08")]
pub fn fall_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x10_0000);
    let landed = rd::<u8>(this, 8);
    let step = || ftol32(gf(TIME_STEP).fmul_m(gm(0x858B38)).fmul_m(gm(0x858C4C))) as u32;
    if landed != 0 && (rd::<u8>(ped, 0x46C) & 1 != 0 || rd::<u32>(ped, 0x470) & 0x8_0000 != 0) {
        let n = step();
        let t = rd::<u32>(this, 0x1C);
        if t <= n {
            wr::<u32>(this, 0x1C, 0);
            return 1;
        }
        let t = t - n;
        wr::<u32>(this, 0x1C, t);
        if t > g::<u32>(0x8D2EF4).read()
            && ped_is_player(ped) != 0
            && rd::<u8>(ped, 0x474) & 0x40 == 0
            && rd::<u32>(ped, 0x530) != 0x3F
            && !player_get_pad(ped).is_null()
            && rd::<u16>(player_get_pad(ped), 0x10E) == 0
        {
            let n = step();
            wr::<u32>(this, 0x1C, g::<u32>(0x8D2EF4).read().wrapping_sub(n));
        }
        return 0;
    }
    if rd::<u32>(this, 0x14) == 0 && landed == 0 {
        fall_start_anim(this, ped);
    }
    fall_process_fall(this, ped);
    0
}

/// `CTaskComplexEvasiveStep::CreateSubTask(int32 type)` (0x6533E0, `ret 4`): 0x1F5 → `CTaskSimpleEvasiveStep(+0x18)`;
/// 0x386 → `CTaskSimpleAchieveHeading(f32(GetRadianAngleBetweenPoints(-dir.x, -dir.y, 0, 0)), 2, [0x86FC80])` (the
/// angle computed after the allocation); anything else null.
#[hook(addr = 0x6533E0, conv = thiscall, name = "CTaskComplexEvasiveStep::CreateSubTask", expect = "6A FF 68 06 34 84 00")]
pub fn evasive_step_create_sub_task(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0x1F5 => {
            let t = task_new(0x14);
            if t.is_null() { GPtr::NULL } else { evasive_step_simple_ctor(t, rd::<u32>(this, 0x18)) }
        }
        0x386 => {
            let t = task_new(0x18);
            if t.is_null() {
                return GPtr::NULL;
            }
            let y = f32_of(x(rd::<f32>(this, 0x10)).fchs());
            let xx = f32_of(x(rd::<f32>(this, 0xC)).fchs());
            let h = f32_of(X87::from_f64_exact(general_get_radian_angle_between_points(xx, y, 0.0, 0.0)));
            let k = g::<u32>(0x86FC80).read();
            achieve_heading_ctor(t, h.to_bits(), 0x4000_0000, k)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskSimpleEvasiveStep::StartAnim(CPed*)` (0x655EA0, `ret 4`): blends anim 0x7E (delta 8) into +0x10, clears
/// its +0x2E bit 2 and sets `FinishAnimEvasiveStepCB` (0x653290) with this task.
#[hook(addr = 0x655EA0, conv = thiscall, name = "CTaskSimpleEvasiveStep::StartAnim", expect = "8B 44 24 04 56")]
pub fn evasive_step_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x7E, 0x4100_0000);
    wr::<GPtr<u8>>(this, 0x10, a);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xFB);
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x10), 0x653290, this.addr() as u32);
}

/// `fild` of a dword the original treats as unsigned (`fild`, then `fadd [0x858C54]` = 2^32 when negative).
fn fild_u32(v: i32) -> X87 {
    let f = xi(v);
    if v < 0 { f.fadd_m(gm(0x858C54)) } else { f }
}

/// `CTaskSimpleGetUp::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x677FE0, `ret 0xC`). Priority 2: the
/// anim (+0xC) gets blend delta -1000, ped +0x478 bit 3 cleared, true. Priority 1: a damage event (type 9) with
/// both +0x40 and +0xC bytes set keeps the anim (but aborts); another damage event keeps it when one frame
/// (`_ftol(step·[0x858B38]·[0x858C4C])`·[0x858B3C]) is less than the time since the event (+0x18), both as unsigned
/// floats; other events abort only with priority >= 0x3D. Aborting drops the anim (default finish callback),
/// sets +9 and +8 and clears ped +0x478 bit 3; with a kept anim the task aborts only when already finished (+8)
/// or for the first damage case. Other priorities: false.
#[hook(addr = 0x677FE0, conv = thiscall, name = "CTaskSimpleGetUp::MakeAbortable", expect = "8B 44 24 08 83 F8 02")]
pub fn get_up_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    let done = |ped: GPtr<u8>| {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFF_FFF7);
        1
    };
    if prio == 2 {
        let a = rd::<GPtr<u8>>(this, 0xC);
        if !a.is_null() {
            wr::<u32>(a, 0x1C, 0xC47A_0000);
        }
        return done(ped);
    }
    if prio != 1 {
        return 0;
    }
    let mut keep = false;
    let mut flag = false;
    if !ev.is_null() {
        if v_get_event_type(ev) == 9 {
            if rd::<u8>(ev, 0x40) != 0 && rd::<u8>(ev, 0xC) != 0 {
                flag = true;
            } else {
                let d = fild_u32(g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(ev, 0x18)) as i32);
                let n = ftol32(gf(TIME_STEP).fmul_m(gm(0x858B38)).fmul_m(gm(0x858C4C)));
                let f = fild_u32(n).fmul_m(gm(0x858B3C));
                if !f.fcom(d).test_parity(5) {
                    keep = true;
                }
            }
        } else if (v_get_event_priority(ev) as i32) < 0x3D {
            return 0;
        }
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() && !keep {
        assoc_set_finish_callback(a, DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0xC, 0);
        wr::<u8>(this, 9, 1);
        wr::<u8>(this, 8, 1);
        return done(ped);
    }
    if rd::<u8>(this, 8) != 0 || flag {
        return done(ped);
    }
    0
}

/// `CTaskSimpleFall::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x678370, `ret 0xC`): the get-up anim
/// 0x7D (else 0x7C) of the ped; priority 2: the fall anim (+0xC id) and that anim get delta -1000, +8 = 1, +0x1C =
/// 0, true. Priority 1 with a damage event (type 9) on a ped whose +0x540 < [0x858624], or an event of type 0x42,
/// priority 0x47 or type 0x5D (each asked again): blend delta [0x85B320] for 0x42/0x5D, else -2, written to the
/// get-up anim and the task's anim (+0x14, dropped with the default finish callback); +0x14/+0x1C = 0, +8 = 1,
/// true. Otherwise false.
#[hook(addr = 0x678370, conv = thiscall, name = "CTaskSimpleFall::MakeAbortable", expect = "53 8B 5C 24 08")]
pub fn fall_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    let mut up = clump_get_association(rd::<u32>(ped, 0x18), 0x7D);
    if up.is_null() {
        up = clump_get_association(rd::<u32>(ped, 0x18), 0x7C);
    }
    if prio == 2 {
        let a = clump_get_association(rd::<u32>(ped, 0x18), rd::<u32>(this, 0xC));
        if !a.is_null() {
            wr::<u32>(a, 0x1C, 0xC47A_0000);
        }
        if !up.is_null() {
            wr::<u32>(up, 0x1C, 0xC47A_0000);
        }
        wr::<u8>(this, 8, 1);
        wr::<u32>(this, 0x1C, 0);
        return 1;
    }
    if prio != 1 || ev.is_null() {
        return 0;
    }
    let go = (v_get_event_type(ev) == 9 && !x(rd::<f32>(ped, 0x540)).fcom(gf(0x858624)).test_parity(5))
        || v_get_event_type(ev) == 0x42
        || v_get_event_priority(ev) == 0x47
        || v_get_event_type(ev) == 0x5D;
    if !go {
        return 0;
    }
    let v = if v_get_event_type(ev) == 0x42 || v_get_event_type(ev) == 0x5D { gf(0x85B320) } else { x(-2.0) };
    if !up.is_null() {
        wr::<f32>(up, 0x1C, f32_of(v));
    }
    let a = rd::<GPtr<u8>>(this, 0x14);
    if !a.is_null() {
        wr::<f32>(a, 0x1C, f32_of(v));
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
    }
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u8>(this, 8, 1);
    1
}

/// `CTaskComplexFallAndGetUp::CreateSubTask(int32 type)` (0x678900, `ret 4`): 0xCD → the inlined `CTaskSimpleGetUp`
/// (pool slot 0x10: `CTaskSimple` base, +8/+9 = 0, +0xC = 0, vtable 0x8703E0); 0xCF → `CTaskSimpleFall(+0xC, +0x10,
/// +0x14)`; anything else null.
#[hook(addr = 0x678900, conv = thiscall, name = "CTaskComplexFallAndGetUp::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn fall_and_get_up_create_sub_task(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0xCD => {
            let t = task_new(0x10);
            if t.is_null() {
                return GPtr::NULL;
            }
            task_simple_ctor(t);
            wr::<u8>(t, 8, 0);
            wr::<u8>(t, 9, 0);
            wr::<u32>(t, 0xC, 0);
            wr::<u32>(t, 0, 0x8703E0);
            t
        }
        0xCF => {
            let t = task_new(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            let (c, b, a) = (rd::<u32>(this, 0x14), rd::<u32>(this, 0x10), rd::<u32>(this, 0xC));
            fall_ctor(t, a, b, c)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskSimpleFall::StartAnim(CPed*)` (0x67CA40, `ret 4`): false while the ped stands on something (+0xFC).
/// Anim 0xBF reuses 0x69 (else 0x6A) when the ped plays it; another anim id (+0xC) already playing is restarted
/// (time 0, blend 0, delta 8, flags |5 & ~8); else it is blended in (group +0x10, delta 8, flags |4 & ~8) and 0x6A
/// starts at 0.4. Every found/blended anim gets `FinishFallAnimCB` (0x6786B0) with this task. True.
#[hook(addr = 0x67CA40, conv = thiscall, name = "CTaskSimpleFall::StartAnim", expect = "56 57 8B 7C 24 0C")]
pub fn fall_start_anim_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0xFC) != 0 {
        return 0;
    }
    let id = rd::<u32>(this, 0xC);
    let me = this.addr() as u32;
    if id == 0xBF {
        let mut a = clump_get_association(rd::<u32>(ped, 0x18), 0x69);
        wr::<GPtr<u8>>(this, 0x14, a);
        if a.is_null() {
            a = clump_get_association(rd::<u32>(ped, 0x18), 0x6A);
            wr::<GPtr<u8>>(this, 0x14, a);
        }
        let a = rd::<GPtr<u8>>(this, 0x14);
        if !a.is_null() {
            assoc_set_finish_callback(a, 0x6786B0, me);
        }
        return 1;
    }
    let a = clump_get_association(rd::<u32>(ped, 0x18), id);
    wr::<GPtr<u8>>(this, 0x14, a);
    if !a.is_null() {
        assoc_set_current_time(a, 0.0);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x18, 0);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x1C, 0x4100_0000);
        let a = rd::<GPtr<u8>>(this, 0x14);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 5);
        let a = rd::<GPtr<u8>>(this, 0x14);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xF7);
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), 0x6786B0, me);
        return 1;
    }
    let (id, grp) = (rd::<u32>(this, 0xC), rd::<u32>(this, 0x10));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), grp, id, 0x4100_0000);
    wr::<GPtr<u8>>(this, 0x14, a);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
    let a = rd::<GPtr<u8>>(this, 0x14);
    wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xF7);
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), 0x6786B0, me);
    if rd::<u32>(this, 0xC) == 0x6A {
        assoc_set_current_time(rd::<GPtr<u8>>(this, 0x14), f32::from_bits(0x3ECC_CCCD));
    }
    1
}

/// `CTaskSimpleFall::ProcessFall(CPed*)` (0x6784C0, `ret 4`; EAX not meaningful). A falling player (+0x470 bit 4
/// or sign bit) not yet on the ground: the get-up anim (the first 0x10-flag association when it is 0x7C/0x7D,
/// else 0x7C, else 0x7D) is fast-forwarded to [0x87039C] (flags |1) once it crossed [0x8703A0] with blend >
/// [0x858C24] and delta >= [0x858B50]; without one, a first 0x10-flag anim past [0x858C98] of its length blends
/// 0x7D (its flag bit 11) or 0x7C. Otherwise (or a non-player) on the ground (+0x46C bit 0, not bit 1): a get-up
/// anim 0x7C/0x7D gets speed 3 and clears +0x470 bit 4; else the bit is cleared when the first 0x10-flag anim
/// has flags bit 0 clear.
#[hook(addr = 0x6784C0, conv = thiscall, name = "CTaskSimpleFall::ProcessFall", expect = "53 55 56 8B 74 24 10")]
pub fn fall_process_fall_port(_this: GPtr<u8>, ped: GPtr<u8>) {
    let falling = |ped: GPtr<u8>| {
        let fl = rd::<u32>(ped, 0x470);
        fl & 0x10 != 0 || (fl as i32) < 0
    };
    if ped_is_player(ped) != 0 && falling(ped) && rd::<u8>(ped, 0x46C) & 1 == 0 {
        let first = clump_get_first_association_flags(rd::<u32>(ped, 0x18), 0x10);
        let mut up = if !first.is_null() && matches!(rd::<u16>(first, 0x2C), 0x7C | 0x7D) {
            first
        } else {
            clump_get_association(rd::<u32>(ped, 0x18), 0x7C)
        };
        if up.is_null() {
            up = clump_get_association(rd::<u32>(ped, 0x18), 0x7D);
        }
        if up.is_null() {
            if first.is_null() {
                return;
            }
            let cur = x(rd::<f32>(first, 0x20));
            let part = x(rd::<f32>(rd::<GPtr<u8>>(first, 0x14), 0x10)).fmul_m(gm(0x858C98));
            if part.fcom(cur).test_parity(5) {
                return;
            }
            let bit = (rd::<u16>(first, 0x2E) as u32 >> 0xB) & 1 != 0;
            let id = if bit { 0x7D } else { 0x7C };
            anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 0x4100_0000);
            return;
        }
        if x(rd::<f32>(up, 0x18)).fcom(gf(0x858C24)).test_any(0x41)
            || x(rd::<f32>(up, 0x1C)).fcom(gf(0x858B50)).test_any(1)
            || x(rd::<f32>(up, 0x20)).fcom(gf(0x8703A0)).test_any(0x41)
            || x(rd::<f32>(up, 0x20)).fsub_m(rd::<f32>(up, 0x28)).fcom(gf(0x8703A0)).test_parity(0x41)
        {
            return;
        }
        assoc_set_current_time(up, gm(0x87039C));
        wr::<u16>(up, 0x2E, rd::<u16>(up, 0x2E) | 1);
        return;
    }
    if !falling(ped) || rd::<u8>(ped, 0x46C) & 1 == 0 || (rd::<u32>(ped, 0x46C) >> 1) & 1 != 0 {
        return;
    }
    let mut a = clump_get_association(rd::<u32>(ped, 0x18), 0x7C);
    if a.is_null() {
        a = clump_get_association(rd::<u32>(ped, 0x18), 0x7D);
    }
    if !a.is_null() {
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFFF_FFEF);
        wr::<u32>(a, 0x24, 0x4040_0000);
        return;
    }
    let f = clump_get_first_association_flags(rd::<u32>(ped, 0x18), 0x10);
    if f.is_null() || rd::<u8>(f, 0x2E) & 1 != 0 {
        return;
    }
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFFF_FFEF);
}

/// The damage branch of `CTaskSimpleGetUp::StartAnim` (0x67C870): ped +0x478 |= 8; a player takes `step` (when >
/// [0x858B50]), any other ped with pad 0 word +0x10E set takes 1000, as weapon 0x32 damage to body part 3 from the
/// blocking vehicle: stack `CPedDamageResponseCalculator` + `CEventDamage` (the bool argument is the dword
/// `(ped +0x46C >> 8) & 0xFFFFFF01`, as pushed), `ComputeDamageResponse` into the event's response (+0x38) when
/// it affects the ped, else event +0x42 = 1; added to the ped's event group (+0x68 of intelligence); false.
fn get_up_damage(ped: GPtr<u8>, veh: GPtr<u8>) -> u8 {
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 8);
    let dmg = if ped_is_player(ped) != 0 {
        let ts = gm(TIME_STEP);
        if x(ts).fcom(gf(0x858B50)).test_any(0x41) {
            return 0;
        }
        ts
    } else {
        if rd::<u16>(pad_get_pad(0), 0x10E) == 0 {
            return 0;
        }
        1000.0
    };
    let mut calc = [0u32; 5];
    let mut ev = [0u32; 0x11];
    let cp = buf(&mut calc);
    let ep = buf(&mut ev);
    damage_calc_ctor(cp, veh, dmg.to_bits(), 0x32, 3, 0);
    let flag = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ep, veh, g::<u32>(TIME_MS).read(), 0x32, 3, 0, 0, flag);
    if event_damage_affects_ped(ep, ped) != 0 {
        damage_calc_compute_damage_response(cp, ped, ep.byte_add(0x38), 1);
    } else {
        wr::<u8>(ep, 0x42, 1);
    }
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    event_damage_dtor(ep);
    damage_calc_dtor(cp);
    0
}

/// `CTaskSimpleGetUp::StartAnim(CPed*)` (0x67C770, `ret 4`). A car blocking the spot (`IsPositionClearOfCars`, not
/// wrecked/state 9, not +0x594 == 2, not the ped's ground entity +0xFC or +0x568) → damage branch. Otherwise a
/// valid vehicle in ped +0x128 (type bits 2, same checks): on the ped's 8-frame slot ((+0x20 + frame - 3) & 7 ==
/// 0) its collision with the ped is tested (`ProcessColModels` into 0xC18F98), a hit or another frame → damage
/// branch. Else ped +0x128 = 0, the get-up anim 0x73 (with an 0x800-flag association) or 0x70 is blended (delta
/// 1000, `FinishGetUpAnimCB` with this task, stored in +0xC), `SetPedState(1)`, true. The original keeps the step /
/// 1000 in its ped argument slot (a Rust local here).
#[hook(addr = 0x67C770, conv = thiscall, name = "CTaskSimpleGetUp::StartAnim", expect = "6A FF 68 30 63 84 00")]
pub fn get_up_start_anim_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let veh = ped_placement_is_position_clear_of_cars(ped);
    if !veh.is_null()
        && rd::<u32>(veh, 0x590) != 9
        && rd::<u32>(veh, 0x594) != 2
        && veh != rd::<GPtr<u8>>(ped, 0xFC)
        && veh != rd::<GPtr<u8>>(ped, 0x568)
    {
        return get_up_damage(ped, veh);
    }
    let a = rd::<GPtr<u8>>(ped, 0x128);
    if !a.is_null()
        && rd::<u8>(a, 0x36) & 7 == 2
        && rd::<u32>(a, 0x590) != 9
        && rd::<u32>(a, 0x594) != 2
        && vehicle_is_pointer_valid(a) != 0
    {
        let slot = (rd::<u8>(ped, 0x20) as u32).wrapping_add(g::<u32>(0xB7CB4C).read()).wrapping_sub(3);
        if slot & 7 != 0 {
            return get_up_damage(ped, veh);
        }
        let mi = |e: GPtr<u8>| g::<GPtr<u8>>(0xA9B0C8 + (rd::<i16>(e, 0x22) as i32 as usize).wrapping_mul(4)).read();
        let vc = rd::<u32>(mi(rd::<GPtr<u8>>(ped, 0x128)), 0x14);
        let pc = rd::<u32>(mi(ped), 0x14);
        let pm = rd::<u32>(ped, 0x14);
        let vm = placeable_get_matrix(rd::<GPtr<u8>>(ped, 0x128));
        if collision_process_col_models(pm, pc, vm, vc, 0xC18F98, 0, 0, 0) > 0 {
            return get_up_damage(ped, veh);
        }
    }
    wr::<u32>(ped, 0x128, 0);
    let f = clump_get_first_association_flags(rd::<u32>(ped, 0x18), 0x800);
    let id = if f.is_null() { 0x70 } else { 0x73 };
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 0x447A_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_finish_callback(a, 0x678110, this.addr() as u32);
    ped_set_ped_state(ped, 1);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IN_AIR_AND_LAND_CTOR,
    HOOK_IN_AIR_CTOR_PORT,
    HOOK_LAND_FINISH_ANIM_CB,
    HOOK_LAND_RIGHT_FOOT_LANDED,
    HOOK_LAND_LEFT_FOOT_LANDED,
    HOOK_IN_AIR_AND_LAND_CREATE_FIRST_SUB_TASK,
    HOOK_IN_AIR_AND_LAND_CREATE_NEXT_SUB_TASK,
    HOOK_IN_AIR_AND_LAND_CONTROL_SUB_TASK,
    HOOK_GET_UP_PROCESS_PED,
    HOOK_GET_UP_FINISH_ANIM_CB,
    HOOK_FALL_PROCESS_PED,
    HOOK_EVASIVE_STEP_CREATE_SUB_TASK,
    HOOK_EVASIVE_STEP_START_ANIM,
    HOOK_GET_UP_MAKE_ABORTABLE,
    HOOK_FALL_MAKE_ABORTABLE,
    HOOK_FALL_AND_GET_UP_CREATE_SUB_TASK,
    HOOK_FALL_START_ANIM_PORT,
    HOOK_FALL_PROCESS_FALL_PORT,
    HOOK_GET_UP_START_ANIM_PORT,
];
