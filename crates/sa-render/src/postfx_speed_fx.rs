//! STD-15 (batch `std15-render-1`): `CPostEffects::SpeedFX(float speed)` (0x7030A0, unreversed in gta-reversed) and
//! `CPostEffects::DrawQuad` (0x700EC0), reached every frame the player drives. Ported from the 1.0 US bytes; the two
//! `rand()` calls run at their original call sites. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0)
    fn im_render_states_store() = cdecl 0x700CC0 => unported;
    /// `CPostEffects::ImmediateModeRenderStatesSet()` (0x700D70)
    fn im_render_states_set() = cdecl 0x700D70 => unported;
    /// `CPostEffects::ImmediateModeRenderStatesReStore()` (0x700E00)
    fn im_render_states_restore() = cdecl 0x700E00 => unported;
    /// `RwRenderStateSet(state, value)` (RwEngineInstance +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734E90)
    fn rw_im2d_render_primitive(ty: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
}

fn gu(a: usize) -> u32 {
    GPtr::<u32>::from_addr(a).read()
}
fn gf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}
fn su(a: usize, v: u32) {
    GPtr::<u32>::from_addr(a).write(v)
}
fn sf(a: usize, v: f32) {
    GPtr::<f32>::from_addr(a).write(v)
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// The quad's four `RwIm2DVertex`es (0x1C bytes each) at 0xC401D8.
const VERTS: usize = 0xC401D8;

/// `CPostEffects::DrawQuad(float x, float y, float w, float h, uint8 r, uint8 g, uint8 b, uint8 a, RwRaster*)`
/// (0x700EC0, cdecl): sets the texture raster (render state 1), writes the screen positions (x, y), (x + w, y),
/// (x, y + h), (x + w, y + h) with the z at 0xC40150 and the colour `a r g b` into the four vertices (the UVs stay
/// as the caller set them) and draws them as a triangle strip.
#[hook(addr = 0x700EC0, conv = cdecl, name = "CPostEffects::DrawQuad", expect = "8B 44 24 24 8B 0D 24 7B C9 00",
       unverified = "std15-render-1", route = "L2", hazards = "none", recs = "car-drive")]
#[allow(clippy::too_many_arguments)]
pub fn draw_quad(px: f32, py: f32, w: f32, h: f32, r: u32, g: u32, b: u32, a: u32, raster: u32) {
    rw_render_state_set(1, raster);
    let xw = x(px).fadd_m(w).to_f32();
    let yh = x(py).fadd_m(h).to_f32();
    let z = gu(0xC40150);
    let c = ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    sf(VERTS, px);
    sf(VERTS + 4, py);
    su(VERTS + 8, z);
    su(VERTS + 0x10, c);
    sf(VERTS + 0x1C, xw);
    sf(VERTS + 0x20, py);
    su(VERTS + 0x24, z);
    su(VERTS + 0x2C, c);
    sf(VERTS + 0x38, px);
    sf(VERTS + 0x3C, yh);
    su(VERTS + 0x40, z);
    su(VERTS + 0x48, c);
    sf(VERTS + 0x54, xw);
    sf(VERTS + 0x58, yh);
    su(VERTS + 0x5C, z);
    su(VERTS + 0x64, c);
    rw_im2d_render_primitive(4, GPtr::from_addr(VERTS), 4);
}

/// `CPostEffects::SpeedFX(float speed)` (0x7030A0, cdecl). The camera mode (`TheCamera.Cams[ActiveCam]` +0x... at
/// 0xB6F1C4 + 0x238 * the active index 0xB6F081) 1 or 2 halves the jitter and zeroes the shake. The speed picks the
/// highest of the seven (speed, passes, jitter, shake) rows at 0x8D5190 whose speed it reaches; without passes it
/// returns. Otherwise it stores and sets the immediate-mode render states; a shake (> 0) scales two `rand()` values
/// by the screen size (0xC4016C, 0xC40170) as the UV offsets; the jitter (the stack slot the second `rand()`
/// overwrote, when there was a shake) scales the per-pass UV steps. Each pass draws the full-screen quad (the
/// raster 0xC402D8, alpha 0x8D5104) with UVs moved out from the screen centre, the mode-1 / mode-2 slots cleared
/// every pass as the exe does. Afterwards the default UVs are restored and the render states restored (tail call).
#[hook(addr = 0x7030A0, conv = cdecl, name = "CPostEffects::SpeedFX", expect = "83 EC 3C 0F B6 05 81 F0 B6 00",
       unverified = "std15-render-1", route = "L2", hazards = "stack-residue", recs = "car-drive",
       note = "the jitter reads the slot the shake's second rand() overwrote, as the exe")]
pub fn speed_fx(speed: f32) {
    let active = u32::from(GPtr::<u8>::from_addr(0xB6F081).read());
    let mode = gu(0xB6F1C4 + (active * 0x238) as usize);
    let mode1 = mode == 1;
    let mode2 = mode == 2;
    // s[k] = the dword at B + 4k (B = ESP after `sub esp, 0x3C`)
    let mut s = [0u32; 16];
    let f = |s: &[u32; 16], o: usize| f32::from_bits(s[o / 4]);
    let (mut jitter, mut shake, mut passes) = (0i32, 0i32, 0i32);
    let mut row = 6usize;
    loop {
        if !x(speed).fcom(x(gf(0x8D5190 + row * 0x10))).c0() {
            passes = gu(0x8D5194 + row * 0x10) as i32;
            jitter = gu(0x8D5198 + row * 0x10) as i32;
            shake = gu(0x8D519C + row * 0x10) as i32;
            break;
        }
        if row == 0 {
            break;
        }
        row -= 1;
    }
    s[0x34 / 4] = jitter as u32;
    s[0x38 / 4] = shake as u32;
    if mode1 || mode2 {
        s[0x34 / 4] = (jitter / 2) as u32;
        shake = 0;
        s[0x38 / 4] = 0;
    }
    if passes <= 0 {
        return;
    }
    im_render_states_store();
    im_render_states_set();
    s[0x10 / 4] = 0;
    s[0xC / 4] = 0;
    s[0x14 / 4] = 0;
    s[0x28 / 4] = 0;
    if shake > 0 {
        let k = sa_math::x87::X87::from_i32(s[0x34 / 4] as i32).fmul_m(gf(0x859CE0));
        s[0x30 / 4] = x(gf(0xC4016C)).fmul(k).to_f32().to_bits();
        s[0x2C / 4] = x(gf(0xC40170)).fmul(k).to_f32().to_bits();
        let r1 = sa_core::orig::at_site(0x70_31A5, sa_core::crt::rand);
        s[0x34 / 4] = r1 as u32;
        s[0x10 / 4] = X87::from_i32(r1)
            .fmul_m(gf(0x858C7C))
            .fmul_m(f(&s, 0x30))
            .to_f32()
            .to_bits();
        let r2 = sa_core::orig::at_site(0x70_31C0, sa_core::crt::rand);
        s[0x34 / 4] = r2 as u32;
        s[0x14 / 4] = s[0x10 / 4];
        s[0xC / 4] = X87::from_i32(r2)
            .fmul_m(gf(0x858C7C))
            .fmul_m(f(&s, 0x2C))
            .to_f32()
            .to_bits();
        s[0x28 / 4] = s[0xC / 4];
    }
    let m = X87::from_i32(s[0x34 / 4] as i32);
    let dx = x(gf(0xC4016C)).fmul(m).fmul_m(gf(0x86C340)).to_f32();
    s[4 / 4] = dx.to_bits();
    let dy = m.fmul_m(gf(0xC40170)).fmul_m(gf(0x86C340)).to_f32();
    s[0x2C / 4] = dx.to_bits();
    s[0x38 / 4] = dx.to_bits();
    s[8 / 4] = dy.to_bits();
    let ndx = (-dx).to_bits();
    let ndy = (-dy).to_bits();
    s[0x30 / 4] = ndx;
    s[0x18 / 4] = dy.to_bits();
    s[0x34 / 4] = ndx;
    s[0x1C / 4] = dy.to_bits();
    s[0x20 / 4] = ndy;
    s[0x24 / 4] = ndy;
    let mut n = passes;
    while n > 0 {
        if mode1 {
            for o in [0x10, 0x14, 0xC, 0x28, 0x18, 0x30, 0x1C, 0x34, 0x20, 0x24] {
                s[o / 4] = 0;
            }
        }
        if mode2 {
            for o in [0x10, 0x14, 0xC, 0x28, 0x2C, 0x18, 0x1C, 0x20, 0x38, 0x24] {
                s[o / 4] = 0;
            }
        }
        let raster = gu(0xC402D8);
        let alpha = u32::from(GPtr::<u8>::from_addr(0x8D5104).read());
        let (u0, v0, u1, v1) = (0xC40164, 0xC40168, 0xC4016C, 0xC40170);
        sf(
            0xC401EC,
            x(gf(u0)).fadd_m(f(&s, 0x2C)).fadd_m(f(&s, 0x10)).to_f32(),
        );
        sf(
            0xC401F0,
            x(gf(v0)).fadd_m(f(&s, 0x18)).fadd_m(f(&s, 0xC)).to_f32(),
        );
        sf(
            0xC40208,
            x(gf(u1)).fadd_m(f(&s, 0x30)).fsub_m(f(&s, 0x14)).to_f32(),
        );
        sf(
            0xC4020C,
            x(gf(v0)).fadd_m(f(&s, 0x1C)).fadd_m(f(&s, 0xC)).to_f32(),
        );
        sf(
            0xC40224,
            x(gf(u0)).fadd_m(f(&s, 0x38)).fadd_m(f(&s, 0x10)).to_f32(),
        );
        sf(
            0xC40228,
            x(gf(v1)).fadd_m(f(&s, 0x24)).fsub_m(f(&s, 0x14)).to_f32(),
        );
        sf(
            0xC40240,
            x(gf(u1)).fadd_m(f(&s, 0x34)).fsub_m(f(&s, 0x14)).to_f32(),
        );
        sf(
            0xC40244,
            x(gf(v1)).fadd_m(f(&s, 0x20)).fsub_m(f(&s, 0x28)).to_f32(),
        );
        let w = X87::from_i32(gu(0xC17044) as i32).to_f32();
        let h = X87::from_i32(gu(0xC17048) as i32).to_f32();
        draw_quad_call(
            0.0f32.to_bits(),
            0.0f32.to_bits(),
            w.to_bits(),
            h.to_bits(),
            0xFF,
            0xFF,
            0xFF,
            alpha,
            raster,
        );
        let step = |s: &mut [u32; 16], o: usize, d: usize, add: bool| {
            let v = if add {
                x(f(s, o)).fadd_m(f(s, d))
            } else {
                x(f(s, o)).fsub_m(f(s, d))
            };
            s[o / 4] = v.to_f32().to_bits();
        };
        n -= 1;
        step(&mut s, 0x2C, 4, true);
        step(&mut s, 0x18, 8, true);
        step(&mut s, 0x30, 4, false);
        step(&mut s, 0x1C, 8, true);
        step(&mut s, 0x34, 4, false);
        step(&mut s, 0x20, 8, false);
        step(&mut s, 0x38, 4, true);
        step(&mut s, 0x24, 8, false);
    }
    sf(0xC401EC, 0.0);
    sf(0xC401F0, 0.0);
    sf(0xC40208, 1.0);
    sf(0xC4020C, 0.0);
    sf(0xC40224, 0.0);
    sf(0xC40228, 1.0);
    sf(0xC40240, 1.0);
    sf(0xC40244, 1.0);
    im_render_states_restore();
}

orig! {
    /// `CPostEffects::DrawQuad` (0x700EC0), called as the original calls it
    #[allow(clippy::too_many_arguments)]
    fn draw_quad_call(px: u32, py: u32, w: u32, h: u32, r: u32, g: u32, b: u32, a: u32, raster: u32) = cdecl 0x700EC0 => draw_quad_bits;
}

#[allow(clippy::too_many_arguments)]
fn draw_quad_bits(px: u32, py: u32, w: u32, h: u32, r: u32, g: u32, b: u32, a: u32, raster: u32) {
    draw_quad(
        f32::from_bits(px),
        f32::from_bits(py),
        f32::from_bits(w),
        f32::from_bits(h),
        r,
        g,
        b,
        a,
        raster,
    );
}

pub const HOOKS: &[HookReg] = &[HOOK_DRAW_QUAD, HOOK_SPEED_FX];
