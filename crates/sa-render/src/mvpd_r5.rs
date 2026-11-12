//! WRITER-MVP-D (batch `mvpd-render-5`): the rotated buffered sprite of the in-world effects path.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Sprite.cpp` as reading aid. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jp, rd, wr, x};

orig! {
    /// `CSprite::FlushSpriteBuffer()` (0x70CF20)
    fn sprite_flush_sprite_buffer() = cdecl 0x70CF20 => unported;
}

/// `CSprite::RenderBufferedOneXLUSprite_Rotate_Dimension(CVector pos, float w, float h, uint8 r, g, b,
/// int16 intensity, float recipNearZ, float angle, uint8 alpha)` (0x70EAB0): 3D mode (0xC6A15C = 0); the
/// corners of the `w × h` quad rotated by `angle` (c = cos, s = sin): x = `(x - wc) - hs`, `hs + (x -
/// wc)`, `(hs + wc) + x`, `(wc + x) - hs`, y = `(y - ch) + sw`, `(ch + sw) + y`, `(ch + y) - sw`, `(y -
/// ch) - sw` (hs, ch and y - ch rounded to f32: the original keeps them in its angle, x and h argument
/// slots). Nothing when all four x (or y) are below 0 or above the screen width (height); z as in
/// `RenderBufferedOneXLUSprite`, rhw = `recipNearZ`, colour `alpha << 24 | (rgb * intensity) >> 8`, uv
/// (0,0), (0,1), (1,1), (1,0); then the index block (flush at 0x180 sprites).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70EAB0, conv = cdecl, name = "CSprite::RenderBufferedOneXLUSprite_Rotate_Dimension", expect = "83 EC 20 D9 44 24 4C",
       route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "grove-walk", note = "FSIN/FCOS; temporaries written into its own argument slots (angle, x, h, the r slot's byte 2)")]
pub fn render_buffered_one_xlu_sprite_rotate_dimension(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: f32, alpha: u32) {
    let a = x(angle);
    let s = a.fsin();
    let c = a.fcos();
    g::<u32>(0xC6A15C).write(0);
    let wc = c.fmul_m(w);
    let xmwc = x(px).fsub(wc);
    let hs = s.fmul_m(h).to_f32();
    let xs0 = xmwc.fsub_m(hs).to_f32();
    let xs1 = x(hs).fadd(xmwc).to_f32();
    let xs2 = x(hs).fadd(wc).fadd_m(px).to_f32();
    let xs3 = wc.fadd_m(px).fsub_m(hs).to_f32();
    let ch = c.fmul_m(h).to_f32();
    let ymch = x(py).fsub_m(ch).to_f32();
    let sw = s.fmul_m(w);
    let ys0 = x(ymch).fadd(sw).to_f32();
    let ys1 = x(ch).fadd(sw).fadd_m(py).to_f32();
    let ys2 = x(ch).fadd_m(py).fsub(sw).to_f32();
    let ys3 = x(ymch).fsub(sw).to_f32();
    let xs = [xs0, xs1, xs2, xs3];
    let ys = [ys0, ys1, ys2, ys3];
    let below = |v: &[f32; 4]| v.iter().all(|&t| !jp(x(t), X87::zero(), 5));
    if below(&xs) || below(&ys) {
        return;
    }
    let wmax = X87::from_i32(g::<i32>(0xC17044).read()).to_f32();
    if xs.iter().all(|&t| !x(t).fcom(x(wmax)).test_any(0x41)) {
        return;
    }
    let hmax = X87::from_i32(g::<i32>(0xC17048).read()).to_f32();
    if ys.iter().all(|&t| !x(t).fcom(x(hmax)).test_any(0x41)) {
        return;
    }
    let idx = g::<u32>(0xC6A158).read();
    let v = g::<u8>(0xC5F958).byte_add(idx as isize * 0x70);
    let num = x(g::<f32>(0xC4B8D4).read()).fsub_m(g::<f32>(0xC4B8D8).read()).fmul(x(pz).fsub_m(g::<f32>(0xC3EFA0).read())).fmul_m(g::<f32>(0xC3EF9C).read());
    let den = x(g::<f32>(0xC3EF9C).read()).fsub_m(g::<f32>(0xC3EFA0).read()).fmul_m(pz);
    let z = num.fdiv(den).fadd_m(g::<f32>(0xC4B8D8).read()).to_f32();
    let ii = i32::from(intensity as u16 as i16);
    let cr = ((i32::from(r as u8) * ii) >> 8) as u8;
    let cg = ((i32::from(gg as u8) * ii) >> 8) as u8;
    let cb = ((i32::from(b as u8) * ii) >> 8) as u8;
    let col = (u32::from(alpha as u8) << 24) | (u32::from(cr) << 16) | (u32::from(cg) << 8) | u32::from(cb);
    let uv: [(u32, u32); 4] = [(0, 0), (0, 0x3F80_0000), (0x3F80_0000, 0x3F80_0000), (0x3F80_0000, 0)];
    for (cidx, (u, vv)) in uv.iter().enumerate() {
        let e = v.byte_add(cidx as isize * 0x1C);
        wr::<f32>(e, 0, xs[cidx]);
        wr::<f32>(e, 4, ys[cidx]);
        wr::<f32>(e, 8, z);
        wr::<u32>(e, 0xC, recip_z);
        wr::<u32>(e, 0x10, col);
        wr::<u32>(e, 0x14, *u);
        wr::<u32>(e, 0x18, *vv);
    }
    let il = g::<u16>(0xC4B958).byte_add(idx as isize * 12);
    let c4 = (idx * 4) as u16;
    for (j, d) in [0u16, 1, 2, 3, 0, 2].iter().enumerate() {
        il.wrapping_index(j as i32).write(c4 + d);
    }
    g::<u32>(0xC6A158).write(idx + 1);
    if (idx + 1) as i32 >= 0x180 {
        sprite_flush_sprite_buffer();
    }
}

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlantMgr::_ProcessEntryCollisionDataSections_RemoveLocTris(entry, center, iTriMax, first, last)`
    /// (0x5DBF20; bool in AL)
    fn plant_remove_loc_tris(entry: GPtr<u8>, center: GPtr<u8>, max: u32, first: i32, last: i32) -> u8 = cdecl 0x5DBF20 => unported;
    /// `CPlantMgr::_ProcessEntryCollisionDataSections_AddLocTris(entry, center, iTriMax, first, last)`
    /// (0x5DC8B0; bool in AL)
    fn plant_add_loc_tris(entry: GPtr<u8>, center: GPtr<u8>, max: u32, first: i32, last: i32) -> u8 = cdecl 0x5DC8B0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(this: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0)
    fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => crate::mvpd_unk::simple_transform_update_matrix;
    /// `TransformPoints(RwV3d* out, int32 n, const CMatrix&, const RwV3d* in)` (0x54EEF0)
    fn transform_points(out: GPtr<u8>, n: u32, m: GPtr<u8>, inp: GPtr<u8>) = cdecl 0x54EEF0 => crate::mvpb_r1::transform_points;
    /// `CBox::Set(const CVector&, const CVector&)` (0x40EDE0)
    fn box_set(b: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>) = thiscall 0x40EDE0 => unported;
    /// `CBox::Recalc()` (0x40EE20)
    fn box_recalc(b: GPtr<u8>) = thiscall 0x40EE20 => unported;
    /// `CSphere::Set(float radius, const CVector& center)` (0x40FCF0)
    fn sphere_set(s: GPtr<u8>, r: u32, c: GPtr<u8>) = thiscall 0x40FCF0 => unported;
    /// `CCollision::TestSphereBox(const CSphere&, const CBox&)` (0x4120C0, bool in AL)
    fn test_sphere_box(s: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x4120C0 => unported;
}

/// `CPlantMgr::_ProcessEntryCollisionDataSections(const CPlantColEntEntry&, const CVector& center, int32
/// iTriMax)` (0x5DCD80, cdecl, bool in AL): false when the entity's col data is gone or its triangle
/// count changed (entry +8, u16). Otherwise `_RemoveLocTris(entry, center, max, 0, n - 1)`; without
/// triangle sections (col data +7 bit 1) `_AddLocTris(…, 0, n - 1)`; else each section (count at
/// sections - 4, 0x1C bytes each going down from sections - 0x20: box min/max, first/last i16 +0x18/
/// +0x1A) whose box, transformed by the entity matrix (allocated when missing), touches the 100-unit
/// sphere around `center` gets `_AddLocTris(…, first, last)`. Returns true.
#[hook(addr = 0x5DCD80, conv = cdecl, name = "CPlantMgr::_ProcessEntryCollisionDataSections", expect = "83 EC 60 53 56",
       unverified = "mvpd-render-5", route = "L2", hazards = "heap-writes,ax-al-return",
       recs = "grove-walk", note = "may allocate the entity matrix; bool in AL")]
pub fn plant_process_entry_collision_data_sections(entry: GPtr<u8>, center: GPtr<u8>, max: u32) -> u8 {
    let e = rd::<GPtr<u8>>(entry, 0);
    let cd = rd::<GPtr<u8>>(get_col_model(e), 0x2C);
    if cd.is_null() {
        return 0;
    }
    let n = i32::from(rd::<u16>(entry, 8));
    if i32::from(rd::<i16>(cd, 4)) != n {
        return 0;
    }
    plant_remove_loc_tris(entry, center, max, 0, n - 1);
    if rd::<u8>(cd, 7) & 2 == 0 {
        plant_add_loc_tris(entry, center, max, 0, n - 1);
        return 1;
    }
    let count = rd::<i32>(rd::<GPtr<u8>>(cd, 0x18), -4);
    let mut k = 0isize;
    while (k as i32) < count {
        let sec = rd::<GPtr<u8>>(cd, 0x18).byte_add(-(k * 0x1C) - 0x20);
        let mut inp: [u32; 6] = sec.cast::<[u32; 6]>().read();
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut out = [0u32; 6];
        transform_points(lp(&mut out), 2, rd::<GPtr<u8>>(e, 0x14), lp(&mut inp));
        let mut bx = [0u32; 8];
        let o = lp(&mut out);
        box_set(lp(&mut bx), o, o.byte_add(0xC));
        box_recalc(lp(&mut bx));
        let mut sph = [0u32; 4];
        sphere_set(lp(&mut sph), 0x42C8_0000, center);
        if test_sphere_box(lp(&mut sph), lp(&mut bx)) != 0 {
            plant_add_loc_tris(entry, center, max, i32::from(rd::<i16>(sec, 0x18)), i32::from(rd::<i16>(sec, 0x1A)));
        }
        k += 1;
    }
    1
}

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

orig! {
    /// `ProcObjectMan_c::ProcessTriangleAdded(CPlantLocTri*)` (0x5A3F20, bool in AL)
    fn proc_object_process_triangle_added(this: GPtr<u8>, t: GPtr<u8>) -> u8 = thiscall 0x5A3F20 => unported;
    /// `CPlantLocTri::Release()` (0x5DB6D0)
    fn plant_loc_tri_release(this: GPtr<u8>) = thiscall 0x5DB6D0 => unported;
}

/// `(z² + y²) + x²` of a difference vector, in the original's operand order.
fn len2_zyx(v: [X87; 3]) -> X87 {
    v[2].fmul(v[2]).fadd(v[1].fmul(v[1])).fadd(v[0].fmul(v[0]))
}

/// `CPlantMgr::_ProcessEntryCollisionDataSections_RemoveLocTris(const CPlantColEntEntry&, const CVector&
/// center, int32 iTriMax, int32 first, int32 last)` (0x5DBF20, cdecl). For each loc tri `first..=last` of
/// the entry (+4): a tri with plants (+0x48 bit 1) not yet given objects (bit 2) gets bit 2 when
/// `g_procObjMan.ProcessTriangleAdded` succeeds. Then, unless `iTriMax` is 0xFAFAFAFA, only tris whose
/// `i & 7` equals it are checked: a tri whose three corners, centre (+0x24) and three edge midpoints
/// (`(dA + dB) * 0.5` of the differences to `center`) are all at least 100 away (squared distance
/// `(z² + y²) + x²` not below 10000) is `Release`d and its slot cleared.
#[hook(addr = 0x5DBF20, conv = cdecl, name = "CPlantMgr::_ProcessEntryCollisionDataSections_RemoveLocTris", expect = "83 EC 6C 53 8B 5C 24 74",
       unverified = "mvpd-render-5", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "releases plant loc tris (pool) and sets their flags")]
pub fn plant_remove_loc_tris_port(entry: GPtr<u8>, center: GPtr<u8>, max: u32, first: i32, last: i32) {
    get_col_model(rd::<GPtr<u8>>(entry, 0));
    let mut i = first;
    while i <= last {
        let slot = || rd::<GPtr<u8>>(entry, 4).cast::<GPtr<u8>>().wrapping_index(i);
        let t = slot().read();
        if !t.is_null() {
            let fl = rd::<u8>(t, 0x48);
            if fl & 2 != 0 && fl & 4 == 0 && proc_object_process_triangle_added(g(0xBB7CB0), t) != 0 {
                wr::<u8>(t, 0x48, rd::<u8>(t, 0x48) | 4);
            }
        }
        if max != 0xFAFA_FAFA && max != (i as u32 & 7) {
            i += 1;
            continue;
        }
        let t = slot().read();
        if t.is_null() {
            i += 1;
            continue;
        }
        let c = |o: isize| rd::<f32>(center, o);
        let d = |b: isize| [x(c(0)).fsub_m(rd::<f32>(t, b)), x(c(4)).fsub_m(rd::<f32>(t, b + 4)), x(c(8)).fsub_m(rd::<f32>(t, b + 8))];
        let f = |v: [X87; 3]| [x(v[0].to_f32()), x(v[1].to_f32()), x(v[2].to_f32())];
        let d0 = d(0);
        let d1 = f(d(0xC));
        let d2 = f(d(0x18));
        let dc = f(d(0x24));
        let half = |a: X87, b: X87| x(a.fadd(b).fmul_m(f32::from_bits(0x3F00_0000)).to_f32());
        let m01 = [half(d1[0], d0[0]), half(d1[1], d0[1]), half(d1[2], d0[2])];
        let m12 = [half(d2[0], d1[0]), half(d2[1], d1[1]), half(d2[2], d1[2])];
        let m20 = [half(d2[0], d0[0]), half(d2[1], d0[1]), half(d2[2], d0[2])];
        let far = |v: [X87; 3]| !len2_zyx(v).fcom(x(f32::from_bits(0x461C_4000))).c0();
        let d0far = !d0[2].fmul(d0[2]).fadd(d0[0].fmul(d0[0])).fadd(d0[1].fmul(d0[1])).fcom(x(f32::from_bits(0x461C_4000))).c0();
        if d0far && far(d1) && far(d2) && far(dc) && far(m01) && far(m12) && far(m20) {
            plant_loc_tri_release(t);
            slot().write(GPtr::NULL);
        }
        i += 1;
    }
}

orig! {
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734E90)
    fn im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
}

/// `CSprite::RenderOneXLUSprite_Rotate_Aspect(float x, y, z, w, h, uint8 r, g, b, int16 intensity, float
/// recipNearZ, float angle, uint8 alpha)` (0x70D490): nothing below z 1.3; between 1.3 and 2.3 the
/// colour and intensity are faded by `ftol((z - 1.3) * 255)` (`(c * f) >> 8`, signed). With s/c of the
/// angle and k1 = -c - s, k2 = s - c, k3 = c + s, k4 = c - s (f32): corners `(k1 w + x, k2 h + y)`,
/// `(k2 w + x, k3 h + y)`, `(k3 w + x, k4 h + y)`, `(k4 w + x, k1 h + y)`. Nothing when all four x (or
/// y) are below 0 or above the screen size. The four `CSprite` 2D vertices (0xC4B8E0) get the corners,
/// z = `(far2D - near2D) * (z - nearClip) * farClip / ((farClip - nearClip) * z) + near2D` from the
/// engine's Im2D z range (+0x18/+0x1C), rhw = `recipNearZ`, colour `alpha << 24 | (rgb * intensity)
/// >> 8`, uv (0,0), (0,1), (1,1), (1,0), drawn as a triangle fan.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70D490, conv = cdecl, name = "CSprite::RenderOneXLUSprite_Rotate_Aspect", expect = "D9 44 24 0C 83 EC 2C",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSIN/FCOS")]
pub fn render_one_xlu_sprite_rotate_aspect(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: f32, alpha: u32) {
    let (mut cr, mut cg, mut cb, mut ii) = (r as u8 as i32, gg as u8 as i32, b as u8 as i32, i32::from(intensity as u16 as i16));
    if !jp(x(pz), x(f32::from_bits(0x4013_3333)), 5) {
        if !x(pz).fcom(x(f32::from_bits(0x3FA6_6666))).test_parity(5) {
            return;
        }
        let f = x(pz).fsub_m(f32::from_bits(0x3FA6_6666)).fmul_m(f32::from_bits(0x437F_0000)).ftol() as u32 as i32;
        cr = (cr * f) >> 8;
        cg = (cg * f) >> 8;
        cb = (cb * f) >> 8;
        ii = (ii * f) >> 8;
    }
    let a = x(angle);
    let s = a.fsin();
    let c = a.fcos();
    let k1 = c.fchs().fsub(s).to_f32();
    let k2 = s.fsub(c).to_f32();
    let k3 = c.fadd(s).to_f32();
    let k4 = c.fsub(s);
    let xs = [
        x(k1).fmul_m(w).fadd_m(px).to_f32(),
        x(k2).fmul_m(w).fadd_m(px).to_f32(),
        x(k3).fmul_m(w).fadd_m(px).to_f32(),
        x(w).fmul(k4).fadd_m(px).to_f32(),
    ];
    let ys = [
        x(k2).fmul_m(h).fadd_m(py).to_f32(),
        x(k3).fmul_m(h).fadd_m(py).to_f32(),
        k4.fmul_m(h).fadd_m(py).to_f32(),
        x(k1).fmul_m(h).fadd_m(py).to_f32(),
    ];
    let below = |v: &[f32; 4]| v.iter().all(|&t| !jp(x(t), X87::zero(), 5));
    if below(&xs) || below(&ys) {
        return;
    }
    let wmax = X87::from_i32(g::<i32>(0xC17044).read()).to_f32();
    if xs.iter().all(|&t| !x(t).fcom(x(wmax)).test_any(0x41)) {
        return;
    }
    let hmax = X87::from_i32(g::<i32>(0xC17048).read()).to_f32();
    if ys.iter().all(|&t| !x(t).fcom(x(hmax)).test_any(0x41)) {
        return;
    }
    let v = g::<u8>(0xC4B8E0);
    for i in 0..4isize {
        wr::<f32>(v, i * 0x1C, xs[i as usize]);
        wr::<f32>(v, i * 0x1C + 4, ys[i as usize]);
    }
    let eng = g::<GPtr<u8>>(0xC97B24).read();
    let near = g::<f32>(0xC3EFA0).read();
    let far = g::<f32>(0xC3EF9C).read();
    let num = x(rd::<f32>(eng, 0x1C)).fsub_m(rd::<f32>(eng, 0x18)).fmul(x(pz).fsub_m(near)).fmul_m(far);
    let z = num.fdiv(x(far).fsub_m(near).fmul_m(pz)).fadd_m(rd::<f32>(eng, 0x18)).to_f32();
    let s16 = i32::from(ii as i16);
    let g8 = (((cg as u8 as i32) * s16) >> 8) as u8;
    let b8 = (((cb as u8 as i32) * s16) >> 8) as u8;
    let r8 = (((cr as u8 as i32) * s16) >> 8) as u8;
    let col = (u32::from(alpha as u8) << 24) | (u32::from(r8) << 16) | (u32::from(g8) << 8) | u32::from(b8);
    let uv: [(u32, u32); 4] = [(0, 0), (0, 0x3F80_0000), (0x3F80_0000, 0x3F80_0000), (0x3F80_0000, 0)];
    for i in 0..4isize {
        wr::<u32>(v, i * 0x1C + 0xC, recip_z);
        wr::<f32>(v, i * 0x1C + 8, z);
        wr::<u32>(v, i * 0x1C + 0x10, col);
        wr::<u32>(v, i * 0x1C + 0x14, uv[i as usize].0);
        wr::<u32>(v, i * 0x1C + 0x18, uv[i as usize].1);
    }
    im2d_render_primitive(5, v, 4);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_BUFFERED_ONE_XLU_SPRITE_ROTATE_DIMENSION,
    HOOK_PLANT_PROCESS_ENTRY_COLLISION_DATA_SECTIONS,
    HOOK_PLANT_REMOVE_LOC_TRIS_PORT,
    HOOK_RENDER_ONE_XLU_SPRITE_ROTATE_ASPECT,
];
