//! `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox` (the 6-argument overload, 0x5F3DD0, batch
//! `writerq-ai-1`): the corner route a ped takes round an entity's bounding rectangle. Ported from the 1.0 US bytes;
//! gta-reversed (`PedGeometryAnalyser.cpp`) was a reading aid only: the bytes differ from it in several places (kept
//! here, see the function's doc). Every x87 value that stays on the register stack is an `X87`, every value the
//! original stores to its frame is an `f32`, in the original's order. Unverified (docs/verify/HOWTO.md).
#![allow(
    clippy::unreadable_literal,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector* corners)` (0x5F1FA0)
    fn corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) -> u8 = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached(float z, const CVector* corners, CVector* normals,
    /// float* ds)` (0x5F1670)
    fn planes_uncached(z: u32, corners: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F1670 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxSegmentPlanesUncached(const CVector* corners, CVector& centre,
    /// CVector* normals, float* ds)` (0x5F1750)
    fn segment_planes_uncached(corners: GPtr<u8>, centre: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F1750 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, const CVector* normals, const float* ds)` (0x5F1450)
    fn hit_side(pos: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) -> i32 = cdecl 0x5F1450 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingSphere(const CPed&, CEntity&, CColSphere&)` (0x5F3C20)
    fn bounding_sphere(ped: GPtr<u8>, e: GPtr<u8>, s: GPtr<u8>) = cdecl 0x5F3C20 => unported;
    /// `CColSphere::IntersectRay(const CVector& start, const CVector& dir, CVector& p1, CVector& p2)` (0x40FF20, `ret 0x10`)
    fn sphere_intersect_ray(s: GPtr<u8>, start: GPtr<u8>, dir: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 = thiscall 0x40FF20 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8)` (0x56BA00)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32, shoot: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
}

/// `CPedGeometryAnalyser::ms_fPedNominalRadius`
const PED_NOMINAL_RADIUS: usize = 0x8D22B0;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

type V3 = [f32; 3];

fn p3(v: &mut V3) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn pa<T, const N: usize>(v: &mut [T; N]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn v3(p: GPtr<u8>) -> V3 {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn row(a: &[f32; 12], i: usize) -> V3 {
    [a[i * 3], a[i * 3 + 1], a[i * 3 + 2]]
}

/// `((v[o0]*n[o0] + v[o1]*n[o1]) + v[o2]*n[o2])` on the register stack (each product `fld` / `fmul m32`).
fn dot(v: V3, n: V3, o: [usize; 3]) -> X87 {
    x(v[o[0]])
        .fmul_m(n[o[0]])
        .fadd(x(v[o[1]]).fmul_m(n[o[1]]))
        .fadd(x(v[o[2]]).fmul_m(n[o[2]]))
}

/// The line `origin + dir * t` meets the plane (n, d): t = -((origin·n + d) / (dir·n)) (both dot products in the order
/// `o`); the point is (f32(dir.x*t + o.x), f32(f32(dir.y*t) + o.y), f32(f32(dir.z*t) + o.z)).
fn intersect(origin: V3, dir: V3, n: V3, d: f32, o: [usize; 3]) -> V3 {
    let t = dot(origin, n, o).fadd_m(d).fdiv(dot(dir, n, o)).fchs();
    let py = x(dir[1]).fmul(t).to_f32();
    let pz = x(dir[2]).fmul(t).to_f32();
    [
        x(dir[0]).fmul(t).fadd_m(origin[0]).to_f32(),
        x(py).fadd_m(origin[1]).to_f32(),
        x(pz).fadd_m(origin[2]).to_f32(),
    ]
}

/// The inlined box centre of the hit-side blocks: (f32((f32(c0.x + c1.x) + c2.x + c3.x) * 0.25),
/// f32((f32(c0.y + c1.y + c2.y) + c3.y) * 0.25), z).
fn hit_centre(c: &[f32; 12], z: f32) -> V3 {
    let x01 = x(c[0]).fadd_m(c[3]).to_f32();
    let y012 = x(c[1]).fadd_m(c[4]).fadd_m(c[7]).to_f32();
    let xs = x(x01).fadd_m(c[6]).fadd_m(c[9]);
    let ys = x(y012).fadd_m(c[10]);
    [xs.fmul_m(0.25).to_f32(), ys.fmul_m(0.25).to_f32(), z]
}

/// The inlined `ComputeEntityHitSide(pos, entity)`: corners at `pos.z` into a frame array, the centre
/// (`hit_centre`), the segment planes and `ComputeEntityHitSide(pos, normals, ds)`.
fn hit_side_at(pos: GPtr<u8>, e: GPtr<u8>) -> i32 {
    let z = rd::<f32>(pos, 8);
    let mut c = [0f32; 12];
    corners_uncached(z.to_bits(), e, pa(&mut c));
    let mut centre = hit_centre(&c, z);
    let mut n = [0f32; 12];
    let mut d = [0f32; 4];
    segment_planes_uncached(pa(&mut c), p3(&mut centre), pa(&mut n), pa(&mut d));
    hit_side(pos, pa(&mut n), pa(&mut d))
}

/// One candidate route checked for blockers (the `force == 0` loops 0x5F50C0 / 0x5F5390). Returns (clear, last
/// point index reached, blocking entity output).
fn check_route(pts: &[V3; 8], count: i32, centre: V3, ped: GPtr<u8>, e: GPtr<u8>) -> (bool, i32, u32) {
    let mut ent = 0u32;
    let mut cp = [0u32; 0x2C / 4];
    let mut k = 1;
    while k < count {
        let mut prev = pts[k as usize - 1];
        let mut cur = pts[k as usize];
        let mut dir = [
            x(cur[0]).fsub_m(prev[0]).to_f32(),
            x(cur[1]).fsub_m(prev[1]).to_f32(),
            0.0,
        ];
        vector_normalise(p3(&mut dir));
        let ox = x(dir[1]).fmul_m(0.5).to_f32();
        let oy = x(dir[0]).fmul_m(0.5).fchs().to_f32();
        let mut off = [ox, oy, 0.0];
        let s = x(cur[0])
            .fsub_m(centre[0])
            .fmul_m(off[0])
            .fadd(x(cur[1]).fsub_m(centre[1]).fmul_m(off[1]))
            .fadd(x(cur[2]).fsub_m(centre[2]).fmul_m(0.0));
        if !s.fcom(X87::zero()).test_parity(0x05) {
            for v in &mut off {
                *v = x(*v).fmul_m(-1.0).to_f32();
            }
        }
        let mut a = [
            x(prev[0]).fadd_m(off[0]).to_f32(),
            x(prev[1]).fadd_m(off[1]).to_f32(),
            x(prev[2]).fadd_m(off[2]).to_f32(),
        ];
        let mut b = [
            x(off[0]).fadd_m(cur[0]).to_f32(),
            x(off[1]).fadd_m(cur[1]).to_f32(),
            x(off[2]).fadd_m(cur[2]).to_f32(),
        ];
        let entp: GPtr<u8> = GPtr::from_raw(&raw mut ent).cast();
        let hit = process_line_of_sight(
            p3(&mut prev),
            p3(&mut cur),
            pa(&mut cp),
            entp,
            1,
            1,
            1,
            1,
            0,
            0,
            0,
            0,
        ) != 0
            || process_line_of_sight(p3(&mut a), p3(&mut b), pa(&mut cp), entp, 1, 1, 1, 1, 0, 0, 0, 0) != 0;
        if hit {
            let h: GPtr<u8> = GPtr::from_addr(ent as usize);
            if h == e || h == ped {
                ent = 0;
            } else if rd::<u8>(h, 0x36) & 7 == 3 && rd::<u32>(h, 0x534) != 1 {
                if rd::<u32>(h, 0x14) == 0 {
                    placeable_allocate_matrix(h);
                    simple_transform_update_matrix(h.byte_add(4), rd::<GPtr<u8>>(h, 0x14));
                }
                let f = rd::<GPtr<u8>>(h, 0x14).byte_add(0x10);
                let dz = x(cur[2]).fsub_m(prev[2]).fmul_m(rd::<f32>(f, 8));
                let dy = x(cur[1]).fsub_m(prev[1]).fmul_m(rd::<f32>(f, 4));
                let dx = x(cur[0]).fsub_m(prev[0]).fmul_m(rd::<f32>(f, 0));
                if !dz.fadd(dy).fadd(dx).fcom(X87::zero()).test_parity(0x05) {
                    return (false, k, ent);
                }
            } else {
                return (false, k, ent);
            }
        }
        k += 1;
    }
    (true, count, ent)
}

/// Register sum of the segment lengths `sqrt((dz*dz + dy*dy) + dx*dx)` of `pts[..count]`.
fn route_length(pts: &[V3; 8], count: i32) -> X87 {
    let mut acc = X87::zero();
    for i in 1..count.max(1) as usize {
        let dx = x(pts[i][0]).fsub_m(pts[i - 1][0]);
        let dy = x(pts[i][1]).fsub_m(pts[i - 1][1]);
        let dz = x(pts[i][2]).fsub_m(pts[i - 1][2]);
        acc = acc.fadd(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt());
    }
    acc
}

/// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed& ped, const CVector& from, CEntity& entity,
/// const CVector& to, CPointRoute& route, int32 force)` (0x5F3DD0, cdecl). Returns 0 (no route), 1 (left) or 2
/// (right). x87 throughout.
///
/// route.count = 0. With `ms_fPedNominalRadius` set to 0.175: corners at the ped's z into frame array A (discarded
/// later), corners again and the box planes (`PlanesUncached`); the radius is restored. A plane (n, d) with
/// `((n.z*p.z + n.y*p.y) + p.x*n.x) + d` above 0 for both `from` and `to` → 0. With the radius restored: corners at
/// the ped's z, the inlined centre, the segment planes; equal `ComputeEntityHitSide` for `from` and `to` → 0. Then
/// corners A and the planes again (restored radius).
///
/// Pass 1 (F = from, T = to, dir = normalised f32(T - F), origin = F): per plane, s = sign of `((p.y*n.y + p.x*n.x) +
/// p.z*n.z) + d` (1 when the register value is above 0.2, -1 when its f32 is below -0.2, else 0) for F and T. sF = -1:
/// T on the plane's line through origin/dir when sT = 1. sT = -1: F likewise when sF = 1. Neither -1: a flag is set and
/// each point with sign 0 moves by n * (0.2 - its f32 value). Without the flag: the 2D f32(to - from) of the original
/// arguments has length 0 → 0; else normalised, and `CColSphere::IntersectRay` of the entity's bounding sphere from
/// `from` along it writes F and T (miss → 0).
///
/// Pass 2 (origin = F, dir = normalised f32(T - F)): per plane, the register value of F and the f32 value of T
/// (plane 0 in the order (y + z) + x, the others (x + y) + z). Both exactly 0: skipped. F not below 0 and T not above 0
/// (unordered counts as above): F moves onto the plane, fromPlane = i. Else F not above 0 and T not below 0: T moves,
/// toPlane = i. Either still -1 → 0.
///
/// Routes: left = from, corners A[(fromPlane + 3) % 4], then decreasing indices down to and including toPlane, to;
/// right = from, A[fromPlane], increasing up to and including (toPlane + 3) % 4, to. `force` 1 → left, any other
/// non-zero → right. `force` 0: each route is checked segment by segment against the centre of A (inlined, f32 steps
/// as stored): offset = (f32(dir.y*0.5), f32(-(dir.x*0.5)), 0) of the normalised 2D segment, negated when it points
/// towards the centre; `ProcessLineOfSight(prev, cur)`, and when clear `ProcessLineOfSight(prev + offset, cur +
/// offset)` (buildings, vehicles, peds, objects). A hit on the entity or the ped clears the output and goes on to the
/// next segment; a moving ped (move state not 1) whose forward vector has a non-negative dot product with (cur - prev)
/// goes on; anything else blocks. Both clear: the shorter route (left only when strictly shorter); one clear: that
/// one; else left blocked at segment 1 and right later → right, right at 1 and left later → left, the same blocker →
/// the shorter, else the route whose blocker has the smaller collision radius (left when the right's is larger).
///
/// The chosen route's corners (not from / to) are added to `route` (at most 8), then `to`. With more than one point:
/// when the inlined hit side of `from` equals that of route.points\[1\] (not points\[0\]), the first point is
/// removed. Finally the last point is dropped (count - 1, also when the route has at most one point).
///
/// Differences from gta-reversed kept from the bytes: the offset is perpendicular to the segment; the first line of
/// sight runs prev → cur at their own heights; a hit on the entity / ped skips the offset check; zero dot products
/// count as on the side being tested in pass 2; the pop-front compares route.points\[1\].
#[hook(addr = 0x5F3DD0, conv = cdecl, name = "CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox-2",
       expect = "81 EC F0 01 00 00", unverified = "writerq-ai-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk", note = "writes the caller's CPointRoute (task memory) and restores ms_fPedNominalRadius")]
pub fn compute_route_round_entity_bounding_box(
    ped: GPtr<u8>,
    from: GPtr<u8>,
    e: GPtr<u8>,
    to: GPtr<u8>,
    route: GPtr<u8>,
    force: i32,
) -> i32 {
    wr::<u32>(route, 0, 0);
    let radius = g::<u32>(PED_NOMINAL_RADIUS);
    let saved = radius.read();
    radius.write(0x3E33_3333);
    let ped_pos = || {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            ped.byte_add(4)
        } else {
            m.byte_add(0x30)
        }
    };
    let pz = rd::<f32>(ped_pos(), 8);
    let mut ca = [0f32; 12];
    corners_uncached(x(pz).to_f32().to_bits(), e, pa(&mut ca));
    let z = rd::<u32>(ped_pos(), 8);
    let mut cb = [0f32; 12];
    corners_uncached(z, e, pa(&mut cb));
    let mut normals = [0f32; 12];
    let mut ds = [0f32; 4];
    planes_uncached(z, pa(&mut cb), pa(&mut normals), pa(&mut ds));
    radius.write(saved);

    let fv = v3(from);
    let tv = v3(to);
    for i in 0..4 {
        let n = row(&normals, i);
        if !dot(fv, n, [2, 1, 0])
            .fadd_m(ds[i])
            .fcom(X87::zero())
            .test_any(0x41)
            && !dot(tv, n, [2, 1, 0])
                .fadd_m(ds[i])
                .fcom(X87::zero())
                .test_any(0x41)
        {
            return 0;
        }
    }

    let z = rd::<u32>(ped_pos(), 8);
    let mut cc = [0f32; 12];
    corners_uncached(z, e, pa(&mut cc));
    let mut centre = hit_centre(&cc, f32::from_bits(z));
    let mut sn = [0f32; 12];
    let mut sd = [0f32; 4];
    segment_planes_uncached(pa(&mut cc), p3(&mut centre), pa(&mut sn), pa(&mut sd));
    let side_from = hit_side(from, pa(&mut sn), pa(&mut sd));
    if side_from == hit_side(to, pa(&mut sn), pa(&mut sd)) {
        return 0;
    }

    let pz = rd::<f32>(ped_pos(), 8);
    corners_uncached(x(pz).to_f32().to_bits(), e, pa(&mut ca));
    let z = rd::<u32>(ped_pos(), 8);
    corners_uncached(z, e, pa(&mut cb));
    planes_uncached(z, pa(&mut cb), pa(&mut normals), pa(&mut ds));

    // Pass 1.
    let mut fp = fv;
    let mut tp = tv;
    let mut dir = [
        x(tp[0]).fsub_m(fp[0]).to_f32(),
        x(tp[1]).fsub_m(fp[1]).to_f32(),
        x(tp[2]).fsub_m(fp[2]).to_f32(),
    ];
    vector_normalise(p3(&mut dir));
    let origin = fp;
    let mut flag = false;
    let sign = |v: X87| -> (i32, f32) {
        let f = v.to_f32();
        if !v.fcom(x(0.2)).test_any(0x41) {
            (1, f)
        } else if !x(f).fcom(x(-0.2)).test_parity(0x05) {
            (-1, f)
        } else {
            (0, f)
        }
    };
    for i in 0..4 {
        let n = row(&normals, i);
        let (sf, df) = sign(dot(fp, n, [1, 0, 2]).fadd_m(ds[i]));
        let (st, dt) = sign(dot(tp, n, [1, 0, 2]).fadd_m(ds[i]));
        if sf == -1 {
            if st == 1 {
                tp = intersect(origin, dir, n, ds[i], [1, 0, 2]);
            }
        } else if st == -1 {
            if sf == 1 {
                fp = intersect(origin, dir, n, ds[i], [1, 0, 2]);
            }
        } else {
            flag = true;
            let nudge = |p: &mut V3, d: f32| {
                let t = x(0.2).fsub_m(d);
                let my = t.fmul_m(n[1]).to_f32();
                let mz = t.fmul_m(n[2]).to_f32();
                p[0] = t.fmul_m(n[0]).fadd_m(p[0]).to_f32();
                p[1] = x(my).fadd_m(p[1]).to_f32();
                p[2] = x(mz).fadd_m(p[2]).to_f32();
            };
            if sf == 0 {
                nudge(&mut fp, df);
            }
            if st == 0 {
                nudge(&mut tp, dt);
            }
        }
    }
    if !flag {
        let mut sphere = [0u32; 5];
        bounding_sphere(ped, e, pa(&mut sphere));
        let mut d2 = [
            x(tv[0]).fsub_m(fv[0]).to_f32(),
            x(tv[1]).fsub_m(fv[1]).to_f32(),
            0.0,
        ];
        let len = x(d2[0]).fmul_m(d2[0]).fadd(x(d2[1]).fmul_m(d2[1])).fsqrt();
        if !len.fcom(X87::zero()).test_parity(0x44) {
            return 0;
        }
        vector_normalise(p3(&mut d2));
        let mut start = fv;
        if sphere_intersect_ray(
            pa(&mut sphere),
            p3(&mut start),
            p3(&mut d2),
            p3(&mut fp),
            p3(&mut tp),
        ) == 0
        {
            return 0;
        }
    }

    // Pass 2.
    let origin = fp;
    let mut dir = [
        x(tp[0]).fsub_m(fp[0]).to_f32(),
        x(tp[1]).fsub_m(fp[1]).to_f32(),
        x(tp[2]).fsub_m(fp[2]).to_f32(),
    ];
    vector_normalise(p3(&mut dir));
    let mut from_plane = -1i32;
    let mut to_plane = -1i32;
    for i in 0..4 {
        let n = row(&normals, i);
        let (od, oi) = if i == 0 {
            ([1, 2, 0], [2, 1, 0])
        } else {
            ([0, 1, 2], [0, 2, 1])
        };
        let df = dot(fp, n, od).fadd_m(ds[i]);
        let dt = x(dot(tp, n, od).fadd_m(ds[i]).to_f32());
        let cf = df.fcom(X87::zero());
        let ct = dt.fcom(X87::zero());
        if !cf.test_parity(0x44) && !ct.test_parity(0x44) {
            continue;
        }
        if !cf.test_any(0x01) && !ct.test_parity(0x41) {
            from_plane = i as i32;
            fp = intersect(origin, dir, n, ds[i], oi);
        } else if !cf.test_parity(0x41) && !ct.test_any(0x01) {
            to_plane = i as i32;
            tp = intersect(origin, dir, n, ds[i], oi);
        }
    }
    if to_plane < 0 || from_plane < 0 {
        return 0;
    }

    let m4 = |v: i32| v.wrapping_rem(4);
    let corner = |k: i32| row(&ca, k as usize);
    let (lo_to, to4) = (m4(to_plane + 3), m4(to_plane + 4));
    let mut left = [[0f32; 3]; 8];
    let mut nl = 0usize;
    left[0] = fv;
    let mut k = m4(from_plane + 3);
    left[1] = corner(k);
    nl += 2;
    while k != to4 {
        k = m4(k + 3);
        left[nl] = corner(k);
        nl += 1;
    }
    left[nl] = tv;
    nl += 1;
    let mut right = [[0f32; 3]; 8];
    let mut nr = 0usize;
    right[0] = fv;
    let mut k = m4(from_plane + 4);
    right[1] = corner(k);
    nr += 2;
    while k != lo_to {
        k = m4(k + 5);
        right[nr] = corner(k);
        nr += 1;
    }
    right[nr] = tv;
    nr += 1;
    let (nl, nr) = (nl as i32, nr as i32);

    let pick = if force == 0 {
        let x01 = x(ca[0]).fadd_m(ca[3]).to_f32();
        let x012 = x(x01).fadd_m(ca[6]).to_f32();
        let y012 = x(ca[1]).fadd_m(ca[4]).fadd_m(ca[7]).to_f32();
        let zs = x(ca[2]).fadd_m(ca[5]).fadd_m(ca[8]).fadd_m(ca[11]).to_f32();
        let ys = x(y012).fadd_m(ca[10]).to_f32();
        let centre = [
            x(x012).fadd_m(ca[9]).fmul_m(0.25).to_f32(),
            x(ys).fmul_m(0.25).to_f32(),
            x(zs).fmul_m(0.25).to_f32(),
        ];
        let (clear_l, last_l, ent_l) = check_route(&left, nl, centre, ped, e);
        let (clear_r, last_r, ent_r) = check_route(&right, nr, centre, ped, e);
        let len_l = route_length(&left, nl);
        let len_r = route_length(&right, nr);
        let shorter = || if len_l.fcom(len_r).test_parity(0x05) { 2 } else { 1 };
        let by_blocker = || {
            if ent_l == ent_r {
                return shorter();
            }
            let radius = |ent: u32| {
                let mi = g::<GPtr<u8>>(
                    MODEL_INFO_PTRS
                        + (i32::from(rd::<i16>(GPtr::from_addr(ent as usize), 0x22)) * 4) as usize,
                )
                .read();
                rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)
            };
            let (rl, rr) = (radius(ent_l), radius(ent_r));
            if x(rr).fcom(x(rl)).test_any(0x41) { 2 } else { 1 }
        };
        if clear_l {
            if clear_r { shorter() } else { 1 }
        } else if clear_r {
            2
        } else if last_l == 1 {
            if last_r > 1 { 2 } else { by_blocker() }
        } else if last_l > 1 && last_r == 1 {
            1
        } else {
            by_blocker()
        }
    } else if force == 1 {
        1
    } else {
        2
    };

    let (pts, n) = if pick == 1 { (&left, nl) } else { (&right, nr) };
    let add = |p: V3| {
        let c = rd::<i32>(route, 0);
        if c < 8 {
            let at = route.byte_add(4 + 12 * c as isize);
            for (j, v) in p.iter().enumerate() {
                wr::<f32>(at, 4 * j as isize, *v);
            }
            wr::<i32>(route, 0, c + 1);
        }
    };
    if n - 1 > 1 {
        for i in 1..(n - 1) as usize {
            add(pts[i]);
        }
    }
    add(v3(to));
    if rd::<i32>(route, 0) > 1 {
        let side = hit_side_at(from, e);
        if side == hit_side_at(route.byte_add(0x10), e) {
            let mut i = 0;
            while i < rd::<i32>(route, 0) - 1 {
                let src = route.byte_add(4 + 12 * (i as isize + 1));
                let dst = route.byte_add(4 + 12 * i as isize);
                for j in 0..3 {
                    wr::<u32>(dst, 4 * j, rd::<u32>(src, 4 * j));
                }
                i += 1;
            }
            wr::<i32>(route, 0, rd::<i32>(route, 0) - 1);
        }
    }
    wr::<i32>(route, 0, rd::<i32>(route, 0) - 1);
    pick
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COMPUTE_ROUTE_ROUND_ENTITY_BOUNDING_BOX];
