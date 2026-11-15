//! FRONTIER-2 turn 3, lane B (batch `frontier2-b10`): RENDER functions on the phase B frontier. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{ge_or_nan, gt, le_or_nan};

use crate::common::{f32_of, g, rd, wr, x};
use crate::glass::report_glass_collision_event;
use crate::glass_break_physically::generate_panes_for_window;
use sa_core::bindings::matrix_mul_vec;
use sa_stream::bindings::{placeable_allocate_matrix, simple_transform_update_matrix};
use sa_world::bindings::uncompress_vector;
use sa_world::world::common::entity_pos;

/// The entity's matrix, allocated from its simple transform first when it has none (`GetMatrix`).
fn matrix_of(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `CGlass::WindowRespondsToCollision(CEntity*, float damage, CVector moveSpeed, CVector point, bool max1)` (0x71BC40,
/// cdecl). Nothing once the glass is broken altogether (+0x140 bit 5); else +0x140 bit 4 (broken). For a collision
/// model of exactly two triangles: its first four (compressed) vertices; minZ / maxZ of their z (pairwise, the x87
/// compares' NaN choices kept); the vertex farthest (2D, strictly greater, from 1) from vertex 0; vertex 0 and that
/// vertex at minZ through the matrix (allocated when missing); the glass-break audio event 0x7E at the position; the
/// panes (type 1 up to damage 300 or NaN, else 0) from the first point, forward (0, 0, maxZ - minZ), right (second -
/// first), the move speed, the point as centre, drag 0.1, shatter `(+0x140 >> 4) & 0xFFFFFF01`, `max1`, one section.
/// Always: no collision, not visible (+0x1C `&= !0x81`), broken altogether (+0x140 bit 5).
#[hook(addr = 0x71BC40, conv = cdecl, name = "CGlass::WindowRespondsToCollision", expect = "83 EC 68 56 8B 74 24 70",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original keeps minZ in its own (caller-owned) entity argument slot (not modelled); generates panes")]
#[allow(clippy::too_many_arguments)]
pub fn glass_window_responds_to_collision(e: GPtr<u8>, damage: f32, sx: f32, sy: f32, sz: f32, px: f32, py: f32, pz: f32, max1: u32) {
    let flags = rd::<u32>(e, 0x140);
    if flags & 0x20 != 0 {
        return;
    }
    wr::<u32>(e, 0x140, flags | 0x10);
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(e, 0x22) as usize).read();
    let data = rd::<GPtr<u8>>(rd::<GPtr<u8>>(mi, 0x14), 0x2C);
    if !data.is_null() && rd::<i16>(data, 4) == 2 {
        let cv = rd::<GPtr<u8>>(data, 0x14);
        let mut v = [[0f32; 3]; 4];
        for (k, vk) in v.iter_mut().enumerate() {
            uncompress_vector(GPtr::from_raw(vk.as_mut_ptr()).cast(), cv.byte_add(6 * k as isize));
        }
        let lo = |a: f32, b: f32| if ge_or_nan(x(a), x(b)) { b } else { a };
        let m = lo(v[2][2], v[3][2]);
        let a = lo(v[0][2], v[1][2]);
        let min_z = if ge_or_nan(x(a), x(m)) { m } else { a };
        let hi = |a: f32, b: f32| if le_or_nan(x(a), x(b)) { b } else { a };
        let m2 = hi(v[2][2], v[3][2]);
        let a2 = hi(v[0][2], v[1][2]);
        let max_z = if le_or_nan(x(a2), x(m2)) { m2 } else { a2 };
        let dist = |k: usize| {
            let dx = x(v[k][0]).fsub_m(v[0][0]);
            let dy = x(v[k][1]).fsub_m(v[0][1]);
            dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
        };
        let mut far = 1;
        let mut best = f32_of(dist(1));
        for k in 2..4 {
            let d = dist(k);
            if gt(d, x(best)) {
                best = f32_of(d);
                far = k;
            }
        }
        let mut p0 = [0f32; 3];
        let mut p1 = [0f32; 3];
        let mut q = [v[0][0], v[0][1], min_z];
        let m = matrix_of(e);
        matrix_mul_vec(GPtr::from_raw(p0.as_mut_ptr()).cast(), m, GPtr::from_raw(q.as_mut_ptr()).cast());
        q = [v[far][0], v[far][1], min_z];
        let m = matrix_of(e);
        matrix_mul_vec(GPtr::from_raw(p1.as_mut_ptr()).cast(), m, GPtr::from_raw(q.as_mut_ptr()).cast());
        let ty = if le_or_nan(x(damage), x(300.0)) { 1 } else { 0 };
        report_glass_collision_event(GPtr::from_addr(0xB6_BC90), 0x7E, entity_pos(e).cast());
        let rx = f32_of(x(p1[0]).fsub_m(p0[0]));
        let ry = f32_of(x(p1[1]).fsub_m(p0[1]));
        let rz = f32_of(x(p1[2]).fsub_m(p0[2]));
        let fz = f32_of(x(max_z).fsub_m(min_z));
        let shatter = (rd::<u32>(e, 0x140) >> 4) & 0xFFFF_FF01;
        generate_panes_for_window(ty, p0[0], p0[1], p0[2], 0.0, 0.0, fz, rx, ry, rz, sx, sy, sz, px, py, pz,
                                  f32::from_bits(0x3DCC_CCCD), shatter, max1, 1, 0);
    }
    wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x20);
    wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_FF7E);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GLASS_WINDOW_RESPONDS_TO_COLLISION];
