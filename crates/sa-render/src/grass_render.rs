//! WRITER-RENDER-BIG-2: `CGrassRenderer::DrawTriPlants` (0x5DAD00). Ported from the 1.0 US bytes;
//! gta-reversed `GrassRenderer.cpp` as reading aid (it replaces the original's `srand` / `rand` with a
//! local generator; the port keeps the CRT calls, each with the original's return address). x87
//! arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.
//!
//! `PPTriPlant` (0x5C bytes): +0 / +0xC / +0x18 the triangle, +0x24 centre, +0x30 model id (u16), +0x32
//! plant count (u16), +0x34 / +0x38 scale xy / z, +0x3C texture, +0x40..+0x43 colour, +0x44 intensity,
//! +0x48 seed, +0x4C / +0x50 scale variation xy / z, +0x54 wind bend scale, +0x58 wind bend variation.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x, xi};

orig! {
    /// `Fx_c::GetFxQuality()` (0x49ea50)
    fn get_fx_quality(this: GPtr<u8>) -> u32 = thiscall 0x49ea50 => crate::fx::fx_get_fx_quality;
    /// `RpGeometryForAllMaterials(geometry, callback, data)` (0x74c790)
    fn rp_geometry_for_all_materials(geo: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x74c790 => unported;
    /// `CGrassRenderer::GenPointInTriangle(CVector* out, v0, v1, v2, float r1, float r2)` (0x5dab00)
    fn gen_point_in_triangle(out: GPtr<f32>, v0: GPtr<f32>, v1: GPtr<f32>, v2: GPtr<f32>, r1: f32, r2: f32) = cdecl 0x5dab00 => crate::mvpb_r1::grass_gen_point_in_triangle;
    /// `RwFrameTranslate(frame, const RwV3d*, combine)` (0x7f0e30)
    fn rw_frame_translate(frame: GPtr<u8>, v: GPtr<f32>, combine: u32) -> GPtr<u8> = cdecl 0x7f0e30 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7f18a0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7f18a0 => unported;
    /// `atomic->renderCallBack(atomic)` (the pointer at RpAtomic +0x48)
    fn atomic_render_callback(f: u32, a: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

/// `CGrassRenderer::m_vecCameraPos`, `m_closeDist`, `m_farDist` (.data), `m_windBending`;
/// `RenderGrassTexture`; `g_fx`; `CPPTriPlantBuffer::SetGrassMaterialCB`.
const CAMERA_POS: usize = 0xc0_2ddc;
const CLOSE_DIST: usize = 0xc0_2dbc;
const FAR_DIST: usize = 0x8d_132c;
const WIND_BENDING: usize = 0xc0_2db8;
const RENDER_GRASS_TEXTURE: usize = 0xc0_2dc0;
const G_FX: usize = 0xa9_ae00;
const SET_GRASS_MATERIAL_CB: u32 = 0x5d_ac10;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn rand_at(ret: u32) -> i32 {
    sa_core::orig::at_site(ret, sa_core::crt::rand)
}

/// `rand() * (1 / 32767)` (0x858C7C).
fn rand_unit(ret: u32) -> X87 {
    xi(rand_at(ret)).fmul_m(k(0x3800_0100))
}

/// `|p - camera|` in the original's order: `(dz² + dy²) + dx²`.
fn cam_dist(p: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(p, 0)).fsub_m(gf(CAMERA_POS));
    let dy = x(rd::<f32>(p, 4)).fsub_m(gf(CAMERA_POS + 4));
    let dz = x(rd::<f32>(p, 8)).fsub_m(gf(CAMERA_POS + 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `CGrassRenderer::DrawTriPlants(PPTriPlant*, int32 count, RpAtomic** models)` (0x5DAD00); returns 1.
/// Per plant: `d = |centre - camera|`; the far distance is `m_farDist` (FX quality >= 2) or half of it;
/// beyond `far + 20` the plant is skipped. `srand(ftol(seed))`. Alpha: the colour's below `far`, else
/// `ftol(floor((far + 20 - d) * a * 0.05))` clamped to 255 (unordered: 255) and then to > 0 (else 0).
/// RGB `(c * min(intensity, 255)) >> 8`. `RenderGrassTexture = texture`, `RpGeometryForAllMaterials(
/// geometry, SetGrassMaterialCB, &colour)`. Per plant instance: two `rand`s place it in the triangle
/// (`GenPointInTriangle`); closer than `m_closeDist - 2` to the camera it takes three more `rand`s and
/// is skipped; otherwise the frame is translated there, `right.x` and `up.y` scale by `rand * varXY +
/// scale.x`, `at.z` by `rand * varZ + scale.z`, `at.x = at.y = (rand * windVar + 1) * (m_windBending *
/// windScale)`, `RwMatrixUpdate` and the atomic's render callback.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x5dad00, conv = cdecl, name = "CGrassRenderer::DrawTriPlants", expect = "83 EC 38 8B 44 24 40",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "FSQRT; srand + 8 rand sites; writes the plant atomics' frame matrices (RW heap)")]
pub fn draw_tri_plants(plants: GPtr<u8>, count: i32, models: GPtr<u32>) -> u32 {
    let mut i = 0;
    while i < count {
        let p = plants.byte_add(0x5c * i as isize);
        i += 1;
        let d = cam_dist(p.byte_add(0x24)).to_f32();
        let far = if get_fx_quality(GPtr::from_addr(G_FX)) as i32 >= 2 {
            gf(FAR_DIST)
        } else {
            x(gf(FAR_DIST)).fmul_m(0.5).to_f32()
        };
        // 20 (0x858BA4)
        let far20 = x(far).fadd_m(k(0x41a0_0000)).to_f32();
        if !jne(x(d), x(far20), 0x41) {
            continue;
        }
        let atomic = GPtr::<u8>::from_addr(g::<u32>(models.addr() + 4 * rd::<u16>(p, 0x30) as usize).read() as usize);
        let frame = rd::<GPtr<u8>>(atomic, 4);
        let m = frame.byte_add(0x10);
        let seed = ftol32(x(rd::<f32>(p, 0x48))) as u32;
        sa_core::orig::at_site(0x5d_add2, || sa_core::crt::srand(seed));
        let a = rd::<u8>(p, 0x43);
        let alpha = if !jp(x(d), x(far), 0x5) {
            a
        } else {
            // 0.05 (0x858C28)
            let v = x(far20).fsub_m(d).fmul(xi(i32::from(a))).fmul_m(k(0x3d4c_cccd));
            let fl = ftol32(X87::from_f64_exact(sa_world::physical::base::floor_f64(v.to_f64())));
            let c255 = x(k(0x437f_0000));
            let v = xi(fl);
            let v = if !jp(v, c255, 0x5) { v } else { c255 };
            let v = xi(ftol32(v));
            let v = if !jne(v, x(0.0), 0x41) { v } else { x(0.0) };
            ftol32(v) as u8
        };
        let c255 = x(k(0x437f_0000));
        let iv = xi(i32::from(rd::<u8>(p, 0x44)));
        let mult = ftol32(if !jp(iv, c255, 0x5) { iv } else { c255 }) as u16 as i32;
        let sc = |off: isize| ((i32::from(rd::<u8>(p, off)) * mult) >> 8) as u8;
        let mut colour = u32::from_le_bytes([sc(0x40), sc(0x41), sc(0x42), alpha]);
        g::<u32>(RENDER_GRASS_TEXTURE).write(rd::<u32>(p, 0x3c));
        rp_geometry_for_all_materials(rd::<GPtr<u8>>(atomic, 0x18), SET_GRASS_MATERIAL_CB, GPtr::<u32>::from_raw(&mut colour).cast());
        let n = i32::from(rd::<u16>(p, 0x32));
        for _ in 0..n {
            let r1 = rand_unit(0x5d_aef5).to_f32();
            let r2 = rand_unit(0x5d_af0c).to_f32();
            let mut pos = [0f32; 3];
            gen_point_in_triangle(GPtr::from_raw(pos.as_mut_ptr()), p.cast(), p.byte_add(0xc).cast(), p.byte_add(0x18).cast(), r1, r2);
            let pd = cam_dist(GPtr::<f32>::from_raw(pos.as_mut_ptr()).cast());
            // 2 (0x858CA0)
            let close = x(gf(CLOSE_DIST)).fsub_m(k(0x4000_0000));
            if !jne(close, pd, 0x41) {
                rand_at(0x5d_af90);
                rand_at(0x5d_af95);
                rand_at(0x5d_af9a);
                continue;
            }
            rw_frame_translate(frame, GPtr::from_raw(pos.as_mut_ptr()), 0);
            let xy = rand_unit(0x5d_afb8).fmul_m(rd::<f32>(p, 0x4c)).fadd_m(rd::<f32>(p, 0x34));
            wr::<f32>(m, 0, xy.fmul_m(rd::<f32>(m, 0)).to_f32());
            wr::<f32>(m, 0x14, xy.fmul_m(rd::<f32>(m, 0x14)).to_f32());
            let z = rand_unit(0x5d_afe5).fmul_m(rd::<f32>(p, 0x50)).fadd_m(rd::<f32>(p, 0x38));
            wr::<f32>(m, 0x28, z.fmul_m(rd::<f32>(m, 0x28)).to_f32());
            let w1 = rand_unit(0x5d_b00c).fmul_m(rd::<f32>(p, 0x58)).fadd_m(1.0);
            let w = w1.fmul(x(gf(WIND_BENDING)).fmul_m(rd::<f32>(p, 0x54))).to_f32();
            wr::<f32>(m, 0x20, w);
            wr::<f32>(m, 0x24, w);
            rw_matrix_update(m);
            atomic_render_callback(rd::<u32>(atomic, 0x48), atomic);
        }
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_TRI_PLANTS];
