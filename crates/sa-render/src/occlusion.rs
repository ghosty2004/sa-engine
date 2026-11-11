//! PORT-RENDER: `COcclusion` / `COccluder` / `CActiveOccluder` leaves (the occluder tests the
//! visibility code uses). Ported from the 1.0 US bytes (0x71DCA0..0x71FAD5); gta-reversed
//! `Occlusion.cpp` / `Occluder.cpp` / `ActiveOccluder.cpp` as reading aids.
//!
//! `CActiveOccluder` (0xAC bytes): `m_aLines[6]` of 0x14 bytes at +0 (origin x/y, direction x/y,
//! length), `m_nNumLines` (int8) +0x7A, `m_nNumFaces` (int8) +0x7B, `m_aFaceNormals[3]` (CVector) +0x7C,
//! `m_afFaceOffsets[3]` +0xA0. `COccluder` (0x12 bytes): centre x/y/z, width, length (int16, quarter
//! units) +0/+2/+4/+6/+8.

use sa_core::abi::HookReg;
use sa_core::game::CURR_AREA;
use sa_core::mem::GPtr;
use sa_core::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x, xi};

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
    /// `COcclusion::NumActiveOccluders`
    pub static NUM_ACTIVE_OCCLUDERS @ 0xC73CC0: i32 = 0;
    /// `COcclusion::MaxYInOccluder`, `MinYInOccluder`, `MaxXInOccluder`, `MinXInOccluder`
    pub static OCCLUDER_BOUNDS @ 0xC73CA0: [f32; 4] = [0.0; 4];
    /// Per occluder corner: nonzero when its projected `aCoors[i]` is used as is (bool)
    pub static CORNER_FLAGS @ 0xC73CB0: [u8; 16] = [0; 16];
    /// `COcclusion::aInteriorOccluders[40]` (COccluder, 0x12 bytes)
    pub static INTERIOR_OCCLUDERS @ 0xC73CC8: [u8; 40 * 0x12] = [0; 40 * 0x12];
    /// `COcclusion::aOccluders[1000]` (COccluder, 0x12 bytes)
    pub static OCCLUDERS @ 0xC73FA0: [u8; 1000 * 0x12] = [0; 1000 * 0x12];
    /// `COcclusion::aActiveOccluders[28]` (CActiveOccluder, 0xAC bytes)
    pub static ACTIVE_OCCLUDERS @ 0xC78610: [u8; 28 * 0xAC] = [0; 28 * 0xAC];
    /// The occluder corners in world space (`aVecs[8]`, CVector)
    pub static CORNER_VECS @ 0xC798E0: [f32; 3 * 8] = [0.0; 3 * 8];
    /// `CenterOnScreen` (CVector)
    pub static CENTER_ON_SCREEN @ 0xC79940: [f32; 3] = [0.0; 3];
    /// The corners' projected coordinates (`aCoors[8]`, CVector)
    pub static CORNER_COORS @ 0xC79950: [f32; 3 * 8] = [0.0; 3 * 8];
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

/// `TheCamera.m_mViewMatrix` (CMatrix).
const VIEW_MATRIX: usize = 0xB6FA2C;

orig! {
    /// `CalcScreenCoors(const CVector& in, CVector* out, float* w, float* h)` (0x71DA00)
    pub fn calc_screen_coors(v: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>) -> u8 = cdecl 0x71DA00 => unported;
    /// `DoesInfiniteLineTouchScreen(float x, float y, float dx, float dy)` (0x71DB80); raw dwords
    pub fn does_infinite_line_touch_screen(x: u32, y: u32, dx: u32, dy: u32) -> u8 = cdecl 0x71DB80 => unported;
    /// `operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890)
    pub fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `COccluder::ProcessOneOccluder(CActiveOccluder*)` (0x71E5D0)
    pub fn process_one_occluder(this: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x71E5D0 => unported;
    /// `COccluder::NearCamera()` (0x71F960)
    pub fn call_near_camera(this: GPtr<u8>) -> i32 = thiscall 0x71F960 => near_camera;
    /// `COcclusion::OccluderHidesBehind(CActiveOccluder*, CActiveOccluder*)` (0x71E080)
    pub fn call_occluder_hides_behind(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x71E080 => occluder_hides_behind;
    /// `CActiveOccluder::IsPointWithinOcclusionArea(float, float, float)` (0x71E580); raw dwords
    pub fn call_is_point_within_occlusion_area(this: GPtr<u8>, px: u32, py: u32, r: u32) -> u8 = thiscall 0x71E580 => unported;
    /// `CActiveOccluder::IsPointBehindOccluder(CVector, float)` (0x71FA40); raw dwords
    pub fn call_is_point_behind_occluder(this: GPtr<u8>, px: u32, py: u32, pz: u32, r: u32) -> u8 = thiscall 0x71FA40 => unported;
}

/// `COcclusion::IsPositionOccluded(CVector pos, float radius)` (0x7200B0): with active occluders, the
/// position projects on screen (`CalcScreenCoors`) and some active occluder nearer than `z - radius`
/// covers the projected point with radius `max(w, h) * radius` and hides `pos` in 3D. The occluder
/// count is re-read before the loop and after every occluder.
#[hook(addr = 0x7200B0, conv = cdecl, name = "COcclusion::IsPositionOccluded", expect = "A1 C0 3C C7 00")]
pub fn is_position_occluded(px: u32, py: u32, pz: u32, radius: u32) -> u8 {
    if NUM_ACTIVE_OCCLUDERS.get() == 0 {
        return 0;
    }
    let mut v = [f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz)];
    let mut out = [0f32; 3];
    let mut wh = [0f32; 2];
    let wp = GPtr::from_raw(wh.as_mut_ptr());
    if calc_screen_coors(
        GPtr::from_raw(v.as_mut_ptr()),
        GPtr::from_raw(out.as_mut_ptr()),
        wp,
        wp.wrapping_index(1),
    ) == 0
    {
        return 0;
    }
    let (w, h) = (wh[0], wh[1]);
    // `fcomp h` + `test ah, 0x41` + `jne`: h when w <= h (or unordered), otherwise w.
    let m = if jne(x(w), x(h), 0x41) { x(h) } else { x(w) };
    let r = m.fmul_m(f32::from_bits(radius)).to_f32();
    if NUM_ACTIVE_OCCLUDERS.get() <= 0 {
        return 0;
    }
    let zr = x(out[2]).fsub_m(f32::from_bits(radius)).to_f32();
    let mut i = 0;
    loop {
        let o = active(i);
        // `fild depth` + `fcomp zr` + `test ah, 5` + `jp`: skipped unless depth < zr.
        if !jp(xi(i32::from(rd::<i16>(o, 0x78))), x(zr), 5)
            && call_is_point_within_occlusion_area(o, out[0].to_bits(), out[1].to_bits(), r.to_bits()) != 0
            && call_is_point_behind_occluder(o, px, py, pz, radius) != 0
        {
            return 1;
        }
        i += 1;
        if i >= NUM_ACTIVE_OCCLUDERS.get() {
            return 0;
        }
    }
}

fn occluder(i: i16) -> GPtr<u8> {
    OCCLUDERS.ptr().cast::<u8>().byte_add(i as isize * 0x12)
}

/// The 15-bit signed `m_nNextIndex` of `aOccluders[i]` (`+0x10`, the top bit is a flag): `shl ax, 1`
/// + `sar ax, 1`.
fn next_index(i: i16) -> i16 {
    let w = rd::<u16>(occluder(i), 0x10);
    ((w << 1) as i16) >> 1
}

/// `aOccluders[i].m_nNextIndex = v` (low 15 bits; the flag bit kept).
fn set_next_index(i: i16, v: u16) {
    let p = occluder(i);
    let w = rd::<u16>(p, 0x10);
    wr::<u16>(p, 0x10, w ^ ((w ^ v) & 0x7FFF));
}

fn active(i: i32) -> GPtr<u8> {
    ACTIVE_OCCLUDERS.ptr().cast::<u8>().byte_add(i as isize * 0xAC)
}

/// One `ProcessOneOccluder` call into the next active slot while fewer than 28 are active; the count
/// is re-read after the call and incremented when the occluder was added.
fn process_into_next(occ: GPtr<u8>, n: &mut i32) {
    if *n < 28 {
        let r = process_one_occluder(occ, active(*n));
        *n = NUM_ACTIVE_OCCLUDERS.get();
        if r != 0 {
            *n += 1;
            NUM_ACTIVE_OCCLUDERS.set(*n);
        }
    }
}

/// `COcclusion::ProcessBeforeRendering()` (0x7201C0): rebuilds the active occluders. Outside interiors
/// it walks 16 entries of the far-away list (moving occluders near the camera to the nearby list), then
/// processes every nearby occluder (moving those no longer near back to the far-away list); in an
/// interior it processes every interior occluder. At most 28 active occluders. Finally it removes every
/// active occluder hidden behind a nearer one (`rep movsd` moves the tail down).
#[hook(addr = 0x7201C0, conv = cdecl, name = "COcclusion::ProcessBeforeRendering", expect = "51 A1 14 29 B7 00")]
pub fn process_before_rendering() {
    let mut n: i32 = 0;
    NUM_ACTIVE_OCCLUDERS.set(0);
    if CURR_AREA.get() == 0 {
        let mut si = LIST_WALK_THROUGH_FA.get();
        let mut bp: i16;
        let walk = if si == -1 {
            bp = -1;
            si = FAR_AWAY_LIST.get();
            PREVIOUS_LIST_WALK_THROUGH_FA.set(bp);
            LIST_WALK_THROUGH_FA.set(si);
            si != -1
        } else {
            bp = PREVIOUS_LIST_WALK_THROUGH_FA.get();
            true
        };
        if walk {
            let mut count = 0;
            while si != -1 {
                if call_near_camera(occluder(si)) as u8 != 0 {
                    let cx = LIST_WALK_THROUGH_FA.get();
                    if bp == -1 {
                        FAR_AWAY_LIST.set(next_index(si));
                    } else {
                        set_next_index(bp, rd::<u16>(occluder(si), 0x10));
                    }
                    let nsi = next_index(si);
                    set_next_index(cx, NEARBY_LIST.get() as u16);
                    NEARBY_LIST.set(cx);
                    si = nsi;
                } else {
                    bp = si;
                    si = next_index(si);
                    PREVIOUS_LIST_WALK_THROUGH_FA.set(bp);
                }
                count += 1;
                LIST_WALK_THROUGH_FA.set(si);
                if count >= 16 {
                    break;
                }
            }
        }
        let mut si = NEARBY_LIST.get();
        let mut bp: i16 = -1;
        while si != -1 {
            process_into_next(occluder(si), &mut n);
            if call_near_camera(occluder(si)) as u8 == 0 {
                let cx = si;
                if bp == -1 {
                    NEARBY_LIST.set(next_index(si));
                } else {
                    set_next_index(bp, rd::<u16>(occluder(si), 0x10));
                }
                si = next_index(si);
                set_next_index(cx, FAR_AWAY_LIST.get() as u16);
                FAR_AWAY_LIST.set(cx);
            } else {
                bp = si;
                si = next_index(si);
            }
        }
    } else if NUM_INTERIOR_OCCLUDERS_ON_MAP.get() > 0 {
        let mut i = 0;
        loop {
            process_into_next(
                INTERIOR_OCCLUDERS.ptr().cast::<u8>().byte_add(i as isize * 0x12),
                &mut n,
            );
            i += 1;
            if i >= NUM_INTERIOR_OCCLUDERS_ON_MAP.get() {
                break;
            }
        }
    }
    let mut i: i32 = 0;
    while i < n {
        let mut j: i32 = 0;
        while j < n {
            if i != j
                && rd::<i16>(active(j), 0x78) < rd::<i16>(active(i), 0x78)
                && call_occluder_hides_behind(active(i), active(j)) != 0
            {
                let last = n - 1;
                if i < last {
                    let words = ((last - i) as u32).wrapping_mul(0xAC) >> 2;
                    let (d, s) = (active(i), active(i + 1));
                    for w in 0..words as isize {
                        wr::<u32>(d, w * 4, rd::<u32>(s, w * 4));
                    }
                }
                n = last;
                i -= 1;
                NUM_ACTIVE_OCCLUDERS.set(n);
                j = n;
            }
            j += 1;
        }
        i += 1;
    }
}

/// The near-plane crossing of the corner pair `(a, b)`: both transformed by the camera's view matrix
/// (`operator*` 0x59C890, into one stack vector), `t = |zb - 1.1| / (|za - 1.1| + |zb - 1.1|)` with
/// `za - 1.1` stored as a float, the world point `t * A + (1 - t) * B` (the `(1 - t) * B` x/y and the
/// `t * A` z terms stored as floats), projected with `CalcScreenCoors` into `out`. The original's `h`
/// out pointer is its `corner2` argument slot and it reuses the `corner1` slot for `za` (stack only);
/// locals here.
fn near_plane_crossing(a: i32, b: i32, out: GPtr<f32>) -> u8 {
    let va = CORNER_VECS.ptr().cast::<f32>().wrapping_index(a * 3);
    let vb = CORNER_VECS.ptr().cast::<f32>().wrapping_index(b * 3);
    let mut tmp = [0f32; 3];
    let tp = GPtr::from_raw(tmp.as_mut_ptr());
    let r = matrix_mul_vec(tp, g(VIEW_MATRIX), va);
    let za = x(r.wrapping_index(2).read()).fsub_m(k(0x3F8C_CCCD)).to_f32();
    let r = matrix_mul_vec(tp, g(VIEW_MATRIX), vb);
    let zb = X87::from_f64_exact(
        x(r.wrapping_index(2).read())
            .fsub_m(k(0x3F8C_CCCD))
            .to_f64()
            .abs(),
    );
    let za = X87::from_f64_exact(f64::from(za).abs());
    let t = zb.fdiv(za.fadd(zb));
    let s = x(k(0x3F80_0000)).fsub(t);
    let c = |p: GPtr<f32>, i: i32| p.wrapping_index(i).read();
    let sbx = s.fmul_m(c(vb, 0)).to_f32();
    let sby = s.fmul_m(c(vb, 1)).to_f32();
    let sbz = s.fmul_m(c(vb, 2));
    let tax = t.fmul_m(c(va, 0));
    let tay = t.fmul_m(c(va, 1));
    let taz = t.fmul_m(c(va, 2)).to_f32();
    let mut p = [
        tax.fadd_m(sbx).to_f32(),
        tay.fadd_m(sby).to_f32(),
        x(taz).fadd(sbz).to_f32(),
    ];
    let mut wh = [0f32; 2];
    let wp = GPtr::from_raw(wh.as_mut_ptr());
    calc_screen_coors(GPtr::from_raw(p.as_mut_ptr()), out, wp, wp.wrapping_index(1))
}

/// min (`test ah, 5` + `jp` / `jnp` pair) or max (`test ah, 0x41` + `jne` / `je` pair) of a bound and
/// two values, as `ProcessLineSegment` updates `MinX/MaxX/MinY/MaxY`.
fn bound(cur: f32, p: f32, q: f32, min: bool) -> f32 {
    if min {
        let m = if jp(x(cur), x(p), 5) { p } else { cur };
        if jp(x(m), x(q), 5) { q } else { m }
    } else {
        let m = if jne(x(cur), x(p), 0x41) { p } else { cur };
        if jne(x(m), x(q), 0x41) { q } else { m }
    }
}

/// Copies the projected `aCoors[c]` into `out`.
fn copy_coors(c: i32, out: &mut [f32; 3]) {
    let coors = CORNER_COORS.ptr().cast::<f32>();
    for (i, v) in out.iter_mut().enumerate() {
        *v = coors.wrapping_index(c * 3 + i as i32).read();
    }
}

/// `COccluder::ProcessLineSegment(int32 corner1, int32 corner2, CActiveOccluder* occl)` (0x71E130,
/// `ret 0xC`, `this` unused): with neither corner flagged nothing is added (returns 1). Otherwise each
/// corner's screen point is its projected `aCoors` entry when flagged, else the projected near-plane
/// crossing (returns 1 when that projection fails). The screen bounds grow by both points; a line from
/// the first point towards the second (reversed when the screen centre is on its right) is stored as
/// `occl->m_aLines[m_nNumLines]` and counted when it touches the screen (returns 0). Otherwise returns
/// 0 unless the screen middle lies on the line's outer side (1). The entry's protection chunk 0x401759
/// (`cl = flags[corner1]`) is inlined.
#[hook(addr = 0x71E130, conv = thiscall, name = "COccluder::ProcessLineSegment", expect = "8B 44 24 04 90 E9 1F 36 CE FF")]
pub fn process_line_segment(_this: GPtr<u8>, c1: i32, c2: i32, occl: GPtr<u8>) -> u8 {
    let flags = CORNER_FLAGS.ptr().cast::<u8>();
    let f1 = rd::<u8>(flags, c1 as isize);
    if f1 == 0 && rd::<u8>(flags, c2 as isize) == 0 {
        return 1;
    }
    let mut p = [0f32; 3];
    let mut q = [0f32; 3];
    if f1 != 0 {
        copy_coors(c1, &mut p);
    } else if near_plane_crossing(c1, c2, GPtr::from_raw(p.as_mut_ptr())) == 0 {
        return 1;
    }
    if rd::<u8>(flags, c2 as isize) != 0 {
        copy_coors(c2, &mut q);
    } else if near_plane_crossing(c1, c2, GPtr::from_raw(q.as_mut_ptr())) == 0 {
        return 1;
    }
    let b = OCCLUDER_BOUNDS.ptr().cast::<f32>();
    let upd = |i: i32, p: f32, q: f32, min: bool| {
        b.wrapping_index(i)
            .write(bound(b.wrapping_index(i).read(), p, q, min));
    };
    upd(3, p[0], q[0], true);
    upd(2, p[0], q[0], false);
    upd(1, p[1], q[1], true);
    upd(0, p[1], q[1], false);
    let (mut ox, mut oy) = (p[0], p[1]);
    let mut dx = x(q[0]).fsub_m(p[0]);
    let mut dy = x(q[1]).fsub_m(p[1]);
    let centre = CENTER_ON_SCREEN.ptr().cast::<f32>();
    let a = x(centre.read()).fsub_m(p[0]).fmul(dy);
    let bb = x(centre.wrapping_index(1).read()).fsub_m(p[1]).fmul(dx);
    if !jp(a.fsub(bb), x(k(0)), 5) {
        ox = dx.fadd_m(p[0]).to_f32();
        oy = x(p[1]).fadd(dy).to_f32();
        dx = dx.fchs();
        dy = dy.fchs();
    }
    let len = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    let line = |o: GPtr<u8>| o.byte_add(isize::from(rd::<i8>(o, 0x7A)) * 0x14);
    wr::<f32>(line(occl), 0x10, len.to_f32());
    let inv = x(k(0x3F80_0000)).fdiv_m(rd::<f32>(line(occl), 0x10));
    wr::<f32>(line(occl), 0, ox);
    let dirx = inv.fmul(dx).to_f32();
    let diry = inv.fmul(dy).to_f32();
    wr::<f32>(line(occl), 4, oy);
    wr::<f32>(line(occl), 8, dirx);
    wr::<f32>(line(occl), 0xC, diry);
    if does_infinite_line_touch_screen(ox.to_bits(), oy.to_bits(), dirx.to_bits(), diry.to_bits()) != 0 {
        wr::<i8>(occl, 0x7A, rd::<i8>(occl, 0x7A).wrapping_add(1));
        return 0;
    }
    let half = k(0x3F00_0000);
    let a = xi(RS_MAXIMUM_WIDTH.get()).fmul_m(half).fsub_m(ox).fmul_m(diry);
    let bb = xi(RS_MAXIMUM_HEIGHT.get()).fmul_m(half).fsub_m(oy).fmul_m(dirx);
    if jp(a.fsub(bb), x(k(0)), 5) { 0 } else { 1 }
}

/// An angle in degrees brought into [0, 360] as `AddOne` does on the x87 stack: `+ 360` while below 0
/// (`test ah, 5` + `jnp`), then `- 360` while above 360 (`test ah, 0x41`; exactly 360 stays).
fn wrap_degrees(a: f32) -> X87 {
    let full = k(0x43B4_0000);
    let mut v = x(a);
    if !jp(v, x(k(0)), 5) {
        loop {
            v = v.fadd_m(full);
            if jp(v, x(k(0)), 5) {
                break;
            }
        }
    }
    if !jne(v, x(full), 0x41) {
        loop {
            v = v.fsub_m(full);
            if jne(v, x(full), 0x41) {
                break;
            }
        }
    }
    v
}

/// `COcclusion::AddOne(float x, float y, float z, float width, float length, float height, float rotX,
/// float rotY, float rotZ, uint32 flags, bool isInterior)` (0x71DCD0): the three sizes become
/// `_ftol(|size|)` (stored into their own argument slots); with more than one of them 0 nothing is
/// added. The rotations are wrapped into [0, 360] degrees and converted to radians (`* 0.017453292`,
/// stored as floats). A new `COccluder` at the end of `aOccluders` (below 1000; linked at the head of
/// the far-away list, flag bit from `flags != 0`) or of `aInteriorOccluders` (below 40): centre
/// `_ftol(c * 4)`, sizes `_ftol(int * 4)` in the order length, width, height (+6, +8, +0xA), angles
/// `_ftol(rad * 40.743664)` as bytes rotZ, rotY, rotX at +0xE, +0xD, +0xC.
#[hook(addr = 0x71DCD0, conv = cdecl, name = "COcclusion::AddOne", expect = "D9 44 24 14 53 56")]
#[allow(clippy::too_many_arguments)]
pub fn add_one(
    cx: f32,
    cy: f32,
    cz: f32,
    width: f32,
    length: f32,
    height: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    flags: u32,
    interior: u8,
) {
    let abs = |v: f32| X87::from_f64_exact(f64::from(v).abs());
    let il = ftol32(abs(length));
    let iw = ftol32(abs(width));
    let ih = ftol32(abs(height));
    let zeros = i32::from(il == 0) + i32::from(iw == 0) + i32::from(ih == 0);
    if zeros > 1 {
        return;
    }
    let ax = wrap_degrees(rx);
    let ay = wrap_degrees(ry);
    let az = wrap_degrees(rz);
    let rad = k(0x3C8E_FA35);
    let rz = az.fmul_m(rad).to_f32();
    let ry = ay.fmul_m(rad).to_f32();
    let rx = ax.fmul_m(rad).to_f32();
    let four = k(0x4080_0000);
    let to_byte = k(0x4222_F983);
    let fill = |o: GPtr<u8>| {
        wr::<u16>(o, 0, ftol32(x(cx).fmul_m(four)) as u16);
        wr::<u16>(o, 2, ftol32(x(cy).fmul_m(four)) as u16);
        wr::<u16>(o, 4, ftol32(x(cz).fmul_m(four)) as u16);
        wr::<u16>(o, 6, ftol32(xi(il).fmul_m(four)) as u16);
        wr::<u16>(o, 8, ftol32(xi(iw).fmul_m(four)) as u16);
        wr::<u16>(o, 0xA, ftol32(xi(ih).fmul_m(four)) as u16);
        wr::<u8>(o, 0xE, ftol32(x(rz).fmul_m(to_byte)) as u8);
        wr::<u8>(o, 0xD, ftol32(x(ry).fmul_m(to_byte)) as u8);
        wr::<u8>(o, 0xC, ftol32(x(rx).fmul_m(to_byte)) as u8);
    };
    if interior == 0 {
        let n = NUM_OCCLUDERS_ON_MAP.get();
        if n >= 1000 {
            return;
        }
        let o = occluder(n as i16);
        fill(o);
        if flags != 0 {
            wr::<u16>(o, 0x10, rd::<u16>(o, 0x10) | 0x8000);
        } else {
            wr::<u8>(o, 0x11, rd::<u8>(o, 0x11) & 0x7F);
        }
        let head = FAR_AWAY_LIST.get() as u16;
        FAR_AWAY_LIST.set(n as i16);
        set_next_index(n as i16, head);
        NUM_OCCLUDERS_ON_MAP.set(n + 1);
    } else {
        let n = NUM_INTERIOR_OCCLUDERS_ON_MAP.get();
        if n >= 40 {
            return;
        }
        fill(INTERIOR_OCCLUDERS.ptr().cast::<u8>().byte_add(n as isize * 0x12));
        NUM_INTERIOR_OCCLUDERS_ON_MAP.set(n + 1);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_OCCLUDER_HIDES_BEHIND,
    HOOK_IS_POINT_WITHIN_OCCLUSION_AREA,
    HOOK_NEAR_CAMERA,
    HOOK_IS_POINT_BEHIND_OCCLUDER,
    HOOK_IS_POSITION_OCCLUDED,
    HOOK_PROCESS_BEFORE_RENDERING,
    HOOK_PROCESS_LINE_SEGMENT,
    HOOK_ADD_ONE,
];
