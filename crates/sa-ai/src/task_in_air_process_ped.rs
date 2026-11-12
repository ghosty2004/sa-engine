//! `CTaskSimpleInAir::ProcessPed` (0x680600, batch `std11-ai-1`): the ped falling / jumping (stepping off a kerb).
//! Ported from the 1.0 US bytes, including the part behind the protection chunk 0x404880 (`mov eax, [edi+0x530]`, back
//! to 0x680A9C); gta-reversed's version is a reading aid. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use crate::common::*;

orig! {
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: GPtr<u8>, anim: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn blend_animation(clump: GPtr<u8>, group: u32, anim: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetDeleteCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBC0)
    fn set_delete_callback(a: GPtr<u8>, f: u32, data: u32) = thiscall 0x4CEBC0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0)
    fn set_finish_callback(a: GPtr<u8>, f: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::Say(uint16 id, uint32 offset, float volume, uint8, uint8, uint8)` (0x5EFFE0)
    fn ped_say(p: GPtr<u8>, id: u32, offset: u32, volume: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x5EFFE0 => unported;
    /// `CPhysical::ApplyMoveForce(float, float, float)` (0x4ABBA0)
    fn apply_move_force_xyz(p: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x4ABBA0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, result through the first argument)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(p: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float z, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTaskSimpleClimb::TestForClimb(CPed*, CVector&, float&, uint8&, bool)` (0x6803A0)
    fn climb_test_for_climb(ped: GPtr<u8>, pos: GPtr<u8>, heading: GPtr<u8>, surf: GPtr<u8>, launch: u32) -> GPtr<u8> = cdecl 0x6803A0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `CTaskSimpleInAir::DeleteAnimCB`
const DELETE_ANIM_CB: u32 = 0x678E60;
/// `CDefaultAnimCallback::DefaultAnimCB`
const DEFAULT_ANIM_CB: u32 = 0x4D1390;

fn kf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// `fcomp; fnstsw; test ah, 0x41; jp skip`: true when v <= c (ordered).
fn le(v: X87, c: X87) -> bool {
    !v.fcom(c).test_parity(0x41)
}

/// `fcomp; fnstsw; test ah, 5; jp skip`: true when v < c (ordered).
fn lt(v: X87, c: X87) -> bool {
    !v.fcom(c).test_parity(0x05)
}

/// `CTaskSimpleInAir::ProcessPed(CPed*)` (0x680600, thiscall, `ret 4`).
///
/// Task fields: +8 climb position, +0x14 climb angle, +0x18 surface, +0x1C anim, +0x20 minimum z speed, byte +0x24
/// flags (1 jump glide, 2 fall glide), +0x28 process counter, +0x2C timer (start, interval +0x30, started +0x34,
/// changed +0x35), +0x38 climb entity. Ped: matrix +0x14, clump +0x18, move speed +0x44, mass +0x8C, flags +0x46C
/// (1 standing, 0x200 in the air), state +0x530, attached-to +0xFC.
///
/// Stack: the original overwrites its own `ped` argument slot twice (the forward-speed limit `f32(0.1 * 0.5)` and
/// `&climbEntity`), and reads the uninitialised colpoint z when the ped is already standing (the outcome does not
/// depend on it: standing returns true either way). Returns AL; EAX's upper bytes (status word / callee residue) are
/// not reproduced.
#[hook(addr = 0x680600, conv = thiscall, name = "CTaskSimpleInAir::ProcessPed", expect = "83 EC 4C 53 55 56 57 8B 7C 24 60",
       unverified = "std11-ai-1", route = "L2", hazards = "ax-al-return,stack-residue,uninit-locals", recs = "grove-walk")]
pub fn task_simple_in_air_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let mat = rd::<GPtr<u8>>(ped, 0x14);
    let mut p = [rd::<u32>(mat, 0x30), f32_of(x(rd::<f32>(mat, 0x34))).to_bits(), f32_of(x(rd::<f32>(mat, 0x38))).to_bits()];
    let col_dist = f32_of(x(rd::<f32>(mat, 0x38)).fsub_m(g::<f32>(0x858B90).read()));
    let speed_z = || x(rd::<f32>(ped, 0x4C));

    // the timer: out of time -> the high-fall pain line
    if rd::<u8>(this, 0x34) != 0 {
        if rd::<u8>(this, 0x35) != 0 {
            wr::<u32>(this, 0x2C, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x35, 0);
        }
        let end = rd::<u32>(this, 0x30).wrapping_add(rd::<u32>(this, 0x2C));
        if g::<u32>(TIME_MS).read() >= end
            && rd::<u32>(ped, 0x530) != 0x27
            && le(speed_z().fmul_m(g::<f32>(0x858B40).read()), kf(0x870644))
        {
            let id = if ped_is_player(ped) != 0 { 0x166 } else { 0x15A };
            ped_say(ped, id, 0, 0x3F80_0000, 0, 0, 0);
            wr::<u8>(this, 0x34, 0);
        }
    }

    let anim = || rd::<GPtr<u8>>(this, 0x1C);
    let clump = || rd::<GPtr<u8>>(ped, 0x18);
    // blend amount below 1 and blend delta not above 0
    let fading = |a: GPtr<u8>| lt(x(rd::<f32>(a, 0x18)), kf(0x858624)) && le(x(rd::<f32>(a, 0x1C)), kf(0x858B50));
    if anim().is_null() {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x200);
        let fl = rd::<u8>(this, 0x24);
        if fl & 1 != 0 {
            let mut a = clump_get_association(clump(), 0x76);
            wr::<GPtr<u8>>(this, 0x1C, a);
            if a.is_null() {
                a = clump_get_association(clump(), 0x80);
                wr::<GPtr<u8>>(this, 0x1C, a);
            }
            if a.is_null() || fading(a) {
                blend_animation(clump(), 0, 0x76, 4.0);
            }
        } else if fl & 2 != 0 {
            if le(speed_z(), kf(0x8703A8)) {
                let a = blend_animation(clump(), 0, 0x79, 4.0);
                wr::<GPtr<u8>>(this, 0x1C, a);
                set_delete_callback(a, DELETE_ANIM_CB, this.addr() as u32);
                wr::<u32>(anim(), 0x1C, 0x3F00_0000);
                wr::<u32>(anim(), 0x18, 0x3F80_0000);
            }
        } else {
            if anim().is_null() {
                wr::<GPtr<u8>>(this, 0x1C, clump_get_association(clump(), 0x80));
            }
            let a = anim();
            if a.is_null() || fading(a) {
                wr::<GPtr<u8>>(this, 0x1C, blend_animation(clump(), 0, 0x79, 4.0));
            }
        }
        if !anim().is_null() {
            set_delete_callback(anim(), DELETE_ANIM_CB, this.addr() as u32);
        }
    }

    let a = anim();
    let climb_jump_rising = !a.is_null()
        && rd::<u16>(a, 0x2C) == 0x80
        && rd::<u32>(ped, 0x46C) & 1 == 0
        && !x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)).fcom(x(rd::<f32>(a, 0x20))).test_any(0x41)
        && (!x(rd::<f32>(a, 0x18)).fcom(kf(0x858624)).test_any(0x01)
            || !x(rd::<f32>(a, 0x1C)).fcom(kf(0x858B50)).test_any(0x41));
    if climb_jump_rising {
        let f = f32_of(
            kf(TIME_STEP)
                .fmul_m(rd::<f32>(ped, 0x8C))
                .fmul_m(g::<f32>(0x8703AC).read())
                .fmul_m(g::<f32>(0x863984).read()),
        );
        apply_move_force_xyz(ped, 0.0, 0.0, f);
    } else {
        // `m_vecMoveSpeed.z > 0 && !IsZero()` (unordered counts as above / non-zero) or, without the fall glide, dying
        let zero = kf(0x858B50);
        let nonzero = |off: isize| x(rd::<f32>(ped, off)).fcom(zero).test_parity(0x44);
        let rising = !le(speed_z(), zero) && (nonzero(0x44) || nonzero(0x48) || nonzero(0x4C));
        let fall_glide = rd::<u8>(this, 0x24) & 2 != 0;
        let state = rd::<u32>(ped, 0x530);
        if rising || (!fall_glide && (state == 0x36 || state == 0x37)) {
            if !fall_glide {
                jump_forward_push(this, ped);
            }
        } else if falling(this, ped, &mut p, col_dist) {
            return 1;
        }
    }
    climb_test(this, ped);
    0
}

/// The rising part of a `TASK_COMPLEX_JUMP` (0x6808E3): the forward push while the forward speed is below
/// `0.1 * 0.5` and the counter below 1000.
fn jump_forward_push(this: GPtr<u8>, ped: GPtr<u8>) {
    let parent = rd::<GPtr<u8>>(this, 4);
    if parent.is_null() {
        return;
    }
    let pp = rd::<GPtr<u8>>(parent, 4);
    if pp.is_null() || task_get_type(pp) != 0xD3 {
        return;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let fwd = m.byte_add(0x10);
    let dot = f32_of(
        x(rd::<f32>(ped, 0x4C))
            .fmul_m(rd::<f32>(fwd, 8))
            .fadd(x(rd::<f32>(ped, 0x48)).fmul_m(rd::<f32>(fwd, 4)))
            .fadd(x(rd::<f32>(ped, 0x44)).fmul_m(rd::<f32>(fwd, 0))),
    );
    let max = f32_of(kf(0x8703B0).fmul_m(g::<f32>(0x8703A4).read()));
    if !lt(x(dot), x(max)) || rd::<i32>(this, 0x28) >= 1000 {
        return;
    }
    let s = f32_of(x(max).fsub_m(dot).fmul_m(rd::<f32>(ped, 0x8C)));
    let mut out = [0f32; 3];
    let r = vector_mul_scalar(GPtr::from_raw(out.as_mut_ptr()).cast(), fwd, s);
    apply_move_force(ped, CVector { x: rd::<f32>(r, 0), y: rd::<f32>(r, 4), z: rd::<f32>(r, 8) });
    let ms = ftol32(kf(TIME_STEP).fmul_m(g::<f32>(0x858B38).read()).fmul_m(g::<f32>(0x858C4C).read()));
    wr::<i32>(this, 0x28, rd::<i32>(this, 0x28).wrapping_add(ms));
}

/// The falling part (0x6809AF). True when the fall ends (the task finishes: anim faded out with the fall glide, the
/// in-the-air flag cleared).
fn falling(this: GPtr<u8>, ped: GPtr<u8>, p: &mut [u32; 3], col_dist: f32) -> bool {
    let speed_z = || x(rd::<f32>(ped, 0x4C));
    if lt(speed_z(), x(rd::<f32>(this, 0x20))) {
        wr::<u32>(this, 0x20, rd::<u32>(ped, 0x4C));
    }
    // the colpoint (0x2C bytes) and entity are stack locals of the original, uninitialised
    let mut cp = [0u32; 11];
    let mut ent = 0u32;
    let mut stop = rd::<u8>(ped, 0x46C) & 1 != 0;
    if !stop {
        stop = world_process_vertical_line(
            GPtr::from_raw(p.as_mut_ptr()).cast(),
            col_dist.to_bits(),
            GPtr::from_raw(cp.as_mut_ptr()).cast(),
            GPtr::from_raw(&mut ent as *mut u32).cast(),
            1,
            1,
            0,
            1,
            0,
            0,
            0,
        ) != 0;
    }
    let mut ended = false;
    if rd::<u8>(this, 0x24) & 2 != 0 {
        if le(speed_z(), kf(0x858B50)) && !speed_z().fcom(kf(0x8D2EFC)).test_any(0x41) {
            let n = rd::<i32>(this, 0x28).wrapping_add(1);
            wr::<i32>(this, 0x28, n);
            if n > g::<i32>(0x8D2EF8).read() {
                wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
                ended = true;
            }
        } else {
            wr::<i32>(this, 0x28, 0);
        }
    }
    if ended || stop {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let near = lt(x(rd::<f32>(m, 0x38)).fsub_m(f32::from_bits(cp[2])), kf(0x859918));
        if near || rd::<u32>(ped, 0x46C) & 1 != 0 || rd::<u32>(ped, 0xFC) != 0 {
            let a = rd::<GPtr<u8>>(this, 0x1C);
            if !a.is_null() && rd::<u8>(this, 0x24) & 2 != 0 {
                wr::<u32>(a, 0x1C, 0xC47A_0000);
                wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
                set_finish_callback(rd::<GPtr<u8>>(this, 0x1C), DEFAULT_ANIM_CB, 0);
                wr::<u32>(this, 0x1C, 0);
            }
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
            return true;
        }
        return false;
    }
    if rd::<u32>(ped, 0x530) == 0x27 || !lt(speed_z(), kf(0x858EF4)) {
        return false;
    }
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if a.is_null() || rd::<u16>(a, 0x2C) == 0x78 {
        return false;
    }
    set_delete_callback(a, DEFAULT_ANIM_CB, 0);
    let n = blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, 0x78, 4.0);
    wr::<GPtr<u8>>(this, 0x1C, n);
    set_delete_callback(n, DELETE_ANIM_CB, this.addr() as u32);
    false
}

/// The jump glide's climb test (0x680A7A, through the protection chunk 0x404880).
fn climb_test(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x24) & 1 == 0 || rd::<u32>(this, 0x38) != 0 {
        return;
    }
    let state = rd::<u32>(ped, 0x530);
    if state == 0x36 || state == 0x37 || ped_is_player(ped) == 0 || g::<u32>(0xB72914).read() != 0 {
        return;
    }
    if x(rd::<f32>(ped, 0x4C)).fcom(kf(0x858FDC)).test_any(0x41) {
        return;
    }
    let (sx, sy) = (x(rd::<f32>(ped, 0x44)), x(rd::<f32>(ped, 0x48)));
    let mag = sx.fmul(sx).fadd(sy.fmul(sy)).fsqrt();
    if mag.fcom(kf(0x858C28)).test_any(0x41) {
        return;
    }
    let mut land = GPtr::<u8>::NULL;
    let parent = rd::<GPtr<u8>>(this, 4);
    if !parent.is_null() && task_get_type(parent) == 0xF0 {
        land = rd::<GPtr<u8>>(this, 4);
        if !land.is_null() && rd::<u8>(land, 0xE) != 0 {
            return;
        }
    }
    let e = climb_test_for_climb(ped, this.byte_add(8), this.byte_add(0x14), this.byte_add(0x18), 0);
    wr::<GPtr<u8>>(this, 0x38, e);
    if !e.is_null() {
        entity_register_reference(e, this.byte_add(0x38));
        return;
    }
    if lt(x(rd::<f32>(this, 0x14)), kf(0x859948)) && !land.is_null() {
        wr::<u8>(land, 0xE, 1);
    }
}

/// `CTaskSimpleInAir::DeleteAnimCB(CAnimBlendAssociation* anim, void* task)` (0x678E60, cdecl): with both set and the
/// task's anim (+0x1C) being `anim`, the task's anim cleared.
#[hook(addr = 0x678E60, conv = cdecl, name = "CTaskSimpleInAir::DeleteAnimCB", expect = "8B 44 24 08 85 C0 74 14",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_in_air_delete_anim_cb(anim: GPtr<u8>, task: GPtr<u8>) {
    if !task.is_null() && !anim.is_null() && rd::<GPtr<u8>>(task, 0x1C) == anim {
        wr::<u32>(task, 0x1C, 0);
    }
}

/// 0x678E80 (thiscall, `ret 4`): the in-air task's empty roll / pitch step `CPed::PreRenderAfterTest` calls (a lone
/// `ret 4`, too short for a hook: registered for phase B only).
#[sa_core::native(addr = 0x678E80, conv = thiscall, name = "FUN_00678e80")]
pub fn task_simple_in_air_678e80(_this: GPtr<u8>, _ped: GPtr<u8>) {}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_TASK_SIMPLE_IN_AIR_PROCESS_PED, HOOK_TASK_SIMPLE_IN_AIR_DELETE_ANIM_CB];
