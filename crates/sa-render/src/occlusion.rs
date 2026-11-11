//! PORT-RENDER: `COcclusion` / `COccluder` / `CActiveOccluder` leaves (the occluder tests the
//! visibility code uses). Ported from the 1.0 US bytes (0x71DCA0..0x71FAD5); gta-reversed
//! `Occlusion.cpp` / `Occluder.cpp` / `ActiveOccluder.cpp` as reading aids.
//!
//! `CActiveOccluder` (0xAC bytes): `m_aLines[6]` of 0x14 bytes at +0 (origin x/y, direction x/y,
//! length), `m_nNumLines` (int8) +0x7A, `m_nNumFaces` (int8) +0x7B, `m_aFaceNormals[3]` (CVector) +0x7C,
//! `m_afFaceOffsets[3]` +0xA0. `COccluder` (0x12 bytes): centre x/y/z, width, length (int16, quarter
//! units) +0/+2/+4/+6/+8.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{g, jne, jp, k, rd, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `COcclusion::NumInteriorOcculdersOnMap`
    pub static NUM_INTERIOR_OCCLUDERS_ON_MAP @ 0xC73CC4: i32 = 0;
    /// `COcclusion::NumOccludersOnMap`
    pub static NUM_OCCLUDERS_ON_MAP @ 0xC73F98: i32 = 0;
    /// `COcclusion::FarAwayList`
    pub static FAR_AWAY_LIST @ 0x8D5D68: i16 = -1;
    /// `COcclusion::NearbyList`
    pub static NEARBY_LIST @ 0x8D5D6C: i16 = -1;
    /// `COcclusion::ListWalkThroughFA`
    pub static LIST_WALK_THROUGH_FA @ 0x8D5D70: i16 = -1;
    /// `COcclusion::PreviousListWalkThroughFA`
    pub static PREVIOUS_LIST_WALK_THROUGH_FA @ 0x8D5D74: i16 = -1;
}

/// `COcclusion::Init()` (0x71DCA0).
#[hook(addr = 0x71DCA0, conv = cdecl, name = "COcclusion::Init", expect = "33 C0 A3 98 3F C7 00")]
pub fn init() {
    NUM_OCCLUDERS_ON_MAP.set(0);
    NUM_INTERIOR_OCCLUDERS_ON_MAP.set(0);
    FAR_AWAY_LIST.set(-1);
    NEARBY_LIST.set(-1);
    LIST_WALK_THROUGH_FA.set(-1);
    PREVIOUS_LIST_WALK_THROUGH_FA.set(-1);
}

/// `(px - line.ox) * line.dy - (py - line.oy) * line.dx` with the original's operand order (`p` from
/// registers).
fn side(px: X87, py: X87, line: GPtr<u8>) -> X87 {
    let a = px.fsub_m(rd::<f32>(line, 0)).fmul_m(rd::<f32>(line, 0xC));
    let b = py.fsub_m(rd::<f32>(line, 4)).fmul_m(rd::<f32>(line, 8));
    a.fsub(b)
}

/// `fcomp` + `test ah, 5` + `jnp` taken: `a < b` (ordered).
#[inline]
fn lt(a: X87, b: X87) -> bool {
    !jp(a, b, 5)
}

/// `COcclusion::OccluderHidesBehind(CActiveOccluder* a, CActiveOccluder* b)` (0x71E080): every line end
/// of `a` lies on the inner side of every line of `b`. With no lines in `a`: whether `b` has none either.
#[hook(addr = 0x71E080, conv = cdecl, name = "COcclusion::OccluderHidesBehind", expect = "8B 54 24 04 8A 42 7A")]
pub fn occluder_hides_behind(a: GPtr<u8>, b: GPtr<u8>) -> u8 {
    let na = rd::<i8>(a, 0x7A);
    if na == 0 {
        return u8::from(rd::<i8>(b, 0x7A) == 0);
    }
    for i in 0..i32::from(na) {
        let la = a.byte_add(i as isize * 0x14);
        let nb = i32::from(rd::<i8>(b, 0x7A));
        for j in 0..nb {
            let lb = b.byte_add(j as isize * 0x14);
            if lt(side(x(rd::<f32>(la, 0)), x(rd::<f32>(la, 4)), lb), x(k(0))) {
                return 0;
            }
            let len = rd::<f32>(la, 0x10);
            let ex = x(rd::<f32>(la, 8)).fmul_m(len).fadd_m(rd::<f32>(la, 0));
            let ey = x(rd::<f32>(la, 0xC)).fmul_m(len).fadd_m(rd::<f32>(la, 4));
            if lt(side(ex, ey, lb), x(k(0))) {
                return 0;
            }
        }
    }
    1
}

/// `CActiveOccluder::IsPointWithinOcclusionArea(float x, float y, float radius)` (0x71E580, `ret 0xC`).
#[hook(addr = 0x71E580, conv = thiscall, name = "CActiveOccluder::IsPointWithinOcclusionArea", expect = "56 0F BE 71 7A")]
pub fn is_point_within_occlusion_area(this: GPtr<u8>, px: f32, py: f32, radius: f32) -> u8 {
    for i in 0..i32::from(rd::<i8>(this, 0x7A)) {
        let line = this.byte_add(i as isize * 0x14);
        if lt(side(x(px), x(py), line), x(radius)) {
            return 0;
        }
    }
    1
}

/// `COccluder::NearCamera()` (0x71F960): `|camera - centre| - max(width, length) / 2 < 250`, with the
/// int16 fields in quarter units. Returns 0 / 1 in EAX.
#[hook(addr = 0x71F960, conv = thiscall, name = "COccluder::NearCamera", expect = "83 EC 10 0F BF 41 06")]
pub fn near_camera(this: GPtr<u8>) -> i32 {
    let q = k(0x3E80_0000); // 0.25
    let w = xi(i32::from(rd::<i16>(this, 6))).fmul_m(q);
    let l = xi(i32::from(rd::<i16>(this, 8))).fmul_m(q).to_f32();
    // `fcom` + `test ah, 0x41` + `je`: keep the width when it is greater.
    let r = if jne(w, x(l), 0x41) { x(l) } else { w };
    let cx = xi(i32::from(rd::<i16>(this, 0))).fmul_m(q);
    let cy = xi(i32::from(rd::<i16>(this, 2))).fmul_m(q).to_f32();
    let cz = xi(i32::from(rd::<i16>(this, 4))).fmul_m(q).to_f32();
    let m = rd::<GPtr<u8>>(g(0xB6F03C), 0);
    let cam = if m.is_null() {
        g::<u8>(0xB6F02C)
    } else {
        m.byte_add(0x30)
    };
    let dx = cx.fsubr_m(rd::<f32>(cam, 0));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(cy);
    let dz = x(rd::<f32>(cam, 8)).fsub_m(cz);
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let t = d.fsub(r.fmul_m(k(0x3F00_0000)));
    i32::from(!jp(t, x(k(0x437A_0000)), 5))
}

/// `CActiveOccluder::IsPointBehindOccluder(CVector point, float radius)` (0x71FA40, `ret 0x10`): the
/// point and the camera are on opposite sides of every face plane, and the point is at least `radius`
/// from each. The camera-matrix load runs in the protection chunk 0x4063FD (`ebx = [0x9643B1 -
/// 0xFFDF5374 - 1]` = `[0xB6F03C]`); inlined.
#[hook(addr = 0x71FA40, conv = thiscall, name = "CActiveOccluder::IsPointBehindOccluder", expect = "53 56 57 0F BE 79 7B")]
pub fn is_point_behind_occluder(this: GPtr<u8>, px: f32, py: f32, pz: f32, radius: f32) -> u8 {
    let n = i32::from(rd::<i8>(this, 0x7B));
    if n <= 0 {
        return 1;
    }
    let m = rd::<GPtr<u8>>(g(0xB6F03C), 0);
    for i in 0..n {
        let nrm = this.byte_add(0x7C + i as isize * 0xC);
        let off = rd::<f32>(this, 0xA0 + i as isize * 4);
        let (nx, ny, nz) = (rd::<f32>(nrm, 0), rd::<f32>(nrm, 4), rd::<f32>(nrm, 8));
        let p = x(py)
            .fmul_m(ny)
            .fadd(x(pz).fmul_m(nz))
            .fadd(x(px).fmul_m(nx))
            .fsub_m(off);
        let cam = if m.is_null() {
            g::<u8>(0xB6F02C)
        } else {
            m.byte_add(0x30)
        };
        let c = x(rd::<f32>(cam, 8))
            .fmul_m(nz)
            .fadd(x(rd::<f32>(cam, 4)).fmul_m(ny))
            .fadd(x(rd::<f32>(cam, 0)).fmul_m(nx))
            .fsub_m(off);
        // `fcomp 0.0` + `test ah, 1` + `je`: not on opposite sides (C0 clear).
        if !c.fmul(p).fcom(x(k(0))).test_any(1) {
            return 0;
        }
        if lt(X87::from_f64_exact(p.to_f64().abs()), x(radius)) {
            return 0;
        }
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_OCCLUDER_HIDES_BEHIND,
    HOOK_IS_POINT_WITHIN_OCCLUSION_AREA,
    HOOK_NEAR_CAMERA,
    HOOK_IS_POINT_BEHIND_OCCLUDER,
];
