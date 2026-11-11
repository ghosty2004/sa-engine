//! WRITER-MVP-B (helper A, batch `mvpb-weapon-1`): WEAPON functions on the MVP call path. Ported from
//! the 1.0 US bytes; docs/re/specs/WEAPON as reading aids. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_world::world::common::{f32_of, g, rd, x, xi};

orig! {
    /// `RwEngineInstance->dOpenDevice.fpRenderStateSet(RwRenderState, void*)` (`[0xC97B24] + 0x20`)
    fn rw_render_state_set(state: u32, v: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CSprite::RenderOneXLUSprite_Rotate_Aspect(float x, float y, float z, float w, float h, uint8 r,
    /// uint8 g, uint8 b, int16 intensity, float rz, float rotation, uint8 alpha)` (0x70D490)
    fn render_one_xlu_sprite_rotate_aspect(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, rz: f32, rotation: u32, alpha: u32) = cdecl 0x70D490 => unported;
}

/// History tables of the crosshair trail (`[2][5]` each, row stride 0x14).
const HIST_T: usize = 0xB9B8F8;
const HIST_Y: usize = 0xB9B920;
const HIST_X: usize = 0xB9B948;

/// `FUN_0056ef90` (0x56EF90, `ret 4`; docs/re/specs/WEAPON/_unattributed.md, the crosshair trail of
/// `CWeaponEffects::Render`; ECX = `&CWorld::Players[i] + 0x158`): nothing unless the activated byte
/// is set. Render states ZWRITE 0, ZTEST 0, VERTEXALPHA 1, SRCBLEND 5, DESTBLEND 8, raster of
/// `gpCoronaTexture[[0x8CDF1C]]`. Colour (r, g, b) = (0x32, 0, 0xFF) for player 1, else (0, 0xFF, 0x32).
/// The 5-entry history shifts by one (X, Y; the time slot is filled from the fixed address 0xB9B92C -
/// 4k, i.e. `Y[0][k]`'s bits: original bug, kept), entry 0 = this +4 / +8 and the current time. For
/// each entry j: `pulse = sin((T[idx][0] & 0x3FF) * 2pi/1024) * 0.2 + 1` (the original reads the time
/// of entry 0 for every j: the row offset is reloaded, never advanced), then 3 rings `r = m * 10 + 20`
/// of 4 sprites at `a = k * pi/2 + pi/4`: `(sin a * r * pulse + (X + 1) * W * 0.5, cos a * r * pulse +
/// (Y + 1) * H * 0.5, 100)`, size 15x15, intensity 255, rz 0.01, rotation 0, alpha 255; the colour
/// halves (logical shifts) after each entry. Finally ZWRITE 1, ZTEST 1.
#[hook(addr = 0x56EF90, conv = thiscall, name = "FUN_0056ef90", this = "CPlayerInfo", expect = "83 EC 18 56 8B F1",
       route = "L2", hazards = "x87-transcendental",
       note = "spec says T[idx][j] for the pulse phase; the bytes read T[idx][0] for every entry (ECX reloaded from esp+0x24 = o). The original keeps b in its own argument slot (entry esp+4)")]
pub fn crosshair_trail_render(this: GPtr<u8>, idx: i32) {
    if rd::<u8>(this, 0) == 0 {
        return;
    }
    rw_render_state_set(8, 0);
    rw_render_state_set(6, 0);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 8);
    let ti = g::<i32>(0x8CDF1C).read();
    let tex = g::<GPtr<u8>>((0xC3E000u32 as usize).wrapping_add((ti as usize).wrapping_mul(4))).read();
    rw_render_state_set(1, rd::<u32>(tex, 0));
    let (mut r, mut gg, mut b): (u32, u32, u32) = if idx != 0 { (0x32, 0, 0xFF) } else { (0, 0xFF, 0x32) };
    let o = (idx as usize).wrapping_mul(0x14);
    let mut src = 0xB9B92Cusize;
    let mut e = o;
    loop {
        g::<u32>(e.wrapping_add(0xB9B958)).write(g::<u32>(e.wrapping_add(0xB9B954)).read());
        g::<u32>(e.wrapping_add(0xB9B930)).write(g::<u32>(e.wrapping_add(0xB9B92C)).read());
        g::<u32>(e.wrapping_add(0xB9B908)).write(g::<u32>(src).read());
        src -= 4;
        e = e.wrapping_sub(4);
        if src < 0xB9B920 {
            break;
        }
    }
    g::<u32>(o.wrapping_add(HIST_X)).write(rd::<u32>(this, 4));
    let yv = rd::<u32>(this, 8);
    let now = g::<u32>(0xB7CB84).read();
    g::<u32>(o.wrapping_add(HIST_Y)).write(yv);
    g::<u32>(o.wrapping_add(HIST_T)).write(now);
    let mut ent = o;
    for _ in 0..5 {
        let t = (g::<u32>(o.wrapping_add(HIST_T)).read() & 0x3FF) as i32;
        let pulse = f32_of(
            xi(t)
                .fmul_m(f32::from_bits(0x3BC9_0FDB))
                .fsin()
                .fmul_m(f32::from_bits(0x3E4C_CCCD))
                .fadd_m(1.0),
        );
        for m in 0..3 {
            let rad = f32_of(xi(m).fmul_m(10.0).fadd_m(20.0));
            for k in 0..4 {
                let a = xi(k).fmul_m(f32::from_bits(0x3FC9_0FDB)).fadd_m(f32::from_bits(0x3F49_0FDB));
                let h = xi(g::<i32>(0xC17048).read());
                let py = a
                    .fcos()
                    .fmul_m(rad)
                    .fmul_m(pulse)
                    .fadd(x(g::<f32>(ent.wrapping_add(HIST_Y)).read()).fadd_m(1.0).fmul(h).fmul_m(0.5));
                let py = f32_of(py);
                let w = xi(g::<i32>(0xC17044).read());
                let px = a
                    .fsin()
                    .fmul_m(rad)
                    .fmul_m(pulse)
                    .fadd(x(g::<f32>(ent.wrapping_add(HIST_X)).read()).fadd_m(1.0).fmul(w).fmul_m(0.5));
                let px = f32_of(px);
                render_one_xlu_sprite_rotate_aspect(px, py, 100.0, 15.0, 15.0, r, gg, b, 0xFF, f32::from_bits(0x3C23_D70A), 0, 0xFF);
            }
        }
        b >>= 1;
        r >>= 1;
        gg >>= 1;
        ent = ent.wrapping_add(4);
    }
    rw_render_state_set(8, 1);
    rw_render_state_set(6, 1);
}

pub const HOOKS: &[HookReg] = &[HOOK_CROSSHAIR_TRAIL_RENDER];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
