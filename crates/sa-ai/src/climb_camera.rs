//! WRITER-CLIMB (batch `climb-4`): the climb task's camera hooks, `CTaskSimpleClimb::GetCameraTargetPos`
//! (0x67A390) and `GetCameraStickModifier` (0x67A5D0). Ported from the 1.0 US bytes; gta-reversed
//! (`TaskSimpleClimb.cpp`) as a reading aid (its stick modifier reads the climb entity where the original reads the
//! entity argument). x87 values stay in registers where the original keeps them there.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7)` (0x5674E0)
    #[allow(clippy::too_many_arguments)]
    fn world_process_vertical_line(o: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, ob: u32, d: u32, see: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

fn c(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn buf(v: &mut [u32]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `ms_fHangingOffsetHorz/Vert` (0x8D2F1C / 0x8D2F20), `ms_fAtEdgeOffsetHorz/Vert` (0x8D2F24 / 0xC18F78),
/// `ms_fStandUpOffsetHorz/Vert` (0x8D2F28 / 0x8D2F2C), `ms_fVaultOffsetHorz/Vert` (0x8D2F30 / 0xC18F7C)
const HANG: (usize, usize) = (0x8D_2F1C, 0x8D_2F20);
const EDGE: (usize, usize) = (0x8D_2F24, 0xC1_8F78);
const STAND: (usize, usize) = (0x8D_2F28, 0x8D_2F2C);
const VAULT: (usize, usize) = (0x8D_2F30, 0xC1_8F7C);

/// `from * (1 - p) + to * p` per axis, as the original's register order (`to * p` loaded first).
fn lerp(from: (usize, usize), to: (usize, usize), p: f32) -> (f32, f32) {
    let th = x(c(to.0)).fmul_m(p);
    let tv = x(c(to.1)).fmul_m(p);
    let q = x(c(0x85_8624)).fsub_m(p);
    (x(c(from.0)).fmul(q).fadd(th).to_f32(), q.fmul_m(c(from.1)).fadd(tv).to_f32())
}

/// The handhold in world space with its heading kept in ST0: a physical entity's matrix (allocated when missing)
/// moves the position and adds `atan2(-fwd.x, fwd.y)` (else nothing changes).
fn handhold(this: GPtr<u8>) -> ([u32; 3], X87) {
    let mut pos = [rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20)];
    let stored = rd::<f32>(this, 0x14);
    let e = rd::<GPtr<u8>>(this, 0x24);
    let ty = rd::<u8>(e, 0x36) & 7;
    if ty <= 1 || ty >= 5 {
        return (pos, x(stored));
    }
    if rd::<u32>(e, 0x14) == 0 {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    let mut out = [0u32; 3];
    let r = matrix_mul_vec(buf(&mut out), rd::<GPtr<u8>>(e, 0x14), buf(&mut pos));
    pos = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    let e = rd::<GPtr<u8>>(this, 0x24);
    let m = rd::<GPtr<u8>>(e, 0x14);
    let h = if m.is_null() { x(rd::<f32>(e, 0x10)) } else { x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))) };
    (pos, h.fadd_m(stored))
}

/// `void CTaskSimpleClimb::GetCameraTargetPos(CPed* ped, CVector& out)` (0x67A390, `ret 8`).
///
/// `out` = the ped's position. With an anim (+0x2C) whose id is 0x79 / 0x86 (vault offsets), 0x82 (hanging to edge
/// by the anim progress `time / total`), 0x83 (edge to stand-up), 0x84 (stand-up) or 0x85 (edge to vault): `out` =
/// the handhold plus `(-sin(h) horz, cos(h) horz, vert)`, the heading unrounded.
#[hook(addr = 0x67A390, conv = thiscall, name = "CTaskSimpleClimb::GetCameraTargetPos",
       expect = "83 EC 24 53 56 8B F1 8B",
       unverified = "climb-4", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "fsin / fcos / fpatan on an unrounded heading")]
pub fn climb_get_camera_target_pos(this: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    for k in 0..3 {
        wr::<u32>(out, 4 * k, rd::<u32>(p, 4 * k));
    }
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if a.is_null() {
        return;
    }
    let prog = x(rd::<f32>(a, 0x20)).fdiv_m(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)).to_f32();
    let (horz, vert) = match i32::from(rd::<i16>(a, 0x2C)) {
        0x79 | 0x86 => (c(VAULT.0), c(VAULT.1)),
        0x82 => lerp(HANG, EDGE, prog),
        0x83 => lerp(EDGE, STAND, prog),
        0x84 => (c(STAND.0), c(STAND.1)),
        0x85 => lerp(EDGE, VAULT, prog),
        _ => return,
    };
    let (pos, h) = handhold(this);
    for (k, w) in pos.iter().enumerate() {
        wr::<u32>(out, 4 * k as isize, *w);
    }
    wr::<f32>(out, 0, x(rd::<f32>(out, 0)).fsub(h.fsin().fmul_m(horz)).to_f32());
    wr::<f32>(out, 4, h.fcos().fmul_m(horz).fadd_m(rd::<f32>(out, 4)).to_f32());
    wr::<f32>(out, 8, x(vert).fadd_m(rd::<f32>(out, 8)).to_f32());
}

/// `void CTaskSimpleClimb::GetCameraStickModifier(CEntity* entity, float& alpha, float& beta, float& stickAlpha,
/// float& stickBeta)` (0x67A5D0, `ret 0x14`).
///
/// `CLIMB_JUMP_B` (0x85): the fall height +0x10 (signed byte) is measured once (while negative): a vertical line
/// from the vault point (handhold + vault offset) 3 down; a hit gives `ftol(max(z - hit.z - 1, 0) * 10)`, else 20.
/// Then with `d` = -0.6 above 8, else -0.1: `alpha > d` sets `stickAlpha = max((alpha - d) * -0.05, -0.05)`.
/// `CLIMB_IDLE` (0x81): the ENTITY ARGUMENT's heading minus pi/2, wrapped to within pi of `beta`, steers the stick
/// beta: 0.02 beyond `beta + pi/2 + 0.2`, 0 (from negative) beyond `beta + pi/2`, -0.02 below `beta - pi/2 - 0.2`,
/// 0 (from positive) below `beta - pi/2`.
#[hook(addr = 0x67A5D0, conv = thiscall, name = "CTaskSimpleClimb::GetCameraStickModifier",
       expect = "83 EC 44 56 8B F1 8B 46",
       unverified = "climb-4", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "fsin / fcos / fpatan; the idle branch reads the entity argument")]
pub fn climb_get_camera_stick_modifier(this: GPtr<u8>, entity: GPtr<u8>, alpha: GPtr<u8>, beta: GPtr<u8>, stick_alpha: GPtr<u8>, stick_beta: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(this, 0x2C);
    if a.is_null() {
        return;
    }
    let id = rd::<u16>(a, 0x2C);
    if id == 0x85 {
        if rd::<i8>(this, 0x10) < 0 {
            let (mut pos, h) = handhold_for_vault(this);
            let horz = c(VAULT.0);
            pos[0] = x(f32::from_bits(pos[0])).fsub(h.fsin().fmul_m(horz)).to_f32().to_bits();
            pos[1] = h.fcos().fmul_m(horz).fadd_m(f32::from_bits(pos[1])).to_f32().to_bits();
            pos[2] = x(c(VAULT.1)).fadd_m(f32::from_bits(pos[2])).to_f32().to_bits();
            let zmin = x(f32::from_bits(pos[2])).fsub_m(c(0x85_8B3C)).to_f32();
            let mut cp = [0u32; 11];
            let mut hit_entity = [0u32; 1];
            let hit = world_process_vertical_line(buf(&mut pos), zmin, buf(&mut cp), buf(&mut hit_entity), 1, 1, 0, 1, 0, 0, 0);
            if hit != 0 {
                let mut v = x(f32::from_bits(pos[2])).fsub_m(f32::from_bits(cp[2])).fsub_m(c(0x85_8624));
                let zero = x(c(0x85_8B50));
                if !zero.fcom(v).test_any(0x41) {
                    v = zero;
                }
                wr::<u8>(this, 0x10, ftol32(v.fmul_m(c(0x85_862C))) as u8);
            } else {
                wr::<u8>(this, 0x10, 0x14);
            }
        }
        let d = if rd::<i8>(this, 0x10) > 8 { x(c(0x87_03C8)) } else { x(c(0x87_03C4)) };
        let al = rd::<f32>(alpha, 0);
        if d.fcom(x(al)).test_parity(0x05) {
            return;
        }
        let mut v = x(al).fsub(d).fmul_m(c(0x87_03CC));
        let lim = x(c(0x87_03D0));
        if !lim.fcom(v).test_any(0x41) {
            v = lim;
        }
        wr::<f32>(stick_alpha, 0, v.to_f32());
        return;
    }
    if id != 0x81 {
        return;
    }
    let m = rd::<GPtr<u8>>(entity, 0x14);
    let h0 = if m.is_null() { x(rd::<f32>(entity, 0x10)) } else { x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))) };
    let half_pi = c(0x85_8FE4);
    let pi = c(0x85_8CB8);
    let mut h = h0.fsub_m(half_pi);
    let b = || x(rd::<f32>(beta, 0));
    if !h.fcom(b().fadd_m(pi)).test_any(0x41) {
        h = h.fsub_m(c(0x85_8CBC));
    } else if !h.fcom(b().fsub_m(pi)).test_parity(0x05) {
        h = h.fadd_m(c(0x85_8CBC));
    }
    if !h.fcom(b().fadd_m(c(0x87_05C0))).test_any(0x41) {
        wr::<f32>(stick_beta, 0, c(0x87_03D4));
        return;
    }
    let zero = c(0x85_8B50);
    if !h.fcom(b().fadd_m(half_pi)).test_any(0x41) && !x(rd::<f32>(stick_beta, 0)).fcom(x(zero)).test_parity(0x05) {
        wr::<u32>(stick_beta, 0, 0);
        return;
    }
    let t = b().fsub_m(half_pi);
    if !h.fcom(t.fsub(x(c(0x85_8CC4)))).test_parity(0x05) {
        wr::<f32>(stick_beta, 0, x(c(0x87_03D4)).fchs().to_f32());
        return;
    }
    if h.fcom(t).test_parity(0x05) {
        return;
    }
    if x(rd::<f32>(stick_beta, 0)).fcom(x(zero)).test_any(0x41) {
        return;
    }
    wr::<u32>(stick_beta, 0, 0);
}

/// The stick modifier's handhold: like [`handhold`] but through `GetMatrix()` (no explicit allocation), the
/// heading added to the stored one rounded to f32 (the original stores it in an argument slot first).
fn handhold_for_vault(this: GPtr<u8>) -> ([u32; 3], X87) {
    let mut pos = [rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20)];
    let stored = rd::<f32>(this, 0x14);
    let e = rd::<GPtr<u8>>(this, 0x24);
    let ty = rd::<u8>(e, 0x36) & 7;
    if ty <= 1 || ty >= 5 {
        return (pos, x(stored));
    }
    let gm = placeable_get_matrix(e);
    let mut out = [0u32; 3];
    let r = matrix_mul_vec(buf(&mut out), gm, buf(&mut pos));
    pos = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    let e = rd::<GPtr<u8>>(this, 0x24);
    let m = rd::<GPtr<u8>>(e, 0x14);
    let h = if m.is_null() { x(rd::<f32>(e, 0x10)) } else { x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))) };
    (pos, h.fadd_m(stored))
}

pub const HOOKS: &[HookReg] = &[HOOK_CLIMB_GET_CAMERA_TARGET_POS, HOOK_CLIMB_GET_CAMERA_STICK_MODIFIER];
