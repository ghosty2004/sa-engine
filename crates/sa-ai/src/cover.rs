//! `CCover` / `CFormation` cover points (FRONTIER-2 part 4, batch `frontier2-8`): what
//! `CTaskComplexKillPedOnFootArmed::CreateFirstSubTask` / `CreateNextSubTask` need when the armed police look for cover
//! (crossing run, 4000 frames). Ported from the 1.0 US bytes; gta-reversed (Cover.cpp, Formation.cpp) was read as an
//! aid and differs where noted.
//!
//! `CCover::m_aPoints` (0xC197C8, 100 points of 0x1C bytes): +0 type (i8: 0 free, 1 entity, 2 vehicle box, 3 point on
//! the map), +2 direction (byte, 256 steps per turn), +4 position, +0x10 entity, +0x14 / +0x18 the peds using it.
//! `CCover::m_NumPoints` (0xC197A4). `CPointList`: +0 count, +4 points (24 vectors), +0x124 24 claimed flags.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{gt, lt};
use sa_math::x87::X87;

use crate::common::*;
use sa_core::bindings::{entity_register_reference, matrix_mul_vec, vector_normalise};
use sa_core::field::gf;
use sa_world::world::common::entity_pos as pos_of;

orig! {
    /// `CCover::FindVectorFromDir(uint8)` (0x698D60, cdecl, the vector returned through a hidden pointer)
    fn find_vector_from_dir(out: GPtr<u8>, dir: u32) -> GPtr<u8> = cdecl 0x698D60 => unported;
    /// `CPointList::Empty()` (0x699F00)
    fn point_list_empty_b(l: GPtr<u8>) = thiscall 0x699F00 => point_list_empty;
    /// `CFormation::FindCoverPointsBehindBox(CPointList*, CVector, CMatrix*, const CVector*, const CVector*, const CVector*,
    /// float)` (0x699FF0)
    fn find_cover_points_behind_box_b(l: GPtr<u8>, tx: u32, ty: u32, tz: u32, m: GPtr<u8>, c: GPtr<u8>, min: GPtr<u8>,
        max: GPtr<u8>, cut: f32) = cdecl 0x699FF0 => find_cover_points_behind_box;
}

const POINTS: usize = 0xC1_97C8;
const NUM_POINTS: usize = 0xC1_97A4;

fn point(i: usize) -> GPtr<u8> {
    GPtr::from_addr(POINTS + 0x1C * i)
}


/// The inlined `RemoveCoverPointIfEntityLost`: a point of type 1..3 without an entity is freed (`m_NumPoints - 1`).
fn remove_if_entity_lost(cp: GPtr<u8>) {
    let t = rd::<i8>(cp, 0);
    if t > 0 && t <= 3 && rd::<u32>(cp, 0x10) == 0 {
        g::<u32>(NUM_POINTS).write(g::<u32>(NUM_POINTS).read().wrapping_sub(1));
        wr::<u8>(cp, 0, 0);
    }
}

/// `CPointList::Empty()` (0x699F00): count 0 and the 24 claimed flags cleared.
#[hook(addr = 0x699F00, conv = thiscall, name = "CPointList::Empty", expect = "57 C7 01 00 00 00 00",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn point_list_empty(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0);
    for i in 0..6 {
        wr::<u32>(this, 0x124 + 4 * i, 0);
    }
}

/// `CCover::DoesCoverPointStillProvideCover(CCoverPoint*, CVector pos)` (0x698DD0, cdecl, AL): false without a point;
/// a point that lost its entity is freed; a free point false; a point of type 1 or 2 true; a point on the map (3) true
/// when `pos - point` (each a float) has a positive dot product with `FindVectorFromDir(dir)` (summed z, y, x).
#[hook(addr = 0x698DD0, conv = cdecl, name = "CCover::DoesCoverPointStillProvideCover", expect = "8B 44 24 04 83 EC 18",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
pub fn does_cover_point_still_provide_cover(cp: GPtr<u8>, x: f32, y: f32, z: f32) -> u8 {
    if cp.is_null() {
        return 0;
    }
    remove_if_entity_lost(cp);
    match rd::<i8>(cp, 0) {
        0 => return 0,
        3 => {}
        _ => return 1,
    }
    let d = [
        f32_of(X87::from_f32(x).fsub_m(rd::<f32>(cp, 4))),
        f32_of(X87::from_f32(y).fsub_m(rd::<f32>(cp, 8))),
        f32_of(X87::from_f32(z).fsub_m(rd::<f32>(cp, 0xC))),
    ];
    let mut tmp = [0f32; 3];
    let v = find_vector_from_dir(GPtr::from_raw(tmp.as_mut_ptr()).cast(), u32::from(rd::<u8>(cp, 2)));
    let dot = X87::from_f32(d[2]).fmul_m(rd::<f32>(v, 8)).fadd(X87::from_f32(d[1]).fmul_m(rd::<f32>(v, 4)))
        .fadd(X87::from_f32(d[0]).fmul_m(rd::<f32>(v, 0)));
    u8::from(gt(dot, X87::zero()))
}

/// `CCover::FindAndReserveCoverPoint(CPed*, const CVector& target, bool closer)` (0x6992B0, cdecl). Over the 100
/// points (each first freed when it lost its entity): a used point that can take the ped — type 2 when its first ped
/// slot is free or its second is taken (as the bytes test it), any other type when its first slot is free; its
/// position (type 1 / 2: the entity's, type 3: its own; other types keep the previous point's, an uninitialised local
/// for the first); a point on the map only when `(target - pos) · (-sin a, cos a, z * 0)` > 0 (a = dir * 2π/256, x87
/// FSIN / FCOS, cos rounded to a float); `|ped.z - pos.z|` < 4; the 2D distance ped → point `d` < 15; with `closer`
/// also `|point - target|` (2D) < `|ped - target|` (2D, a float) + 4. The nearest (`d` as a float, strictly less, from
/// 99999.9) wins. None: null. Otherwise the ped is put into the first free slot (with `RegisterReference`) unless it is
/// in one already; the point is returned even when both slots hold other peds. gta-reversed takes the farthest and
/// returns null unless the ped already uses the point.
#[hook(addr = 0x6992B0, conv = cdecl, name = "CCover::FindAndReserveCoverPoint", expect = "83 EC 28 D9 05 9C 99 85 00",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes,uninit-locals,x87-transcendental",
       note = "the position local of a point of type > 3 / < 0 is the previous point's (uninitialised for the first)")]
pub fn find_and_reserve_cover_point(ped: GPtr<u8>, target: GPtr<u8>, closer: u32) -> GPtr<u8> {
    let mut best = X87::from_f32(gf(0x85_999C));
    let mut best_i: i32 = -1;
    let mut p = [0f32; 3];
    let (tx, ty, tz) = (rd::<f32>(target, 0), rd::<f32>(target, 4), rd::<f32>(target, 8));
    for i in 0..100 {
        let cp = point(i);
        remove_if_entity_lost(cp);
        let t = rd::<i8>(cp, 0);
        if t == 0 {
            continue;
        }
        let ok = if t == 2 { rd::<u32>(cp, 0x14) == 0 || rd::<u32>(cp, 0x18) != 0 } else { rd::<u32>(cp, 0x14) == 0 };
        if !ok {
            continue;
        }
        if t > 0 && t <= 2 {
            let e = pos_of(rd::<GPtr<u8>>(cp, 0x10));
            p = [rd::<f32>(e, 0), rd::<f32>(e, 4), rd::<f32>(e, 8)];
        } else if t == 3 {
            p = [rd::<f32>(cp, 4), rd::<f32>(cp, 8), rd::<f32>(cp, 0xC)];
        }
        if t == 3 {
            let a = X87::from_f32(tx).fsub_m(rd::<f32>(cp, 4));
            let b = X87::from_f32(ty).fsub_m(rd::<f32>(cp, 8));
            let c = X87::from_f32(tz).fsub_m(rd::<f32>(cp, 0xC));
            let ang = X87::from_i32(i32::from(rd::<u8>(cp, 2))).fmul_m(gf(0x85_9BBC));
            let nsin = ang.fsin().fchs();
            let cos = f32_of(ang.fcos());
            let dot = b.fmul_m(cos).fadd(nsin.fmul(a)).fadd(c.fmul_m(0.0));
            if !gt(dot, X87::zero()) {
                continue;
            }
        }
        let pp = pos_of(ped);
        let dz = X87::from_f32(rd::<f32>(pp, 8)).fsub_m(p[2]);
        let dz = if lt(dz, X87::zero()) { dz.fchs() } else { dz };
        if !lt(dz, X87::from_f32(4.0)) {
            continue;
        }
        let pp = pos_of(ped);
        let x = X87::from_f32(rd::<f32>(pp, 0)).fsub_m(p[0]);
        let y = X87::from_f32(rd::<f32>(pp, 4)).fsub_m(p[1]);
        let d = y.fmul(y).fadd(x.fmul(x)).fsqrt();
        if !lt(d, X87::from_f32(15.0)) {
            continue;
        }
        let df = X87::from_f32(f32_of(d));
        let pp = pos_of(ped);
        let x = X87::from_f32(rd::<f32>(pp, 0)).fsub_m(tx);
        let y = X87::from_f32(rd::<f32>(pp, 4)).fsub_m(ty);
        let d_ped = f32_of(y.fmul(y).fadd(x.fmul(x)).fsqrt());
        let x = X87::from_f32(p[0]).fsub_m(tx);
        let y = X87::from_f32(p[1]).fsub_m(ty);
        let d_point = y.fmul(y).fadd(x.fmul(x)).fsqrt();
        if !lt(df, best) {
            continue;
        }
        if closer & 0xFF != 0 && !lt(d_point, X87::from_f32(d_ped).fadd_m(4.0)) {
            continue;
        }
        best_i = i as i32;
        best = df;
    }
    if best_i < 0 {
        return GPtr::NULL;
    }
    let cp = point(best_i as usize);
    for j in 0..2 {
        let slot = cp.byte_add(0x14 + 4 * j);
        let s = rd::<GPtr<u8>>(slot, 0);
        if s == ped {
            return cp;
        }
        if s.is_null() {
            wr::<GPtr<u8>>(slot, 0, ped);
            entity_register_reference(ped, slot);
            return cp;
        }
    }
    cp
}

/// `CCover::FindCoordinatesCoverPoint(CCoverPoint*, CPed*, const CVector& threat, CVector& out)` (0x699570, cdecl, AL):
/// false unless the ped holds one of the point's slots. Type 3: the point's position. Type 2 (an entity's box): the
/// cover points behind its bounding box away from `threat` (`FindCoverPointsBehindBox(list, threat, matrix, centre,
/// min, max, 1e6)`); exactly two are needed (else false), the one nearer to the ped (2D; the first distance squared is
/// rounded to a float before its root; a tie takes the second). Type 1: the entity's position moved away from `threat`
/// by `max(max.x, max.y) + 0.4` along the 2D direction threat - entity (normalised; a NaN compare takes max.y). Without
/// an entity, or another type: false.
#[hook(addr = 0x699570, conv = cdecl, name = "CCover::FindCoordinatesCoverPoint", expect = "8B 44 24 04 8B 50 14",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes,uninit-locals")]
pub fn find_coordinates_cover_point(cp: GPtr<u8>, ped: GPtr<u8>, threat: GPtr<u8>, out: GPtr<u8>) -> u8 {
    if rd::<GPtr<u8>>(cp, 0x18) != ped && rd::<GPtr<u8>>(cp, 0x14) != ped {
        return 0;
    }
    let copy = |src: GPtr<u8>| {
        for i in 0..3 {
            wr::<u32>(out, 4 * i, rd::<u32>(src, 4 * i));
        }
    };
    match rd::<i8>(cp, 0) {
        3 => {
            copy(cp.byte_add(4));
            1
        }
        2 => {
            let e = rd::<GPtr<u8>>(cp, 0x10);
            if e.is_null() {
                return 0;
            }
            let mut list = [0u32; 0x13C / 4];
            let lp: GPtr<u8> = GPtr::from_raw(list.as_mut_ptr()).cast();
            point_list_empty_b(lp);
            let mi = g::<GPtr<u8>>(0xA9_B0C8usize.wrapping_add((4 * i32::from(rd::<i16>(e, 0x22))) as isize as usize)).read();
            let col = rd::<GPtr<u8>>(mi, 0x14);
            find_cover_points_behind_box_b(lp, rd::<u32>(threat, 0), rd::<u32>(threat, 4), rd::<u32>(threat, 8),
                rd::<GPtr<u8>>(e, 0x14), col.byte_add(0x18), col, col.byte_add(0xC), 1.0e6);
            if rd::<u32>(lp, 0) != 2 {
                return 0;
            }
            let pp = pos_of(ped);
            let x0 = X87::from_f32(rd::<f32>(lp, 4)).fsub_m(rd::<f32>(pp, 0));
            let y0 = X87::from_f32(rd::<f32>(lp, 8)).fsub_m(rd::<f32>(pp, 4));
            let d0 = X87::from_f32(f32_of(y0.fmul(y0).fadd(x0.fmul(x0)))).fsqrt();
            let pp = pos_of(ped);
            let x1 = X87::from_f32(rd::<f32>(lp, 0x10)).fsub_m(rd::<f32>(pp, 0));
            let y1 = X87::from_f32(rd::<f32>(lp, 0x14)).fsub_m(rd::<f32>(pp, 4));
            let d1 = y1.fmul(y1).fadd(x1.fmul(x1)).fsqrt();
            copy(lp.byte_add(if gt(d1, d0) { 4 } else { 0x10 }));
            1
        }
        1 => {
            let e = rd::<GPtr<u8>>(cp, 0x10);
            if e.is_null() {
                return 0;
            }
            let mi = g::<GPtr<u8>>(0xA9_B0C8usize.wrapping_add((4 * i32::from(rd::<i16>(e, 0x22))) as isize as usize)).read();
            let col = rd::<GPtr<u8>>(mi, 0x14);
            let (mx, my) = (rd::<f32>(col, 0xC), rd::<f32>(col, 0x10));
            let r = f32_of(X87::from_f32(if gt(X87::from_f32(mx), X87::from_f32(my)) { mx } else { my }).fadd_m(gf(0x85_8EE8)));
            let ep = pos_of(e);
            let mut v = [0f32; 3];
            v[1] = f32_of(X87::from_f32(rd::<f32>(threat, 4)).fsub_m(rd::<f32>(ep, 4)));
            v[0] = f32_of(X87::from_f32(rd::<f32>(threat, 0)).fsub_m(rd::<f32>(ep, 0)));
            vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
            let a = X87::from_f32(v[0]).fmul_m(r);
            let b = X87::from_f32(v[1]).fmul_m(r);
            let cz = f32_of(X87::from_f32(v[2]).fmul_m(r));
            let ep = pos_of(e);
            let ox = f32_of(X87::from_f32(rd::<f32>(ep, 0)).fsub(a));
            let oy = f32_of(X87::from_f32(rd::<f32>(ep, 4)).fsub(b));
            let oz = f32_of(X87::from_f32(rd::<f32>(ep, 8)).fsub_m(cz));
            wr::<f32>(out, 0, ox);
            wr::<f32>(out, 4, oy);
            wr::<f32>(out, 8, oz);
            1
        }
        _ => 0,
    }
}

/// `CFormation::FindCoverPointsBehindBox(CPointList* out, CVector target, CMatrix* m, const CVector* centre (unused),
/// const CVector* min, const CVector* max, float cutoff)` (0x699FF0, cdecl). Nothing when `|target.z - m.pos.z|` >= 7.
/// The four corners `m * (min.x, min.y, 0)`, `(min.x, max.y)`, `(max.x, max.y)`, `(max.x, min.y)` and each one's 2D
/// distance to the target (a float); `k` = the nearest corner (strictly less, from corner 0). Every other corner `i`
/// that is not opposite to it (`(i - 2) & 3 != k`), closer to the target than `cutoff` while the list has room (24):
/// `p = corner(i) + normalise(corner((k - 2) & 3) - corner(i))` (2D direction), then `p += 0.75 * normalise(p -
/// target)` (z of the direction zeroed), each component rounded to a float, appended. gta-reversed measures the sides
/// (corner to corner) and steps along the side to the other corner; the bytes measure the corners' distance to the
/// target and step towards the corner opposite the nearest one.
#[hook(addr = 0x699FF0, conv = cdecl, name = "CFormation::FindCoverPointsBehindBox", expect = "D9 44 24 10 83 EC 70",
       unverified = "frontier2-8", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn find_cover_points_behind_box(out: GPtr<u8>, tx: u32, ty: u32, tz: u32, m: GPtr<u8>, _centre: GPtr<u8>,
    min: GPtr<u8>, max: GPtr<u8>, cutoff: f32) {
    let (tx, ty, tz) = (f32::from_bits(tx), f32::from_bits(ty), f32::from_bits(tz));
    let h = X87::from_f32(tz).fsub_m(rd::<f32>(m, 0x38));
    let h = if lt(h, X87::zero()) { h.fchs() } else { h };
    if !lt(h, X87::from_f32(gf(0x85_8F48))) {
        return;
    }
    let mut corners = [[0f32; 3]; 4];
    let xy = [(min, min), (min, max), (max, max), (max, min)];
    for (c, (xs, ys)) in corners.iter_mut().zip(xy) {
        let mut v = [rd::<f32>(xs, 0), rd::<f32>(ys, 4), 0.0];
        let mut r = [0f32; 3];
        let rp = matrix_mul_vec(GPtr::from_raw(r.as_mut_ptr()).cast(), m, GPtr::from_raw(v.as_mut_ptr()).cast());
        *c = [rd::<f32>(rp, 0), rd::<f32>(rp, 4), rd::<f32>(rp, 8)];
    }
    let mut dist = [0f32; 4];
    for (d, c) in dist.iter_mut().zip(corners.iter()) {
        let x = X87::from_f32(c[0]).fsub_m(tx);
        let y = X87::from_f32(c[1]).fsub_m(ty);
        *d = f32_of(y.fmul(y).fadd(x.fmul(x)).fsqrt());
    }
    let mut k = 0usize;
    let mut best = dist[0];
    for (i, &d) in dist.iter().enumerate().skip(1) {
        if lt(X87::from_f32(d), X87::from_f32(best)) {
            k = i;
            best = d;
        }
    }
    let o = corners[(k.wrapping_sub(2)) & 3];
    for (i, c) in corners.iter().enumerate() {
        if i == k || (i.wrapping_sub(2) & 3) == k || rd::<i32>(out, 0) >= 0x18 {
            continue;
        }
        if !lt(X87::from_f32(dist[i]), X87::from_f32(cutoff)) {
            continue;
        }
        let mut p = *c;
        let mut dir = [f32_of(X87::from_f32(o[0]).fsub_m(p[0])), f32_of(X87::from_f32(o[1]).fsub_m(p[1])), 0.0];
        vector_normalise(GPtr::from_raw(dir.as_mut_ptr()).cast());
        for j in 0..3 {
            p[j] = f32_of(X87::from_f32(dir[j]).fadd_m(p[j]));
        }
        let mut v = [f32_of(X87::from_f32(p[0]).fsub_m(tx)), f32_of(X87::from_f32(p[1]).fsub_m(ty)), 0.0];
        vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
        let k75 = gf(0x85_8F34);
        let az = f32_of(X87::from_f32(v[2]).fmul_m(k75));
        p[0] = f32_of(X87::from_f32(v[0]).fmul_m(k75).fadd_m(p[0]));
        p[1] = f32_of(X87::from_f32(v[1]).fmul_m(k75).fadd_m(p[1]));
        p[2] = f32_of(X87::from_f32(az).fadd_m(p[2]));
        let n = rd::<i32>(out, 0);
        if n >= 0x18 {
            continue;
        }
        let dst = out.byte_add(4 + 12 * n as isize);
        for (j, v) in p.iter().enumerate() {
            wr::<f32>(dst, 4 * j as isize, *v);
        }
        wr::<i32>(out, 0, n + 1);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_POINT_LIST_EMPTY,
    HOOK_DOES_COVER_POINT_STILL_PROVIDE_COVER,
    HOOK_FIND_AND_RESERVE_COVER_POINT,
    HOOK_FIND_COORDINATES_COVER_POINT,
    HOOK_FIND_COVER_POINTS_BEHIND_BOX,
];
