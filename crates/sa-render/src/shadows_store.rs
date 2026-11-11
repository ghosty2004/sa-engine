//! WRITER-RENDER-BIG-2: `CShadows::StoreShadowForVehicle` (0x70BDA0). Ported from the 1.0 US bytes
//! (gta-reversed has not reversed it). x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, x, xi};

orig! {
    /// `CStencilShadows::GraphicsHighQuality()` (0x70f9b0)
    fn graphics_high_quality() -> u8 = cdecl 0x70f9b0 => crate::stencilshadows::graphics_high_quality;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `FindPlayerVehicle(int, bool)` (0x56e0d0)
    fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56e0d0 => sa_world::world::cworld::find_player_vehicle;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59c730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector> = cdecl 0x59c730 => sa_world::collision::data::cross_product_port;
    /// `CShadows::StoreShadowToBeRendered(type, RwTexture*, CVector*, fx, fy, sx, sy, intensity, r, g, b,
    /// zDistance, drawOnWater, scale, CRealTimeShadow*, drawOnBuildings)` (0x707390; dwords as pushed)
    fn store_shadow_to_be_rendered_tex(ty: u32, tex: GPtr<u8>, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, water: u32, scale: u32, rts: GPtr<u8>, bld: u32) = cdecl 0x707390 => crate::mvpd_r1::store_shadow_to_be_rendered_tex;
    /// `CShadows::StoreStaticShadow(id, type, RwTexture*, CVector*, fx, fy, sx, sy, intensity, r, g, b,
    /// zDistance, scale, drawDistance, temporary, upDistance)` (0x70ba00; dwords as pushed)
    fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, intensity: u32, r: u32, gg: u32, b: u32, zd: u32, scale: u32, dd: u32, temp: u32, up: u32) -> u8 = cdecl 0x70ba00 => crate::mvpd_r1::store_static_shadow;
}

/// `gpShadowCarTex`, `gpShadowBikeTex`, `gpShadowHeliTex`, `gpShadowBaronTex` / plane shadow texture.
const TEX_CAR: usize = 0xc4_03e0;
const TEX_HELI: usize = 0xc4_03e8;
const TEX_BIKE: usize = 0xc4_03ec;
const TEX_PLANE: usize = 0xc4_03f0;
/// `CTimeCycle::m_CurrentColours.m_nShadowStrength` (the dword the original reads).
const SHADOW_STRENGTH: usize = 0xb7_c4e8;
/// `CCutsceneMgr::ms_running`, `TheCamera.m_fLODDistMultiplier`, `CTimer::ms_fTimeStep`.
const CUTSCENE_RUNNING: usize = 0xb5_f851;
const LOD_DIST_MULT: usize = 0xb6_f118;
const TIME_STEP: usize = 0xb7_cb5c;
/// `TheCamera.m_matrix` (pointer) and `TheCamera.m_placement` position.
const CAMERA_MATRIX: usize = 0xb6_f03c;
const CAMERA_PLACEMENT_POS: usize = 0xb6_f02c;

/// The per-model size tweak (the jump tables at 0x70C46C / 0x70C4B8): `(scale, length mult, width mult,
/// width doubled)`; `None` for the model that casts no shadow (0x21B), `Some` default `(1, 1, 1)`.
enum Tweak {
    None,
    Skip,
    /// scale, length multiplier (`None`: unchanged), width multiplier (`None`: unchanged), width doubled
    Set(Option<u32>, Option<u32>, Option<u32>, bool),
}

fn model_tweak(m: i32) -> Tweak {
    // 0.5 / x1.4 length / x3 width (0x70BFB2)
    let a = Tweak::Set(Some(0x3f00_0000), Some(0x3fb3_3333), Some(0x4040_0000), false);
    // 0.05 / x1.2 length (0x70BFD4)
    let b = Tweak::Set(Some(0x3d4c_cccd), Some(0x3f99_999a), None, false);
    // 0.03 / x1.5 length (0x70C01F)
    let c = Tweak::Set(Some(0x3cf5_c28f), Some(0x3fc0_0000), None, false);
    // 0.2 / x1.5 length / width doubled (0x70BF8D)
    let d = Tweak::Set(Some(0x3e4c_cccd), Some(0x3fc0_0000), None, true);
    match m {
        417 | 425 | 447 | 469 | 487 | 497 => a,
        448 | 461 | 462 | 481 | 509 | 510 | 521 | 522 => b,
        463 | 468 | 523 => c,
        465 | 501 => d,
        539 => Tweak::Skip,
        // x0.9 length, x0.4 width, scale kept (0x70C001)
        -2 => Tweak::Set(None, Some(0x3f66_6666), Some(0x3ecc_cccd), false),
        _ => Tweak::None,
    }
}

/// `CShadows::StoreShadowForVehicle(CVehicle*, VEH_SHADOW_TYPE)` (0x70BDA0). Nothing with high-quality
/// stencil shadows or a zero shadow strength (its low word). `d2 = dx² + dy²` to the camera (`/ (4 *
/// lodMult²)` in cutscenes); the maximum distance is 144 (types 2..4), 288 (type 5), 18 (others),
/// compared squared; beyond (or unordered) nothing. Past 3/4 of it the intensity fades to `ftol((1 -
/// (d - 0.75m) / (0.25m)) * strength)` (strength as i16).
///
/// Size: the col model's `maxY - minY` (length) and `maxX - minX` (width), then the per-model tweak
/// (`model_tweak`; model 0x21B returns); the position moves back by `(length * 0.5 - maxY) * scale`
/// along the forward vector (x, y). Per type: 0 car texture; 1 bike texture, width `* (|lean (+0x648)|
/// * 16/pi + 1)` plus `|right.z| * 4` when the status bits (+0x36 & 0xF8) are 0x20 and `|right.z| >
/// 0.6`; 2 heli texture, z distance 50 for the player's vehicle; 3 and 5 plane texture, the full
/// strength again (no fade), z distance 50 for the player's vehicle; 4 plane texture, length * 1.5,
/// width * 2.2; others keep the car texture.
///
/// Front = forward.xy; side = `forward x (0, 0, 1)`.xy, normalised to length 0.5 when shorter than 0.5;
/// negated when up.z < 0; type 1 with `|right.z| > 0.57`... (0.6 for type 1, 0.57 for type 2) takes
/// up.xy as side, type 2 also as front when `|forward.z| > 0.57`. Moving slowly (`|moveSpeed| *
/// timestep <= 0.1`, unordered too) and neither the player's vehicle nor flag +0x42C bit 7: a static
/// shadow `StoreStaticShadow(vehicle + 1, 1, tex, &pos, front * len/2, side * width/2 (negated when
/// up.z <= 0), strength x4, 4.5, 1.0, 0, 0, 0.1)`; otherwise `StoreShadowToBeRendered(1, tex, &pos,
/// .., strength x4, zDistance, flag, 1.0, null, flag)` where `flag` is the dword whose low byte is the
/// player / bit-7 test and whose upper bytes are the earlier scale float's (an original quirk kept).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x70bda0, conv = cdecl, name = "CShadows::StoreShadowForVehicle", expect = "83 EC 44 53 E8 07 3C 00 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk,driving",
       note = "FSQRT; the drawOnWater / drawOnBuildings dword carries the scale float's upper bytes")]
pub fn store_shadow_for_vehicle(veh: GPtr<u8>, ty: u32) {
    if graphics_high_quality() != 0 {
        return;
    }
    let mut tex = g::<u32>(TEX_CAR).read();
    let mut strength = g::<u32>(SHADOW_STRENGTH).read();
    if strength as u16 == 0 {
        return;
    }
    let mut zdist: u32 = 0x4090_0000;
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let vp = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
    let mut pos = [rd::<f32>(vp, 0), rd::<f32>(vp, 4), rd::<f32>(vp, 8)];
    let cm = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    let cp = if cm.is_null() { GPtr::<u8>::from_addr(CAMERA_PLACEMENT_POS) } else { cm.byte_add(0x30) };
    let dy = x(pos[1]).fsub_m(rd::<f32>(cp, 4));
    let dx = x(pos[0]).fsub_m(rd::<f32>(cp, 0));
    let mut d2 = dx.fmul(dx).fadd(dy.fmul(dy));
    let mut d2s = d2.to_f32();
    if g::<u8>(CUTSCENE_RUNNING).read() != 0 {
        let l = g::<f32>(LOD_DIST_MULT).read();
        // 4 (0x858B90)
        d2 = d2.fdiv(x(l).fmul_m(l).fmul_m(k(0x4080_0000)));
        d2s = d2.to_f32();
    }
    // 144² / 144 (0x87273C / 0x86C824), 288² / 288 (0x872738 / 0x872734), 18² / 18 (0x872730 / 0x859008)
    let (lim2, lim) = match ty.wrapping_sub(2) {
        0..=2 => (k(0x46a2_0000), k(0x4310_0000)),
        3 => (k(0x47a2_0000), k(0x4390_0000)),
        _ => (k(0x43a2_0000), k(0x4190_0000)),
    };
    if jp(d2, x(lim2), 0x5) {
        return;
    }
    let dist = x(d2s).fsqrt();
    let maxd = x(lim);
    // 0.75 (0x858F34), 0.25 (0x858C84)
    let fade = maxd.fmul_m(k(0x3f40_0000));
    if jp(dist, fade, 0x5) {
        let q = dist.fsub(fade).fdiv(maxd.fmul_m(k(0x3e80_0000)));
        strength = ftol32(q.fsubr_m(1.0).fmul(xi(i32::from(strength as u16 as i16)))) as u32;
    }
    let model = i32::from(rd::<u16>(veh, 0x22) as i16);
    let col = get_col_model(veh);
    let mut scale: u32 = 0x3f80_0000;
    let mut len = x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)).to_f32();
    let mut width = x(rd::<f32>(col, 0xc)).fsub_m(rd::<f32>(col, 0)).to_f32();
    match model_tweak(model) {
        Tweak::Skip => return,
        Tweak::None => {}
        Tweak::Set(s, lm, wm, dbl) => {
            if let Some(s) = s {
                scale = s;
            }
            if dbl {
                len = x(len).fmul_m(k(lm.unwrap_or(0x3f80_0000))).to_f32();
                width = x(width).fadd(x(width)).to_f32();
            } else {
                if let Some(wm) = wm {
                    // the -2 model scales length first, the others width first (no rounding difference)
                    width = x(width).fmul_m(k(wm)).to_f32();
                }
                if let Some(lm) = lm {
                    len = x(len).fmul_m(k(lm)).to_f32();
                }
            }
        }
    }
    let off = x(len).fmul_m(0.5).fsub_m(rd::<f32>(col, 0x10)).fmul_m(f32::from_bits(scale));
    let mat = rd::<GPtr<u8>>(veh, 0x14);
    pos[0] = off.fmul_m(rd::<f32>(mat, 0x10)).fsubr_m(pos[0]).to_f32();
    pos[1] = off.fmul_m(rd::<f32>(mat, 0x14)).fsubr_m(pos[1]).to_f32();
    match ty {
        1 => {
            // 16/pi (0x8722D8)
            let w1 = x(rd::<f32>(veh, 0x648)).fabs().fmul_m(k(0x40a2_f983)).fadd_m(1.0);
            let mut w = w1;
            if rd::<u8>(veh, 0x36) & 0xf8 == 0x20 {
                let rz = x(rd::<f32>(mat, 8)).fabs();
                // 0.6 (0x858CC8)
                if !jne(rz, x(k(0x3f19_999a)), 0x41) {
                    w = w1.fadd(rz.fmul_m(k(0x4080_0000)));
                }
            }
            tex = g::<u32>(TEX_BIKE).read();
            width = w.fmul_m(width).to_f32();
        }
        2 => {
            tex = g::<u32>(TEX_HELI).read();
            if veh == find_player_vehicle(-1, 0) {
                zdist = 0x4248_0000;
            }
        }
        3 | 5 => {
            tex = g::<u32>(TEX_PLANE).read();
            strength = g::<u32>(SHADOW_STRENGTH).read();
            if veh == find_player_vehicle(-1, 0) {
                zdist = 0x4248_0000;
            }
        }
        4 => {
            tex = g::<u32>(TEX_PLANE).read();
            // 1.5 (0x858CE8), 2.2 (0x87272C)
            len = x(len).fmul_m(k(0x3fc0_0000)).to_f32();
            width = x(width).fmul_m(k(0x400c_cccd)).to_f32();
        }
        0 => tex = g::<u32>(TEX_CAR).read(),
        _ => {}
    }
    let fwd = mat.byte_add(0x10);
    let mut front = [rd::<f32>(fwd, 0), rd::<f32>(fwd, 4)];
    let mut up = [0u32, 0, 0x3f80_0000];
    let mut out = [0u32; 3];
    cross_product(
        GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
        fwd.cast(),
        GPtr::<u32>::from_raw(up.as_mut_ptr()).cast(),
    );
    let (ox, oy) = (f32::from_bits(out[0]), f32::from_bits(out[1]));
    let mut side = [ox, oy];
    let mlen = x(oy).fmul_m(oy).fadd(x(ox).fmul_m(ox)).fsqrt();
    if !jp(mlen, x(0.5), 0x5) {
        let inv = mlen.fdivr_m(1.0);
        side[0] = x(ox).fmul(inv).fmul_m(0.5).to_f32();
        side[1] = inv.fmul_m(oy).fmul_m(0.5).to_f32();
    }
    let mat = rd::<GPtr<u8>>(veh, 0x14);
    if !jp(x(rd::<f32>(mat, 0x28)), x(0.0), 0x5) {
        side = [x(side[0]).fchs().to_f32(), x(side[1]).fchs().to_f32()];
    }
    let upxy = [rd::<f32>(mat, 0x20), rd::<f32>(mat, 0x24)];
    if ty == 1 {
        if !jne(x(rd::<f32>(mat, 8)).fabs(), x(k(0x3f19_999a)), 0x41) {
            side = upxy;
        }
    } else if ty == 2 {
        // 0.57 (0x872728)
        if !jne(x(rd::<f32>(mat, 8)).fabs(), x(k(0x3f11_eb85)), 0x41) {
            side = upxy;
        }
        if !jne(x(rd::<f32>(mat, 0x18)).fabs(), x(k(0x3f11_eb85)), 0x41) {
            front = upxy;
        }
    }
    let flag: u8 = if rd::<u8>(veh, 0x42c) & 0x80 != 0 { 1 } else { u8::from(veh == find_player_vehicle(-1, 0)) };
    let flag_dword = (scale & 0xffff_ff00) | u32::from(flag);
    let (vx, vy, vz) = (x(rd::<f32>(veh, 0x44)), x(rd::<f32>(veh, 0x48)), x(rd::<f32>(veh, 0x4c)));
    let speed = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt().fmul_m(g::<f32>(TIME_STEP).read());
    // 0.1 (0x858B1C)
    let fast = !jne(speed, x(k(0x3dcc_cccd)), 0x41);
    let mat = rd::<GPtr<u8>>(veh, 0x14);
    let up_pos = !jne(x(rd::<f32>(mat, 0x28)), x(0.0), 0x41);
    let w2 = x(width).fmul_m(0.5);
    let l2 = x(len).fmul_m(0.5);
    let neg = |v: X87| if up_pos { v } else { v.fchs() };
    let sy = neg(w2.fmul_m(side[1])).to_f32().to_bits();
    let sx = neg(w2.fmul_m(side[0])).to_f32().to_bits();
    let fy = x(front[1]).fmul(l2).to_f32().to_bits();
    let fx = l2.fmul_m(front[0]).to_f32().to_bits();
    let p = GPtr::<f32>::from_raw(pos.as_mut_ptr()).cast::<u8>();
    let s = strength;
    if !fast && flag == 0 {
        let id = veh.addr().wrapping_add(1) as u32;
        store_static_shadow(id, 1, tex, p, fx, fy, sx, sy, s, s, s, s, 0x4090_0000, 0x3f80_0000, 0, 0, 0x3dcc_cccd);
    } else {
        store_shadow_to_be_rendered_tex(1, GPtr::from_addr(tex as usize), p, fx, fy, sx, sy, s, s, s, s, zdist, flag_dword, 0x3f80_0000, GPtr::from_addr(0), flag_dword);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_STORE_SHADOW_FOR_VEHICLE];
