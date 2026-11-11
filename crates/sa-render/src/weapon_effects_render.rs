//! WRITER-RENDER-BIG-2: `CWeaponEffects::Render` (0x742CF0), the crosshair pass. Ported from the 1.0 US
//! bytes (gta-reversed has not reversed it). x87 arithmetic follows the original's instruction order on
//! the `sa_math::x87::X87` model.
//!
//! `gCrossHair[2]` (0xC8A838, 0x2C bytes): +0 active, +4 clear time (ms; -1 never), +8 position, +0x14
//! r, g, b, a, +0x18 size, +0x1C rotation, +0x20 flash distance, +0x24 flash alpha, +0x28 kind (1: 3D
//! target sprite, else the rotating triangles). `gCrossHairFlashDist[2]` (0xC8A8A0, floats).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jne, jp, k, rd, x, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xc97b24 0x20 => unported;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70ce30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, a: u8, b: u8) -> u8 = cdecl 0x70ce30 => crate::sprite::calc_screen_coors;
    /// `CSprite::RenderOneXLUSprite_Rotate_Aspect(x, y, z, w, h, r, g, b, intensity, recipZ, angle, alpha)` (0x70d490)
    fn rotate_aspect(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: f32, alpha: u32) = cdecl 0x70d490 => crate::mvpd_r5::render_one_xlu_sprite_rotate_aspect;
    /// `CSprite::RenderOneXLUSprite_Triangle(x1, y1, x2, y2, x3, y3, z, r, g, b, intensity, recipZ, alpha)` (0x70d320)
    fn sprite_triangle(x1: u32, y1: u32, x2: u32, y2: u32, x3: u32, y3: u32, pz: f32, r: u8, gg: u8, b: u8, intensity: u32, recip_z: u32, alpha: u8) = cdecl 0x70d320 => crate::sprite_render::render_one_xlu_sprite_triangle;
    /// `std::min<float>` (0x404330, ST0)
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => self::min_f_port;
    /// `std::max<float>` (0x420800, ST0)
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => self::max_f_port;
    /// `CPlayerPed::GetWeaponRadiusOnScreen()` (0x609cd0, ST0)
    fn get_weapon_radius_on_screen(ped: GPtr<u8>) -> f64 = thiscall 0x609cd0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CPed::GetWeaponSkill()` (0x5e6580)
    fn get_weapon_skill(ped: GPtr<u8>) -> u32 = thiscall 0x5e6580 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743c60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743c60 => unported;
    /// `CWeapon::FindNearestTargetEntityWithScreenCoors(float x, float y, float range, CVector pos,
    /// float*, float*)` (0x73e240; dwords as pushed)
    fn find_nearest_target(sx: u32, sy: u32, range: u32, px: u32, py: u32, pz: u32, a: u32, b: u32) -> GPtr<u8> = cdecl 0x73e240 => unported;
    /// `FUN_0056ef90(int index)` on `CWorld::Players[i]` (0x56ef90)
    fn player_info_crosshair(this: GPtr<u8>, index: u32) = thiscall 0x56ef90 => unported;
}

pub fn min_f_port(a: f32, b: f32) -> f64 {
    sa_world::world::mvpa_math::min_f(a, b).to_f64()
}

pub fn max_f_port(a: f32, b: f32) -> f64 {
    sa_world::world::mvpa_math::max_f(a, b).to_f64()
}

const CROSSHAIRS: usize = 0xc8_a838;
const FLASH_DIST: usize = 0xc8_a8a0;
/// `CWorld::Players[i].m_pPed` (0x190 apart).
const PLAYERS: usize = 0xb7_cd98;
/// `CWeaponEffects` textures: target, shadow ring, triangle / sniper dot.
const TEX_TARGET: usize = 0xc8_a810;
const TEX_RING: usize = 0xc8_a814;
const TEX_SNIPER: usize = 0xc8_a818;
/// `CWeaponEffects::bFlash` (per player bytes), the arrow size, the radius factor.
const FLASH_ON: usize = 0x8d_6144;
const ARROW_SIZE: usize = 0x8d_6140;
const RADIUS_FACTOR: usize = 0x8d_6148;
/// The last aimed-at entity and when it changed.
const LAST_TARGET: usize = 0xc8_a894;
const LAST_TARGET_TIME: usize = 0xc8_a890;
const TIME_MS: usize = 0xb7_cb84;
const RS_WIDTH: usize = 0xc1_7044;
const RS_HEIGHT: usize = 0xc1_7048;
/// `TheCamera`: `m_bWideScreenOn`-like skip flag, `ActiveCam`, `Cams[]` (mode +0x180, crosshair x/y
/// +0x27C / +0x280 relative to 0xB6F028 + 0x238 * active), the view matrix rows used for the coop arrow.
const CAMERA_SKIP: usize = 0xb6_f065;
const ACTIVE_CAM: usize = 0xb6_f081;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn rs(s: u32, v: u32) {
    rw_render_state_set(s, v);
}

/// `min(v, cap)` as `fld cap; fcomp v; test ah, 5; jp` picks it: `v` when `cap >= v` or unordered.
fn cap_at(v: f32, cap: f32) -> f32 {
    if jp(x(cap), x(v), 0x5) { v } else { cap }
}

/// `(ms & 0x3FF) * 2pi/1024` (0x865034).
fn spin_angle() -> X87 {
    xi((g::<u32>(TIME_MS).read() & 0x3ff) as i32).fmul_m(k(0x3bc9_0fdb))
}

/// `CalcScreenCoors(pos, &out, &w, &h, 1, 1)`, then sizes scaled up so that `h >= 20` (`k = 20 / h`
/// applied when `k > 1`).
fn screen_size(pos: GPtr<u8>, out: &mut [f32; 3], w: &mut f32, h: &mut f32) -> bool {
    let mut inp = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    if calc_screen_coors(GPtr::from_raw(inp.as_mut_ptr()), GPtr::from_raw(out.as_mut_ptr()), GPtr::from_raw(w), GPtr::from_raw(h), 1, 1) == 0 {
        return false;
    }
    // 20 (0x858BA4)
    let kk = x(k(0x41a0_0000)).fdiv_m(*h);
    if !jne(kk, x(1.0), 0x41) {
        *w = x(*w).fmul(kk).to_f32();
        *h = kk.fmul_m(*h).to_f32();
    }
    true
}

/// `CWeaponEffects::Render()` (0x742CF0). Nothing with the camera's skip flag (0xB6F065). Per player
/// crosshair: an expired clear time deactivates it (flash distance 0), any clear time other than -1
/// zeroes the flash distance, no player ped deactivates it.
///
/// Kind 1 (3D target): on screen (size scaled to h >= 20): ZWRITE / ZTEST off, VERTEXALPHA, SRCALPHA /
/// INVSRCALPHA, the target texture; two black shadows at 0.95 and 1.05 of `(min(w, 28), min(h, 20)) *
/// 1.8`, then the coloured sprite, all at `recipZ 0.01`; the ring texture; the ring's rotation
/// `[+0x20] -= 20 * angle` (`angle = (ms & 0x3FF) * 2pi/1024`), floored at 0, `[+0x24] = [+0x20] == 0 ?
/// 1 : 0`; offset `(sin, cos) * [+0x20]` (0xC8A89C / 0xC8A898); two black rings (0.95 / 1.05 * 0.8) and
/// the coloured ring (intensity = the colour's alpha in the low word; the original sets only DX, the upper
/// word is whatever the previous call left in EDX and the callee reads the low word), rotated by the angle; ZWRITE / ZTEST on.
///
/// Other kinds: ZWRITE / ZTEST off, VERTEXALPHA, no texture; `CalcScreenCoors(pos, .., 0, 0)` (result
/// ignored). Off screen (x outside 0..W or y outside 0..H): in coop games an arrow at the screen edge
/// towards the target (two triangles). On screen: size `max(min(size, 1.2), 0.3)`, `r1 = 5 * size`,
/// `r2 = 25 * size`, z at least 2.5; with a positive weapon radius `r1 = radius * 30`, `r2 = size * 20 +
/// r1`; one-blend; three black triangles at 120 * k degrees (`+/- 15` for the outer corners) around the
/// rotation, each shrunk by 0.75 towards its centroid and drawn again in colour (cull mode 1). Rotation
/// and flash: with a pending clear time `rot += 0.75`, `flash += 2`, `size *= 0.9`, else `rot += 0.05`;
/// above 2pi it wraps to 0; with no clear time the flash distance pulses (`bFlash`).
///
/// After both players: the sniper dot in camera mode 0x31 (and the aimed-at entity's lock sprite, its
/// change time tracked in 0xC8A890 / 0xC8A894); `FUN_0056ef90(i)` for both `CWorld::Players`.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x742cf0, conv = cdecl, name = "CWeaponEffects::Render", expect = "A0 65 F0 B6 00 83 EC 58",
       unverified = "renderbig2-5", route = "L2", hazards = "x87-transcendental",
       recs = "weapon-combat",
       note = "FSIN/FCOS/FPATAN; the coloured ring passes the intensity with a stale EDX upper word (the port passes 0; the callee reads the low word)")]
pub fn weapon_effects_render() {
    if g::<u8>(CAMERA_SKIP).read() != 0 {
        return;
    }
    for i in 0..2usize {
        let ch = GPtr::<u8>::from_addr(CROSSHAIRS + 0x2c * i);
        let flash = GPtr::<f32>::from_addr(FLASH_DIST + 4 * i);
        let player = GPtr::<u8>::from_addr(PLAYERS + 0x190 * i);
        let t = rd::<u32>(ch, 4);
        if t != 0 && t < g::<u32>(TIME_MS).read() {
            crate::common::wr::<u8>(ch, 0, 0);
            crate::common::wr::<u32>(ch, 4, 0);
            flash.cast::<u32>().write(0);
        }
        if rd::<u32>(ch, 4) != 0xffff_ffff {
            flash.cast::<u32>().write(0);
        }
        if rd::<u32>(player, 0) == 0 {
            crate::common::wr::<u8>(ch, 0, 0);
        }
        if rd::<u8>(ch, 0) == 0 {
            continue;
        }
        if rd::<u8>(ch, 0x28) == 1 {
            target_sprite(ch);
        } else {
            triangles(ch, flash, player, i);
        }
    }
    let cam = 0x238 * usize::from(g::<u8>(ACTIVE_CAM).read());
    if g::<u16>(0xb6_f1a8 + cam).read() == 0x31 {
        sniper(cam);
    }
    for i in 0..2usize {
        player_info_crosshair(GPtr::from_addr(0xb7_cef0 + 0x190 * i), i as u32);
    }
}

/// Kind 1 (0x742D61..0x743144).
fn target_sprite(ch: GPtr<u8>) {
    let mut out = [0f32; 3];
    let (mut w, mut h) = (0f32, 0f32);
    if !screen_size(ch.byte_add(8), &mut out, &mut w, &mut h) {
        rs(8, 1);
        rs(6, 1);
        return;
    }
    for (s, v) in [(8, 0), (6, 0), (12, 1), (10, 5), (11, 6)] {
        rs(s, v);
    }
    rs(1, rd::<u32>(g::<GPtr<u8>>(TEX_TARGET).read(), 0));
    // 0.95, 1.05; 20 (0x858BA4), 28 (0x859AD8), 1.8 (0x859BB4), 0.01
    let scales = [k(0x3f73_3333), k(0x3f86_6666)];
    let (c20, c28, k18) = (k(0x41a0_0000), k(0x41e0_0000), k(0x3fe6_6666));
    for sc in scales {
        let hh = x(cap_at(h, c20)).fmul_m(sc).fmul_m(k18).to_f32();
        let ww = x(cap_at(w, c28)).fmul_m(sc).fmul_m(k18).to_f32();
        rotate_aspect(out[0], out[1], out[2], ww, hh, 0, 0, 0, 0xff, 0x3c23_d70a, 0.0, 0xff);
    }
    let hh = x(cap_at(h, c20)).fmul_m(k18).to_f32();
    let ww = x(cap_at(w, c28)).fmul_m(k18).to_f32();
    let c = |o: isize| u32::from(rd::<u8>(ch, o));
    rotate_aspect(out[0], out[1], out[2], ww, hh, c(0x14), c(0x15), c(0x16), 0xff, 0x3c23_d70a, 0.0, c(0x17));
    rs(1, rd::<u32>(g::<GPtr<u8>>(TEX_RING).read(), 0));
    let ang = spin_angle();
    let angf = ang.to_f32();
    let d = x(c20).fmul(ang).fsubr_m(rd::<f32>(ch, 0x20));
    crate::common::wr::<f32>(ch, 0x20, d.to_f32());
    if !jp(d, x(0.0), 0x5) {
        crate::common::wr::<u32>(ch, 0x20, 0);
    }
    let z = !jp(x(rd::<f32>(ch, 0x20)), x(0.0), 0x44);
    crate::common::wr::<u32>(ch, 0x24, if z { 0x3f80_0000 } else { 0 });
    let sv = ang.fsin().fmul_m(rd::<f32>(ch, 0x20)).to_f32();
    let cv = ang.fcos().fmul_m(rd::<f32>(ch, 0x20)).to_f32();
    g::<f32>(0xc8_a89c).write(sv);
    g::<f32>(0xc8_a898).write(cv);
    // 0.8 (0x858C98)
    let k08 = k(0x3f4c_cccd);
    for sc in scales {
        let hh = x(cap_at(h, c20)).fmul_m(sc).fmul_m(k08).to_f32();
        let ww = x(cap_at(w, c28)).fmul_m(sc).fmul_m(k08).to_f32();
        let py = x(out[1]).fsub_m(gf(0xc8_a898)).to_f32();
        let px = x(out[0]).fsub_m(gf(0xc8_a89c)).to_f32();
        rotate_aspect(px, py, out[2], ww, hh, 0, 0, 0, 0xff, 0x3c23_d70a, angf, 0xff);
    }
    let hh = x(cap_at(h, c20)).fmul_m(k08).to_f32();
    let ww = x(cap_at(w, c28)).fmul_m(k08).to_f32();
    let py = x(out[1]).fsub_m(gf(0xc8_a898)).to_f32();
    let px = x(out[0]).fsub_m(gf(0xc8_a89c)).to_f32();
    rotate_aspect(px, py, out[2], ww, hh, c(0x14), c(0x15), c(0x16), c(0x17), 0x3c23_d70a, angf, 0xff);
    rs(8, 1);
    rs(6, 1);
}

/// The rotating triangles and the coop arrow (0x743149..0x7438D8).
#[allow(clippy::cognitive_complexity)]
fn triangles(ch: GPtr<u8>, flash: GPtr<f32>, player: GPtr<u8>, i: usize) {
    for (s, v) in [(8, 0), (6, 0), (12, 1), (1, 0)] {
        rs(s, v);
    }
    let mut out = [0f32; 3];
    let (mut w, mut h) = (0f32, 0f32);
    let mut inp = [rd::<f32>(ch, 8), rd::<f32>(ch, 0xc), rd::<f32>(ch, 0x10)];
    calc_screen_coors(GPtr::from_raw(inp.as_mut_ptr()), GPtr::from_raw(out.as_mut_ptr()), GPtr::from_raw(&mut w), GPtr::from_raw(&mut h), 0, 0);
    let wv = xi(g::<i32>(RS_WIDTH).read());
    let hv = xi(g::<i32>(RS_HEIGHT).read());
    let lt = |a: X87, b: X87| !jp(a, b, 0x5);
    if lt(wv, x(out[0])) || lt(x(out[0]), x(0.0)) || lt(hv, x(out[1])) || lt(x(out[1]), x(0.0)) {
        if is_coop_game_going_on() != 0 {
            coop_arrow(ch, player);
        }
        rs(12, 0);
        rs(8, 1);
        rs(6, 1);
        return;
    }
    // 1.2, 0.3; 5 (0x858C80), 25 (0x858FE8), 2.5 (0x858FA0), 30 (0x8D6148)
    let v = X87::from_f64_exact(min_f(rd::<f32>(ch, 0x18), k(0x3f99_999a))).to_f32();
    let size = X87::from_f64_exact(max_f(v, k(0x3e99_999a)));
    crate::common::wr::<f32>(ch, 0x18, size.to_f32());
    let mut r1 = x(k(0x40a0_0000)).fmul(size).to_f32();
    let mut r2 = size.fmul_m(k(0x41c8_0000)).to_f32();
    if !jp(x(out[2]), x(k(0x4020_0000)), 0x5) {
        out[2] = k(0x4020_0000);
    }
    let rad = X87::from_f64_exact(get_weapon_radius_on_screen(GPtr::from_addr(rd::<u32>(player, 0) as usize)));
    if !jne(rad, x(0.0), 0x41) {
        r1 = rad.fmul_m(gf(RADIUS_FACTOR)).to_f32();
        r2 = x(rd::<f32>(ch, 0x18)).fmul_m(k(0x41a0_0000)).fadd_m(r1).to_f32();
    }
    rs(10, 1);
    rs(11, 1);
    // 120 (0x858BB0), pi/180 (0x8595EC), 15 (0x858B48), 1/3 (0x859040), 0.75 (0x858F34)
    let deg = k(0x3c8e_fa35);
    for j in 0..3 {
        let a = xi(j).fmul_m(k(0x42f0_0000));
        let rot = rd::<f32>(ch, 0x1c);
        let ang1 = x(deg).fmul(a).fadd_m(rot);
        let ra = x(r1).fadd_m(flash.read());
        let p0x = ang1.fsin().fmul(ra).fchs().to_f32();
        let p0y = ang1.fcos().fmul(ra).fchs();
        let ang2 = a.fadd_m(k(0x4170_0000)).fmul_m(deg).fadd_m(rot);
        let rb = x(r2).fadd_m(flash.read());
        let p1x = ang2.fsin().fmul(rb).fchs().to_f32();
        let p1y = ang2.fcos().fmul(rb).fchs().to_f32();
        let ang3 = a.fsub_m(k(0x4170_0000)).fmul_m(deg).fadd_m(rot);
        let p2x = ang3.fsin().fmul(rb).fchs().to_f32();
        let p2y = ang3.fcos().fmul(rb).fchs().to_f32();
        let mut p = [
            x(p0x).fadd_m(out[0]).to_f32(),
            p0y.fadd_m(out[1]).to_f32(),
            x(p1x).fadd_m(out[0]).to_f32(),
            x(p1y).fadd_m(out[1]).to_f32(),
            x(p2x).fadd_m(out[0]).to_f32(),
            x(p2y).fadd_m(out[1]).to_f32(),
        ];
        let a7 = rd::<u8>(ch, 0x17);
        sprite_triangle(p[0].to_bits(), p[1].to_bits(), p[2].to_bits(), p[3].to_bits(), p[4].to_bits(), p[5].to_bits(), out[2], 0, 0, 0, 0xff, 0x3f80_0000, a7);
        let third = k(0x3eaa_aaab);
        let cx = x(p[4]).fadd_m(p[2]).fadd_m(p[0]).fmul_m(third);
        let cy = x(p[5]).fadd_m(p[3]).fadd_m(p[1]).fmul_m(third);
        for (n, c) in [(0, cx), (1, cy), (2, cx), (3, cy), (4, cx), (5, cy)] {
            p[n] = x(p[n]).fsub(c).fmul_m(k(0x3f40_0000)).fadd(c).to_f32();
        }
        rs(10, 5);
        rs(11, 6);
        rs(20, 1);
        let c = |o: isize| rd::<u8>(ch, o);
        sprite_triangle(p[0].to_bits(), p[1].to_bits(), p[2].to_bits(), p[3].to_bits(), p[4].to_bits(), p[5].to_bits(), out[2], c(0x14), c(0x15), c(0x16), 0xff, 0x3f80_0000, c(0x17));
    }
    let t = rd::<u32>(ch, 4);
    if t != 0 && t != 0xffff_ffff {
        // 0.75, 2 (0x858CA0), 0.9 (0x858C20)
        crate::common::wr::<f32>(ch, 0x1c, x(rd::<f32>(ch, 0x1c)).fadd_m(k(0x3f40_0000)).to_f32());
        flash.write(x(flash.read()).fadd_m(k(0x4000_0000)).to_f32());
        crate::common::wr::<f32>(ch, 0x18, x(rd::<f32>(ch, 0x18)).fmul_m(k(0x3f66_6666)).to_f32());
    } else {
        // 0.05 (0x858C28)
        crate::common::wr::<f32>(ch, 0x1c, x(rd::<f32>(ch, 0x1c)).fadd_m(k(0x3d4c_cccd)).to_f32());
    }
    // 2pi (0x858CBC)
    if !jne(x(rd::<f32>(ch, 0x1c)), x(k(0x40c9_0fdb)), 0x41) {
        crate::common::wr::<u32>(ch, 0x1c, 0);
    }
    if t == 0 {
        let fl = GPtr::<u8>::from_addr(FLASH_ON + i);
        if fl.read() != 0 {
            let s = rd::<f32>(ch, 0x18);
            let nv = x(s).fadd(x(s)).fadd_m(flash.read());
            flash.write(nv.to_f32());
            if !jne(nv, x(s).fmul_m(k(0x41a0_0000)), 0x41) {
                fl.write(0);
            }
        } else {
            let nv = x(flash.read()).fsub_m(k(0x4000_0000));
            flash.write(nv.to_f32());
            if !jp(nv, x(0.0), 0x5) {
                fl.write(1);
            }
        }
    }
    rs(12, 0);
    rs(8, 1);
    rs(6, 1);
}

/// The coop arrow (0x7435CE..0x7438B0): the angles of the target and of the player ped in the camera's
/// view plane (FPATAN of `row1 . d` over `row0 . d`), the arrow at `((1 - cos a) W / 2, (1 - sin a) H /
/// 2)` pointing along the second angle.
fn coop_arrow(ch: GPtr<u8>, player: GPtr<u8>) {
    let ped = GPtr::<u8>::from_addr(rd::<u32>(player, 0) as usize);
    let view = |dx: X87, dy: X87, dz: X87| {
        let a = x(gf(0xb6_f9bc)).fmul(dx).fadd(x(gf(0xb6_f9c4)).fmul(dz)).fadd(x(gf(0xb6_f9c0)).fmul(dy));
        let b = dx.fmul_m(gf(0xb6_f99c)).fadd(x(gf(0xb6_f9a4)).fmul(dz)).fadd(x(gf(0xb6_f9a0)).fmul(dy));
        a.fpatan(b)
    };
    let (px, py, pz) = (rd::<f32>(ch, 8), rd::<f32>(ch, 0xc), rd::<f32>(ch, 0x10));
    let t1 = view(x(px).fsub_m(gf(0xb6_f9cc)), x(py).fsub_m(gf(0xb6_f9d0)), x(pz).fsub_m(gf(0xb6_f9d4)));
    let s1 = t1.fsin().to_f32();
    let c1 = t1.fcos();
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let t2 = view(x(px).fsub_m(rd::<f32>(pp, 0)), x(py).fsub_m(rd::<f32>(pp, 4)), x(pz).fsub_m(rd::<f32>(pp, 8)));
    let s2 = t2.fsin().to_f32();
    let c2 = t2.fcos().to_f32();
    let half = 0.5;
    let ax = x(1.0).fsub(c1).fmul(xi(g::<i32>(RS_WIDTH).read())).fmul_m(half).to_f32();
    let ay = x(1.0).fsub_m(s1).fmul(xi(g::<i32>(RS_HEIGHT).read())).fmul_m(half).to_f32();
    let size = gf(ARROW_SIZE);
    let t_a = x(s2).fmul_m(half).fadd_m(c2);
    let p2x = t_a.fmul_m(size).fadd_m(ax).to_f32();
    let ka = x(size).fmul_m(s2);
    let kb = x(c2).fmul_m(size);
    let p2y = ka.fadd_m(ay).fsub(kb.fmul_m(half)).to_f32();
    let p3x = kb.fadd_m(ax).fsub(ka.fmul_m(half)).to_f32();
    let t_b = x(c2).fmul_m(half).fadd_m(s2);
    let p3y = t_b.fmul_m(size).fadd_m(ay).to_f32();
    // 2.5
    let z = k(0x4020_0000);
    sprite_triangle(ax.to_bits(), ay.to_bits(), p2x.to_bits(), p2y.to_bits(), p3x.to_bits(), p3y.to_bits(), z, 0, 0, 0, 0xff, 0x3f80_0000, 0xff);
    // 0.1 (0x86EEE0, double), 0.1 (0x858B1C), 0.8 (0x858C98)
    let ax2 = x(c2).fmul_m(size).fmul(X87::from_f64_exact(f64::from_bits(0x3fb9_9999_9999_999a))).fadd_m(ax).to_f32();
    let ks = x(size).fmul_m(s2);
    let ay2 = x(k(0x3dcc_cccd)).fmul(ks).fadd_m(ay).to_f32();
    let k08 = k(0x3f4c_cccd);
    let q2x = x(t_a.to_f32()).fmul_m(size).fmul_m(k08).fadd_m(ax2).to_f32();
    let kc = x(c2).fmul_m(size);
    let q2y = ks.fsub(x(0.5).fmul(kc)).fmul_m(k08).fadd_m(ay2).to_f32();
    let q3x = kc.fsub(ks.fmul_m(0.5)).fmul_m(k08).fadd_m(ax2).to_f32();
    let q3y = x(t_b.to_f32()).fmul_m(size).fmul_m(k08).fadd_m(ay2).to_f32();
    let c = |o: isize| rd::<u8>(ch, o);
    sprite_triangle(ax2.to_bits(), ay2.to_bits(), q2x.to_bits(), q2y.to_bits(), q3x.to_bits(), q3y.to_bits(), z, c(0x14), c(0x15), c(0x16), 0xff, 0x3f80_0000, 0xff);
}

/// The sniper dot and the lock-on sprite (0x74391F..0x743BF5).
fn sniper(cam: usize) {
    for (s, v) in [(8, 0), (6, 0), (12, 1), (10, 5), (11, 6)] {
        rs(s, v);
    }
    rs(1, rd::<u32>(g::<GPtr<u8>>(TEX_SNIPER).read(), 0));
    let ang = spin_angle().to_f32();
    let cx = gf(0xb6_f2a4 + cam);
    let cy = gf(0xb6_f2a8 + cam);
    let sy = x(cy).fadd_m(1.0).fmul(xi(g::<i32>(RS_HEIGHT).read())).fmul_m(0.5).to_f32();
    let sx = x(cx).fadd_m(1.0).fmul(xi(g::<i32>(RS_WIDTH).read())).fmul_m(0.5).to_f32();
    // 100, 10, 10
    rotate_aspect(sx, sy, k(0x42c8_0000), k(0x4120_0000), k(0x4120_0000), 0xff, 0x80, 0, 0x80, 0x3c23_d70a, ang, 0xff);
    let p0 = g::<GPtr<u8>>(PLAYERS).read();
    let veh = rd::<GPtr<u8>>(p0, 0x58c);
    let mut ped = g::<GPtr<u8>>(PLAYERS + 0x190).read();
    if !veh.is_null() && rd::<u32>(veh, 0x460) != p0.addr() as u32 {
        ped = p0;
    }
    if !ped.is_null() {
        let slot = i32::from(rd::<u8>(ped, 0x718) as i8);
        let ty = rd::<u32>(ped, (0x5a0 + 0x1c * slot) as isize);
        let skill = get_weapon_skill(ped);
        let info = get_weapon_info(ty, skill);
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pp = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        let range = x(rd::<f32>(info, 8));
        let range = range.fadd(range).to_f32();
        let cam = 0x238 * usize::from(g::<u8>(ACTIVE_CAM).read());
        let e = find_nearest_target(
            g::<u32>(0xb6_f2a4 + cam).read(),
            g::<u32>(0xb6_f2a8 + cam).read(),
            range.to_bits(),
            rd::<u32>(pp, 0),
            rd::<u32>(pp, 4),
            rd::<u32>(pp, 8),
            0,
            0,
        );
        if e.addr() as u32 != g::<u32>(LAST_TARGET).read() {
            g::<u32>(LAST_TARGET).write(e.addr() as u32);
            g::<u32>(LAST_TARGET_TIME).write(g::<u32>(TIME_MS).read());
        }
        if !e.is_null() {
            let em = rd::<GPtr<u8>>(e, 0x14);
            let ep = if em.is_null() { e.byte_add(4) } else { em.byte_add(0x30) };
            let mut out = [0f32; 3];
            let (mut w, mut h) = (0f32, 0f32);
            if screen_size(ep, &mut out, &mut w, &mut h) {
                let ms = g::<u32>(TIME_MS).read();
                let dt = ms.wrapping_sub(g::<u32>(LAST_TARGET_TIME).read()) as i32;
                // 1/512 (0x863A44), 3 (0x858B3C)
                let s = xi(dt).fmul_m(k(0x3b00_0000)).fsubr_m(k(0x4040_0000));
                let s = if jne(x(1.0), s, 0x41) { s } else { x(1.0) };
                let a = (dt / 4).wrapping_add(0x46);
                let a = if a > 0xff { 0xff } else { a } as u32;
                let ang = spin_angle().to_f32();
                let hh = x(h).fmul(s).to_f32();
                let ww = s.fmul_m(w).to_f32();
                rotate_aspect(out[0], out[1], out[2], ww, hh, a, 0, 0, a, 0x3c23_d70a, ang, 0xff);
            }
        }
    }
    rs(8, 1);
    rs(6, 1);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WEAPON_EFFECTS_RENDER];
