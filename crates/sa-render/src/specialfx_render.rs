//! WRITER-RENDER-BIG-2: `CSpecialFX::Render2DFXs` (0x721660) and `CSprite2d::Draw2DPolygon` (0x7285B0).
//! Ported from the 1.0 US bytes; gta-reversed `SpecialFX.cpp` / `Sprite2d.cpp` as reading aid. x87
//! arithmetic follows the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, k, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xc97b24 0x20 => unported;
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734e90)
    fn im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734e90 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170c0); returns `this`
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170c0 => unported;
    /// `CSprite2d::Draw2DPolygon(x1, y1, .. y4, const CRGBA&)` (0x7285b0; floats as raw dwords)
    fn draw_2d_polygon_call(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, x4: u32, y4: u32, c: GPtr<u8>) = cdecl 0x7285b0 => self::draw_2d_polygon;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727b60)
    fn draw_rect(r: GPtr<u8>, c: GPtr<u8>) = cdecl 0x727b60 => crate::sprite2d::draw_rect;
    /// `CFont::SetScale(float, float)` (0x719380)
    fn font_set_scale(w: f32, h: f32) = cdecl 0x719380 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195c0)
    fn font_set_background(a: u32, b: u32) = cdecl 0x7195c0 => unported;
    /// `CFont::SetProportional(bool)` (0x7195b0)
    fn font_set_proportional(on: u32) = cdecl 0x7195b0 => unported;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(o: u32) = cdecl 0x719610 => unported;
    /// `CFont::SetColor(CRGBA)` (0x719430; the colour as a dword)
    fn font_set_color(c: u32) = cdecl 0x719430 => unported;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(s: u32) = cdecl 0x719490 => unported;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71a700)
    fn font_print_string(px: f32, py: f32, s: GPtr<u8>) = cdecl 0x71a700 => unported;
    /// CRT `sprintf(buf, fmt, int)` (0x821bb5)
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, v: u32) -> i32 = cdecl 0x821bb5 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => sa_core::common::ascii_to_gxt_char;
}

/// `CSprite2d::maVertices` (4 x `RwIm2DVertex`).
const SPRITE2D_VERTICES: usize = 0xc8_0468;
/// `CSpecialFX::bVideoCam`, `bLiftCam`, `bSnapShotActive`, `SnapShotFrames`.
const VIDEO_CAM: usize = 0xc7_c70c;
const LIFT_CAM: usize = 0xc7_c70d;
const SNAPSHOT_ACTIVE: usize = 0xc7_c714;
const SNAPSHOT_FRAMES: usize = 0xc7_c710;
/// `RsGlobal.maximumWidth` / `maximumHeight`; `CTimer::m_FrameCounter`, `m_snTimeInMilliseconds`,
/// `ms_fTimeScale`; `TheCamera.ActiveCam`.
const RS_WIDTH: usize = 0xc1_7044;
const RS_HEIGHT: usize = 0xc1_7048;
const FRAME_COUNTER: usize = 0xb7_cb4c;
const TIME_MS: usize = 0xb7_cb84;
const TIME_SCALE: usize = 0xb7_cb64;
const ACTIVE_CAM: usize = 0xb6_f081;
/// `gString`, `"%d"`, `gGxtString`.
const G_STRING: usize = 0xb7_1670;
const FMT_D: usize = 0x85_9508;
const G_GXT_STRING: usize = 0xc1_b100;

/// `CSprite2d::Draw2DPolygon(float x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA& c)` (0x7285B0):
/// `SetVertices(x1 .. y4, c, c, c, c)`, TEXTURERASTER null, SHADEMODE 1, VERTEXALPHA on unless `c.a ==
/// 255`, `RwIm2DRenderPrimitive(TRIFAN, maVertices, 4)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x7285b0, conv = cdecl, name = "CSprite2d::Draw2DPolygon", expect = "8B 44 24 20 8B 4C 24 1C",
       unverified = "renderbig2-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn draw_2d_polygon(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, x4: u32, y4: u32, c: GPtr<u8>) {
    crate::sprite2d::set_vertices_quad_call(x1, y1, x2, y2, x3, y3, x4, y4, c, c, c, c);
    rw_render_state_set(1, 0);
    rw_render_state_set(7, 1);
    let a = crate::common::rd::<u8>(c, 3);
    rw_render_state_set(12, u32::from(a != 0xff));
    im2d_render_primitive(5, GPtr::from_addr(SPRITE2D_VERTICES), 4);
}

fn fi(v: i32) -> u32 {
    xi(v).to_f32().to_bits()
}

/// A local `CRGBA` built by the original's constructor call.
fn rgba(r: u32, gg: u32, b: u32, a: u32) -> [u8; 4] {
    let mut c = [0u8; 4];
    rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), r, gg, b, a);
    c
}

/// The scan-line overlay of the video / lift camera: rows `y = 3, 7, ..` while `y - 3 < height`
/// (signed, at least one when height > 0): a one-blended `(0, y-3)..(W, y-2)` strip in `bright`, then
/// an alpha-blended `(0, y-1)..(W, y)` strip in (0, 0, 0, 150).
fn scan_lines(bright: (u32, u32, u32, u32)) {
    let h = g::<i32>(RS_HEIGHT).read();
    if h <= 0 {
        return;
    }
    let mut y: i32 = 3;
    loop {
        rw_render_state_set(10, 2);
        rw_render_state_set(11, 2);
        let mut c = rgba(bright.0, bright.1, bright.2, bright.3);
        let w = fi(g::<i32>(RS_WIDTH).read());
        let (a, b) = (fi(y - 3), fi(y - 2));
        draw_2d_polygon_call(0, a, w, a, 0, b, w, b, GPtr::from_raw(c.as_mut_ptr()));
        rw_render_state_set(10, 5);
        rw_render_state_set(11, 6);
        let mut c = rgba(0, 0, 0, 0x96);
        let w = fi(g::<i32>(RS_WIDTH).read());
        let (a, b) = (fi(y - 1), fi(y));
        draw_2d_polygon_call(0, a, w, a, 0, b, w, b, GPtr::from_raw(c.as_mut_ptr()));
        y += 4;
        if y - 3 >= g::<i32>(RS_HEIGHT).read() {
            break;
        }
    }
}

/// The moving band: `yt = ftol(ftol((height + 70) / 2048 * (ms & 0x7FF)) - 70)`; SRCALPHA /
/// INVSRCALPHA; `(0, yt)..(W, yt + 70)` in `c`.
fn band(c: (u32, u32, u32, u32)) {
    // 70 (0x858CE0), 1/2048 (0x863990)
    let seventy = k(0x428c_0000);
    let t = (g::<u32>(TIME_MS).read() & 0x7ff) as i32;
    let v = xi(g::<i32>(RS_HEIGHT).read()).fadd_m(seventy).fmul_m(k(0x3a00_0000)).fmul(xi(t));
    let yt = ftol32(xi(ftol32(v)).fsub_m(seventy));
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    let a = xi(yt);
    let ya = a.to_f32().to_bits();
    let yb = a.fadd_m(seventy).to_f32().to_bits();
    let mut col = rgba(c.0, c.1, c.2, c.3);
    let w = fi(g::<i32>(RS_WIDTH).read());
    draw_2d_polygon_call(0, ya, w, ya, 0, yb, w, yb, GPtr::from_raw(col.as_mut_ptr()));
}

/// `SetScale(W / 640 * 1.5, H / 448 * 1.5)`.
fn font_scale() {
    // 1/448 (0x859524), 1/640 (0x859520), 1.5 (0x858CE8)
    let h = xi(g::<i32>(RS_HEIGHT).read()).fmul_m(k(0x3b12_4925)).fmul_m(k(0x3fc0_0000)).to_f32();
    let w = xi(g::<i32>(RS_WIDTH).read()).fmul_m(k(0x3acc_cccd)).fmul_m(k(0x3fc0_0000)).to_f32();
    font_set_scale(w, h);
}

/// `CSpecialFX::Render2DFXs()` (0x721660). Video camera: font setup (scale, no background,
/// proportional, orientation 1, colour (0, 255, 0, 200), style 1), `gString = sprintf("%d", frame &
/// 63)` printed at `((W * 8) / 10, (H * 8) / 10)` (integer divisions), the scan lines in (0, 100, 0,
/// 100) and the band in (0, 100, 0, 60). Lift camera: the same setup (orientation before
/// proportional) in (100, 100, 100, 200) without text, scan lines (100, 100, 100, 100), band (100,
/// 100, 100, 60), then 200 rects `(x, y, x + 20, y + 2)` with `x = rand() % W`, `y = rand() % H` in
/// (255, 255, 255, 64). Snapshot: the active camera's alpha / beta speeds zeroed; frames + 1 above 20
/// ends it (time scale 1); otherwise time scale 0 and below 10 frames a one-blended full-screen quad of
/// shade `ftol((255 - frames * 255 / 10) * 0.65)` (gta-reversed omits the 0.65).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x721660, conv = cdecl, name = "CSpecialFX::Render2DFXs", expect = "A0 0C C7 C7 00 83 EC 28",
       unverified = "renderbig2-4", route = "L2", hazards = "none", recs = "grove-walk",
       note = "rand() at 0x721B85 / 0x721B97 (lift camera only)")]
pub fn render_2d_fxs() {
    if g::<u8>(VIDEO_CAM).read() != 0 {
        font_scale();
        font_set_background(0, 0);
        font_set_proportional(1);
        font_set_orientation(1);
        let c = rgba(0, 0xff, 0, 0xc8);
        font_set_color(u32::from_le_bytes(c));
        font_set_font_style(1);
        sprintf1(GPtr::from_addr(G_STRING), GPtr::from_addr(FMT_D), g::<u32>(FRAME_COUNTER).read() & 0x3f);
        ascii_to_gxt_char(GPtr::from_addr(G_STRING), GPtr::from_addr(G_GXT_STRING));
        let py = xi(g::<i32>(RS_HEIGHT).read().wrapping_mul(8) / 10).to_f32();
        let px = xi(g::<i32>(RS_WIDTH).read().wrapping_mul(8) / 10).to_f32();
        font_print_string(px, py, GPtr::from_addr(G_GXT_STRING));
        scan_lines((0, 0x64, 0, 0x64));
        band((0, 0x64, 0, 0x3c));
    }
    if g::<u8>(LIFT_CAM).read() != 0 {
        font_scale();
        font_set_background(0, 0);
        font_set_orientation(1);
        font_set_proportional(1);
        let c = rgba(0x64, 0x64, 0x64, 0xc8);
        font_set_color(u32::from_le_bytes(c));
        font_set_font_style(1);
        scan_lines((0x64, 0x64, 0x64, 0x64));
        band((0x64, 0x64, 0x64, 0x3c));
        for _ in 0..200 {
            let rx = sa_core::orig::at_site(0x72_1b85, sa_core::crt::rand).wrapping_rem(g::<i32>(RS_WIDTH).read());
            let ry = sa_core::orig::at_site(0x72_1b97, sa_core::crt::rand).wrapping_rem(g::<i32>(RS_HEIGHT).read());
            let mut c = rgba(0xff, 0xff, 0xff, 0x40);
            let mut rect = [fi(rx), fi(ry.wrapping_add(2)), fi(rx.wrapping_add(0x14)), fi(ry)];
            draw_rect(GPtr::<u32>::from_raw(rect.as_mut_ptr()).cast(), GPtr::from_raw(c.as_mut_ptr()));
        }
    }
    if g::<u8>(SNAPSHOT_ACTIVE).read() == 0 {
        return;
    }
    let cam = 0xb6_f24c + 0x238 * usize::from(g::<u8>(ACTIVE_CAM).read());
    g::<u32>(cam).write(0);
    let cam = 0xb6_f25c + 0x238 * usize::from(g::<u8>(ACTIVE_CAM).read());
    g::<u32>(cam).write(0);
    let frames = g::<u32>(SNAPSHOT_FRAMES).read().wrapping_add(1);
    g::<u32>(SNAPSHOT_FRAMES).write(frames);
    if frames > 0x14 {
        g::<u8>(SNAPSHOT_ACTIVE).write(0);
        g::<u32>(TIME_SCALE).write(0x3f80_0000);
        return;
    }
    g::<u32>(TIME_SCALE).write(0);
    if frames >= 10 {
        return;
    }
    let shade = 0xffu32.wrapping_sub(frames.wrapping_mul(0xff) / 10);
    let mut v = xi(shade as i32);
    if (shade as i32) < 0 {
        v = v.fadd_m(k(0x4f80_0000));
    }
    // 0.65 (0x858F50)
    let s = ftol32(v.fmul_m(k(0x3f26_6666))) as u32;
    rw_render_state_set(10, 2);
    rw_render_state_set(11, 2);
    let h = fi(g::<i32>(RS_HEIGHT).read());
    let w = fi(g::<i32>(RS_WIDTH).read());
    let mut c = rgba(s, s, s, s);
    draw_2d_polygon_call(0, 0, w, 0, 0, h, w, h, GPtr::from_raw(c.as_mut_ptr()));
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_2D_POLYGON, HOOK_RENDER_2D_FXS];
