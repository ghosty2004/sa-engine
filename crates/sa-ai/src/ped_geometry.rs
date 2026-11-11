//! `CPedGeometryAnalyser::{IsInAir, LiesInsideBoundingBox}` (batch `mvpi-ai-1`). Ported from the 1.0 US bytes; unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use sa_math::x87::Cmp;

use crate::common::*;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn intel_get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    fn intel_get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float z, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity* ignore, bool x6)` (0x569E20)
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector* corners)` (0x5F1FA0)
    fn compute_entity_bounding_box_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached(float z, const CVector* corners, CVector* normals, float* ds)` (0x5F1670)
    fn compute_entity_bounding_box_planes_uncached(z: u32, corners: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F1670 => unported;
    fn world_test_sphere_against_world(cx: u32, cy: u32, cz: u32, r: u32, ignore: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, c: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
}

/// `CPedGeometryAnalyser::IsInAir(const CPed&)` (0x5F1CB0, cdecl). Not in air (AL = 0, EAX's upper
/// bytes those of the last value tested, reproduced) with ped flag +0x46C bit 8, or, with an active
/// task, a swim task, a jetpack task or a simplest active task of type 0xFE. Else "falling" = the
/// active task's type is 0xD3; a vertical line from the ped's position down to z - 1.5 (x87, stored as
/// f32; buildings, vehicles, objects) hits, or, when it does not and the ped is not falling, a sphere
/// of radius 0.15 at (x, y, z - 1.0) against buildings (ignoring the ped) hits: EAX = 0, else EAX = 1.
///
/// Stack: the original keeps "falling" in the low byte of its own `ped` argument slot (S+4); not
/// reproduced. Its colpoint and entity outputs are stack locals (zero-initialised here).
#[hook(addr = 0x5f1cb0, conv = cdecl, name = "CPedGeometryAnalyser::IsInAir", expect = "83 EC 40 53 56",
       unverified = "mvpi-ai-1", route = "L2", hazards = "stack-residue,uninit-locals", recs = "grove-walk")]
pub fn is_in_air(ped: GPtr<u8>) -> u32 {
    let fl = rd::<u32>(ped, 0x46C);
    if fl & 0x100 != 0 {
        return fl & 0xFFFF_FF00;
    }
    let tm = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(4);
    if !tm_get_active_task(tm(ped)).is_null() {
        let t = intel_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C));
        if !t.is_null() {
            return t.addr() as u32 & 0xFFFF_FF00;
        }
        let t = intel_get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C));
        if !t.is_null() {
            return t.addr() as u32 & 0xFFFF_FF00;
        }
        let ty = task_get_type(tm_get_simplest_active_task(tm(ped)));
        if ty == 0xFE {
            return 0;
        }
    }
    let mut falling = false;
    if !tm_get_active_task(tm(ped)).is_null() && task_get_type(tm_get_active_task(tm(ped))) == 0xD3 {
        falling = true;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p: GPtr<f32> = if m.is_null() { ped.byte_add(4).cast() } else { m.byte_add(0x30).cast() };
    let mut pos = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
    let z_end = x(p.wrapping_index(2).read()).fsub_m(1.5).to_f32();
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = 0u32;
    let mut hit = world_process_vertical_line(
        GPtr::from_raw(pos.as_mut_ptr()).cast(),
        z_end.to_bits(),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&raw mut ent).cast(),
        1,
        1,
        0,
        1,
        0,
        0,
        0,
    ) != 0;
    if !hit && !falling {
        pos[2] = x(pos[2]).fsub_m(1.0).to_f32();
        let e = world_test_sphere_against_world(
            pos[0].to_bits(),
            pos[1].to_bits(),
            pos[2].to_bits(),
            0x3E19_999A,
            ped,
            1,
            0,
            0,
            0,
            0,
            0,
        );
        if !e.is_null() {
            hit = true;
        }
    }
    u32::from(!hit)
}

fn ent_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

/// `CPedGeometryAnalyser::LiesInsideBoundingBox(const CPed& ped, const CVector& pos, CEntity& entity)`
/// (0x5F3880, cdecl). x87: outside the entity's bounding sphere (colmodel +0x24 radius; `r*r` not
/// greater than `dz*dz + dy*dy + dx*dx`, d = pos - entity position) → AL = 0. Else the box corners
/// and planes at the ped's z (`ComputeEntityBoundingBoxCornersUncached`,
/// `ComputeEntityBoundingBoxPlanesUncached`, stack arrays); AL = 1 at the first of the four planes with
/// `n.x*p.x + n.z*p.z + p.y*n.y + d` below 0, else 0. AH holds the FPU status word in the original
/// (`fnstsw ax`).
#[hook(addr = 0x5f3880, conv = cdecl, name = "CPedGeometryAnalyser::LiesInsideBoundingBox", expect = "8B 4C 24 0C 8B 41 14",
       unverified = "mvpi-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn lies_inside_bounding_box(ped: GPtr<u8>, pos: GPtr<f32>, e: GPtr<u8>) -> u8 {
    let ep = ent_pos(e);
    let c = |i: i32| x(pos.wrapping_index(i).read()).fsub_m(ep.wrapping_index(i).read());
    let (dx, dy, dz) = (c(0), c(1), c(2));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + (i32::from(rd::<i16>(e, 0x22)) as isize * 4) as usize).read();
    let r = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24));
    let sum = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !matches!(r.fmul(r).fcom(sum), Cmp::Greater) {
        return 0;
    }
    let z = rd::<u32>(ent_pos(ped).cast::<u8>(), 8);
    let mut corners = [0f32; 12];
    let mut normals = [0f32; 12];
    let mut ds = [0f32; 4];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    compute_entity_bounding_box_corners_uncached(z, e, cp);
    compute_entity_bounding_box_planes_uncached(
        z,
        cp,
        GPtr::from_raw(normals.as_mut_ptr()).cast(),
        GPtr::from_raw(ds.as_mut_ptr()).cast(),
    );
    let (px, py, pz) = (pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read());
    for i in 0..4 {
        let n = &normals[i * 3..i * 3 + 3];
        let v = x(n[0]).fmul_m(px).fadd(x(n[2]).fmul_m(pz)).fadd(x(py).fmul_m(n[1])).fadd_m(ds[i]);
        if matches!(v.fcom(X87::zero()), Cmp::Less) {
            return 1;
        }
    }
    0
}

/// `FLT_MAX` (0x863A3C)
const K_FLT_MAX: usize = 0x863A3C;
/// 0.35f (0x8D22B0): the clipping epsilon / clear-target margin and step
const K_CLEAR_MARGIN: usize = 0x8D22B0;
/// 5.0f (0x86C6A4): the scanner-entity and clear-target radius
const K_CLEAR_RADIUS: usize = 0x86C6A4;
/// `CWorld::ms_iProcessLineNumCrossings` (0xB7CD60)
const PROCESS_LINE_NUM_CROSSINGS: usize = 0xB7CD60;

orig! {
    /// CRT `floor(double)` (0x8219F0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8)` (0x56BA00)
    fn world_process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32, shoot: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool waves, CVector* normal)` (0x6EB690)
    fn water_get_water_level(px: u32, py: u32, pz: u32, level: GPtr<u8>, waves: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingSphere(const CPed&, CEntity&, CColSphere&)` (0x5F3C20)
    fn compute_entity_bounding_sphere(ped: GPtr<u8>, e: GPtr<u8>, s: GPtr<u8>) = cdecl 0x5F3C20 => unported;
    /// `CColSphere::IntersectRay(const CVector& start, const CVector& dir, CVector& p1, CVector& p2)` (0x40FF20, `ret 0x10`)
    fn col_sphere_intersect_ray(s: GPtr<u8>, start: GPtr<u8>, dir: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 = thiscall 0x40FF20 => unported;
}

fn fp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `CPedGeometryAnalyser::ComputeClosestSurfacePoint(const CVector& pos, const CVector corners[4], CVector& out)`
/// (0x5F2C10, cdecl). x87, best = `FLT_MAX` (kept in a register; f32 after the first update). For each edge
/// a = corners[i], b = corners[(i+1)&3]: d = b - a, len = f32 sqrt(dz*f32(dz) + dy*dy + dx*dx), inv = f32
/// (1/len), u = f32(d*inv) (z from the stored f32 dz); t = (p - a)·u (z, y, x order); with t not below 0
/// and f32 t not above len: q = (f32(u.x*t + a.x), f32(u.y*t + a.y), f32(f32(u.z*t) + a.z)), and when the
/// register |p - q|² is below best, best = f32 of it, out = q, found. Without an edge point, the four
/// corners in order: |c - p|² below best → best = its f32, out = c. Returns AL = found (EAX's upper bytes
/// are left from the FPU status / index arithmetic in the original).
#[hook(addr = 0x5f2c10, conv = cdecl, name = "CPedGeometryAnalyser::ComputeClosestSurfacePoint-rect",
       expect = "D9 05 3C 3A 86 00", unverified = "mvpj-ai-2", route = "L2", hazards = "x87-transcendental,ax-al-return",
       recs = "grove-walk")]
pub fn compute_closest_surface_point_rect(pos: GPtr<f32>, corners: GPtr<f32>, out: GPtr<f32>) -> u8 {
    let p = |i: i32| pos.wrapping_index(i).read();
    let c = |k: i32, i: i32| corners.wrapping_index(k * 3 + i).read();
    let mut best = x(g::<f32>(K_FLT_MAX).read());
    let mut found = false;
    for i in 0..4 {
        let (a, b) = (i, (i + 1) & 3);
        let dx = x(c(b, 0)).fsub_m(c(a, 0));
        let dy = x(c(b, 1)).fsub_m(c(a, 1));
        let dz = x(c(b, 2)).fsub_m(c(a, 2));
        let dzf = dz.to_f32();
        let len = dz.fmul_m(dzf).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let inv = x(1.0).fdiv_m(len).to_f32();
        let ux = dx.fmul_m(inv).to_f32();
        let uy = dy.fmul_m(inv).to_f32();
        let uz = x(dzf).fmul_m(inv).to_f32();
        let t = x(p(2))
            .fsub_m(c(a, 2))
            .fmul_m(uz)
            .fadd(x(p(1)).fsub_m(c(a, 1)).fmul_m(uy))
            .fadd(x(p(0)).fsub_m(c(a, 0)).fmul_m(ux));
        let tf = t.to_f32();
        if t.fcom(X87::zero()).c0() {
            continue;
        }
        if x(tf).fcom(x(len)).test_parity(0x41) {
            continue;
        }
        let qx = x(ux).fmul_m(tf).fadd_m(c(a, 0)).to_f32();
        let qy = x(uy).fmul_m(tf).fadd_m(c(a, 1)).to_f32();
        let qz = x(x(uz).fmul_m(tf).to_f32()).fadd_m(c(a, 2)).to_f32();
        let ex = x(p(0)).fsub_m(qx);
        let ey = x(p(1)).fsub_m(qy);
        let ez = x(p(2)).fsub_m(qz);
        let dsq = ez.fmul(ez).fadd(ey.fmul(ey)).fadd(ex.fmul(ex));
        if !dsq.fcom(best).test_parity(0x05) {
            best = x(dsq.to_f32());
            out.write(qx);
            out.wrapping_index(1).write(qy);
            out.wrapping_index(2).write(qz);
            found = true;
        }
    }
    if found {
        return 1;
    }
    for k in 0..4 {
        let ex = x(c(k, 0)).fsub_m(p(0));
        let ey = x(c(k, 1)).fsub_m(p(1));
        let ez = x(c(k, 2)).fsub_m(p(2));
        let dsq = ez.fmul(ez).fadd(ey.fmul(ey)).fadd(ex.fmul(ex));
        if !dsq.fcom(best).test_parity(0x05) {
            best = x(dsq.to_f32());
            for i in 0..3 {
                out.wrapping_index(i).write(c(k, i));
            }
            found = true;
        }
    }
    u8::from(found)
}

/// `CPedGeometryAnalyser::IsWanderPathClear(const CVector& a, const CVector& b, float maxHeightDiff, int32
/// maxSteps)` (0x5F2F70, cdecl). x87. |a.z - b.z| above `maxHeightDiff` → 0. z = a.z when below b.z, else
/// b.z; `CWorld::GetIsLineOfSightClear((a.x, a.y, z), (b.x, b.y, z), buildings only)` false → 1. d = f32(b -
/// a); n = `_ftol(floor(sqrt(dz*f32(dz) + dy²+dx²)))` (f32 d.y/d.x squared from memory), capped at
/// `maxSteps`; 0 → 4. d normalised (`CVector::Normalise`). Steps i = 1..n-1: p = (f32(d.x*i + a.x),
/// f32(f32(d.y*i) + a.y), f32(f32(d.z*i) + a.z)); over water (`GetWaterLevel(p, &level, false, NULL)`) a
/// vertical line from (p.x, p.y, level) down to the higher of a.z/b.z (buildings) must hit, else 2 (its
/// entity output is the loop-counter stack slot, zeroed first). Then a vertical line from a down to a.z - 5
/// (buildings) must hit, else 3; h = f32(hit z + 0.5); steps j = 1..n-1: from (f32(d.x*j + a.x),
/// f32(f32(d.y*j) + a.y), h) down to f32(h - 2) must hit with |hit z - h| not above 1, else 3 (this
/// loop's entity output is not re-zeroed); h = f32(hit z + 0.5). → 4.
#[hook(addr = 0x5f2f70, conv = cdecl, name = "CPedGeometryAnalyser::IsWanderPathClear", expect = "81 EC 98 00 00 00",
       unverified = "mvpj-ai-2", route = "L2", hazards = "x87-transcendental", recs = "grove-walk")]
pub fn is_wander_path_clear(a: GPtr<f32>, b: GPtr<f32>, max_height_diff: f32, max_steps: i32) -> u32 {
    let av = |i: i32| a.wrapping_index(i).read();
    let bv = |i: i32| b.wrapping_index(i).read();
    if !x(av(2)).fsub_m(bv(2)).fabs().fcom(x(max_height_diff)).test_any(0x41) {
        return 0;
    }
    let z = if x(av(2)).fcom(x(bv(2))).test_parity(0x05) { bv(2) } else { av(2) };
    let mut from = [av(0), av(1), z];
    let mut to = [bv(0), bv(1), z];
    if world_get_is_line_of_sight_clear(fp(&mut from), fp(&mut to), 1, 0, 0, 0, 0, 0, 0) == 0 {
        return 1;
    }
    let dz = x(bv(2)).fsub_m(av(2));
    let mut d = [x(bv(0)).fsub_m(av(0)).to_f32(), x(bv(1)).fsub_m(av(1)).to_f32(), dz.to_f32()];
    let len = dz.fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0])).fsqrt();
    let mut n = ftol32(X87::from_f64_exact(crt_floor(len.to_f64())));
    if n > max_steps {
        n = max_steps;
    }
    if n == 0 {
        return 4;
    }
    vector_normalise(fp(&mut d));
    let mut cp = [0u32; 0x2C / 4];
    let mut i = 1;
    while i < n {
        let fi = xi(i);
        let px = x(d[0]).fmul(fi).fadd_m(av(0)).to_f32();
        let py = x(x(d[1]).fmul(fi).to_f32()).fadd_m(av(1)).to_f32();
        let pz = x(x(d[2]).fmul(fi).to_f32()).fadd_m(av(2)).to_f32();
        let mut level = 0f32;
        if water_get_water_level(px.to_bits(), py.to_bits(), pz.to_bits(), fp(&mut level), 0, 0) != 0 {
            let z_end = if x(av(2)).fcom(x(bv(2))).test_any(0x41) { bv(2) } else { av(2) };
            let mut pos = [px, py, level];
            let mut ent = 0u32;
            if world_process_vertical_line(fp(&mut pos), z_end.to_bits(), fp(&mut cp), fp(&mut ent), 1, 0, 0, 0, 0, 0, 0) == 0 {
                return 2;
            }
        }
        i += 1;
    }
    let mut ent = 0u32;
    let z_end = x(av(2)).fsub_m(5.0).to_f32();
    if world_process_vertical_line(a.cast(), z_end.to_bits(), fp(&mut cp), fp(&mut ent), 1, 0, 0, 0, 0, 0, 0) == 0 {
        return 3;
    }
    let hit_z = |cp: &[u32; 11]| f32::from_bits(cp[2]);
    let mut h = x(hit_z(&cp)).fadd_m(0.5).to_f32();
    let mut j = 1;
    while j < n {
        let fj = xi(j);
        let px = x(d[0]).fmul(fj).fadd_m(av(0)).to_f32();
        let py = x(x(d[1]).fmul(fj).to_f32()).fadd_m(av(1)).to_f32();
        let mut pos = [px, py, h];
        let z_end = x(h).fsub_m(2.0).to_f32();
        if world_process_vertical_line(fp(&mut pos), z_end.to_bits(), fp(&mut cp), fp(&mut ent), 1, 0, 0, 0, 0, 0, 0) == 0 {
            return 3;
        }
        if !x(hit_z(&cp)).fsub_m(h).fabs().fcom(X87::one()).test_any(0x41) {
            return 3;
        }
        h = x(hit_z(&cp)).fadd_m(0.5).to_f32();
        j += 1;
    }
    4
}

/// `CPedGeometryAnalyser::GetIsLineOfSightClear(const CPed& ped, const CVector& target, CEntity& entity, float&
/// dist)` (stub 0x5F5A30, cdecl; `nop; jmp` to the frame setup). x87. P = the ped's position, T = target; the
/// entity's bounding sphere (`ComputeEntityBoundingSphere`); dir = f32(T - P), `NormaliseAndMag`. The ray
/// misses the sphere (`CColSphere::IntersectRay`) → AL = 1. Else the box corners and planes at the ped's z;
/// dist = 0; eps = [0x8D22B0]; A = P in registers. Per plane (normal n, d): sA = sign of n·A + d against
/// ±eps (1 above eps, -1 below -eps), sT likewise for f32 n·T + d. sA ≥ 0 and sT ≥ 0 → AL = 1 (dist stays 0).
/// sA > 0, sT < 0 (entering): with dir·n above 0.001, s = (-1/(dir·n)) * f32(n·A + d) and A += (dir.x*s,
/// f32(dir.y*s), f32(dir.z*s)) (registers). sA ≤ 0 (or sA = 0 with sT < 0), sT > 0 (leaving): same test and
/// s, T = f32(A + that). After the four planes: dist = f32 |T - A| (A.y, A.z stored as f32 first) and AL = 0.
/// EAX's upper bytes are the original's leftovers (`IntersectRay`'s on the miss path, else the FPU status and
/// the `dist` pointer); not reproduced.
#[hook(addr = 0x5f5a30, conv = cdecl, name = "CPedGeometryAnalyser::GetIsLineOfSightClear-ped",
       expect = "90 E9 E0 47 E1 FF", unverified = "mvpj-ai-2", route = "L2", hazards = "x87-transcendental,ax-al-return",
       recs = "grove-walk")]
pub fn get_is_line_of_sight_clear_ped(ped: GPtr<u8>, target: GPtr<f32>, e: GPtr<u8>, dist: GPtr<f32>) -> u8 {
    let pp = ent_pos(ped);
    let mut p = [pp.read(), pp.wrapping_index(1).read(), pp.wrapping_index(2).read()];
    let mut t = [target.read(), target.wrapping_index(1).read(), target.wrapping_index(2).read()];
    let mut sphere = [0u32; 5];
    compute_entity_bounding_sphere(ped, e, fp(&mut sphere));
    let mut dir = [
        x(t[0]).fsub_m(p[0]).to_f32(),
        x(t[1]).fsub_m(p[1]).to_f32(),
        x(t[2]).fsub_m(p[2]).to_f32(),
    ];
    vector_normalise_and_mag(fp(&mut dir));
    let mut p1 = [0f32; 3];
    let mut p2 = [0f32; 3];
    if col_sphere_intersect_ray(fp(&mut sphere), fp(&mut p), fp(&mut dir), fp(&mut p1), fp(&mut p2)) == 0 {
        return 1;
    }
    let z = rd::<u32>(ent_pos(ped).cast::<u8>(), 8);
    let mut corners = [0f32; 12];
    let mut normals = [0f32; 12];
    let mut ds = [0f32; 4];
    compute_entity_bounding_box_corners_uncached(z, e, fp(&mut corners));
    compute_entity_bounding_box_planes_uncached(z, fp(&mut corners), fp(&mut normals), fp(&mut ds));
    dist.cast::<u32>().write(0);
    let eps = g::<f32>(K_CLEAR_MARGIN).read();
    let (mut ax, mut ay, mut az) = (x(p[0]), x(p[1]), x(p[2]));
    for i in 0..4 {
        let n = [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]];
        let da = ax.fmul_m(n[0]).fadd(ay.fmul_m(n[1])).fadd(az.fmul_m(n[2])).fadd_m(ds[i]);
        let daf = da.to_f32();
        let dt = x(t[0]).fmul_m(n[0]).fadd(x(t[1]).fmul_m(n[1])).fadd(x(t[2]).fmul_m(n[2])).fadd_m(ds[i]).to_f32();
        let sa: i32 = if !da.fcom(x(eps)).test_any(0x41) {
            1
        } else if !da.fcom(x(eps).fchs()).test_parity(0x05) {
            -1
        } else {
            0
        };
        let st: i32 = if !x(dt).fcom(x(eps)).test_any(0x41) {
            1
        } else if !x(eps).fchs().fcom(x(dt)).test_any(0x41) {
            -1
        } else {
            0
        };
        let entering = sa >= 0 && st < 0 && sa > 0;
        if sa >= 0 && st >= 0 {
            return 1;
        }
        let leaving = !entering && st > 0;
        if !entering && !leaving {
            continue;
        }
        let den = x(dir[0]).fmul_m(n[0]).fadd(x(dir[1]).fmul_m(n[1])).fadd(x(dir[2]).fmul_m(n[2]));
        if den.fcom(x(0.001)).test_any(0x41) {
            continue;
        }
        let s = den.fdivr_m(-1.0).fmul_m(daf);
        let sx = x(dir[0]).fmul(s);
        let sy = x(dir[1]).fmul(s).to_f32();
        let sz = x(dir[2]).fmul(s).to_f32();
        if entering {
            ax = ax.fadd(sx);
            ay = ay.fadd_m(sy);
            az = az.fadd_m(sz);
        } else {
            t[0] = sx.fadd(ax).to_f32();
            t[1] = x(sy).fadd(ay).to_f32();
            t[2] = x(sz).fadd(az).to_f32();
        }
    }
    let azf = az.to_f32();
    let ayf = ay.to_f32();
    let dx = x(t[0]).fsub(ax);
    let dy = x(t[1]).fsub_m(ayf);
    let dz = x(t[2]).fsub_m(azf);
    dist.write(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32());
    0
}

/// One pass of `ComputeClearTarget` over a scanner's 16 entities (`list`): an entity within 5 of `out`
/// (register |e - out|² below the register 5*5), whose box contains `out` (`LiesInsideBoundingBox`) and that
/// blocks the line (`GetIsLineOfSightClear` false, its distance written to `dist`) pushes `out` away from the
/// ped: u = normalised f32(out - ped), s = 0.35 + dist; out.x = f32(out.x - u.x*s), out.y/z less the f32
/// products.
fn clear_target_pass(ped: GPtr<u8>, pp: GPtr<f32>, list: GPtr<GPtr<u8>>, out: GPtr<f32>, dist: &mut f32) {
    let r = g::<f32>(K_CLEAR_RADIUS).read();
    for k in 0..16 {
        let e = list.wrapping_index(k).read();
        if e.is_null() {
            continue;
        }
        let ep = ent_pos(e);
        let dx = x(ep.read()).fsub_m(out.read());
        let dy = x(ep.wrapping_index(1).read()).fsub_m(out.wrapping_index(1).read());
        let dz = x(ep.wrapping_index(2).read()).fsub_m(out.wrapping_index(2).read());
        let dsq = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if x(r).fmul_m(r).fcom(dsq).test_any(0x41) {
            continue;
        }
        if lies_inside_bounding_box(ped, out, e) == 0 {
            continue;
        }
        if get_is_line_of_sight_clear_ped(ped, out, e, GPtr::from_raw(dist as *mut f32)) != 0 {
            continue;
        }
        let mut u = [
            x(out.read()).fsub_m(pp.read()).to_f32(),
            x(out.wrapping_index(1).read()).fsub_m(pp.wrapping_index(1).read()).to_f32(),
            x(out.wrapping_index(2).read()).fsub_m(pp.wrapping_index(2).read()).to_f32(),
        ];
        vector_normalise(fp(&mut u));
        let s = x(g::<f32>(K_CLEAR_MARGIN).read()).fadd_m(*dist);
        let sx = x(u[0]).fmul(s);
        let sy = x(u[1]).fmul(s).to_f32();
        let sz = x(u[2]).fmul(s).to_f32();
        out.write(x(out.read()).fsub(sx).to_f32());
        out.wrapping_index(1).write(x(out.wrapping_index(1).read()).fsub_m(sy).to_f32());
        out.wrapping_index(2).write(x(out.wrapping_index(2).read()).fsub_m(sz).to_f32());
    }
}

/// `CPedGeometryAnalyser::ComputeClearTarget(const CPed& ped, const CVector& target, CVector& out)` (0x5F5D80,
/// cdecl). x87. out = target; `clear_target_pass` over the vehicle scanner's entities (intelligence +0xE0)
/// then the ped scanner's (+0x130); u = normalised f32(ped - out) scaled to f32(u*0.35); then up to
/// `_ftol(5/0.35)` + 1 steps while out is within 5 of the ped (register compare) and the register (ped - out)·u
/// (z, y, x order) is not below 0: `CWorld::ProcessLineOfSight(ped, out, buildings + dummies)` (result
/// unused) and, when `ms_iProcessLineNumCrossings` is odd (signed % 2 == 1), out += u (f32 each), else stop.
///
/// Stack: the original stores the blocking distance in its own `out` argument slot and the line-of-sight
/// entity output in its `ped` argument slot (and the vehicle list pointer in the `target` slot); not reproduced.
#[hook(addr = 0x5f5d80, conv = cdecl, name = "CPedGeometryAnalyser::ComputeClearTarget", expect = "8B 44 24 08 8B 10 83 EC 44",
       unverified = "mvpj-ai-2", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "the original overwrites its own argument slots (see doc)")]
pub fn compute_clear_target(ped: GPtr<u8>, target: GPtr<f32>, out: GPtr<f32>) {
    for i in 0..3 {
        out.wrapping_index(i).write(target.wrapping_index(i).read());
    }
    let pp = ent_pos(ped);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut dist = 0f32;
    clear_target_pass(ped, pp, intel.byte_add(0xE0).cast(), out, &mut dist);
    clear_target_pass(ped, pp, intel.byte_add(0x130).cast(), out, &mut dist);
    let mut u = [
        x(pp.read()).fsub_m(out.read()).to_f32(),
        x(pp.wrapping_index(1).read()).fsub_m(out.wrapping_index(1).read()).to_f32(),
        x(pp.wrapping_index(2).read()).fsub_m(out.wrapping_index(2).read()).to_f32(),
    ];
    vector_normalise(fp(&mut u));
    let m = g::<f32>(K_CLEAR_MARGIN).read();
    for v in &mut u {
        *v = x(*v).fmul_m(m).to_f32();
    }
    let r = g::<f32>(K_CLEAR_RADIUS).read();
    let steps = ftol32(x(r).fdiv_m(m)).wrapping_add(1);
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = 0u32;
    let mut k = 0;
    while k < steps {
        k += 1;
        let dx = x(pp.read()).fsub_m(out.read());
        let dy = x(pp.wrapping_index(1).read()).fsub_m(out.wrapping_index(1).read());
        let dz = x(pp.wrapping_index(2).read()).fsub_m(out.wrapping_index(2).read());
        let dsq = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if x(r).fmul_m(r).fcom(dsq).test_any(0x41) {
            return;
        }
        let dot = x(u[2]).fmul(dz).fadd(dy.fmul_m(u[1])).fadd(dx.fmul_m(u[0]));
        if dot.fcom(X87::zero()).test_any(0x01) {
            return;
        }
        world_process_line_of_sight(pp.cast(), out.cast(), fp(&mut cp), fp(&mut ent), 1, 0, 0, 0, 1, 0, 0, 0);
        if g::<i32>(PROCESS_LINE_NUM_CROSSINGS).read() % 2 != 1 {
            return;
        }
        for i in 0..3 {
            out.wrapping_index(i).write(x(u[i as usize]).fadd_m(out.wrapping_index(i).read()).to_f32());
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_IN_AIR,
    HOOK_LIES_INSIDE_BOUNDING_BOX,
    HOOK_COMPUTE_CLOSEST_SURFACE_POINT_RECT,
    HOOK_IS_WANDER_PATH_CLEAR,
    HOOK_GET_IS_LINE_OF_SIGHT_CLEAR_PED,
    HOOK_COMPUTE_CLEAR_TARGET,
];
