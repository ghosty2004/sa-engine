//! `CCheckpoint::Render` (0x725C00, batch `samp2-render-1`): SA:MP's checkpoints and race checkpoints.
//! Ported from the 1.0 US bytes (gta-reversed's version differs: the finish-flag render states, the torus
//! distance test's equality case). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, k, rd, wr, x, xi};
use crate::markers::place_marker_call;

orig! {
    /// `C3dMarkers::DirectionArrowSet(CVector pos, float size, int r, int g, int b, int a, float dx, float
    /// dy, float dz)` (0x721140)
    #[allow(clippy::too_many_arguments)]
    fn direction_arrow_set(px: f32, py: f32, pz: f32, size: f32, r: i32, gg: i32, b: i32, a: i32, dx: f32, dy: f32, dz: f32) = cdecl 0x721140 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, max_vis: u8, min_vis: u8) -> u8 = cdecl 0x70CE30 => crate::sprite::calc_screen_coors;
    /// `CSprite::RenderOneXLUSprite(x, y, z, halfW, halfH, r, g, b, intensity, rhw, a, uDir, vDir)` (0x70D000)
    #[allow(clippy::too_many_arguments)]
    fn render_one_xlu_sprite(px: f32, py: f32, pz: f32, hw: f32, hh: f32, r: u32, gg: u32, b: u32, intensity: u32, rhw: f32, a: u32, u_dir: u32, v_dir: u32) = cdecl 0x70D000 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(x, y, z, float* level, float*, float*)` (0x6E8580), AL
    fn get_water_level_no_waves(px: f32, py: f32, pz: f32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 = cdecl 0x6E8580 => unported;
    /// `FindPlayerCoors(CVector* out, int32 player)` (0x56E010)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => sa_world::world::cworld::find_player_coors_port;
    /// `RwRenderStateSet` (`RwEngineInstance` 0xC97B24, slot 0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
}

/// `gpFinishFlagTex` (0xC7C718).
const FINISH_FLAG_TEX: usize = 0xC7_C718;

/// The marker every non-tube kind places: `PlaceMarker(id, 4 (torus), &pos, size, r, g, b, a, period,
/// fraction, rotateRate (+0xE), fwd, false)`.
fn place_torus(this: GPtr<u8>, pos: u32, alpha: u32) {
    place_marker_call(rd::<u32>(this, 4), 4, pos, rd::<u32>(this, 0x2C), u32::from(rd::<u8>(this, 8)),
        u32::from(rd::<u8>(this, 9)), u32::from(rd::<u8>(this, 0xA)), alpha, u32::from(rd::<u16>(this, 0xC)),
        rd::<u32>(this, 0x28), u32::from(rd::<u16>(this, 0xE)), rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20),
        rd::<u32>(this, 0x24), 0);
}

/// `CCheckpoint::Render()` (0x725C00, thiscall). Layout: type u16 +0, id +4, colour r/g/b/a +8..+0xB,
/// pulse period u16 +0xC, rotate rate u16 +0xE, position +0x10, forward +0x1C, pulse fraction +0x28, size
/// +0x2C, multi size +0x34.
///
/// * Tubes (types 0, 1, 2): `GetWaterLevelNoWaves(pos, &level, 0, 0)`; when it succeeds and `!(level <
///   pos.z)` the position's z becomes the level and the z-check flag 0 (else 1). Type 1 (finish): render
///   states raster = the finish flag texture's raster, z-write **off**, z-test on, vertex alpha on, address
///   wrap; `CalcScreenCoors(pos + (0, 0, 3), out, &w, &h, false, true)`, on success `RenderOneXLUSprite(out,
///   w * size * 0.5, h * size * 0.5, r, g, b, 255, 1 / out.z, a, 0, 0)`; states raster 0, z-write on,
///   z-test on, vertex alpha **off**. Then `PlaceMarker(id, 2, &pos, size * 1.5, r, g, b, 128, period,
///   fraction, 1, (0, 0, 0), flag)`; type 0 adds `DirectionArrowSet(pos, size * 0.625, 255, 64, 64, 255,
///   -fwd)`.
/// * Type 3: nothing while `2 * size > |player - pos|` (`(dz² + dy²) + dx²`, sqrt; NaN: nothing), else as 4.
/// * Type 6: three extra tori at `pos + fwd * i * multiSize` (i = 1..3; x: `(i*fwd.x)*m + pos.x` at 80
///   bits, y: `(f32)(i*fwd.y)*m + pos.y`, z: `(f32)((f32)(i*fwd.z)*m) + pos.z`) with alpha `a - 16*i`
///   (byte wrap), then as 4.
/// * Types 4, 5, 7, 8: `PlaceMarker(id, 4, &pos, size, r, g, b, a, period, fraction, rotateRate, fwd, false)`.
///
/// The tube flag is a byte local whose upper bytes the original pushes uninitialised (`PlaceMarker` reads
/// the byte); the port passes 0 there.
#[hook(addr = 0x725C00, conv = thiscall, name = "CCheckpoint::Render", this = "CCheckpoint",
       expect = "83 EC 54 53 55 56 8B F1",
       unverified = "samp2-render-1", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk",
       note = "SA:MP checkpoints / race checkpoints; writes pos.z for tubes under water")]
pub fn checkpoint_render(this: GPtr<u8>) {
    let ty = rd::<u16>(this, 0);
    let pos = this.byte_add(0x10);
    let pos_addr = pos.addr() as u32;
    match ty {
        0..=2 => {
            let mut level = 0f32;
            let mut flag = 1u32;
            let ok = get_water_level_no_waves(rd(pos, 0), rd(pos, 4), rd(pos, 8), GPtr::from_raw(&mut level),
                GPtr::NULL, GPtr::NULL);
            if ok != 0 && !x(level).fcom(x(rd::<f32>(pos, 8))).test_any(1) {
                wr::<f32>(pos, 8, level);
                flag = 0;
            }
            let (r, gg, b) = (u32::from(rd::<u8>(this, 8)), u32::from(rd::<u8>(this, 9)), u32::from(rd::<u8>(this, 0xA)));
            let mut above = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), x(rd::<f32>(pos, 8)).fadd_m(k(0x4040_0000)).to_f32()];
            let mut arrow = false;
            if ty == 1 {
                let tex = g::<GPtr<u8>>(FINISH_FLAG_TEX).read();
                rw_render_state_set(1, rd::<u32>(tex, 0));
                rw_render_state_set(8, 0);
                rw_render_state_set(6, 1);
                rw_render_state_set(0xC, 1);
                rw_render_state_set(2, 1);
                let mut out = [0f32; 3];
                let (mut w, mut h) = (0f32, 0f32);
                if calc_screen_coors(GPtr::from_raw(above.as_mut_ptr()), GPtr::from_raw(out.as_mut_ptr()),
                    GPtr::from_raw(&mut w), GPtr::from_raw(&mut h), 0, 1) != 0
                {
                    let rhw = x(1.0).fdiv(x(out[2])).to_f32();
                    let size = rd::<f32>(this, 0x2C);
                    let hh = x(h).fmul_m(size).fmul_m(0.5).to_f32();
                    let hw = x(w).fmul_m(size).fmul_m(0.5).to_f32();
                    render_one_xlu_sprite(out[0], out[1], out[2], hw, hh, u32::from(rd::<u8>(this, 8)),
                        u32::from(rd::<u8>(this, 9)), u32::from(rd::<u8>(this, 0xA)), 0xFF, rhw,
                        u32::from(rd::<u8>(this, 0xB)), 0, 0);
                }
                rw_render_state_set(1, 0);
                rw_render_state_set(8, 1);
                rw_render_state_set(6, 1);
                rw_render_state_set(0xC, 0);
            } else {
                arrow = true;
            }
            let size = x(rd::<f32>(this, 0x2C)).fmul_m(k(0x3FC0_0000)).to_f32();
            place_marker_call(rd::<u32>(this, 4), 2, pos_addr, size.to_bits(), r, gg, b, 0x80,
                u32::from(rd::<u16>(this, 0xC)), rd::<u32>(this, 0x28), 1, 0, 0, 0, flag);
            if arrow && rd::<u16>(this, 0) == 0 {
                let fx = x(rd::<f32>(this, 0x1C)).fchs().to_f32();
                let fy = x(rd::<f32>(this, 0x20)).fchs().to_f32();
                let fz = x(rd::<f32>(this, 0x24)).fchs().to_f32();
                let s = x(rd::<f32>(this, 0x2C)).fmul_m(k(0x3F20_0000)).to_f32();
                direction_arrow_set(rd(pos, 0), rd(pos, 4), rd(pos, 8), s, 0xFF, 0x40, 0x40, 0xFF, fx, fy, fz);
            }
        }
        3 => {
            let mut buf = [0f32; 3];
            let p = find_player_coors(GPtr::from_raw(buf.as_mut_ptr()), -1);
            let dx = x(p.wrapping_index(0).read()).fsub_m(rd(pos, 0));
            let dy = x(p.wrapping_index(1).read()).fsub_m(rd(pos, 4));
            let dz = x(p.wrapping_index(2).read()).fsub_m(rd(pos, 8));
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            let s2 = x(rd::<f32>(this, 0x2C)).fadd(x(rd::<f32>(this, 0x2C)));
            if s2.fcom(d).test_parity(0x41) {
                return;
            }
            place_torus(this, pos_addr, u32::from(rd::<u8>(this, 0xB)));
        }
        6 => {
            for i in 1..4i32 {
                let ms = rd::<f32>(this, 0x34);
                let fi = xi(i);
                let ifx = fi.fmul_m(rd(this, 0x1C));
                let ify = fi.fmul_m(rd(this, 0x20)).to_f32();
                let ifz = fi.fmul_m(rd(this, 0x24)).to_f32();
                let zo = x(ifz).fmul_m(ms).to_f32();
                let mut p = [
                    ifx.fmul_m(ms).fadd_m(rd(pos, 0)).to_f32(),
                    x(ify).fmul_m(ms).fadd_m(rd(pos, 4)).to_f32(),
                    x(zo).fadd_m(rd(pos, 8)).to_f32(),
                ];
                let a = u32::from(rd::<u8>(this, 0xB).wrapping_sub((i as u8) << 4));
                place_torus(this, GPtr::<f32>::from_raw(p.as_mut_ptr()).addr() as u32, a);
            }
            place_torus(this, pos_addr, u32::from(rd::<u8>(this, 0xB)));
        }
        4 | 5 | 7 | 8 => place_torus(this, pos_addr, u32::from(rd::<u8>(this, 0xB))),
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CHECKPOINT_RENDER];
