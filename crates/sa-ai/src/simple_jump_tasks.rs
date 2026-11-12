//! WRITER-NATIVE (batch `native-27`): `CTaskSimpleJump::ProcessPed` (0x680C60) and `StartLaunchAnim` (0x67D7A0),
//! reached when the player jumps (LShift). Ported from the 1.0 US bytes; gta-reversed (`TaskSimpleJump.cpp`) as a
//! reading aid (the bytes win). x87 compares follow the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `SurfaceInfos_c::IsSteepSlope(surface)` (0x55E770) on `g_surfaceInfos` (0xB79538)
    fn surface_is_steep_slope(s: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E770 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CGameLogic::IsPlayerAllowedToGoInThisDirection(CPed*, CVector dir, float)` (0x441E10)
    fn game_logic_player_allowed_direction(ped: GPtr<u8>, x: u32, y: u32, z: u32, dist: f32) -> u8 = cdecl 0x441E10 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier(i: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBE0 => unported;
    /// `CTaskSimpleJump::StartLaunchAnim(CPed*)` (0x67D7A0)
    fn start_launch_anim_c(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x67D7A0 => start_launch_anim;
    /// `CTaskSimpleClimb::TestForClimb(CPed*, CVector& pos, float& angle, uint8& surface, bool)` (0x6803A0)
    fn climb_test_for_climb(ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, b: u32) -> GPtr<u8> = cdecl 0x6803A0 => unported;
    /// `CTaskSimpleJump::CheckIfJumpBlocked(CPed*)` (0x67D590)
    fn check_if_jump_blocked(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x67D590 => check_if_jump_blocked_port;
    /// `CEventHandler::GetCurrentEventType()` (0x4B8CC0)
    fn event_handler_current_type(h: GPtr<u8>) -> i32 = thiscall 0x4B8CC0 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity*, bool, bool, bool, bool, bool, bool)` (0x569E20)
    #[allow(clippy::too_many_arguments)]
    fn world_test_sphere(x: u32, y: u32, z: u32, r: f32, ignore: u32, b: u32, v: u32, p: u32, o: u32, d: u32, c: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CEntity::TestCollision(bool)` (vtable slot 0x34)
    fn entity_test_collision(e: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0x34 => unported;
    /// `CTaskSimpleJump::Launch(CPed*)` (0x679B80)
    fn launch(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x679B80 => unported;
    /// `CEventSoundQuiet(CEntity*, float, uint32, CVector)` (0x5E05B0)
    #[allow(clippy::too_many_arguments)]
    fn event_sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, volume: f32, time: u32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventSoundQuiet::~CEventSoundQuiet()` body (0x5DEA00)
    fn event_sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => unported;
}

/// `CTaskSimpleJump::JumpAnimFinishCB` (0x67A020), passed as the original address.
const JUMP_ANIM_FINISH_CB: u32 = 0x67_A020;

/// `bool CTaskSimpleJump::StartLaunchAnim(CPed* ped)` (0x67D7A0, `ret 4`).
///
/// An existing launch anim (0x74, else 0x75) becomes +0x28 and the answer is false. On a steep slope (contact
/// surface +0xBA) facing down it (`forward . slope normal +0x578 < 0`, ordered), or for the player when
/// `IsPlayerAllowedToGoInThisDirection(ped, forward, 5)` fails: IK flag +0x528 |= 8, false. Otherwise the phase of
/// the move anim (sprint, else run, else walk, each only with blend >= 0.3; used above 0.3): `time / total +
/// [0x870634]`, minus 1 above 1; launch anim 0x75 from 0.5 on, else 0x74, blended at 8. Player peds (+0x480) get
/// its speed +0x24 = `GetFatAndMuscleModifier(2)`. Finish callback `JumpAnimFinishCB(this)`, aimed rotation +0x55C
/// = current +0x558, true.
#[hook(addr = 0x67D7A0, conv = thiscall, name = "CTaskSimpleJump::StartLaunchAnim",
       expect = "56 8B 74 24 08 8B 46 18",
       unverified = "native-27", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn start_launch_anim(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let clump = rd::<u32>(ped, 0x18);
    let a = clump_get_association(clump, 0x74);
    wr::<GPtr<u8>>(this, 0x28, a);
    if !a.is_null() {
        return 0;
    }
    let a = clump_get_association(rd::<u32>(ped, 0x18), 0x75);
    wr::<GPtr<u8>>(this, 0x28, a);
    if !a.is_null() {
        return 0;
    }
    let blocked = if surface_is_steep_slope(GPtr::from_addr(0xB7_9538), u32::from(rd::<u8>(ped, 0xBA))) != 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let d = x(rd::<f32>(m, 0x18))
            .fmul_m(rd::<f32>(ped, 0x580))
            .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(ped, 0x57C)))
            .fadd(x(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(ped, 0x578)));
        !d.fcom(x(0.0)).test_parity(0x05)
    } else if ped_is_player(ped) != 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        game_logic_player_allowed_direction(ped, rd::<u32>(m, 0x10), rd::<u32>(m, 0x14), rd::<u32>(m, 0x18), 5.0) == 0
    } else {
        false
    };
    if blocked {
        wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
        return 0;
    }
    let min = f32::from_bits(0x3E99_999A);
    let strong = |a: GPtr<u8>| !a.is_null() && x(rd::<f32>(a, 0x18)).fcom(x(min)).test_parity(0x05);
    let mut mv = clump_get_association(rd::<u32>(ped, 0x18), 2);
    if !strong(mv) {
        mv = clump_get_association(rd::<u32>(ped, 0x18), 1);
    }
    if !strong(mv) {
        mv = clump_get_association(rd::<u32>(ped, 0x18), 0);
    }
    let mut phase = x(0.0);
    if !mv.is_null() && !x(rd::<f32>(mv, 0x18)).fcom(x(min)).test_any(0x41) {
        phase = x(rd::<f32>(mv, 0x20))
            .fdiv_m(rd::<f32>(rd::<GPtr<u8>>(mv, 0x14), 0x10))
            .fadd_m(g::<f32>(0x87_0634).read());
        if !phase.fcom(x(1.0)).test_any(0x41) {
            phase = phase.fsub_m(1.0);
        }
    }
    let id = if phase.fcom(x(0.5)).test_parity(0x05) { 0x75 } else { 0x74 };
    let anim = anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 8.0);
    wr::<GPtr<u8>>(this, 0x28, anim);
    if rd::<u32>(ped, 0x480) != 0 {
        let s = X87::from_f64_exact(stats_get_fat_and_muscle_modifier(2)).to_f32();
        wr::<f32>(rd::<GPtr<u8>>(this, 0x28), 0x24, s);
    }
    assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x28), JUMP_ANIM_FINISH_CB, this);
    wr::<u32>(ped, 0x55C, rd::<u32>(ped, 0x558));
    1
}

/// `bool CTaskSimpleJump::ProcessPed(CPed* ped)` (0x680C60, `ret 4`, SEH frame).
///
/// Before the launch anim finished (+0x20 clear): without an anim (+0x28) `StartLaunchAnim`, recording the result
/// in +0x23 (false: true is returned), then +0x20. Finished: for the player or a climbing jump (+0x24) while
/// `[0xB72914]` is 0, `TestForClimb(ped, &+8, &+0x14, &+0x18, true)` -> +0x1C; without a climb entity
/// `CheckIfJumpBlocked`. Not blocked (+0x21): `Launch`. Blocked: the player raises a `CEventSoundQuiet(ped, 45, -1,
/// (0, 0, 0))` in the global event group. True.
#[hook(addr = 0x680C60, conv = thiscall, name = "CTaskSimpleJump::ProcessPed",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "native-27", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn simple_jump_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x20) == 0 {
        if rd::<u32>(this, 0x28) != 0 {
            return rd::<u8>(this, 0x20);
        }
        if start_launch_anim_c(this, ped) == 0 {
            wr::<u8>(this, 0x23, 0);
            return 1;
        }
        wr::<u8>(this, 0x23, 1);
        return rd::<u8>(this, 0x20);
    }
    if (ped_is_player(ped) != 0 || rd::<u8>(this, 0x24) != 0) && g::<u32>(0xB7_2914).read() == 0 {
        let e = climb_test_for_climb(ped, this.byte_add(8), this.byte_add(0x14), this.byte_add(0x18), 1);
        wr::<GPtr<u8>>(this, 0x1C, e);
    }
    if rd::<u32>(this, 0x1C) == 0 {
        check_if_jump_blocked(this, ped);
    }
    if rd::<u8>(this, 0x21) == 0 {
        launch(this, ped);
        return 1;
    }
    if ped_is_player(ped) != 0 {
        let mut pos = [0u32; 3];
        let mut ev = [0u32; 16];
        let evp: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
        event_sound_quiet_ctor(evp, ped, 45.0, 0xFFFF_FFFF, GPtr::from_raw(pos.as_mut_ptr()).cast());
        event_group_add(get_event_global_group(), evp, 0);
        event_sound_quiet_dtor(evp);
    }
    1
}

/// The ped's position: `m_matrix` +0x30, else the placement position +4.
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// Sets every collision sphere radius (+0xC of the 0x14-byte spheres at `colData +8`, count word +0).
fn set_sphere_radii(col: GPtr<u8>, r: u32) {
    for i in 0..i32::from(rd::<i16>(col, 0)) {
        wr::<u32>(rd::<GPtr<u8>>(col, 8), (i * 0x14 + 0xC) as isize, r);
    }
}

/// `bool CTaskSimpleJump::CheckIfJumpBlocked(CPed* ped)` (0x67D590, `ret 4`).
///
/// False for a stuck ped (+0x1C bit 4) or while the current event is 0x5D. +0x22 = a sphere (radius 0.25) at
/// `pos + forward / 4` raised by collision sphere 2's centre z + 0.25 hits the world (buildings, vehicles,
/// objects). The ped is moved to `matrix * (0, 0, 0.75)`, its collision spheres shrunk to 0.3, and +0x21 set when
/// `TestCollision(false)` hits; then position and radii (0.35) are restored. Returns +0x21.
#[hook(addr = 0x67D590, conv = thiscall, name = "CTaskSimpleJump::CheckIfJumpBlocked",
       expect = "83 EC 24 53 55 56 57",
       unverified = "native-27", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "moves the ped and its model's collision spheres temporarily")]
pub fn check_if_jump_blocked_port(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if (rd::<u32>(ped, 0x1C) >> 4) & 1 != 0 {
        return 0;
    }
    if event_handler_current_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x34)) == 0x5D {
        return 0;
    }
    let info = g::<GPtr<u8>>(0xA9_B0C8).wrapping_index(i32::from(rd::<i16>(ped, 0x22))).read();
    let col = rd::<GPtr<u8>>(rd::<GPtr<u8>>(info, 0x14), 0x2C);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let q = 0.25f32;
    let fx = x(rd::<f32>(m, 0x10)).fmul_m(q);
    let fy = x(rd::<f32>(m, 0x14)).fmul_m(q);
    let fz = x(rd::<f32>(m, 0x18)).fmul_m(q).to_f32();
    let p = pos_ptr(ped);
    let px = fx.fadd_m(rd::<f32>(p, 0)).to_f32();
    let py = fy.fadd_m(rd::<f32>(p, 4)).to_f32();
    let pz = x(fz)
        .fadd_m(rd::<f32>(p, 8))
        .fadd_m(rd::<f32>(rd::<GPtr<u8>>(col, 8), 0x30))
        .fadd_m(q)
        .to_f32();
    let hit = world_test_sphere(px.to_bits(), py.to_bits(), pz.to_bits(), q, 0, 1, 1, 0, 1, 0, 0);
    wr::<u8>(this, 0x22, u8::from(!hit.is_null()));
    let p = pos_ptr(ped);
    let saved = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    let mut v = [0f32, 0.0, 0.75];
    let mut out = [0u32; 3];
    let outp: GPtr<u8> = GPtr::from_raw(out.as_mut_ptr()).cast();
    matrix_mul_vec(outp, rd::<GPtr<u8>>(ped, 0x14), GPtr::from_raw(v.as_mut_ptr()).cast());
    let set_pos = |v: [u32; 3]| {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let d = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        for (k, w) in v.iter().enumerate() {
            wr::<u32>(d, 4 * k as isize, *w);
        }
    };
    set_pos(out);
    set_sphere_radii(col, 0x3E99_999A);
    if entity_test_collision(ped, 0) != 0 {
        wr::<u8>(this, 0x21, 1);
    }
    set_pos(saved);
    set_sphere_radii(col, 0x3EB3_3333);
    rd::<u8>(this, 0x21)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_START_LAUNCH_ANIM, HOOK_SIMPLE_JUMP_PROCESS_PED, HOOK_CHECK_IF_JUMP_BLOCKED_PORT];
