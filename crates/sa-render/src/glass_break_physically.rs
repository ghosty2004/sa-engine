//! `CGlass::BreakGlassPhysically(CVector point, float radius)` (0x71CF50, batch `samp2-render-1`). Ported
//! from the 1.0 US bytes (gta-reversed returns after the first hit on unbroken glass; the original keeps
//! scanning). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};
use crate::glass::{mi_as_atomic_model_info_ptr, report_glass_collision_event};

use sa_core::bindings::matrix_mul_vec;
use sa_world::bindings::calculate_triangle_planes;
use sa_world::bindings::test_sphere_triangle;
use sa_world::bindings::uncompress_vector;
orig! {
    /// `CGlass::GeneratePanesForWindow(ePaneType, CVector point, CVector fwd, CVector right, CVector velocity,
    /// CVector center, float dragCoeff, bool shatter, bool max1, int32 sections, bool)` (0x71B620)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn generate_panes_for_window(ty: u32, px: f32, py: f32, pz: f32, fx: f32, fy: f32, fz: f32, rx: f32, ry: f32, rz: f32, vx: f32, vy: f32, vz: f32, cx: f32, cy: f32, cz: f32, drag: f32, shatter: u32, max1: u32, sections: i32, b: u32) = cdecl 0x71B620 => unported;
}

/// `CGlass::LastColCheckMS` (0xC72FA8), `CPools::ms_pObjectPool` (0xB7449C), `ms_modelInfoPtrs` (0xA9B0C8).
const LAST_COL_CHECK: usize = 0xC7_2FA8;
const OBJECT_POOL: usize = 0xB7_449C;
const MODEL_INFOS: usize = 0xA9_B0C8;

use sa_world::world::common::entity_pos as pos;

fn mi_of(e: GPtr<u8>) -> GPtr<u8> {
    let idx = i32::from(rd::<i16>(e, 0x22));
    g::<GPtr<u8>>((MODEL_INFOS as u32).wrapping_add((idx * 4) as u32) as usize).read()
}

use sa_world::world::misc2::below as lt;
use sa_world::world::misc2::above as gt;
/// `CGlass::BreakGlassPhysically(CVector point, float radius)` (0x71CF50, cdecl).
///
/// Nothing within 1000 ms after `LastColCheckMS` (`now < last + 1000 && now >= last`, unsigned). Every
/// object of the pool, from the last slot down: glass models (atomic, `+0x12 & 0x7800` = 0x2000 / 0x2800)
/// with the collision flag (+0x1C bit 0) and col data with at least 2 triangles. The sphere (radius, centre
/// `point - pos` projected on the object's right / forward / up rows, `((dz*r.z) + (dy*r.y)) + dx*r.x` each)
/// is tested against every triangle (`CalculateTrianglePlanes` first; all of them, no early exit). On a
/// hit `LastColCheckMS = now`; glass not yet hit (+0x140 bit 4) reports event 0x7B and sets the bit. Hit
/// glass: the first four vertices uncompressed, `minZ` / `maxZ` of their z (pairwise, NaN rules of the
/// compares), the vertex farthest from vertex 0 in 2D (`sqrt(dy*dy + dx*dx)`, the first on ties), `p0 =
/// M * (v0.x, v0.y, minZ)`, `p1 = M * (vf.x, vf.y, minZ)`, event 0x7E, `GeneratePanesForWindow(1, p0, (0, 0,
/// maxZ - minZ), p1 - p0, 0, point, 0.1, +0x140 bit 4, false, 1, false)`, then +0x140 |= 0x20 and +0x1C
/// &= ~0x81. The scan continues after every hit.
#[hook(addr = 0x71CF50, conv = cdecl, name = "CGlass::BreakGlassPhysically", expect = "81 EC BC 00 00 00",
       unverified = "samp2-render-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
pub fn break_glass_physically(px: f32, py: f32, pz: f32, radius: f32) {
    let now = sa_core::timer::TIME_MS.get();
    let last = g::<u32>(LAST_COL_CHECK).read();
    if now < last.wrapping_add(1000) && now >= last {
        return;
    }
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
        if rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) & 0x80 != 0 {
            continue;
        }
        let o = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x19C);
        if o.is_null() || rd::<u8>(o, 0x36) & 7 != 4 {
            continue;
        }
        let ami = mi_as_atomic_model_info_ptr(mi_of(o));
        if ami.is_null() {
            continue;
        }
        let f = rd::<u16>(ami, 0x12) & 0x7800;
        if (f != 0x2000 && f != 0x2800) || rd::<u8>(o, 0x1C) & 1 == 0 {
            continue;
        }
        let cm: GPtr<u8> = rd(mi_of(o), 0x14);
        let data: GPtr<u8> = rd(cm, 0x2C);
        if data.is_null() || rd::<i16>(data, 4) < 2 {
            continue;
        }
        let m: GPtr<u8> = rd(o, 0x14);
        let p = pos(o);
        let dx = x(px).fsub_m(rd(p, 0));
        let dy = x(py).fsub_m(rd(p, 4));
        let dz = x(pz).fsub_m(rd(p, 8));
        let row = |r: GPtr<u8>| dz.fmul_m(rd(r, 8)).fadd(dy.fmul_m(rd(r, 4))).fadd(dx.fmul_m(rd(r, 0))).to_f32();
        let mut sphere = [0u32; 5];
        sphere[0] = row(m).to_bits();
        sphere[1] = row(rd::<GPtr<u8>>(o, 0x14).byte_add(0x10)).to_bits();
        sphere[2] = row(rd::<GPtr<u8>>(o, 0x14).byte_add(0x20)).to_bits();
        sphere[3] = radius.to_bits();
        let sp: GPtr<u8> = GPtr::from_raw(sphere.as_mut_ptr()).cast();
        calculate_triangle_planes(cm.cast::<sa_world::collision::shared::CColModel>());
        let mut hit = false;
        let mut t = 0i32;
        while t < i32::from(rd::<i16>(data, 4)) {
            let tri = rd::<GPtr<u8>>(data, 0x18).byte_add(t as isize * 8);
            let plane = rd::<GPtr<u8>>(data, 0x1C).byte_add(t as isize * 10);
            if test_sphere_triangle(sp, rd(data, 0x14), tri, plane) != 0 {
                hit = true;
            }
            t += 1;
        }
        if !hit {
            continue;
        }
        g::<u32>(LAST_COL_CHECK).write(sa_core::timer::TIME_MS.get());
        if rd::<u32>(o, 0x140) & 0x10 == 0 {
            report_glass_collision_event(g(0xB6_BC90), 0x7B, pos(o).cast());
            wr::<u32>(o, 0x140, rd::<u32>(o, 0x140) | 0x10);
            continue;
        }
        let verts: GPtr<u8> = rd(data, 0x14);
        let mut v = [[0f32; 3]; 4];
        for (k, vv) in v.iter_mut().enumerate() {
            uncompress_vector(GPtr::from_raw(vv.as_mut_ptr()).cast(), verts.byte_add(6 * k as isize));
        }
        let a = if lt(v[2][2], v[3][2]) { v[2][2] } else { v[3][2] };
        let b = if lt(v[0][2], v[1][2]) { v[0][2] } else { v[1][2] };
        let min_z = if lt(b, a) { b } else { a };
        let c = if gt(v[2][2], v[3][2]) { v[2][2] } else { v[3][2] };
        let d = if gt(v[0][2], v[1][2]) { v[0][2] } else { v[1][2] };
        let max_z = if gt(d, c) { d } else { c };
        let dist = |k: usize| {
            let ex = x(v[k][0]).fsub_m(v[0][0]);
            let ey = x(v[k][1]).fsub_m(v[0][1]);
            ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt()
        };
        let mut far = 1usize;
        let mut best = dist(1).to_f32();
        for k in 2..4 {
            let dk = dist(k);
            if !dk.fcom(x(best)).test_any(0x41) {
                best = dk.to_f32();
                far = k;
            }
        }
        let mut a0 = [v[0][0], v[0][1], min_z];
        let mut p0 = [0f32; 3];
        matrix_mul_vec(GPtr::from_raw(p0.as_mut_ptr()).cast(), rd(o, 0x14), GPtr::from_raw(a0.as_mut_ptr()).cast());
        let mut a1 = [v[far][0], v[far][1], min_z];
        let mut p1 = [0f32; 3];
        matrix_mul_vec(GPtr::from_raw(p1.as_mut_ptr()).cast(), rd(o, 0x14), GPtr::from_raw(a1.as_mut_ptr()).cast());
        report_glass_collision_event(g(0xB6_BC90), 0x7E, pos(o).cast());
        let shatter = (rd::<u32>(o, 0x140) >> 4) & 0xFFFF_FF01;
        let rx = x(p1[0]).fsub_m(p0[0]).to_f32();
        let ry = x(p1[1]).fsub_m(p0[1]).to_f32();
        let rz = x(p1[2]).fsub_m(p0[2]).to_f32();
        let h = x(max_z).fsub_m(min_z).to_f32();
        generate_panes_for_window(1, p0[0], p0[1], p0[2], 0.0, 0.0, h, rx, ry, rz, 0.0, 0.0, 0.0, px, py, pz,
            f32::from_bits(0x3DCC_CCCD), shatter, 0, 1, 0);
        wr::<u32>(o, 0x140, rd::<u32>(o, 0x140) | 0x20);
        wr::<u32>(o, 0x1C, rd::<u32>(o, 0x1C) & 0xFFFF_FF7E);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_BREAK_GLASS_PHYSICALLY];
