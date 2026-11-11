//! `CEntity::ProcessLightsForEntity` (batch `mvph-world-1`): the 2D-effect lights of an entity —
//! sun glare coronas, light coronas (14 flash types), point lights, fog lights and static shadows.
//! Ported from the 1.0 US bytes (no SecuROM fragment; the flash switch is the jump table 0x6FD4AC,
//! the fade mode switch the table 0x6FD4E4), x87 sequences through the [`Fpu`] model (PC = 24).
//! gta-reversed `Entity.cpp` was a reading aid only; where it differs the bytes win (noted on the
//! hook). Unverified (docs/verify/HOWTO.md).
//!
//! `C2dEffect` offsets: pos +0, type +0xC (0 light, 4 sun glare); light: colour r/g/b +0x10..0x12,
//! corona far clip +0x14, point-light range +0x18, corona size +0x1C, shadow size +0x20, flags u16
//! +0x24, flash type +0x26, reflection +0x27, flare type +0x28, shadow colour multiplier +0x29,
//! shadow z distance +0x2A, direction offsets int8 +0x2B..0x2D, corona texture +0x30, shadow
//! texture +0x34.
#![allow(clippy::unreadable_literal, clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::common::{cam_pos, ftol32, g, rd};
use crate::collision::data::normalise;
use crate::collision::fpu::Fpu;
use crate::physical::attach::{allocate_matrix, simple_transform_update_matrix};
use crate::physical::shared::{lp, multiply3x3, pos_ptr, rv};

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `randomSeedRandomiser` (0x8D5028, uint16[8])
const SEED_RANDOMISER: usize = 0x8D5028;
/// `CWeather::SunGlare` (0xC812E0, float)
const SUN_GLARE: usize = 0xC812E0;
/// `CWeather::WetRoads` (0xC81308, float)
const WET_ROADS: usize = 0xC81308;
/// `CWeather::Rain` (0xC81324, float)
const RAIN: usize = 0xC81324;
/// `CTimeCycle::m_CurrentStoredValue` (0xB79FD0, int32)
const CURRENT_STORED_VALUE: usize = 0xB79FD0;
/// `CTimeCycle::m_VectorToSun` (0xB7CA50, CVector[16])
const VECTOR_TO_SUN: usize = 0xB7CA50;
/// `CTimeCycle::m_CurrentColours.m_nSunCoreRed` / Green / Blue (0xB7C4D0 / D2 / D4, int16)
const SUN_CORE_RED: usize = 0xB7C4D0;
const SUN_CORE_GREEN: usize = 0xB7C4D2;
const SUN_CORE_BLUE: usize = 0xB7C4D4;
/// `CTimeCycle::m_CurrentColours.m_fSpriteBrightness` (0xB7C4E4, float)
const SPRITE_BRIGHTNESS: usize = 0xB7C4E4;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::m_FrameCounter` (0xB7CB4C)
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `gpCoronaTexture[0]` (0xC3E000, the shiny star)
const CORONA_TEX_STAR: usize = 0xC3E000;
/// `MODEL_RCBARON`
const MODEL_RCBARON: u16 = 0x1D0;

// rdata constants (bit patterns of the floats the bytes load)
const F_0_96: u32 = 0x3F75_C28F; // 0x85A5CC
const F_0: u32 = 0x0000_0000; // 0x858B50
const F_1: u32 = 0x3F80_0000; // 0x858624
const F_0_5: u32 = 0x3F00_0000; // 0x858B8C
const F_2: u32 = 0x4000_0000; // 0x858CA0
const F_THIRD: u32 = 0x3EAA_AAAB; // 0x859040
const F_0_012: u32 = 0x3C44_9BA6; // 0x859054
const F_1_9: u32 = 0x3FF3_3333; // 0x866C0C
const F_0_1: u32 = 0x3DCC_CCCD; // 0x858B1C
const F_255: u32 = 0x437F_0000; // 0x859AAC
const F_1_256: u32 = 0x3B80_0000; // 0x859AA0
const F_RAIN_MIN: u32 = 0x38D1_B717; // 0x858FC4 (1e-4)
const F_20: u32 = 0x41A0_0000; // 0x858BA4
const F_10: u32 = 0x4120_0000; // 0x85862C
const F_FADE_RATE: u32 = 0x3A6B_F405; // 0x872538 (0.0009)

fn cf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

orig! {
    /// `Light::GetDayNightBalance()` (0x6FAB30, ST0)
    fn get_day_night_balance() -> f64 = cdecl 0x6fab30 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4c4c70 => unported;
    /// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0)
    fn transform_ref(e: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8>
        = thiscall 0x5334f0 => super::entity::transform_from_object_space_ref;
    /// `CCoronas::RegisterCorona(uint32 id, CEntity*, r, g, b, a, const CVector& pos, float size,
    /// float farClip, RwTexture*, flare, reflection, LOS check, trail, float normalAngle, bool
    /// longDistance, float nearClip, uint8, float fadeSpeed, bool onlyFromBelow, bool
    /// updateHeight)` (0x6FC180, cdecl, every argument a dword)
    fn register_corona(id: u32, attach: GPtr<u8>, r: u32, gr: u32, b: u32, a: u32, pos: GPtr<u8>, size: f32,
        far_clip: f32, tex: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: f32, long_distance: u32,
        near_clip: f32, unk: u32, fade_speed: f32, from_below: u32, update_height: u32) = cdecl 0x6fc180 => unported;
    /// `CCoronas::UpdateCoronaCoors(uint32 id, const CVector& pos, float farClip, float angle)` (0x6FC4D0)
    fn update_corona_coors(id: u32, pos: GPtr<u8>, far_clip: f32, angle: f32) = cdecl 0x6fc4d0 => unported;
    /// `CPointLights::AddLight(uint8 type, CVector pos, CVector dir, float radius, float r, float g,
    /// float b, uint8 fogType, bool extraShadows, CEntity*)` (0x7000E0, cdecl, vectors by value)
    fn add_light(ty: u32, px: f32, py: f32, pz: f32, dx: f32, dy: f32, dz: f32, radius: f32, r: f32, gr: f32,
        b: f32, fog: u32, shadows: u32, attached: u32) = cdecl 0x7000e0 => unported;
    /// `CShadows::StoreStaticShadow(uint32 id, uint8 type, RwTexture*, const CVector* pos, float
    /// frontX, float frontY, float sideX, float sideY, int16 intensity, uint8 r, uint8 g, uint8 b,
    /// float zDistance, float scale, float drawDistance, bool temporary, float upDistance)`
    /// (0x70BA00, cdecl)
    fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: f32, fy: f32, sx: f32, sy: f32,
        intensity: u32, r: u32, gr: u32, b: u32, zd: f32, scale: f32, draw: f32, temporary: u32, up: f32)
        = cdecl 0x70ba00 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10, AL)
    fn la_riots_active_here() -> u8 = cdecl 0x441c10 => unported;
    /// `CBridge::ShouldLightsBeFlashing()` (0x41BC90, AL)
    fn bridge_should_lights_be_flashing() -> u8 = cdecl 0x41bc90 => unported;
}

/// `fld a; fcomp b`: the compare flags of `a` against `b`.
fn cmp(a: f32, b: f32) -> Fpu {
    let mut f = Fpu::new();
    f.fld(a);
    f.fcomp_m(b);
    f
}

/// `fild` of an int, `fmul` by the value on `f`'s st0 (left there), `fmul m`, `_ftol` (low dword).
fn scaled_byte(f: &mut Fpu, v: i32, m: f32) -> u32 {
    f.fild(v);
    f.fmul_st(1);
    f.fmul_m(m);
    ftol32(f.pop()) as u32
}

/// `CEntity::ProcessLightsForEntity()` (0x6FC7A0, thiscall).
///
/// Day/night balance `dnb` (`GetDayNightBalance`, stored f32). Nothing for entity flag 0x200
/// (render damaged) or without 0x80 (visible), a vehicle with physical flag 0x20000000 (scorched),
/// or a non-vehicle whose matrix up.z is below 0.96f. Per 2D effect `C` of the model info (count
/// +0xD): `seed = randomSeed (+0x20) ^ randomiser[C & 7]` (0x8D5028), `timeFade = 1`.
///
/// Sun glare (type 4, `SunGlare >= 0`): `p = TransformFromObjectSpace(pos)`, `d = Normalise(p -
/// entityPos)`, `c = camera - p` (x, y on the stack), `dist = sqrt((c.y² + c.z²) + c.x²)`, `s = 2/dist`,
/// `d = Normalise(d + c*s)`, `dot = -((d.y*sun.y + d.z*sun.z) + d.x*sun.x)` (`m_VectorToSun
/// [m_CurrentStoredValue]`); `dot > 0`: `glare = sqrt(dot)*SunGlare` (unrounded), `p += c*s`,
/// `RegisterCorona(this + C + 0x21, 0, ftol((core + 510)*glare*(1/3f)) per colour (red 0xB7C4D0,
/// green D2, blue D4), 255, p, sqrt(dist)*SunGlare*0.5, 120, star texture, 0, 0, 0, 0, 0, 0, 1.5, 0, 15,
/// 0, 0)`. The id is `this + C + 0x21` (gta-reversed: `randomSeed + C + 1`).
///
/// Light (type 0): `p` as above; `lightFade` 1 and "off version" from flags 0x20 (day) / 0x40
/// (night): both, day with `dnb < 1` (`1 - dnb`), night with `dnb > 0` (`dnb`). With "off" or flash
/// type 2 and `WetRoads > 0.5` the flash switch sets `apply` / `on` / `coors`: 0 apply; 1, 2:
/// `(seed ^ time) & 0x60` apply else on, then `((time >> 11) ^ seed) & 3` apply (gta-reversed shifts
/// by 12); 3/4/5: `(C << 7/8/9) + time` bit 0x200/0x400/0x800 apply else coors; 6: `seed & 0xFF >
/// 0x10` apply, else case 1 with `seed*8`; 8: an object with +0x140 bit 0x800: `time & 0x400`
/// apply, inverted for odd `C`; `C >= 4` clears apply; 9: `ShouldLightsBeFlashing` and `time & 0x1FF
/// < 0x3C`; 10: `Rain > 1e-4` sets `timeFade = Rain`, apply; 11..13: `o = ftol(ftol(pos.x*20 +
/// (time + (type-11)*3333))... + pos.y*10)` (`fiadd`, `_ftol` twice), `r = o % 10000`, `mode =
/// r*9/10000`, `fade = (r - 1111*mode)*0.0009`; mode 0 `timeFade = fade`, 1/2 one, 3 `1 - fade`,
/// else no light. In a LA riot zone a lit non-vehicle with `seed & 3 == 0` or (`== 1` and `((seed*8)
/// ^ time) & 0x60`) gets the black "on" corona instead. Flag 0x800 (check direction): the int8
/// offsets through `Multiply3x3(matrix, ..)` (matrix allocated first when missing) dotted with
/// `camera - p` (`(o.y*b + o.z*c) + a*o.x`); negative: no light, coors. A lit light: brightness
/// `timeFade` (flag 0x80: `(1 - (rand() & 31)*0.012)*timeFade`), flag 0x200 zero when `(seed +
/// frame) & 3`, flag 0x1000 halved at `(frame + seed) & 63 == 1` and zero above; the RC Baron (model
/// 0x1D0) times 1.9 with size multiplier 2; `timeFade = sprite brightness * b * 0.1`; corona `(this
/// + C, 0, ftol(r*timeFade).., ftol(lightFade*255), p, size*mult, farClip, tex, flare, reflection,
/// flag 1, 0, 0, flag 0x10, 0.8, 0, 15, flag 0x100, flag 0x400)`. Else "on": black corona with alpha
/// 255, size, 1.5; else coors: `UpdateCoronaCoors(this + C, p, farClip, 0)`.
///
/// Point lights (range != 0, lit): a coloured light `AddLight(0, p, 0, range, c*(lightFade*timeFade/
/// 256), fog (flags >> 1) & 3, 1, 0)`, else `AddLight(2, p, 0, range, 0, 0, 0, 0, 1, 0)` and then
/// the fog lights: flag 4 `AddLight(3, p, 0, 0, c/256, 2, 1, 0)`, or flag 2 with apply and range 0
/// `AddLight(4, .., 1, 1, 0)`. Shadows (size != 0): z distance `(float)+0x2A` or 15; lit:
/// `StoreStaticShadow(this + C, 2, tex, p, size, 0, 0, -size, 128, ftol(c*mult*timeFade/256).., z,
/// 1, 40, 0, 0)` (mult +0x29); "on": the same with intensity and colours 0.
#[hook(addr = 0x6fc7a0, conv = thiscall, name = "CEntity::ProcessLightsForEntity", this = "CEntity",
       expect = "81 EC 94 00 00 00", unverified = "mvph-world-1", route = "L2",
       hazards = "x87-transcendental", recs = "grove-walk,driving",
       note = "FSQRT in the sun glare; rand at return 0x6FCED9 (blinking lights); _ftol of fiadd sums (flash types 11-13)")]
pub fn process_lights_for_entity(this: GPtr<u8>) {
    let dnb = X87::from_f64_exact(get_day_night_balance()).to_f32(); // +0x34
    let ef = rd::<u32>(this, 0x1C);
    if ef & 0x200 != 0 || ef & 0x80 == 0 {
        return;
    }
    let ty = rd::<u8>(this, 0x36) & 7;
    if ty == 2 {
        if rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
            return;
        }
    } else {
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !m.is_null() && !cmp(rd::<f32>(m, 0x28), cf(F_0_96)).jp(5) {
            return;
        }
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(this, 0x22) as isize as usize).read(); // +0x64
    let mut c: i32 = 0; // +0x1C
    while c < i32::from(rd::<u8>(mi, 0xD)) {
        let eff = get_2d_effect(mi, c);
        let seed = u32::from(g::<u16>(SEED_RANDOMISER + 2 * (c & 7) as usize).read() ^ rd::<u16>(this, 0x20)); // +0x2C
        match rd::<u8>(eff, 0xC) {
            4 => {
                if !cmp(g::<f32>(SUN_GLARE).read(), cf(F_0)).jne(1) {
                    sun_glare(this, eff, c);
                }
            }
            0 => light(this, eff, c, seed, dnb),
            _ => {}
        }
        c += 1;
    }
}

/// The sun-glare corona of effect `c` (0x6FC872..0x6FCA88).
fn sun_glare(this: GPtr<u8>, eff: GPtr<u8>, c: i32) {
    let mut ep = CVector::new(0.0, 0.0, 0.0); // +0x20
    transform_ref(this, lp(&mut ep).cast(), eff);
    let pos = pos_ptr(this);
    let e = [ep.x, ep.y, ep.z];
    let mut f = Fpu::new();
    f.fld(e[2]);
    f.fsub_m(rd::<f32>(pos, 8));
    f.fld(e[1]);
    f.fsub_m(rd::<f32>(pos, 4));
    f.fld(e[0]);
    f.fsub_m(rd::<f32>(pos, 0));
    let mut d = CVector::new(0.0, 0.0, 0.0); // +0x3C
    d.x = f.fstp();
    d.y = f.fstp();
    d.z = f.fstp();
    normalise(lp(&mut d));
    let cam = cam_pos();
    f.fld(cam.read());
    f.fsub_m(e[0]);
    f.fld(cam.wrapping_index(1).read());
    f.fsub_m(e[1]);
    f.fld(cam.wrapping_index(2).read());
    f.fsub_m(e[2]);
    let cz = f.fstp(); // +0x50
    f.fld_st(0);
    f.fmul_st(1);
    f.fld(cz);
    f.fmul_m(cz);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    let dist = f.fstp(); // +0x10
    let mut h = Fpu::new();
    h.fld(cf(F_2));
    h.fdiv_m(dist);
    let s = h.fstp(); // +0x2C
    f.fxch(1);
    f.fmul_m(s);
    let sx = f.fstp(); // +0x48
    f.fmul_m(s);
    let sy = f.fstp(); // +0x4C
    h.fld(cz);
    h.fmul_m(s);
    let sz = h.fstp(); // +0x50
    h.fld(sx);
    h.fadd_m(d.x);
    d.x = h.fstp();
    h.fld(d.y);
    h.fadd_m(sy);
    d.y = h.fstp();
    h.fld(d.z);
    h.fadd_m(sz);
    d.z = h.fstp();
    normalise(lp(&mut d));
    let idx = g::<i32>(CURRENT_STORED_VALUE).read();
    let sun = g::<f32>(VECTOR_TO_SUN).wrapping_index(3 * idx);
    let mut f = Fpu::new();
    f.fld(d.y);
    f.fmul_m(sun.wrapping_index(1).read());
    f.fld(d.z);
    f.fmul_m(sun.wrapping_index(2).read());
    f.faddp(1);
    f.fld(d.x);
    f.fmul_m(sun.read());
    f.faddp(1);
    f.fchs();
    f.fcom_m(cf(F_0));
    if f.jne(0x41) {
        return;
    }
    f.fsqrt();
    let glare_k = g::<f32>(SUN_GLARE).read();
    f.fmul_m(glare_k);
    h.fld(sx);
    h.fadd_m(ep.x);
    ep.x = h.fstp();
    h.fld(ep.y);
    h.fadd_m(sy);
    ep.y = h.fstp();
    h.fld(ep.z);
    h.fadd_m(sz);
    ep.z = h.fstp();
    h.fld(dist);
    h.fsqrt();
    h.fmul_m(glare_k);
    h.fmul_m(cf(F_0_5));
    let radius = h.fstp();
    let b = scaled_byte(&mut f, i32::from(g::<u16>(SUN_CORE_BLUE).read()) + 0x1FE, cf(F_THIRD));
    let gr = scaled_byte(&mut f, i32::from(g::<u16>(SUN_CORE_GREEN).read()) + 0x1FE, cf(F_THIRD));
    let r = scaled_byte(&mut f, i32::from(g::<u16>(SUN_CORE_RED).read()) + 0x1FE, cf(F_THIRD));
    let id = (this.addr() as u32).wrapping_add(c as u32).wrapping_add(0x21);
    register_corona(id, GPtr::NULL, r, gr, b, 0xFF, lp(&mut ep).cast(), radius, 120.0,
        g::<u32>(CORONA_TEX_STAR).read(), 0, 0, 0, 0, 0.0, 0, 1.5, 0, 15.0, 0, 0);
}

/// The light effect `c` (0x6FCA8D..0x6FD484).
fn light(this: GPtr<u8>, eff: GPtr<u8>, c: i32, seed: u32, dnb: f32) {
    let mut ep = CVector::new(0.0, 0.0, 0.0); // +0x20
    transform_ref(this, lp(&mut ep).cast(), eff);
    let flags = u32::from(rd::<u16>(eff, 0x24));
    let mut apply = false; // bl
    let mut on = false; // +0x16
    let mut coors = false; // +0x17
    let mut off = false; // dl
    let mut light_fade: f32 = 1.0; // +0x38
    let mut time_fade: f32 = 1.0; // +0x18
    if flags & 0x20 != 0 && flags & 0x40 != 0 {
        off = true;
    } else if flags & 0x20 != 0 {
        if !cmp(dnb, cf(F_1)).jp(5) {
            let mut f = Fpu::new();
            f.fld(cf(F_1));
            f.fsub_m(dnb);
            light_fade = f.fstp();
            off = true;
        }
    } else if flags & 0x40 != 0 && !cmp(dnb, cf(F_0)).jne(0x41) {
        light_fade = dnb;
        off = true;
    }
    let flash = rd::<u8>(eff, 0x26);
    let wet = flash == 2 && !cmp(g::<f32>(WET_ROADS).read(), cf(F_0_5)).jne(0x41);
    let time = g::<u32>(TIME_MS).read();
    let cu = c as u32;
    if (wet || off) && flash <= 0xD {
        let flicker = |s: u32, apply: &mut bool, on: &mut bool| {
            if (s ^ time) & 0x60 != 0 {
                *apply = true;
            } else {
                *on = true;
            }
            if ((time >> 11) ^ seed) & 3 != 0 {
                *apply = true;
            }
        };
        let anim = |shift: u32, bit: u32, apply: &mut bool, coors: &mut bool| {
            if (cu << shift).wrapping_add(time) & bit != 0 {
                *apply = true;
            } else {
                *coors = true;
            }
        };
        match flash {
            0 => apply = true,
            1 | 2 => flicker(seed, &mut apply, &mut on),
            3 => anim(7, 0x200, &mut apply, &mut coors),
            4 => anim(8, 0x400, &mut apply, &mut coors),
            5 => anim(9, 0x800, &mut apply, &mut coors),
            6 => {
                if seed & 0xFF > 0x10 {
                    apply = true;
                } else {
                    flicker(seed.wrapping_mul(8), &mut apply, &mut on);
                }
            }
            8 => {
                if rd::<u8>(this, 0x36) & 7 == 4 && rd::<u32>(this, 0x140) & 0x800 != 0 {
                    if time & 0x400 != 0 {
                        apply = true;
                    }
                    if c & 1 != 0 {
                        apply = !apply;
                    }
                }
                if c >= 4 {
                    apply = false;
                }
            }
            9 => {
                if bridge_should_lights_be_flashing() != 0 && time & 0x1FF < 0x3C {
                    apply = true;
                }
            }
            10 => {
                let rain = g::<f32>(RAIN).read();
                if !cmp(rain, cf(F_RAIN_MIN)).jne(0x41) {
                    time_fade = rain;
                    apply = true;
                }
            }
            11..=13 => {
                let o = u32::from(flash - 11).wrapping_mul(0xD05).wrapping_add(time) as i32;
                let pos = pos_ptr(this);
                let mut f = Fpu::new();
                f.fld(rd::<f32>(pos, 0));
                f.fmul_m(cf(F_20));
                f.fild(o);
                f.faddp(1);
                let o = ftol32(f.pop());
                f.fld(rd::<f32>(pos, 4));
                f.fmul_m(cf(F_10));
                f.fild(o);
                f.faddp(1);
                let o = ftol32(f.pop());
                let r = o % 10000;
                apply = true;
                let mode = (r * 9) / 10000;
                f.fild(r - mode * 0x457);
                f.fmul_m(cf(F_FADE_RATE));
                let fade = f.fstp();
                match mode as u32 {
                    0 => time_fade = fade,
                    1 | 2 => time_fade = 1.0,
                    3 => {
                        f.fld(cf(F_1));
                        f.fsub_m(fade);
                        time_fade = f.fstp();
                    }
                    _ => apply = false,
                }
            }
            _ => {}
        }
    }
    let id = (this.addr() as u32).wrapping_add(cu);
    let far_clip = rd::<f32>(eff, 0x14);
    let tex = rd::<u32>(eff, 0x30);
    let flare = u32::from(rd::<u8>(eff, 0x28));
    let refl = u32::from(rd::<u8>(eff, 0x27));
    let mut riot_on = false;
    if la_riots_active_here() != 0 && apply && rd::<u8>(this, 0x36) & 7 != 2 {
        let s3 = seed & 3;
        if s3 == 0 || (s3 == 1 && (seed.wrapping_mul(8) ^ time) & 0x60 != 0) {
            apply = false;
            on = true;
            riot_on = true;
        }
    }
    let mut lit = false;
    if !riot_on && apply {
        let mut visible = true;
        if flags & 0x800 != 0 {
            let mut o = CVector::new(
                f32::from(rd::<i8>(eff, 0x2B)),
                f32::from(rd::<i8>(eff, 0x2C)),
                f32::from(rd::<i8>(eff, 0x2D)),
            ); // +0x8C
            if rd::<GPtr<u8>>(this, 0x14).is_null() {
                allocate_matrix(this);
                simple_transform_update_matrix(this.byte_add(4), rd::<GPtr<u8>>(this, 0x14));
            }
            let mut out = CVector::new(0.0, 0.0, 0.0); // +0x98
            let l = rv(multiply3x3(lp(&mut out), rd::<GPtr<u8>>(this, 0x14), lp(&mut o)).cast(), 0);
            let cam = cam_pos();
            let mut f = Fpu::new();
            f.fld(cam.read());
            f.fsub_m(ep.x);
            f.fld(cam.wrapping_index(1).read());
            f.fsub_m(ep.y);
            f.fld(cam.wrapping_index(2).read());
            f.fsub_m(ep.z);
            f.fld(l[1]);
            f.fmul_st(2);
            f.fld(l[2]);
            f.fmul_st(2);
            f.faddp(1);
            f.fxch(3);
            f.fmul_m(l[0]);
            f.faddp(3);
            f.fxch(2);
            f.fcomp_m(cf(F_0));
            visible = f.jp(5);
        }
        if visible {
            lit = true;
            let mut f = Fpu::new();
            if flags & 0x80 != 0 {
                let r = sa_core::orig::at_site(0x6FCED9, sa_core::crt::rand) & 0x1F;
                f.fild(r);
                f.fmul_m(cf(F_0_012));
                f.fsubr_m(cf(F_1));
                f.fmul_m(time_fade);
            } else {
                f.fld(time_fade);
            }
            let fc = g::<u32>(FRAME_COUNTER).read();
            if flags & 0x200 != 0 && seed.wrapping_add(fc) & 3 != 0 {
                f.fstp_st(0);
                f.fld(cf(F_0));
            }
            if flags & 0x1000 != 0 {
                match fc.wrapping_add(seed) & 0x3F {
                    0 => {}
                    1 => f.fmul_m(cf(F_0_5)),
                    _ => {
                        f.fstp_st(0);
                        f.fld(cf(F_0));
                    }
                }
            }
            let mut size_mult: f32 = 1.0; // +0x54
            if rd::<u16>(this, 0x22) == MODEL_RCBARON {
                f.fmul_m(cf(F_1_9));
                size_mult = 2.0;
            }
            f.fld(g::<f32>(SPRITE_BRIGHTNESS).read());
            f.fmul_st(1);
            f.fmul_m(cf(F_0_1));
            time_fade = f.fstp();
            f.fstp_st(0);
            let mut h = Fpu::new();
            h.fld(size_mult);
            h.fmul_m(rd::<f32>(eff, 0x1C));
            let size = h.fstp();
            h.fld(light_fade);
            h.fmul_m(cf(F_255));
            let a = ftol32(h.pop()) as u32;
            let col = |off: isize| {
                let mut k = Fpu::new();
                k.fild(i32::from(rd::<u8>(eff, off)));
                k.fmul_m(time_fade);
                ftol32(k.pop()) as u32
            };
            let b = col(0x12);
            let gr = col(0x11);
            let r = col(0x10);
            register_corona(id, GPtr::NULL, r, gr, b, a, lp(&mut ep).cast(), size, far_clip, tex, flare, refl,
                flags & 1, 0, 0.0, (flags >> 4) & 1, 0.8, 0, 15.0, (flags >> 8) & 1, (flags >> 10) & 1);
        } else {
            apply = false;
            coors = true;
        }
    }
    if !lit {
        if on {
            register_corona(id, GPtr::NULL, 0, 0, 0, 0xFF, lp(&mut ep).cast(), rd::<f32>(eff, 0x1C), far_clip, tex,
                flare, refl, flags & 1, 0, 0.0, (flags >> 4) & 1, 1.5, 0, 15.0, 0, 0);
        } else if coors {
            update_corona_coors(id, lp(&mut ep).cast(), far_clip, 0.0);
        }
    }

    // point lights
    let range = rd::<f32>(eff, 0x18);
    let range_zero = || cmp(range, cf(F_0)).ah() & 0x44 == 0x40;
    let (cr, cg, cb) = (rd::<u8>(eff, 0x10), rd::<u8>(eff, 0x11), rd::<u8>(eff, 0x12));
    let by256 = |v: u8| {
        let mut k = Fpu::new();
        k.fild(i32::from(v));
        k.fmul_m(cf(F_1_256));
        k.fstp()
    };
    let mut fog = true;
    if !range_zero() && apply {
        if cr != 0 || cg != 0 || cb != 0 {
            let mut f = Fpu::new();
            f.fld(light_fade);
            f.fmul_m(time_fade);
            f.fmul_m(cf(F_1_256));
            let mut ch = |v: u8| {
                f.fild(i32::from(v));
                f.fmul_st(1);
                f.fstp()
            };
            let b = ch(cb);
            let gg = ch(cg);
            let r = ch(cr);
            add_light(0, ep.x, ep.y, ep.z, 0.0, 0.0, 0.0, range, r, gg, b, (flags >> 1) & 3, 1, 0);
            fog = false;
        } else {
            add_light(2, ep.x, ep.y, ep.z, 0.0, 0.0, 0.0, range, 0.0, 0.0, 0.0, 0, 1, 0);
        }
    }
    if fog {
        if flags & 4 != 0 {
            add_light(3, ep.x, ep.y, ep.z, 0.0, 0.0, 0.0, 0.0, by256(cr), by256(cg), by256(cb), 2, 1, 0);
        } else if flags & 2 != 0 && apply && range_zero() {
            add_light(4, ep.x, ep.y, ep.z, 0.0, 0.0, 0.0, 0.0, by256(cr), by256(cg), by256(cb), 1, 1, 0);
        }
    }

    // static shadow
    let size = rd::<f32>(eff, 0x20);
    if cmp(size, cf(F_0)).ah() & 0x44 == 0x40 {
        return;
    }
    let zd_raw = rd::<u8>(eff, 0x2A);
    let zd: f32 = if zd_raw != 0 { f32::from(zd_raw) } else { 15.0 }; // +0x30
    let stex = rd::<u32>(eff, 0x34);
    if apply {
        let mut f = Fpu::new();
        f.fild(i32::from(rd::<u8>(eff, 0x29)));
        let mut ch = |v: u8| {
            f.fild(i32::from(v));
            f.fmul_st(1);
            f.fmul_m(time_fade);
            f.fmul_m(cf(F_1_256));
            ftol32(f.pop()) as u32
        };
        let b = ch(cb);
        let gg = ch(cg);
        let r = ch(cr);
        store_static_shadow(id, 2, stex, lp(&mut ep).cast(), size, 0.0, 0.0, -size, 0x80, r, gg, b, zd, 1.0, 40.0, 0,
            0.0);
    } else if on {
        store_static_shadow(id, 2, stex, lp(&mut ep).cast(), size, 0.0, 0.0, -size, 0, 0, 0, 0, zd, 1.0, 40.0, 0, 0.0);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_LIGHTS_FOR_ENTITY];
