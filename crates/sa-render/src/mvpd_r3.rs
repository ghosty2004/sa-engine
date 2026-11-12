//! WRITER-MVP-D (batch `mvpd-render-3`): small per-frame render drivers of the in-world path —
//! checkpoints, ropes, motion-blur streaks, plane trails, road signs, the effects manager and the
//! post-effects render-state snapshot.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Checkpoints.cpp`, `Rope*.cpp`, `MotionBlurStreaks.cpp`,
//! `PlaneTrail*.cpp`, `CustomRoadsignMgr.cpp`, `FxManager.cpp`, `PostEffects.cpp` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jp, k, rd, wr, x};

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    fn rs(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwRenderStateGet` (`RwEngineInstance` slot 0x24)
    fn rs_get(state: u32, out: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x24 => unported;
    /// `CCheckpoint::Render()` (0x725C00)
    fn checkpoint_render(this: GPtr<u8>) = thiscall 0x725C00 => unported;
    /// `CRope::Render()` (0x556800)
    fn rope_render_o(this: GPtr<u8>) = thiscall 0x556800 => crate::mvpd_r3::rope_render;
    /// `CRegisteredMotionBlurStreak::Render()` (0x721F70)
    fn streak_render(this: GPtr<u8>) = thiscall 0x721F70 => unported;
    /// `FxSystemBP_c::Render(RwCamera*, float brightness, bool heatHaze)` (0x4AA160)
    fn fx_system_bp_render(this: GPtr<u8>, cam: u32, bright: u32, haze: u32) = thiscall 0x4AA160 => unported;
    /// `FxManager_c::Render(RwCamera*, bool)` (0x4A92A0)
    fn fx_manager_render_o(this: GPtr<u8>, cam: u32, haze: u32) = thiscall 0x4A92A0 => crate::mvpd_r3::fx_manager_render;
    /// `CPlaneTrail::Render(float)` (0x717180)
    fn plane_trail_render_o(this: GPtr<u8>, intensity: u32) = thiscall 0x717180 => crate::mvpd_r3::plane_trail_render;
    /// `CCamera::IsSphereVisible(const CVector&, float, const RwMatrix*)` (0x420C40, bool in AL)
    fn camera_is_sphere_visible_m(this: GPtr<u8>, c: GPtr<u8>, r: u32, m: u32) -> u8 = thiscall 0x420C40 => unported;
    /// `DefinedState()` (0x734650)
    fn defined_state() = cdecl 0x734650 => unported;
    /// `RwIm3DTransform(verts, n, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, n: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(type, indices, n)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(ty: u32, idx: GPtr<u8>, n: u32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `RpGeometryForAllMaterials(RpGeometry*, callback, data)` (0x74C790)
    fn rp_geometry_for_all_materials(geom: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
    /// `RpAtomic::renderCallBack` (the atomic's +0x48 function pointer, cdecl)
    fn atomic_render_cb(cb: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

fn lp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

const THE_CAMERA: usize = 0xB6F028;
const TEMP_VERTS: usize = 0xC4D958;

/// `fild dword` of an unsigned value as MSVC does it (plus 2^32 when negative).
fn fild_u32(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x4F80_0000)) } else { r }
}

/// `CCheckpoints::Render()` (0x726060): `CCheckpoint::Render` for each of the 32 checkpoints (0x38 bytes at
/// 0xC7F158) whose `m_bIsUsed` (+2) is set, then render state 20 (cull mode) = 2.
#[hook(addr = 0x726060, conv = cdecl, name = "CCheckpoints::Render", expect = "56 BE 58 F1 C7 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn checkpoints_render() {
    for i in 0..32isize {
        let c = g::<u8>(0xC7F158).byte_add(i * 0x38);
        if rd::<u8>(c, 2) != 0 {
            checkpoint_render(c);
        }
    }
    rs(0x14, 2);
}

/// `CRopes::Render()` (0x556AE0): `CRope::Render` for each of the 8 ropes (0x328 bytes at 0xB768B8) with
/// a type (+0x325).
#[hook(addr = 0x556AE0, conv = cdecl, name = "CRopes::Render", expect = "56 BE B8 68 B7 00",
       unverified = "mvpd-render-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn ropes_render() {
    for i in 0..8isize {
        let r = g::<u8>(0xB768B8).byte_add(i * 0x328);
        if rd::<u8>(r, 0x325) != 0 {
            rope_render_o(r);
        }
    }
}

/// `CRope::Render()` (0x556800, thiscall): when the segment bounding sphere (+0xC0, radius 20) is visible
/// (`IsSphereVisible` with the matrix 0xB6FA74, or 0xB6FABC when `TheCamera` +0x970 is set) and the 2D
/// camera distance to the first segment is below 120: `DefinedState()`, the 32 segment positions as
/// colour-0x80000000 vertices in the temp buffer, states (8,1), (12,1), (10,5), (11,6), (9,2), (1,0),
/// `RwIm3DTransform(…, 32, NULL, 0)` and the line list 0x8CD818 (0x3E indices). A crane rope (type 6)
/// also draws a line from its first segment to (709.32, 916.20, 53.0).
#[hook(addr = 0x556800, conv = thiscall, name = "CRope::Render", this = "CRope", expect = "56 57 8B F1 68 74 FA B6 00",
       unverified = "mvpd-render-3", route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk", note = "FSQRT distance; writes the Im3D temp vertex buffer")]
pub fn rope_render(this: GPtr<u8>) {
    let sph = this.byte_add(0xC0);
    if camera_is_sphere_visible_m(g(THE_CAMERA), sph, 0x41A0_0000, 0xB6FA74) == 0
        && (g::<u8>(0xB6F998).read() == 0 || camera_is_sphere_visible_m(g(THE_CAMERA), sph, 0x41A0_0000, 0xB6FABC) == 0)
    {
        return;
    }
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let dx = x(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(this, 0));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(this, 4));
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
    if jp(d, x(k(0x42F0_0000)), 5) {
        return;
    }
    defined_state();
    for i in 0..32isize {
        let v = g::<u8>(TEMP_VERTS).byte_add(i * 0x24);
        wr::<u32>(v, 0x18, 0x8000_0000);
        wr::<u32>(v, 0, rd::<u32>(this, i * 12));
        wr::<u32>(v, 4, rd::<u32>(this, i * 12 + 4));
        wr::<u32>(v, 8, rd::<u32>(this, i * 12 + 8));
    }
    rs(8, 1);
    rs(0xC, 1);
    rs(0xA, 5);
    rs(0xB, 6);
    rs(9, 2);
    rs(1, 0);
    if !rw_im3d_transform(g(TEMP_VERTS), 0x20, 0, 0).is_null() {
        rw_im3d_render_indexed_primitive(1, g(0x8CD818), 0x3E);
        rw_im3d_end();
    }
    if rd::<u8>(this, 0x325) != 6 {
        return;
    }
    g::<u32>(0xC4D970).write(0x8000_0000);
    g::<u32>(0xC4D958).write(rd::<u32>(this, 0));
    g::<u32>(0xC4D95C).write(rd::<u32>(this, 4));
    g::<u32>(0xC4D960).write(rd::<u32>(this, 8));
    g::<u32>(0xC4D994).write(0x8000_0000);
    g::<u32>(0xC4D97C).write(0x4431_547B);
    g::<u32>(0xC4D980).write(0x4465_0CCD);
    g::<u32>(0xC4D984).write(0x4254_0000);
    if !rw_im3d_transform(g(TEMP_VERTS), 2, 0, 0).is_null() {
        rw_im3d_render_indexed_primitive(1, g(0x8CD818), 2);
        rw_im3d_end();
    }
}

/// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0; the entry is a SecuROM fragment jump):
/// `RwRenderStateGet` of states 10, 11, 14, 20, 6, 8, 7, 12, 2, 9 into 0xC40280..0xC402A4 in turn.
#[hook(addr = 0x700CC0, conv = cdecl, name = "CPostEffects::ImmediateModeRenderStatesStore", expect = "E9 E5 8D D0 FF 68 80 02 C4 00",
       route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk", note = "entry is a SecuROM fragment jump (reads RwEngineInstance)")]
pub fn immediate_mode_render_states_store() {
    let states = [0xA, 0xB, 0xE, 0x14, 6, 8, 7, 0xC, 2, 9];
    for (i, s) in states.iter().enumerate() {
        rs_get(*s, g::<u8>(0xC40280).byte_add(i as isize * 4));
    }
}

/// `CMotionBlurStreaks::Render()` (0x7240E0): for each of the 4 streaks (0x54 bytes at 0xC7CA08) with an
/// id, `CRegisteredMotionBlurStreak::Render`; before the first one the states (8,0), (12,1), (14,1),
/// fog colour (15) `(((r | 0xFFFFFF00) << 8 | g) << 8) | b` from the time-cycle fog words
/// 0xB7C4CA/CC/CE, (10,5), (11,6), (1,0); after any, (8,1) and (12,0).
#[hook(addr = 0x7240E0, conv = cdecl, name = "CMotionBlurStreaks::Render", expect = "53 56 32 DB BE 08 CA C7 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn motion_blur_streaks_render() {
    let mut set = false;
    for i in 0..4isize {
        let s = g::<u8>(0xC7CA08).byte_add(i * 0x54);
        if rd::<u32>(s, 0) == 0 {
            continue;
        }
        if !set {
            rs(8, 0);
            rs(0xC, 1);
            rs(0xE, 1);
            let r = u32::from(g::<u16>(0xB7C4CA).read());
            let gg = u32::from(g::<u16>(0xB7C4CC).read());
            let b = u32::from(g::<u16>(0xB7C4CE).read());
            rs(0xF, ((((r | 0xFFFF_FF00) << 8) | gg) << 8) | b);
            rs(0xA, 5);
            rs(0xB, 6);
            rs(1, 0);
            set = true;
        }
        streak_render(s);
    }
    if set {
        rs(8, 1);
        rs(0xC, 0);
    }
}

/// `FxManager_c::Render(RwCamera*, bool heatHaze)` (0x4A92A0, thiscall, `ret 8`): +0xB8 = 0; brightness
/// `(1 - [0x8D12C0]) * 0.6 + 0.4`; states (6,1), (8,0), (30,0); every system blueprint of the list (+0,
/// linked through +4) `Render(camera, brightness, heatHaze)`; then (8,1), (6,1).
#[hook(addr = 0x4A92A0, conv = thiscall, name = "FxManager_c::Render", this = "FxManager_c", expect = "51 D9 05 24 86 85 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn fx_manager_render(this: GPtr<u8>, cam: u32, haze: u32) {
    let bright = X87::one().fsub_m(g::<f32>(0x8D12C0).read()).fmul_m(k(0x3F19_999A)).fadd_m(k(0x3ECC_CCCD)).to_f32().to_bits();
    wr::<u8>(this, 0xB8, 0);
    rs(6, 1);
    rs(8, 0);
    rs(0x1E, 0);
    let mut s = rd::<GPtr<u8>>(this, 0);
    while !s.is_null() {
        fx_system_bp_render(s, cam, bright, haze);
        s = rd::<GPtr<u8>>(s, 4);
    }
    rs(8, 1);
    rs(6, 1);
}

/// `Fx_c::Render(RwCamera*, bool heatHaze)` (0x49E650; its entry is a SecuROM fragment jump that loads
/// `g_fxMan` 0xA9AE80 into ECX, then a tail jump): `g_fxMan.Render(camera, heatHaze)`. The caller's ECX
/// is ignored; the callee pops both arguments.
#[hook(addr = 0x49E650, conv = stdcall, name = "Fx_c::Render", expect = "E9 C9 66 F6 FF E9 46 AC 00 00",
       route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk", note = "entry is a SecuROM fragment jump")]
pub fn fx_render(cam: u32, haze: u32) {
    fx_manager_render_o(g(0xA9AE80), cam, haze);
}

/// `CPlaneTrails::Render()` (0x7173A0): intensity = the minimum (ties keep the later operand) of
/// `1 - [0xC81304]`, `1 - [0xC81300]`, `1 - Rain` and the brightest time-cycle fog word / 256; above
/// 0.0001 each of the 3 trails (0x100 bytes at 0xC713A8) is rendered with it.
#[hook(addr = 0x7173A0, conv = cdecl, name = "CPlaneTrails::Render", expect = "83 EC 08 D9 05 24 86 85 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn plane_trails_render() {
    let a = X87::one().fsub_m(g::<f32>(0xC81304).read());
    let b = X87::one().fsub_m(g::<f32>(0xC81300).read()).to_f32();
    let mut s = if jp(a, x(b), 5) { b } else { a.to_f32() };
    let c = X87::one().fsub_m(g::<f32>(0xC81324).read());
    if jp(x(s), c, 5) {
        s = c.to_f32();
    }
    let r = g::<u16>(0xB7C4CA).read();
    let gg = g::<u16>(0xB7C4CC).read();
    let bb = g::<u16>(0xB7C4CE).read();
    let rg = if r > gg { r } else { gg };
    let m = if rg > bb { rg } else { bb };
    let mf = X87::from_i32(i32::from(m)).fmul_m(k(0x3B80_0000));
    if mf.fcom(x(s)).test_any(0x41) {
        s = mf.to_f32();
    }
    if x(s).fcom(x(k(0x38D1_B717))).test_any(0x41) {
        return;
    }
    for i in 0..3isize {
        plane_trail_render_o(g::<u8>(0xC713A8).byte_add(i * 0x100), s.to_bits());
    }
}

/// `CPlaneTrail::Render(float intensity)` (0x717180, thiscall, `ret 4`): alpha = `ftol(intensity * 110)`
/// (none: return). Each of the 16 points (positions +0, times +0xC0) younger than 30000 ms (older ones
/// are cleared, zero times skipped) becomes a temp-buffer vertex with colour `ftol(alpha * min((30000 -
/// age) * 0.0001, 1)) << 24 | 0xFFFFFF`; with at least 2 points: (12,1), (10,5), (11,6), (1,0),
/// `RwIm3DTransform(…, n, NULL, 0x18)` and the line list 0x8D5B98 (`2n - 2` indices). The point
/// counter lives in the original's argument slot.
#[hook(addr = 0x717180, conv = thiscall, name = "CPlaneTrail::Render", this = "CPlaneTrail", expect = "83 EC 08 D9 44 24 0C",
       route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "keeps its loop counter in its own argument slot")]
pub fn plane_trail_render(this: GPtr<u8>, intensity: u32) {
    let alpha = x(f32::from_bits(intensity)).fmul_m(k(0x42DC_0000)).ftol() as u32 as i32;
    if alpha == 0 {
        return;
    }
    let mut n = 0u32;
    let now = g::<u32>(0xB7CB84).read();
    for i in 0..16isize {
        let t = rd::<u32>(this, 0xC0 + i * 4);
        let age = now.wrapping_sub(t);
        if age > 30000 {
            wr::<u32>(this, 0xC0 + i * 4, 0);
            continue;
        }
        if t == 0 {
            continue;
        }
        let mut f = fild_u32(age).fsubr_m(k(0x46EA_6000)).fmul_m(k(0x38D1_B717));
        if !jp(X87::one(), f, 5) {
            f = X87::one();
        }
        let a = X87::from_i32(alpha).fmul(f).ftol() as u32;
        let v = g::<u8>(TEMP_VERTS).byte_add(n as isize * 0x24);
        wr::<u32>(v, 0x18, (a << 24) | 0xFF_FFFF);
        wr::<u32>(v, 0, rd::<u32>(this, i * 12));
        wr::<u32>(v, 4, rd::<u32>(this, i * 12 + 4));
        wr::<u32>(v, 8, rd::<u32>(this, i * 12 + 8));
        n += 1;
    }
    if (n as i32) <= 1 {
        return;
    }
    rs(0xC, 1);
    rs(0xA, 5);
    rs(0xB, 6);
    rs(1, 0);
    if !rw_im3d_transform(g(TEMP_VERTS), n, 0, 0x18).is_null() {
        rw_im3d_render_indexed_primitive(1, g(0x8D5B98), n * 2 - 2);
        rw_im3d_end();
    }
}

/// `CCustomRoadsignMgr::RenderRoadsignAtomic(RpAtomic*, const CVector& camPos)` (0x6FF350, cdecl,
/// returns the atomic): with d² the squared 3D distance from the camera to the frame position (kept in
/// the atomic argument slot): beyond 250000 nothing; below 1600 render state 30 = 100 and the materials
/// at alpha 255, otherwise alpha `ftol((250000 - d²) * 4.0257e-6 * 254)` (low byte); then the atomic's
/// render callback.
#[hook(addr = 0x6FF350, conv = cdecl, name = "CCustomRoadsignMgr::RenderRoadsignAtomic", expect = "8B 4C 24 08 D9 01",
       route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "writes d2 into its own argument slot")]
pub fn render_roadsign_atomic(atomic: GPtr<u8>, cam: GPtr<u8>) -> GPtr<u8> {
    let p = rd::<GPtr<u8>>(atomic, 4).byte_add(0x40);
    let dx = x(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(p, 8));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32();
    if !x(d2).fcom(x(k(0x4874_2400))).test_any(0x41) {
        return atomic;
    }
    let alpha = if x(d2).fcom(x(k(0x44C8_0000))).c0() {
        rs(0x1E, 100);
        0xFF
    } else {
        u32::from(x(k(0x4874_2400)).fsub_m(d2).fmul_m(k(0x3687_150F)).fmul_m(k(0x437E_0000)).ftol() as u8)
    };
    rp_geometry_for_all_materials(rd::<GPtr<u8>>(atomic, 0x18), 0x6FE220, alpha);
    atomic_render_cb(rd::<u32>(atomic, 0x48), atomic);
    atomic
}

orig! {
    /// `CSprite::Set4Vertices2D(RwD3D9Vertex*, const CRect&, const CRGBA& ×4)` (0x70E1C0)
    #[allow(clippy::too_many_arguments)]
    fn sprite_set4_vertices_2d_rect(v: GPtr<u8>, rect: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x70E1C0 => unported;
    /// `CSprite::FlushSpriteBuffer()` (0x70CF20)
    fn sprite_flush_sprite_buffer() = cdecl 0x70CF20 => unported;
}

/// `CSprite::nSpriteBufferIndex`, the sprite vertex buffer (4 × 0x1C per sprite) and the shared index list.
const SPRITE_INDEX: usize = 0xC6A158;
const SPRITE_VERTS: usize = 0xC5F958;
const SPRITE_IDX_LIST: usize = 0xC4B958;

/// The index block and counter tail shared by the buffered sprite renderers: indices `4i, 4i+1, 4i+2,
/// 4i+3, 4i, 4i+2`, index + 1, `FlushSpriteBuffer()` at 0x180 sprites.
fn sprite_buffer_tail(i: u32) {
    let b = g::<u16>(SPRITE_IDX_LIST).byte_add(i as isize * 12);
    let c = (i * 4) as u16;
    b.wrapping_index(1).write(c + 1);
    b.wrapping_index(3).write(c + 3);
    g::<u32>(SPRITE_INDEX).write(i + 1);
    b.wrapping_index(0).write(c);
    b.wrapping_index(2).write(c + 2);
    b.wrapping_index(4).write(c);
    b.wrapping_index(5).write(c + 2);
    if (i + 1) as i32 >= 0x180 {
        sprite_flush_sprite_buffer();
    }
}

/// `CSprite::RenderBufferedOneXLUSprite2D(float x, float y, float w, float h, const RwRGBA& colour, int16
/// intensity, uint8 alpha)` (0x70F440; a SecuROM fragment jump right after the prologue): 2D mode
/// (0xC6A15C = 1); the rect `(x - w, y + h, x + w, y - h)` with colour `(c * intensity) >> 8` (signed)
/// and `alpha`, built in the original's intensity argument slot, goes to `Set4Vertices2D` for the next
/// buffered sprite; then the index block.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70F440, conv = cdecl, name = "CSprite::RenderBufferedOneXLUSprite2D", expect = "83 EC 10 E9 DA 1F CF FF",
       unverified = "mvpd-render-3", route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "writes the scaled colour into its own intensity argument slot")]
pub fn render_buffered_one_xlu_sprite_2d(px: f32, py: f32, w: f32, h: f32, colour: GPtr<u8>, intensity: u32, alpha: u32) {
    let ii = i32::from(intensity as u16 as i16);
    g::<u32>(0xC6A15C).write(1);
    let mut rect: [f32; 4] = [
        x(px).fsub_m(w).to_f32(),
        x(py).fadd_m(h).to_f32(),
        x(px).fadd_m(w).to_f32(),
        x(py).fsub_m(h).to_f32(),
    ];
    let c = colour.cast::<[u8; 4]>().read();
    let mut col: [u8; 4] = [
        ((i32::from(c[0]) * ii) >> 8) as u8,
        ((i32::from(c[1]) * ii) >> 8) as u8,
        ((i32::from(c[2]) * ii) >> 8) as u8,
        alpha as u8,
    ];
    let i = g::<u32>(SPRITE_INDEX).read();
    let cp = lp(&mut col);
    sprite_set4_vertices_2d_rect(g::<u8>(SPRITE_VERTS).byte_add(i as isize * 0x70), lp(&mut rect), cp, cp, cp, cp);
    sprite_buffer_tail(i);
}

/// `CSprite::RenderBufferedOneXLUSprite(CVector pos, float w, float h, uint8 r, g, b, int16 intensity,
/// float recipNearZ, uint8 alpha)` (0x70E4A0): 3D mode (0xC6A15C = 0); the screen quad `pos.xy ± (w, h)`
/// (corners (x-w,y-h), (x-w,y+h), (x+w,y+h), (x+w,y-h), uv 0/1) is clipped to 0..RsGlobal width/height
/// (0xC17044 / 0xC17048), the clipped side's uv moved by `-v / size * 0.5` or `1 - (v - max) * 0.5 /
/// size`; z = `(far2D - near2D) * (z - nearClip) * farClip / ((farClip - nearClip) * z) + near2D`
/// (0xC4B8D4/D8, 0xC3EFA0/9C), rhw = `recipNearZ`, colour `alpha << 24 | (rgb * intensity) >> 8`; then
/// the index block.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x70E4A0, conv = cdecl, name = "CSprite::RenderBufferedOneXLUSprite", expect = "83 EC 40 D9 44 24 44",
       route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "writes the scaled blue into its own r argument slot (byte +2)")]
pub fn render_buffered_one_xlu_sprite(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, alpha: u32) {
    g::<u32>(0xC6A15C).write(0);
    let mut xs = [x(px).fsub_m(w).to_f32(), 0.0, x(px).fadd_m(w).to_f32(), 0.0];
    xs[1] = xs[0];
    xs[3] = xs[2];
    let ym = x(py).fsub_m(h).to_f32();
    let yp = x(py).fadd_m(h).to_f32();
    let mut ys = [ym, yp, yp, ym];
    let mut us = [0f32, 0.0, 1.0, 1.0];
    let mut vs = [0f32, 1.0, 1.0, 0.0];
    let wmax = X87::from_i32(g::<i32>(0xC17044).read());
    let hmax = X87::from_i32(g::<i32>(0xC17048).read());
    for i in 0..4 {
        if !jp(x(xs[i]), X87::zero(), 5) {
            let v = xs[i];
            xs[i] = 0.0;
            us[i] = x(v).fdiv_m(w).fmul_m(k(0xBF00_0000)).to_f32();
        }
        if !x(xs[i]).fcom(wmax).test_any(0x41) {
            us[i] = x(xs[i]).fsub(wmax).fmul_m(k(0x3F00_0000)).fdiv_m(w).fsubr_m(k(0x3F80_0000)).to_f32();
            xs[i] = wmax.to_f32();
        }
        if !jp(x(ys[i]), X87::zero(), 5) {
            let v = ys[i];
            ys[i] = 0.0;
            vs[i] = x(v).fdiv_m(h).fmul_m(k(0xBF00_0000)).to_f32();
        }
        if !jp(hmax, x(ys[i]), 5) {
            vs[i] = x(ys[i]).fsub(hmax).fmul_m(k(0x3F00_0000)).fdiv_m(h).fsubr_m(k(0x3F80_0000)).to_f32();
            ys[i] = hmax.to_f32();
        }
    }
    let idx = g::<u32>(SPRITE_INDEX).read();
    let a = x(g::<f32>(0xC4B8D4).read()).fsub_m(g::<f32>(0xC4B8D8).read());
    let bz = x(pz).fsub_m(g::<f32>(0xC3EFA0).read());
    let v = g::<u8>(SPRITE_VERTS).byte_add(idx as isize * 0x70);
    let ii = i32::from(intensity as u16 as i16);
    let num = a.fmul(bz).fmul_m(g::<f32>(0xC3EF9C).read());
    let den = x(g::<f32>(0xC3EF9C).read()).fsub_m(g::<f32>(0xC3EFA0).read()).fmul_m(pz);
    let z = num.fdiv(den).fadd_m(g::<f32>(0xC4B8D8).read()).to_f32();
    let cr = ((i32::from(r as u8) * ii) >> 8) as u8;
    let cg = ((i32::from(gg as u8) * ii) >> 8) as u8;
    let cb = ((i32::from(b as u8) * ii) >> 8) as u8;
    let col = (u32::from(alpha as u8) << 24) | (u32::from(cr) << 16) | (u32::from(cg) << 8) | u32::from(cb);
    for c in 0..4isize {
        let e = v.byte_add(c * 0x1C);
        wr::<f32>(e, 0, xs[c as usize]);
        wr::<f32>(e, 4, ys[c as usize]);
        wr::<f32>(e, 8, z);
        wr::<u32>(e, 0xC, recip_z);
        wr::<u32>(e, 0x10, col);
        wr::<f32>(e, 0x14, us[c as usize]);
        wr::<f32>(e, 0x18, vs[c as usize]);
    }
    sprite_buffer_tail(idx);
}

orig! {
    /// `CGrassRenderer::DrawTriPlants(PPTriPlant*, int32 count, RpAtomic** models, RwMatrix*)` (0x5DAD00)
    fn grass_draw_tri_plants(plants: GPtr<u8>, n: i32, models: u32, m: u32) = cdecl 0x5DAD00 => unported;
    /// `CPPTriPlantBuffer::Flush()` (0x5DB0C0)
    fn tri_plant_buffer_flush_o(this: GPtr<u8>) = thiscall 0x5DB0C0 => crate::mvpd_r3::tri_plant_buffer_flush;
    /// `RwIm2DRenderIndexedPrimitive` (`RwEngineInstance` slot 0x34)
    fn im2d_render_indexed(prim: u32, verts: GPtr<u8>, nv: i32, idx: GPtr<u8>, ni: i32) -> i32 = cdecl table 0xC97B24 0x34 => unported;
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(x, y, z, float* level, float* bigWaves, float* smallWaves)` (0x6E8580)
    fn water_level_no_waves(px: u32, py: u32, pz: u32, out: GPtr<u8>, big: GPtr<u8>, small: GPtr<u8>) -> u8 = cdecl 0x6E8580 => unported;
}

/// `CPPTriPlantBuffer::Flush()` (0x5DB0C0, thiscall): with plants stored (+0), the model table by pipe
/// mode (+0xB84: 0..3 → +0xB88/+0xB8C/+0xB90/+0xB94, else NULL); `r = rand()`, `DrawTriPlants(+4, count,
/// models, NULL)`, count = 0, `srand(r)`.
#[hook(addr = 0x5DB0C0, conv = thiscall, name = "CPPTriPlantBuffer::Flush", expect = "56 8B F1 83 3E 00",
       route = "L2", hazards = "none",
       recs = "grove-walk", note = "rand site 0x5DB103; reseeds the CRT generator with the value it drew")]
pub fn tri_plant_buffer_flush(this: GPtr<u8>) {
    if rd::<i32>(this, 0) <= 0 {
        return;
    }
    let models = match rd::<u32>(this, 0xB84) {
        0 => rd::<u32>(this, 0xB88),
        1 => rd::<u32>(this, 0xB8C),
        2 => rd::<u32>(this, 0xB90),
        3 => rd::<u32>(this, 0xB94),
        _ => 0,
    };
    let r = sa_core::orig::at_site(0x5D_B103, sa_core::crt::rand);
    grass_draw_tri_plants(this.byte_add(4), rd::<i32>(this, 0), models, 0);
    wr::<u32>(this, 0, 0);
    sa_core::orig::at_site(0x5D_B120, || sa_core::crt::srand(r as u32));
}

/// `CGrassRenderer::AddTriPlant(PPTriPlant*, uint32 pipeMode)` (0x5DB1D0, cdecl): a pipe mode change
/// (0xC0396C) flushes the buffer (0xC02DE8); a full buffer (32) too; the 0x5C-byte plant is copied into
/// the next slot (0xC02DEC), the count raised and the buffer flushed when it reaches 32.
#[hook(addr = 0x5DB1D0, conv = cdecl, name = "CGrassRenderer::AddTriPlant", expect = "A1 6C 39 C0 00",
       unverified = "mvpd-render-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn grass_add_tri_plant(plant: GPtr<u8>, mode: u32) {
    let buf = g::<u8>(0xC02DE8);
    if g::<u32>(0xC0396C).read() != mode {
        tri_plant_buffer_flush_o(buf);
        g::<u32>(0xC0396C).write(mode);
    }
    if rd::<i32>(buf, 0) + 1 > 0x20 {
        tri_plant_buffer_flush_o(buf);
    }
    let dst = g::<u32>(0xC02DEC).byte_add(rd::<i32>(buf, 0) as isize * 0x5C);
    for i in 0..0x17i32 {
        dst.wrapping_index(i).write(plant.cast::<u32>().wrapping_index(i).read());
    }
    if g::<u32>(0xC0396C).read() == mode {
        let n = rd::<i32>(buf, 0) + 1;
        wr::<i32>(buf, 0, n);
        if n >= 0x20 {
            tri_plant_buffer_flush_o(buf);
        }
    }
}

/// `CPostEffects::ColourFilter(RwRGBA pass1, RwRGBA pass2)` (0x703650, cdecl, both by value): states (9,1),
/// (14,0), (6,0), (8,0), raster = the front-buffer copy (0xC402D8), (12,1), (10,5), (11,2); the 4
/// screen vertices (0xC400D8, colour +0x10) get pass 1 then pass 2 as 0xAARRGGBB, each drawn with
/// `RwIm2DRenderIndexedPrimitive(TRILIST, …, 4, 0x8D5174, 6)`; then (9,2), (6,1), (8,1), (1,0), (10,5) and,
/// through a tail jump that rewrites the original's own argument slots, (11,6).
#[hook(addr = 0x703650, conv = cdecl, name = "CPostEffects::ColourFilter", expect = "A1 24 7B C9 00",
       route = "L2", hazards = "stack-residue",
       recs = "grove-walk", note = "its last RwRenderStateSet is a tail jump with (0xB, 6) written over its own argument slots")]
pub fn colour_filter(pass1: u32, pass2: u32) {
    rs(9, 1);
    rs(0xE, 0);
    rs(6, 0);
    rs(8, 0);
    rs(1, g::<u32>(0xC402D8).read());
    rs(0xC, 1);
    rs(0xA, 5);
    rs(0xB, 2);
    for c in [pass1, pass2] {
        let [r, gg, b, a] = c.to_le_bytes();
        let col = (u32::from(a) << 24) | (u32::from(r) << 16) | (u32::from(gg) << 8) | u32::from(b);
        for v in 0..4isize {
            g::<u32>(0xC400E8).byte_add(v * 0x1C).write(col);
        }
        im2d_render_indexed(3, g(0xC400D8), 4, g(0x8D5174), 6);
    }
    rs(9, 2);
    rs(6, 1);
    rs(8, 1);
    rs(1, 0);
    rs(0xA, 5);
    rs(0xB, 6);
}

/// `CWaterLevel::SetUpWaterFog(int32 x0, y0, x1, y1)` (0x6EA9F0; the entry is a SecuROM fragment jump):
/// with water fog on (0x8D37D4) and fewer than 70 fog rects (0xC228E4): the water level at (x0, y0)
/// (`GetWaterLevelNoWaves`; 0 when none; skipped when both wave heights are 0); the player is "in fog"
/// (0xC228E8 = 1) when below level + 12 and inside the rect (inclusive); the rect (i16 ×4 at
/// 0xC21188/0xC21214/0xC212A0/0xC2132C) and level (0xC213B8) are stored, count + 1.
#[hook(addr = 0x6EA9F0, conv = cdecl, name = "CWaterLevel::SetUpWaterFog", expect = "E9 04 91 D1 FF 83 EC 20",
       route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk", note = "entry is a SecuROM fragment jump (reads the water-fog flag)")]
pub fn set_up_water_fog(x0: i32, y0: i32, x1: i32, y1: i32) {
    if g::<u8>(0x8D37D4).read() == 0 || g::<i32>(0xC228E4).read() >= 0x46 {
        return;
    }
    let mut p = [0f32; 3];
    find_player_coors(lp(&mut p), -1);
    let y0f = X87::from_i32(y0).to_f32();
    let x0f = X87::from_i32(x0).to_f32();
    let (mut level, mut big, mut small) = (0f32, 0f32, 0f32);
    let found = water_level_no_waves(x0f.to_bits(), y0f.to_bits(), 0, lp(&mut level), lp(&mut big), lp(&mut small));
    let h = if found == 0 {
        X87::zero()
    } else if jp(x(big), X87::zero(), 0x44) || jp(x(small), X87::zero(), 0x44) {
        x(level)
    } else {
        return;
    };
    let below = !x(g::<f32>(0x8D37E4).read()).fadd(h).fcom(x(p[2])).test_any(0x41);
    let inx = !x(p[0]).fcom(x(x0f)).c0() && !X87::from_i32(x1).fcom(x(p[0])).c0();
    let iny = !x(p[1]).fcom(x(y0f)).c0() && !X87::from_i32(y1).fcom(x(p[1])).c0();
    if below && inx && iny {
        g::<u8>(0xC228E8).write(1);
    }
    let n = g::<i32>(0xC228E4).read() as isize;
    g::<i16>(0xC21188).byte_add(n * 2).write(x0 as i16);
    g::<i16>(0xC21214).byte_add(n * 2).write(y0 as i16);
    g::<i16>(0xC212A0).byte_add(n * 2).write(x1 as i16);
    g::<i16>(0xC2132C).byte_add(n * 2).write(y1 as i16);
    g::<f32>(0xC213B8).byte_add(n * 4).write(h.to_f32());
    g::<i32>(0xC228E4).write(n as i32 + 1);
}

/// 0x6FBA20 (`thunk_FUN_004064a7`, called by `CCoronas::RenderSunReflection`; the entry is a SecuROM
/// fragment jump), cdecl `(float x, float y, float, float r)` returning ST0: d = 2D distance from the
/// camera (stored into the x argument slot); `1.0` when `r + 100 <= d`, `0.0` when `d <= r`, else
/// `(d - r) * 0.01`.
#[hook(addr = 0x6FBA20, conv = cdecl, name = "thunk_FUN_004064a7", expect = "E9 82 AA D0 FF 85 C0",
       unverified = "mvpd-render-3", route = "L2", hazards = "x87-transcendental,stack-residue,entry-protection-read",
       recs = "grove-walk", note = "FSQRT; distance written into its own first argument slot; SecuROM entry jump")]
pub fn sun_reflection_fade(px: f32, py: f32, _unused: u32, r: f32) -> X87 {
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
    let dx = x(rd::<f32>(cam, 0)).fsub_m(px);
    let dy = x(rd::<f32>(cam, 4)).fsub_m(py);
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
    if x(r).fadd_m(k(0x42C8_0000)).fcom(x(d)).test_any(0x41) {
        return X87::one();
    }
    if !x(d).fcom(x(r)).test_parity(0x41) {
        return X87::zero();
    }
    x(d).fsub_m(r).fmul_m(k(0x3C23_D70A))
}

orig! {
    /// `CShinyTexts::RenderOutGeometryBuffer()` (0x7221C0)
    fn shiny_render_out_geometry_buffer() = cdecl 0x7221C0 => unported;
    /// `LittleTest()` (0x541330)
    fn little_test() = cdecl 0x541330 => unported;
}

/// `CShinyTexts::Render()` (0x724890): with registered texts (0xC7C6F8): states (20,1), (8,0), (10,2),
/// (11,2), the temp buffer emptied; for each text (0x58 bytes at 0xC7D258) the buffer is flushed
/// (`RenderOutGeometryBuffer`) past 0xFC0 indices or 0x7C0 vertices; type 1 texts (+0x54) switch the
/// raster to the white texture ([0xC40404]) when it changed (flush first) and add a quad: 4 vertices
/// (corners +0..+0x2C, uv +0x30..+0x4C, colour `0xFF << 24 | rgb +0x55..+0x57`), indices v, v+1,
/// v+2, v+2, v+1, v+3. Leftovers are drawn (`LittleTest()`, `RwIm3DTransform(…, NULL, 1)`, TRILIST).
/// Finally (12,0), count 0, (8,1), (10,5), (11,6).
#[hook(addr = 0x724890, conv = cdecl, name = "CShinyTexts::Render", expect = "83 EC 08 A1 F8 C6 C7 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn shiny_texts_render() {
    let count = g::<i32>(0xC7C6F8);
    if count.read() == 0 {
        return;
    }
    let mut last_tex = 0u32;
    rs(0x14, 1);
    rs(8, 0);
    rs(0xA, 2);
    rs(0xB, 2);
    let tbi = g::<u32>(0xC4B954);
    let tbv = g::<u32>(0xC4B950);
    tbi.write(0);
    tbv.write(0);
    if count.read() > 0 {
        let mut idx = 0u32;
        let mut verts = 0u32;
        let mut i = 0i32;
        while i < count.read() {
            let e = g::<u8>(0xC7D258).byte_add(i as isize * 0x58);
            if idx as i32 > 0xFC0 || verts as i32 > 0x7C0 {
                shiny_render_out_geometry_buffer();
                idx = tbi.read();
                verts = tbv.read();
            }
            if rd::<u8>(e, 0x54) == 1 {
                let white = g::<u32>(0xC40404).read();
                if last_tex != white {
                    shiny_render_out_geometry_buffer();
                    rs(1, GPtr::<u32>::from_addr(g::<u32>(0xC40404).read() as usize).read());
                    last_tex = g::<u32>(0xC40404).read();
                    verts = tbv.read();
                    idx = tbi.read();
                }
                let col = ((((u32::from(rd::<u8>(e, 0x55)) | 0xFFFF_FF00) << 8) | u32::from(rd::<u8>(e, 0x56))) << 8) | u32::from(rd::<u8>(e, 0x57));
                let vb = g::<u8>(TEMP_VERTS).byte_add(verts as isize * 0x24);
                for c in 0..4isize {
                    let v = vb.byte_add(c * 0x24);
                    wr::<u32>(v, 0x18, col);
                    wr::<u32>(v, 0, rd::<u32>(e, c * 12));
                    wr::<u32>(v, 4, rd::<u32>(e, c * 12 + 4));
                    wr::<u32>(v, 8, rd::<u32>(e, c * 12 + 8));
                    wr::<f32>(v, 0x1C, x(rd::<f32>(e, 0x30 + c * 8)).to_f32());
                    wr::<f32>(v, 0x20, x(rd::<f32>(e, 0x34 + c * 8)).to_f32());
                }
                let il = g::<u16>(0xC4B958).byte_add(idx as isize * 2);
                let v16 = verts as u16;
                for (j, d) in [0u16, 1, 2, 2, 1, 3].iter().enumerate() {
                    il.wrapping_index(j as i32).write(v16.wrapping_add(*d));
                }
                idx += 6;
                tbi.write(idx);
                verts += 4;
                tbv.write(verts);
            }
            i += 1;
        }
        if idx != 0 {
            little_test();
            if !rw_im3d_transform(g(TEMP_VERTS), tbv.read(), 0, 1).is_null() {
                rw_im3d_render_indexed_primitive(3, g(0xC4B958), tbi.read());
                rw_im3d_end();
            }
            tbv.write(0);
            tbi.write(0);
        }
    }
    count.write(0);
    rs(0xC, 0);
    rs(8, 1);
    rs(0xA, 5);
    rs(0xB, 6);
}

/// `FxManager_c::CalcFrustumInfo(RwCamera*)` (0x4A9140, thiscall, `ret 4`): with s = |view window|
/// (+0x68/+0x6C, FSQRT), the half angle `a = atan2(s, 1.0) * 57.2958` (kept in the camera argument
/// slot) and `r = sqrt(s² + 1) * farPlane / sin((180 - 2a) * π/180) * sin(a * π/180)`, the frustum
/// sphere centre `pos + at * r` goes to +0x34 and r to +0x40; four of the camera's frustum planes
/// (normal + distance at +0xBC/+0xD0/+0xE4/+0xF8) are copied to +0x48.. (0x10 apart).
#[hook(addr = 0x4A9140, conv = thiscall, name = "FxManager_c::CalcFrustumInfo", this = "FxManager_c", expect = "83 EC 0C 8B 44 24 10",
       route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "grove-walk", note = "FSQRT/FPATAN/FSIN; the angle is stored into its own argument slot")]
pub fn calc_frustum_info(this: GPtr<u8>, cam: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(cam, 4).byte_add(0x10);
    let vx = x(rd::<f32>(cam, 0x68));
    let vy = x(rd::<f32>(cam, 0x6C));
    let s = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
    let a = s.fpatan(X87::from_f64_exact(1.0)).fmul_m(k(0x4265_2EED)).to_f32();
    let d = s.fmul(s).fadd_m(k(0x3F80_0000)).fsqrt().fmul_m(rd::<f32>(cam, 0x84));
    let den = x(a).fdouble().fsubr_m(k(0x4334_0000)).fmul_m(k(0x3C8E_FA2E)).fsin();
    let r = d.fdiv(den).fmul(x(a).fmul_m(k(0x3C8E_FA2E)).fsin());
    let ax = r.fmul_m(rd::<f32>(m, 0x20));
    let ay = r.fmul_m(rd::<f32>(m, 0x24));
    let az = r.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let cx = x(rd::<f32>(m, 0x30)).fadd(ax).to_f32();
    let cy = x(rd::<f32>(m, 0x34)).fadd(ay).to_f32();
    let cz = x(rd::<f32>(m, 0x38)).fadd_m(az).to_f32();
    wr::<f32>(this, 0x34, cx);
    wr::<f32>(this, 0x38, cy);
    wr::<f32>(this, 0x3C, cz);
    wr::<f32>(this, 0x40, r.to_f32());
    for p in 0..4isize {
        for w in 0..4isize {
            wr::<u32>(this, 0x48 + p * 0x10 + w * 4, rd::<u32>(cam, 0xBC + p * 0x14 + w * 4));
        }
    }
}

/// `CSkidmarks::Render()` (0x720640): states (8,0), (12,1), (10,5), (11,6), (14,1), raster of the skid
/// texture ([0xC79A88]). Each of the 32 skidmarks (0x158 bytes at 0xC79AA8: positions +0, side x +0xC0,
/// side y +0x100, fade start +0x148, fade end +0x14C, type +0x150, sections i16 +0x154, state +0x156)
/// in use with at least one section: colour by type (0x8D5D78 / 0x8D5D80 / 0x8D5D7C / 0x8D5D84, else
/// black) scaled by `(1 - [0x8D12C0]) * 0.6 + 0.4`; alpha 255 while state 1 or before the fade, else
/// `(end - now) * 255 / (end - start)` (unsigned); sections 0..=n become vertex pairs `pos ± side` at
/// z + 0.1, u 0/1, v `i * 5.01`, alpha `(0x80 * alpha) >> 8` (0 for the first pair and, in state 2, the
/// last); `LittleTest()`, `RwIm3DTransform(…, 2n + 2, NULL, 1)`, TRILIST with 6n indices from 0xC799C8.
/// Finally (12,0), (8,1), (6,1).
#[hook(addr = 0x720640, conv = cdecl, name = "CSkidmarks::Render", expect = "83 EC 20 A1 24 7B C9 00",
       route = "L2", hazards = "none", recs = "driving")]
pub fn skidmarks_render() {
    rs(8, 0);
    rs(0xC, 1);
    rs(0xA, 5);
    rs(0xB, 6);
    rs(0xE, 1);
    rs(1, g::<GPtr<u32>>(0xC79A88).read().read());
    for m in 0..32isize {
        let b = g::<u8>(0xC79AA8).byte_add(m * 0x158);
        let state = rd::<u8>(b, 0x156);
        if state == 0 {
            continue;
        }
        let n = i32::from(rd::<i16>(b, 0x154));
        if n < 1 {
            continue;
        }
        let tbl = match rd::<u32>(b, 0x150) {
            0 => g::<u32>(0x8D5D78).read(),
            1 => g::<u32>(0x8D5D80).read(),
            2 => g::<u32>(0x8D5D7C).read(),
            3 => g::<u32>(0x8D5D84).read(),
            _ => 0,
        };
        let c = tbl.to_le_bytes();
        let f = X87::one().fsub_m(g::<f32>(0x8D12C0).read()).fmul_m(k(0x3F19_999A)).fadd_m(k(0x3ECC_CCCD));
        let cr = X87::from_i32(i32::from(c[0])).fmul(f).ftol() as u8;
        let cg = X87::from_i32(i32::from(c[1])).fmul(f).ftol() as u8;
        let cb = X87::from_i32(i32::from(c[2])).fmul(f).ftol() as u8;
        let now = g::<u32>(0xB7CB84).read();
        let start = rd::<u32>(b, 0x148);
        let alpha = if state == 1 || now < start {
            0xFF
        } else {
            let end = rd::<u32>(b, 0x14C);
            end.wrapping_sub(now).wrapping_mul(0xFF) / end.wrapping_sub(start)
        };
        let alpha = i32::from(alpha as u16 as i16);
        for i in 0..=n {
            let m80: i32 = if i == 0 || (i == n && state == 2) { 0 } else { 0x80 };
            let a = ((m80 * alpha) >> 8) as i16 as i32 as u32;
            let col = (((((a << 8) | u32::from(cr)) << 8) | u32::from(cg)) << 8) | u32::from(cb);
            let p = b.byte_add(i as isize * 12);
            let sx = rd::<f32>(b, 0xC0 + i as isize * 4);
            let sy = rd::<f32>(b, 0x100 + i as isize * 4);
            let v0 = g::<u8>(TEMP_VERTS).byte_add(i as isize * 0x48);
            let v1 = v0.byte_add(0x24);
            wr::<f32>(v0, 0, x(sx).fadd_m(rd::<f32>(p, 0)).to_f32());
            wr::<f32>(v0, 4, x(sy).fadd_m(rd::<f32>(p, 4)).to_f32());
            let z = x(rd::<f32>(p, 8)).fadd_m(k(0x3DCC_CCCD)).to_f32();
            wr::<f32>(v0, 8, z);
            let t = X87::from_i32(i).fmul_m(k(0x40A0_51EC)).to_f32();
            wr::<f32>(v0, 0x20, t);
            wr::<f32>(v1, 0, x(rd::<f32>(p, 0)).fsub_m(sx).to_f32());
            wr::<u32>(v0, 0x18, col);
            wr::<u32>(v0, 0x1C, 0);
            wr::<u32>(v1, 0x18, col);
            wr::<u32>(v1, 0x1C, 0x3F80_0000);
            wr::<f32>(v1, 4, x(rd::<f32>(p, 4)).fsub_m(sy).to_f32());
            wr::<f32>(v1, 8, z);
            wr::<f32>(v1, 0x20, t);
        }
        little_test();
        if !rw_im3d_transform(g(TEMP_VERTS), (n * 2 + 2) as u32, 0, 1).is_null() {
            rw_im3d_render_indexed_primitive(3, g(0xC799C8), (n * 6) as u32);
            rw_im3d_end();
        }
    }
    rs(0xC, 0);
    rs(8, 1);
    rs(6, 1);
}

orig! {
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440)
    fn fx_rw_matrix_create(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9440 => unported;
    /// `FxManager_c::FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    fn fx_rw_matrix_destroy(this: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4A9460 => unported;
    /// `RwMatrixMultiply(RwMatrix* out, const RwMatrix* a, const RwMatrix* b)` (0x7F18B0)
    fn rw_matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `RwV3dLength(const RwV3d*)` (0x7EDAC0, ST0)
    fn rw_v3d_length(v: GPtr<u8>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `FxSystem_c::IsVisible()` (0x4AAF30, bool in AL)
    fn fx_system_is_visible(this: GPtr<u8>) -> u8 = thiscall 0x4AAF30 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_system_play(this: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::Stop()` (0x4AA390)
    fn fx_system_stop(this: GPtr<u8>) = thiscall 0x4AA390 => unported;
    /// `FxSystem_c::DoFxAudio(CVector)` (0x4AAC90; by value)
    fn fx_system_do_fx_audio_o(this: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4AAC90 => crate::mvpd_r3::fx_system_do_fx_audio;
    /// `FxPrimBP_c::Update(float time, float dt)` (vtable slot 8)
    fn v_prim_update(this: GPtr<u8>, time: u32, dt: u32) = thiscall vslot 0x8 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn key_gen_get_uppercase_key(s: u32) -> u32 = cdecl 0x53CF30 => unported;
    /// `CAEFireAudioEntity::AddAudioEvent(eAudioEvents, const CVector&)` (0x4DD3C0)
    fn fire_audio_add_audio_event(this: GPtr<u8>, ev: u32, pos: GPtr<u8>) = thiscall 0x4DD3C0 => unported;
}

/// `FxSystem_c::DoFxAudio(CVector pos)` (0x4AAC90, thiscall, `ret 0xC`): the upper-case keys of 13 effect
/// names (strings 0x85AA8C … 0x85AA04) are computed in turn; for each that equals the blueprint's
/// name key (`[[this+8]+8]`) the fire audio entity (+0x7C) gets event 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
/// 0x88, 0x89, 0x89, 0x89, 0x8A, 0x8B, 0x8C at `pos`.
#[hook(addr = 0x4AAC90, conv = thiscall, name = "FxSystem_c::DoFxAudio", expect = "83 EC 28 53 55",
       route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "audio entity events (heap)")]
pub fn fx_system_do_fx_audio(this: GPtr<u8>, px: u32, py: u32, pz: u32) {
    let names: [u32; 13] = [
        0x85AA8C, 0x85AA80, 0x85AA74, 0x85AA68, 0x85AA5C, 0x85AA54, 0x85AA44, 0x85A70C, 0x85AA34, 0x85AA24, 0x85AA18, 0x85AA10, 0x85AA04,
    ];
    let keys = names.map(key_gen_get_uppercase_key);
    let events: [u32; 13] = [0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x89, 0x89, 0x8A, 0x8B, 0x8C];
    let mut pos: [u32; 3] = [px, py, pz];
    for i in 0..13 {
        if rd::<u32>(rd::<GPtr<u8>>(this, 8), 8) == keys[i] {
            fire_audio_add_audio_event(this.byte_add(0x7C), events[i], lp(&mut pos));
        }
    }
}

/// `FxSystem_c::Update(RwCamera*, float timeDelta)` (0x4AAF70, thiscall, `ret 8`, bool in AL): status
/// (+0x51) 3 → true; 2 → 3, false. The world matrix (a pooled `FxRwMatrixCreate`, the local matrix +0x10
/// times the parent +0xC when set) is released before its position is read (original behaviour); the
/// camera distance (+0x58) is `RwV3dLength(camera frame pos - pos)`; cull distance = blueprint +0x18
/// (u16) / 256. With a bounding mode (blueprint +0x1A) a system beyond the cull distance or invisible
/// is culled (flag 0x10 of +0x62): a killed system (+0x50 == 1) replays (`Play`) when it comes back in
/// range in mode 3. Otherwise, unless killed: dt = rate (+0x60) * 0.001 * timeDelta; time (+0x54) =
/// const time (+0x5C) / 256 when +0x52, else time + dt; looping (mode 2 with a minimum interval) wraps
/// after length + interval (+0x64) with a new interval `rand() % 10000 * 0.0001 * (max - min) + min`
/// and flag 8 past the length; other modes past the length stop / clamp / wrap / stop (0..3; mode 0
/// also sets status 2 when playing); culled systems run at dt + 0.25; every prim (+0x78, blueprint
/// +0x1B of them) `Update(time, dt)`. Unkilled systems end with `DoFxAudio(pos)`. Returns false. The
/// original keeps dt and the cull distance in its own argument slots.
#[hook(addr = 0x4AAF70, conv = thiscall, name = "FxSystem_c::Update", expect = "83 EC 10 53 55",
       unverified = "mvpd-render-3", route = "L2", hazards = "stack-residue,heap-writes,ax-al-return",
       recs = "grove-walk", note = "rand site 0x4AB166; writes temporaries into both argument slots; reads a released pooled matrix")]
pub fn fx_system_update(this: GPtr<u8>, cam: GPtr<u8>, time_delta: f32) -> u8 {
    let status = rd::<u8>(this, 0x51);
    if status == 3 {
        return 1;
    }
    if status == 2 {
        wr::<u8>(this, 0x51, 3);
        return 0;
    }
    let fxman = g::<u8>(0xA9AE80);
    let mat = fx_rw_matrix_create(fxman);
    let parent = rd::<GPtr<u8>>(this, 0xC);
    if parent.is_null() {
        for i in 0..16isize {
            wr::<u32>(mat, i * 4, rd::<u32>(this, 0x10 + i * 4));
        }
    } else {
        rw_matrix_multiply(mat, this.byte_add(0x10), parent);
    }
    let prev = rd::<f32>(this, 0x58);
    let cp = rd::<GPtr<u8>>(cam, 4).byte_add(0x40);
    fx_rw_matrix_destroy(fxman, mat);
    let mp = mat.byte_add(0x30);
    let mut d: [f32; 3] = [
        x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(mp, 0)).to_f32(),
        x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(mp, 4)).to_f32(),
        x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(mp, 8)).to_f32(),
    ];
    let dist = X87::from_f64_exact(rw_v3d_length(lp(&mut d)));
    wr::<f32>(this, 0x58, dist.to_f32());
    let bp = rd::<GPtr<u8>>(this, 8);
    let cull = X87::from_i32(i32::from(rd::<u16>(bp, 0x18))).fmul_m(k(0x3B80_0000)).to_f32();
    let mut culled = false;
    let mut time_path = false;
    if rd::<u8>(bp, 0x1A) == 0 || (!jp(dist, x(cull), 5) && fx_system_is_visible(this) != 0) {
        time_path = rd::<u8>(this, 0x50) == 0;
    } else {
        culled = true;
    }
    if time_path {
        let mut dt = X87::from_i32(i32::from(rd::<u16>(this, 0x60))).fmul_m(k(0x3A83_126F)).fmul_m(time_delta).to_f32();
        let t = if rd::<u8>(this, 0x52) != 0 {
            X87::from_i32(i32::from(rd::<u16>(this, 0x5C))).fmul_m(k(0x3B80_0000))
        } else {
            x(dt).fadd_m(rd::<f32>(this, 0x54))
        };
        wr::<f32>(this, 0x54, t.to_f32());
        let bp = rd::<GPtr<u8>>(this, 8);
        let mode = rd::<u8>(bp, 0x1A);
        if mode == 2 && !x(rd::<f32>(bp, 0x10)).fcom(X87::zero()).test_any(0x41) {
            if !x(rd::<f32>(this, 0x54)).fcom(x(rd::<f32>(bp, 0xC))).test_any(0x41) {
                wr::<u8>(this, 0x62, rd::<u8>(this, 0x62) | 8);
            }
            let l = x(rd::<f32>(bp, 0xC)).fadd_m(rd::<f32>(this, 0x64));
            if !jp(l, x(rd::<f32>(this, 0x54)), 5) {
                wr::<f32>(this, 0x54, x(rd::<f32>(this, 0x54)).fsub(l).to_f32());
                let bp = rd::<GPtr<u8>>(this, 8);
                let r = sa_core::orig::at_site(0x4A_B166, sa_core::crt::rand) % 10000;
                let fl = rd::<u8>(this, 0x62) & 0xF7;
                let a = X87::from_i32(r).fmul_m(k(0x38D1_B717));
                let b = x(rd::<f32>(bp, 0x14)).fsub_m(rd::<f32>(bp, 0x10));
                wr::<u8>(this, 0x62, fl);
                wr::<f32>(this, 0x64, a.fmul(b).fadd_m(rd::<f32>(bp, 0x10)).to_f32());
            }
        } else if !x(rd::<f32>(this, 0x54)).fcom(x(rd::<f32>(bp, 0xC))).test_any(0x41) {
            match mode {
                0 => {
                    let playing = rd::<u8>(this, 0x51) == 1;
                    fx_system_stop(this);
                    if playing {
                        wr::<u8>(this, 0x51, 2);
                    }
                }
                1 => wr::<u32>(this, 0x54, rd::<u32>(bp, 0xC)),
                2 => wr::<f32>(this, 0x54, x(rd::<f32>(this, 0x54)).fsub_m(rd::<f32>(bp, 0xC)).to_f32()),
                3 => fx_system_stop(this),
                _ => {}
            }
        }
        if rd::<u8>(this, 0x62) & 0x10 != 0 {
            dt = x(dt).fadd_m(k(0x3E80_0000)).to_f32();
        }
        let n = rd::<i8>(rd::<GPtr<u8>>(this, 8), 0x1B);
        let mut i = 0i32;
        while i < i32::from(rd::<i8>(rd::<GPtr<u8>>(this, 8), 0x1B)) && n > 0 {
            let prim = rd::<GPtr<u8>>(this, 0x78).cast::<GPtr<u8>>().wrapping_index(i).read();
            v_prim_update(prim, rd::<u32>(this, 0x54), dt.to_bits());
            i += 1;
        }
    } else if rd::<u8>(this, 0x50) == 1
        && !jp(x(rd::<f32>(this, 0x58)), x(cull), 5)
        && !x(prev).fcom(x(cull)).c0()
        && rd::<u8>(rd::<GPtr<u8>>(this, 8), 0x1A) == 3
    {
        fx_system_play(this);
    }
    let fl = rd::<u8>(this, 0x62);
    wr::<u8>(this, 0x62, fl ^ ((u8::from(culled) << 4 ^ fl) & 0x10));
    if rd::<u8>(this, 0x50) == 0 {
        fx_system_do_fx_audio_o(this, rd::<u32>(mp, 0), rd::<u32>(mp, 4), rd::<u32>(mp, 8));
    }
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FX_SYSTEM_DO_FX_AUDIO,
    HOOK_FX_SYSTEM_UPDATE,
    HOOK_SKIDMARKS_RENDER,
    HOOK_SHINY_TEXTS_RENDER,
    HOOK_CALC_FRUSTUM_INFO,
    HOOK_TRI_PLANT_BUFFER_FLUSH,
    HOOK_GRASS_ADD_TRI_PLANT,
    HOOK_COLOUR_FILTER,
    HOOK_SET_UP_WATER_FOG,
    HOOK_SUN_REFLECTION_FADE,
    HOOK_RENDER_BUFFERED_ONE_XLU_SPRITE_2D,
    HOOK_RENDER_BUFFERED_ONE_XLU_SPRITE,
    HOOK_CHECKPOINTS_RENDER,
    HOOK_ROPES_RENDER,
    HOOK_ROPE_RENDER,
    HOOK_IMMEDIATE_MODE_RENDER_STATES_STORE,
    HOOK_MOTION_BLUR_STREAKS_RENDER,
    HOOK_FX_MANAGER_RENDER,
    HOOK_FX_RENDER,
    HOOK_PLANE_TRAILS_RENDER,
    HOOK_PLANE_TRAIL_RENDER,
    HOOK_RENDER_ROADSIGN_ATOMIC,
];
