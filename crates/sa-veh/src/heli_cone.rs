//! `CHeli::SearchLightCone` (0x6C58E0): the search-light cone mesh, its corona, the hit-point outputs and
//! the ground shadow. Ported from the 1.0 US bytes (not reversed in gta-reversed); every x87 sequence in
//! the original order, values the original keeps on the FPU stack kept as full-width [`X87`]s.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, x, xi};

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `CrossProduct(CVector*, const CVector&, const CVector&)` (0x59C730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => sa_world::collision::data::cross_product_port;
    /// `CWorld::ProcessLineOfSight(...)` (0x56BA00)
    fn process_line_of_sight(o: GPtr<u8>, t: GPtr<u8>, cp: GPtr<u8>, out: GPtr<u8>, b: u8, v: u8, p: u8, ob: u8, d: u8, see: u8, cam: u8, shoot: u8) -> u8
        = cdecl 0x56BA00 => sa_world::world::mvpc_world1::process_line_of_sight;
    /// `CCoronas::RegisterCorona(...)` (0x6FC580), 21 argument words
    fn register_corona(id: u32, attach: u32, r: u32, gg: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, kind: u32, flare: u32, refl: u32, los: u32, trail: u32, angle: u32, neon: u32, pull: u32, bright: u32, fade: u32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
    /// `RwIm3DTransform` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, num: u32, m: u32, flags: u32) -> u32 = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: GPtr<u8>, num: i32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
    /// `CShadows::StoreShadowToBeRendered(uint8, RwTexture*, CVector*, float x4, int16, uint8 x3, float, bool, float, CRealTimeShadow*, bool)` (0x707390)
    fn store_shadow_to_be_rendered(ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, water: u32, scale: u32, rts: u32, bld: u32) = cdecl 0x707390 => unported;
}

/// `TempBufferVerticesStored` / `TempBufferIndicesStored`.
const NUM_VERTS: usize = 0xC4B950;
const NUM_IDX: usize = 0xC4B954;
/// `TempBufferRenderIndexList` (u16) / `TempVertexBuffer` (36-byte `RxObjSpace3DVertex`).
const IDX_BUF: usize = 0xC4B958;
const VERT_BUF: usize = 0xC4D958;

fn v3(p: &mut [f32; 3]) -> GPtr<CVector> {
    GPtr::from_raw(p.as_mut_ptr()).cast()
}

fn p8(p: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(p.as_mut_ptr()).cast()
}

/// `TheCamera.GetPosition()`.
fn cam_pos() -> GPtr<f32> {
    let m = g::<GPtr<u8>>(0xB6F03C).read();
    if m.is_null() { GPtr::from_addr(0xB6F02C) } else { m.byte_add(0x30).cast() }
}

/// `CHeli::SearchLightCone(int32 id, CVector origin, CVector target, float targetRadius, float power,
/// uint8, bool drawShadow, CVector* outTarget, CVector* outSide1, CVector* outSide2, bool, float
/// baseRadius, float, float, float)` (0x6C58E0).
///
/// `dir = normalised(target - origin)`; a line-of-sight test from origin to `target + dir*3` (buildings
/// only) moves the target to the hit point. The corona at `origin` uses `d = max(dir . normalised(cam -
/// origin), 0)`: size `20 d^6`, alpha `ftol(255 d^6)`. Then 41 steps `i = 0..=40` (angle `i * pi/20`):
/// the near vertex `P = origin + (R sin + U cos) * baseRadius` and the far ray end `E = origin + dir*100
/// + (R sin + U cos) * targetRadius` (`R = norm(dir x Z)`, `U = norm(R x dir)`), the far vertex the
/// point of `P->E` at `target.z` clamped to 100 from `P`; hit points 20 / 30 kept unclamped. Per vertex
/// `w = (|normalised(P - origin) . camdir|)^2` (the maximum tracked), alpha `0.15 * 0x8D12C0 + 0.1` near
/// and 0 far; indices `(n, n+3, n+1)` and, when `baseRadius > 0`, `(n, n+2, n+3)` (not on the last
/// step). Colours `v = w * alpha / max`: `(r << 16) | (r << 8) | a`, `r = ftol(200 v)`, `a = ftol(255 v)`
/// (low bytes; the original's byte order kept). Outputs `*outTarget = target`, `*outSide1 = hit20 -
/// target`, `*outSide2 = hit30 - target`. With `drawShadow`: when both side vectors (x1.2, xy) are
/// shorter than 100 and the target lies within 25 (xy) of the camera, a type-2 shadow (texture
/// 0xC403F4) at `target + (0, 0, 5)`.
#[hook(addr = 0x6C58E0, conv = cdecl, name = "CHeli::SearchLightCone", expect = "81 EC 24 04 00 00",
       unverified = "samp-heli-1", route = "L2", hazards = "x87-transcendental",
       note = "fsin/fcos per step and fsqrt; writes TempBufferVertices/Indices (0xC4D958/0xC4B958)")]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn search_light_cone(id: u32, ox: f32, oy: f32, oz: f32, tx: f32, ty: f32, tz: f32, target_radius: f32,
                         power: f32, _unk: u32, shadow: u32, out_target: GPtr<f32>, out_side1: GPtr<f32>,
                         out_side2: GPtr<f32>, _a14: u32, base_radius: f32, _a16: u32, _a17: u32, _a18: u32) {
    let mut o = [ox, oy, oz];
    let mut t = [tx, ty, tz];
    // dir = normalised(target - origin)
    let mut dir = [f32_of(x(tx).fsub_m(ox)), f32_of(x(ty).fsub_m(oy)), f32_of(x(tz).fsub_m(oz))];
    normalise(v3(&mut dir));
    let a = x(dir[0]).fmul_m(3.0);
    let b = x(dir[1]).fmul_m(3.0);
    let c = f32_of(x(dir[2]).fmul_m(3.0));
    t[0] = f32_of(a.fadd_m(t[0]));
    t[1] = f32_of(b.fadd_m(t[1]));
    t[2] = f32_of(x(c).fadd_m(t[2]));
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = 0u32;
    if process_line_of_sight(p8(&mut o), p8(&mut t), GPtr::from_raw(cp.as_mut_ptr()).cast(),
                             GPtr::from_raw(&mut ent).cast(), 1, 0, 0, 0, 0, 0, 0, 0) != 0 {
        t = [f32::from_bits(cp[0]), f32::from_bits(cp[1]), f32::from_bits(cp[2])];
    }
    // far centre: origin + dir * 100
    let a = x(dir[0]).fmul_m(100.0);
    let b = x(dir[1]).fmul_m(100.0);
    let c = f32_of(x(dir[2]).fmul_m(100.0));
    let far = [f32_of(a.fadd_m(o[0])), f32_of(b.fadd_m(o[1])), f32_of(x(c).fadd_m(o[2]))];
    let cam = cam_pos();
    let mut cd = [
        f32_of(x(cam.read()).fsub_m(o[0])),
        f32_of(x(cam.wrapping_index(1).read()).fsub_m(o[1])),
        f32_of(x(cam.wrapping_index(2).read()).fsub_m(o[2])),
    ];
    normalise(v3(&mut cd));
    let mut dot = x(cd[0]).fmul_m(dir[0]).fadd(x(cd[1]).fmul_m(dir[1])).fadd(x(cd[2]).fmul_m(dir[2]));
    if !dot.fcom(x(0.0)).test_parity(0x05) {
        dot = x(0.0);
    }
    let d2 = dot.fmul(dot);
    let d3 = d2.fmul(dot);
    let d4 = d3.fmul(dot);
    let d5 = d4.fmul(dot);
    let d6 = dot.fmul(d5);
    let size = f32_of(x(20.0).fmul(d6));
    let alpha = ftol32(d6.fmul_m(255.0)) as u32;
    register_corona(id, 0, 0xC8, 0xC8, 0xFF, alpha, p8(&mut o), size.to_bits(), 100f32.to_bits(), 0, 0, 1, 0, 0, 0,
                    0, 1.5f32.to_bits(), 0, 15f32.to_bits(), 0, 0);

    let mut maxv = 0f32;
    let nv_g = g::<u32>(NUM_VERTS);
    let ni_g = g::<u32>(NUM_IDX);
    ni_g.write(0);
    nv_g.write(0);
    let mut wv = [0f32; 82];
    let mut av = [0f32; 82];
    let mut h20 = [0f32; 3];
    let mut h30 = [0f32; 3];
    let pi20 = f32::from_bits(0x3E20_D97C);
    let base_pos = x(base_radius).fcom(x(0.0));
    for i in 0..=40i32 {
        let mut z = [0f32, 0.0, 1.0];
        let mut r = [0f32; 3];
        cross_product(v3(&mut r), v3(&mut dir), v3(&mut z));
        normalise(v3(&mut r));
        let mut u = [0f32; 3];
        cross_product(v3(&mut u), v3(&mut r), v3(&mut dir));
        normalise(v3(&mut u));
        let ang = xi(i).fmul_m(pi20);
        let s = f32_of(ang.fsin());
        let co = f32_of(ang.fcos());
        // near vertex P
        let p0 = f32_of(x(r[0]).fmul_m(s));
        let p1w = x(r[1]).fmul_m(s);
        let p2w = x(r[2]).fmul_m(s);
        let p2 = f32_of(p2w);
        let lcc = f32_of(x(p0).fmul_m(base_radius));
        let ld0 = f32_of(p1w.fmul_m(base_radius));
        let p2a = p2w.fmul_m(base_radius);
        let x0 = x(lcc).fadd_m(o[0]);
        let py0 = f32_of(x(ld0).fadd_m(o[1]));
        let pz0 = f32_of(p2a.fadd_m(o[2]));
        let q0w = x(u[0]).fmul_m(co);
        let q0 = f32_of(q0w);
        let q1w = x(u[1]).fmul_m(co);
        let q1 = f32_of(q1w);
        let q2 = f32_of(x(u[2]).fmul_m(co));
        let la8 = f32_of(q0w.fmul_m(base_radius));
        let lac = f32_of(q1w.fmul_m(base_radius));
        let lb0 = f32_of(x(q2).fmul_m(base_radius));
        let px = f32_of(x(la8).fadd(x0));
        let py = f32_of(x(lac).fadd_m(py0));
        let pz = f32_of(x(lb0).fadd_m(pz0));
        // far ray end E
        let ld8 = f32_of(x(p0).fmul_m(target_radius));
        let ldc = f32_of(p1w.fmul_m(target_radius));
        let p2b = x(p2).fmul_m(target_radius);
        let y0 = x(ld8).fadd_m(far[0]);
        let l18 = f32_of(x(ldc).fadd_m(far[1]));
        let l1c = f32_of(p2b.fadd_m(far[2]));
        let l78 = f32_of(x(q0).fmul_m(target_radius));
        let l7c = f32_of(x(q1).fmul_m(target_radius));
        let l80 = f32_of(x(q2).fmul_m(target_radius));
        let ex = y0.fadd_m(l78);
        let ey = x(l7c).fadd_m(l18);
        let ez = f32_of(x(l80).fadd_m(l1c));
        // the point of P->E at target.z
        let tt = f32_of(x(pz).fsub_m(t[2]).fdiv(x(pz).fsub_m(ez)));
        let l13c = f32_of(ex.fsub_m(px));
        let dyw = ey.fsub_m(py);
        let dzw = x(ez).fsub_m(pz);
        let l60 = f32_of(x(l13c).fmul_m(tt));
        let l64 = f32_of(dyw.fmul_m(tt));
        let dzt = dzw.fmul_m(tt);
        let hit = [f32_of(x(l60).fadd_m(px)), f32_of(x(l64).fadd_m(py)), f32_of(dzt.fadd_m(pz))];
        let mut h = hit;
        if i == 20 {
            h20 = hit;
        } else if i == 30 {
            h30 = hit;
        }
        let dx = x(hit[0]).fsub_m(px);
        let dy = x(hit[1]).fsub_m(py);
        let dz = x(hit[2]).fsub_m(pz);
        let len = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        if !len.fcom(x(100.0)).test_any(0x41) {
            let mut n = [f32_of(dx), f32_of(dy), f32_of(dz)];
            normalise(v3(&mut n));
            let a = x(n[0]).fmul_m(100.0);
            let b = x(n[1]).fmul_m(100.0);
            let c = f32_of(x(n[2]).fmul_m(100.0));
            h = [f32_of(a.fadd_m(px)), f32_of(b.fadd_m(py)), f32_of(x(c).fadd_m(pz))];
        }
        let near_alpha = f32_of(x(g::<f32>(0x8D12C0).read()).fmul_m(f32::from_bits(0x3E19_999A))
            .fadd_m(f32::from_bits(0x3DCC_CCCD)));
        let nv = nv_g.read() as usize;
        let vb = GPtr::<f32>::from_addr(VERT_BUF + nv * 0x24);
        vb.write(px);
        vb.wrapping_index(1).write(py);
        vb.wrapping_index(2).write(pz);
        vb.wrapping_index(9).write(h[0]);
        vb.wrapping_index(10).write(h[1]);
        vb.wrapping_index(11).write(h[2]);
        let mut v = [f32_of(x(px).fsub_m(o[0])), f32_of(x(py).fsub_m(o[1])), f32_of(x(pz).fsub_m(o[2]))];
        normalise(v3(&mut v));
        let mut d = x(v[0]).fmul_m(cd[0]).fadd(x(v[2]).fmul_m(cd[2])).fadd(x(v[1]).fmul_m(cd[1]));
        if !d.fcom(x(0.0)).test_parity(0x05) {
            d = d.fchs();
        }
        let w = d.fmul(d);
        if !w.fcom(x(maxv)).test_any(0x41) {
            maxv = f32_of(w);
        }
        av[nv] = near_alpha;
        av[nv + 1] = 0.0;
        wv[nv] = f32_of(w);
        wv[nv + 1] = f32_of(w);
        if i != 40 {
            let mut ni = ni_g.read() as usize;
            let ib = GPtr::<u16>::from_addr(IDX_BUF);
            ib.wrapping_index((ni) as i32).write(nv as u16);
            ib.wrapping_index((ni + 1) as i32).write((nv + 3) as u16);
            ib.wrapping_index((ni + 2) as i32).write((nv + 1) as u16);
            ni += 3;
            ni_g.write(ni as u32);
            if !base_pos.test_any(0x41) {
                ib.wrapping_index((ni) as i32).write(nv as u16);
                ib.wrapping_index((ni + 1) as i32).write((nv + 2) as u16);
                ib.wrapping_index((ni + 2) as i32).write((nv + 3) as u16);
                ni_g.write((ni + 3) as u32);
            }
        }
        nv_g.write((nv + 2) as u32);
    }
    let nv = nv_g.read() as usize;
    let ni = ni_g.read();
    if nv > 0 {
        let inv = x(1.0).fdiv_m(maxv);
        for j in 0..nv {
            let v = x(wv[j]).fmul_m(av[j]).fmul(inv);
            let r = (ftol32(x(200.0).fmul(v)) as u32) & 0xFF;
            let a = (ftol32(v.fmul_m(255.0)) as u32) & 0xFF;
            GPtr::<u32>::from_addr(VERT_BUF + j * 0x24 + 0x18).write((((r << 8) | r) << 8) | a);
        }
    }
    if ni as i32 > 0 && rw_im3d_transform(GPtr::from_addr(VERT_BUF), nv as u32, 0, 0x18) != 0 {
        rw_im3d_render_indexed_primitive(3, GPtr::from_addr(IDX_BUF), ni_g.read() as i32);
        rw_im3d_end();
    }
    out_target.write(t[0]);
    out_target.wrapping_index(1).write(t[1]);
    out_target.wrapping_index(2).write(t[2]);
    let s1 = [f32_of(x(h20[0]).fsub_m(t[0])), f32_of(x(h20[1]).fsub_m(t[1])), f32_of(x(h20[2]).fsub_m(t[2]))];
    out_side1.write(s1[0]);
    out_side1.wrapping_index(1).write(s1[1]);
    out_side1.wrapping_index(2).write(s1[2]);
    let s2 = [f32_of(x(h30[0]).fsub_m(t[0])), f32_of(x(h30[1]).fsub_m(t[1])), f32_of(x(h30[2]).fsub_m(t[2]))];
    out_side2.write(s2[0]);
    out_side2.wrapping_index(1).write(s2[1]);
    out_side2.wrapping_index(2).write(s2[2]);
    if shadow as u8 == 0 {
        return;
    }
    shadow_part(t, s1, s2, power);
}

/// The `drawShadow` tail of `SearchLightCone`.
fn shadow_part(t: [f32; 3], s1: [f32; 3], s2: [f32; 3], power: f32) {
    let mut pos = [t[0], t[1], f32_of(x(t[2]).fadd_m(5.0))];
    let k = f32::from_bits(0x3F99_999A);
    let bx = f32_of(x(s1[0]).fmul_m(k));
    let by = f32_of(x(s1[1]).fmul_m(k));
    let cx = f32_of(x(s2[0]).fmul_m(k));
    let cy = f32_of(x(s2[1]).fmul_m(k));
    if x(bx).fmul_m(bx).fadd(x(by).fmul_m(by)).fsqrt().fcom(x(100.0)).test_parity(0x05) {
        return;
    }
    if x(cx).fmul_m(cx).fadd(x(cy).fmul_m(cy)).fsqrt().fcom(x(100.0)).test_parity(0x05) {
        return;
    }
    let cam = cam_pos();
    let dx = x(t[0]).fsub_m(cam.read());
    let dy = x(t[1]).fsub_m(cam.wrapping_index(1).read());
    let len = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    if !len.fcom(x(25.0)).test_any(0x41) {
        return;
    }
    let v: X87 = len.fmul_m(f32::from_bits(0x3D23_D70A)).fsubr_m(1.0).fmul_m(power).fmul_m(0.5);
    let r = (ftol32(x(200.0).fmul(v)) as u32) & 0xFF;
    let blue = ftol32(x(255.0).fmul(v)) as u32;
    let intensity = ftol32(v.fmul_m(128.0)) as u32;
    store_shadow_to_be_rendered(2, g::<u32>(0xC403F4).read(), p8(&mut pos), bx.to_bits(), by.to_bits(), cx.to_bits(),
                                cy.to_bits(), intensity, r, r, blue, 15f32.to_bits(), 1, 1f32.to_bits(), 0, 0);
}

/// Hooks of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SEARCH_LIGHT_CONE];
