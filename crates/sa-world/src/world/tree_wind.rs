//! `CEntity::ModifyMatrixForTreeInWind` (batch `mvpk-world-2`): the wind sway of trees (the RW frame's "at" x/y).
//! Ported from the 1.0 US bytes (gta-reversed `Entity.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

orig! {
    /// `CWindModifiers::FindWindModifier(CVector pos, float* x, float* y)` (0x72C950; pos by value)
    fn wind_modifiers_find(px: u32, py: u32, pz: u32, ox: GPtr<f32>, oy: GPtr<f32>) -> u8 = cdecl 0x72C950 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
}

/// `CTimer::m_UserPause` (0xB7CB49), `m_CodePause` (0xB7CB48), `m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_USER_PAUSE: usize = 0xB7CB49;
const TIMER_CODE_PAUSE: usize = 0xB7CB48;
const TIMER_TIME_MS: usize = 0xB7CB84;
/// `CWeather::Wind` (0xC812F0), `CWeather::WindDir` x/y (0xC813E0, 0xC813E4)
const WEATHER_WIND: usize = 0xC812F0;
const WEATHER_WIND_DIR_X: usize = 0xC813E0;
const WEATHER_WIND_DIR_Y: usize = 0xC813E4;
/// The 16 tree sway steps (0x8CCF30, f32)
const TREE_SWAY_TABLE: usize = 0x8CCF30;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFOS: usize = 0xA9B0C8;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `fild` of a dword, + 2^32 when negative (the unsigned conversion the original emits).
fn fild_unsigned(v: u32) -> X87 {
    let f = X87::from_i32(v as i32);
    if (v as i32) < 0 { f.fadd_m(k(0x858C54)) } else { f }
}

/// `CEntity::ModifyMatrixForTreeInWind()` (0x534E90, thiscall). x87 (FSIN). Nothing while paused (user or code) or
/// without an RW object (+0x18). The frame's modelling matrix "at" (+0x20) x:
///
/// - wind (0xC812F0) below 0.5 (ordered): `t = (now + this) & 0xFFF`, `fsin(t * 0.0015332) * (wind below 0.2 ? 0.005 :
///   0.008)`;
/// - else `i = (seed +0x20 + now * 8) & 0xFFFF`, `f = (i & 0xFFF) / 4096`, `s = i >> 12`; `((1 - f) * T[s] + f * T[(s +
///   1) & 15] + 1) * wind * 0.015` (T at 0x8CCF30);
///
/// stored f32; models whose flags (+0x12) & 0x7800 are 0x1000 add `wind * 0.03`. Then at.y = at.x, at.x *= the wind
/// direction x (0xC813E0), at.y *= its y (f32), `CWindModifiers::FindWindModifier(position, &at.x, &at.y)` and
/// `RwFrameUpdateObjects(frame)`.
#[hook(addr = 0x534e90, conv = thiscall, name = "CEntity::ModifyMatrixForTreeInWind", expect = "51 A0 49 CB B7 00",
       this = "CEntity", unverified = "mvpk-world-2", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "writes the RW frame matrix (RW heap)")]
pub fn entity_modify_matrix_for_tree_in_wind(this: GPtr<u8>) {
    if g::<u8>(TIMER_USER_PAUSE).read() != 0 || g::<u8>(TIMER_CODE_PAUSE).read() != 0 {
        return;
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() {
        return;
    }
    let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
    if m.is_null() {
        return;
    }
    let wind = k(WEATHER_WIND);
    let now = g::<u32>(TIMER_TIME_MS).read();
    let at = m.byte_add(0x20);
    let v = if !x(wind).fcom(x(k(0x858B8C))).test_parity(5) {
        let t = now.wrapping_add(this.addr() as u32) & 0xFFF;
        let amp = if !x(wind).fcom(x(k(0x858CC4))).test_parity(5) { k(0x858B4C) } else { k(0x863984) };
        fild_unsigned(t).fmul_m(k(0x863988)).fsin().fmul_m(amp)
    } else {
        let i = (u32::from(rd::<u16>(this, 0x20)).wrapping_add(now << 3)) & 0xFFFF;
        let s = ((i >> 12) & 0xF) as usize;
        let f = fild_unsigned(i & 0xFFF).fmul_m(k(0x858BFC));
        let t = |n: usize| g::<f32>(TREE_SWAY_TABLE + 4 * n).read();
        let a = x(k(0x858624)).fsub(f).fmul_m(t(s));
        let b = f.fmul_m(t((s + 1) & 0xF));
        a.fadd(b).fadd_m(k(0x858624)).fmul_m(wind).fmul_m(k(0x859F00))
    };
    wr::<f32>(at, 0, v.to_f32());
    let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFOS + 4 * (rd::<i16>(this, 0x22) as isize as usize)).read();
    if rd::<u16>(mi, 0x12) & 0x7800 == 0x1000 {
        wr::<f32>(at, 0, x(k(WEATHER_WIND)).fmul_m(k(0x858B10)).fadd_m(rd::<f32>(at, 0)).to_f32());
    }
    wr::<u32>(at, 4, rd::<u32>(at, 0));
    wr::<f32>(at, 0, x(k(WEATHER_WIND_DIR_X)).fmul_m(rd::<f32>(at, 0)).to_f32());
    wr::<f32>(at, 4, x(k(WEATHER_WIND_DIR_Y)).fmul_m(rd::<f32>(at, 4)).to_f32());
    let mat = rd::<GPtr<u8>>(this, 0x14);
    let pos = if mat.is_null() { this.byte_add(4) } else { mat.byte_add(0x30) };
    wind_modifiers_find(rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8), at.cast(), at.byte_add(4).cast());
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        rw_frame_update_objects(rd::<GPtr<u8>>(rw, 4));
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_ENTITY_MODIFY_MATRIX_FOR_TREE_IN_WIND];
