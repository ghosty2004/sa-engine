//! `CWeapon::SetUpPelletCol`: the shotgun's pellet lines as a collision model, placed at the hit.

use sa_core::abi::HookReg;
use sa_core::field::{rd, rd3f, wr};
use sa_core::hook;
use sa_core::field::gf;
use sa_core::mem::GPtr;

use crate::common::{g, rand, x, xi, X87};
use crate::weapon::PELLET_TEST_COL;
use crate::weapon_fire::ensure_matrix;
use sa_core::bindings::{cross_product, vector_normalise};
use sa_stream::bindings::col_model_allocate_data;

/// `CWorld::fWeaponSpreadRate` (0xB7CD64), `PELLET_COL_SCALE_RATIO_MULT` 1.3 (0x8D6128); 0.2 (0x858CC4), -2.0
/// (0x858B18), 1 / 32767 (0x858C7C), 2 pi (0x858CBC), pi (0x858CB8), 2.5 (0x858FA0), 0.9 (0x858C20).
const SPREAD_RATE: usize = 0xB7_CD64;
const SCALE_MULT: usize = 0x8D_6128;
const K_0_2: usize = 0x85_8CC4;
const K_M2: usize = 0x85_8B18;
const K_RAND: usize = 0x85_8C7C;
const K_2PI: usize = 0x85_8CBC;
const K_PI: usize = 0x85_8CB8;
const K_2_5: usize = 0x85_8FA0;
const K_0_9: usize = 0x85_8C20;
/// `ms_PelletTestCol`'s fields: the bounding box (+0 min, +0xC max), sphere centre (+0x18) and radius (+0x24), the
/// col slot byte (+0x28), the collision data pointer (+0x2C).
const COL: usize = PELLET_TEST_COL;
const COL_DATA: usize = COL + 0x2C;


fn put3(p: GPtr<u8>, v: [f32; 3]) {
    wr::<f32>(p, 0, v[0]);
    wr::<f32>(p, 4, v[1]);
    wr::<f32>(p, 8, v[2]);
}


/// `out = a x b` through `CVector::CrossProduct` into a temporary, copied to `out` (as the original).
fn cross_into(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) {
    let mut t = [0f32; 3];
    let r = cross_product(GPtr::from_raw(t.as_mut_ptr()).cast(), a, b);
    put3(out, rd3f(r));
}

/// [`cross_into`] with a constant axis vector as `b`.
fn cross_axis(out: GPtr<u8>, a: GPtr<u8>, mut axis: [f32; 3]) {
    cross_into(out, a, GPtr::from_raw(axis.as_mut_ptr()).cast());
}

/// `CWeapon::SetUpPelletCol(int32 n, CEntity* owner, CEntity* victim, CVector& point, CColPoint& cp, CMatrix& out)`
/// (0x73C710, thiscall `ret 0x18`). Allocates `ms_PelletTestCol` (15 lines, sphere (0, 0, 0) radius 1, slot 0) on
/// first use. `depth = |cp.pos - point| * fWeaponSpreadRate * 1.3` (sum y, x, z; stored f32); line 0 = (0, -depth, 0)
/// - (0, depth, 0); lines 1..n: `angle = rand() / 32767 * 2pi - pi` (stored f32, no `& 0xFFFF`), `spread = rand() /
/// 32767 * (depth - lo) + lo` with `lo = 0.2 * depth` (gta-reversed: [0, 0.8 depth)), the line (cos * spread, -2
/// depth, sin * spread) - (.., 2 depth, ..); then the line count, the sphere radius 2.5 depth and the box
/// (-depth, -2 depth, -depth) - (depth, 2 depth, depth). The matrix: for a building victim forward = -normal and right
/// = normal x ((0, 1, 0) when |normal.x| >= 0.9, else (1, 0, 0)) (gta-reversed tests normal.z); otherwise forward =
/// the normalised hit direction and right = dir x (0, 0, 1) when |dir.z| <= 0.9, ownerForward x dir for a ped owner,
/// else dir x (1, 0, 0) (gta-reversed crosses the other way round in the first and last case); right normalised, up =
/// right x forward, pos = cp.pos, and (not a building) pos += -depth * forward * dot(normal, forward).
#[hook(addr = 0x73C710, conv = thiscall, name = "CWeapon::SetUpPelletCol", this = "CWeapon",
       expect = "A1 08 A8 C8 00 83 EC 3C",
       unverified = "frontier2-a26", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSIN/FCOS pellet spread; rand sites 0x73C875, 0x73C898; allocates the pellet col data once")]
#[allow(clippy::too_many_arguments)]
pub fn set_up_pellet_col(_this: GPtr<u8>, n: i32, owner: GPtr<u8>, victim: GPtr<u8>, point: GPtr<u8>, cp: GPtr<u8>, mat: GPtr<u8>) {
    if g::<u32>(COL_DATA).read() == 0 {
        col_model_allocate_data(GPtr::from_addr(COL), 0, 0, 15, 0, 0, 0);
        put3(GPtr::from_addr(COL + 0x18), [0.0; 3]);
        g::<f32>(COL + 0x24).write(1.0);
        g::<u8>(COL + 0x28).write(0);
    }
    let data = g::<GPtr<u8>>(COL_DATA).read();
    let (c, pt) = (rd3f(cp), rd3f(point));
    let mut dir = [0f32; 3];
    for k in 0..3 {
        dir[k] = x(c[k]).fsub_m(pt[k]).to_f32();
    }
    let sq = |v: f32| x(v).fmul_m(v);
    let len = sq(dir[1]).fadd(sq(dir[0])).fadd(sq(dir[2])).fsqrt();
    let depth = len.fmul_m(gf(SPREAD_RATE)).fmul_m(gf(SCALE_MULT)).to_f32();
    let neg = x(depth).fchs().to_f32();
    let lines = rd::<GPtr<u8>>(data, 0x10);
    put3(lines, [0.0, neg, 0.0]);
    put3(lines.byte_add(0x10), [0.0, depth, 0.0]);
    if n > 1 {
        let lo = x(depth).fmul_m(gf(K_0_2)).to_f32();
        let w = x(depth).fsub_m(lo).to_f32();
        let y0 = x(depth).fmul_m(gf(K_M2)).to_f32();
        let y1 = x(depth).fadd(x(depth)).to_f32();
        for i in 1..n as isize {
            let r1 = sa_core::orig::at_site(0x73_C875, rand);
            let angle = xi(r1).fmul_m(gf(K_RAND)).fmul_m(gf(K_2PI)).fsub_m(gf(K_PI)).to_f32();
            let r2 = sa_core::orig::at_site(0x73_C898, rand);
            let spread = xi(r2).fmul_m(gf(K_RAND)).fmul_m(w).fadd_m(lo);
            let px = x(angle).fcos().fmul(spread).to_f32();
            let pz = x(angle).fsin().fmul(spread).to_f32();
            let l = lines.byte_add(0x20 * i);
            put3(l, [px, y0, pz]);
            put3(l.byte_add(0x10), [px, y1, pz]);
        }
    }
    g::<f32>(COL + 0x24).write(x(depth).fmul_m(gf(K_2_5)).to_f32());
    wr::<u8>(data, 6, n as u8);
    put3(GPtr::from_addr(COL), [neg, x(depth).fmul_m(gf(K_M2)).to_f32(), neg]);
    put3(GPtr::from_addr(COL + 0xC), [depth, x(depth).fadd(x(depth)).to_f32(), depth]);

    let normal = cp.byte_add(0x10);
    let fwd = mat.byte_add(0x10);
    let building = rd::<u8>(victim, 0x36) & 7 == 1;
    let dp = GPtr::from_raw(dir.as_mut_ptr()).cast::<u8>();
    if building {
        let nv = rd3f(normal);
        put3(fwd, [x(nv[0]).fchs().to_f32(), x(nv[1]).fchs().to_f32(), x(nv[2]).fchs().to_f32()]);
        let nx = x(nv[0]).fabs();
        let axis = if nx.fcom(x(gf(K_0_9))).test_parity(0x05) { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        cross_axis(mat, normal, axis);
    } else {
        vector_normalise(GPtr::from_raw(&mut dir as *mut [f32; 3]));
        put3(fwd, dir);
        if x(dir[2]).fabs().fcom(x(gf(K_0_9))).test_any(0x41) {
            cross_axis(mat, dp, [0.0, 0.0, 1.0]);
        } else if rd::<u8>(owner, 0x36) & 7 == 3 {
            ensure_matrix(owner);
            cross_into(mat, rd::<GPtr<u8>>(owner, 0x14).byte_add(0x10), dp);
        } else {
            cross_axis(mat, dp, [1.0, 0.0, 0.0]);
        }
    }
    vector_normalise(mat.cast());
    cross_into(mat.byte_add(0x20), mat, fwd);
    let pos = mat.byte_add(0x30);
    put3(pos, c);
    if !building {
        let f = rd3f(fwd);
        let nv = rd3f(normal);
        let px: X87 = x(neg).fmul_m(f[0]);
        let py = x(neg).fmul_m(f[1]).to_f32();
        let pz = x(neg).fmul_m(f[2]).to_f32();
        let dot = x(nv[2]).fmul_m(f[2]).fadd(x(nv[1]).fmul_m(f[1])).fadd(x(nv[0]).fmul_m(f[0])).to_f32();
        let pz = x(pz).fmul_m(dot).to_f32();
        let p0 = rd3f(pos);
        wr::<f32>(pos, 0, px.fmul_m(dot).fadd_m(p0[0]).to_f32());
        wr::<f32>(pos, 4, x(py).fmul_m(dot).fadd_m(p0[1]).to_f32());
        wr::<f32>(pos, 8, x(pz).fadd_m(p0[2]).to_f32());
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_UP_PELLET_COL];
