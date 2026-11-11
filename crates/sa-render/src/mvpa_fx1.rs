//! MVP-A writer batch `mvpa-fx-1`: `FxInfo*_c::GetValue` / `Load` and `FxInterpInfoFloat_c::GetVal`
//! (the FX keyframe evaluation). Ported from the 1.0 US bytes, x87 operations one by one through
//! [`Fpu`]. Every hook is unverified.
//!
//! `FxInfo*_c::GetValue(float currTime, float mult, float timeStep, float totalTime, bool useConst,
//! void* out)` (thiscall, `ret 0x18`): the key time is `mult` when the info's byte +6 is set, else
//! `(float)(currTime / totalTime)` (stored as f32 in the argument slot); the interp member at +8
//! fills a 16-float stack buffer.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, ftol32, k, rd, wr};

orig! {
    /// `FxInterpInfo32_c::GetVal(float* out, float t)` (0x4A89C0)
    pub fn interp_32_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A89C0 => crate::mvpc_fx1::interp_32_get_val;
    /// `FxInterpInfoFloat_c::GetVal(float* out, float t)` (0x4A8470)
    pub fn interp_float_get_val_call(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8470 => interp_float_get_val;
    /// `FxInterpInfoU255_c::GetVal(float* out, float t)` (0x4A8800)
    pub fn interp_u255_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) = thiscall 0x4A8800 => crate::mvpc_fx1::interp_u255_get_val;
    /// `FxInterpInfoFloat_c::GetVal(int index, float t0, float t1)` (0x4A85C0): the second overload,
    /// result in ST0
    pub fn interp_float_get_val2(this: GPtr<u8>, index: i32, t0: u32, t1: u32) -> f64 = thiscall 0x4A85C0 => unported;
}

/// The key time of the GetValue functions: `mult` with byte +6 set, else `(float)(currTime / totalTime)`.
fn key_time(this: GPtr<u8>, curr: u32, mult: u32, total: u32) -> u32 {
    if rd::<u8>(this, 6) != 0 {
        return mult;
    }
    let mut f = Fpu::new();
    f.fld(f32::from_bits(curr));
    f.fdiv_m(f32::from_bits(total));
    f.fstp().to_bits()
}

/// A 16-float stack buffer the interp member fills.
fn eval32(this: GPtr<u8>, t: u32) -> [f32; 16] {
    let mut buf = [0f32; 16];
    interp_32_get_val(this.byte_add(8), GPtr::from_raw(buf.as_mut_ptr()).cast(), t);
    buf
}

/// `FUN_004a5270` (0x4A5270, thiscall): the `FxInfoNoise_c` constructor (same shape as the other
/// `FxInfo*_c` constructors in `fx::info`): vtable 0x85A848, the `FxInterpInfo32_c` member at +8,
/// type word +4 = 0x2001, key count +0xE = 1, `+0x14 = g_fxMan pool GetMem(4, 4)`; returns this. The SEH
/// frame of the original is not needed (nothing in between throws).
#[hook(addr = 0x4A5270, conv = thiscall, name = "FUN_004a5270", expect = "6A FF 68 C3 8F 83 00",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates from the FX memory pool")]
pub fn fx_info_noise_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x85A848);
    crate::fx::info::interp_32_ctor(this.byte_add(8));
    wr::<u16>(this, 4, 0x2001);
    wr::<u8>(this, 0xE, 1);
    let m = crate::fx::pool_get_mem(GPtr::from_addr(crate::fx::FX_POOL), 4, 4);
    wr::<GPtr<u8>>(this, 0x14, m);
    this
}

/// `FxInfoForce_c::GetValue` (0x4A54E0): `out.velocity (+0xC, +0x10, +0x14) += key[i] * timeStep` (x87
/// per component, f32 stores).
#[hook(addr = 0x4A54E0, conv = thiscall, name = "FxInfoForce_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle movement info (caller stack or heap)")]
pub fn fx_info_force_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval32(this, t);
    let step = f32::from_bits(step);
    for (i, o) in [0xC, 0x10, 0x14].into_iter().enumerate() {
        let mut f = Fpu::new();
        f.fld(b[i]);
        f.fmul_m(step);
        f.fadd_m(rd::<f32>(out, o));
        wr::<f32>(out, o, f.fstp());
    }
}

/// `FxInfoFriction_c::GetValue` (0x4A55E0): `p = _CIpow(key[0], timeStep * 50.0f)` (the CRT's x87
/// helper 0x822130, run on the original's code); the three velocity components (+0xC, +0x10, +0x14)
/// are multiplied by p.
#[hook(addr = 0x4A55E0, conv = thiscall, name = "FxInfoFriction_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "mvpa-fx-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "pow through the CRT helper; writes the particle movement info")]
pub fn fx_info_friction_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval32(this, t);
    let e = X87::from_f32(f32::from_bits(step)).fmul_m(k(0x4248_0000));
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, X87::from_f32(b[0]).to_f64(), e.to_f64()));
    for o in [0xC, 0x10, 0x14] {
        wr::<f32>(out, o, p.fmul_m(rd::<f32>(out, o)).to_f32());
    }
}

/// `FxInfoWind_c::GetValue` (0x4A5AA0): `s = key[0] * *g_fxMan.windStrength ([0xA9AEB0]) * timeStep`
/// (kept on the x87 stack); velocity component i += `s * windDir[i]` (`[0xA9AEAC]`, re-read per
/// component).
#[hook(addr = 0x4A5AA0, conv = thiscall, name = "FxInfoWind_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle movement info")]
pub fn fx_info_wind_get_value(this: GPtr<u8>, curr: u32, mult: u32, step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let b = eval32(this, t);
    let strength = crate::common::g::<GPtr<f32>>(0xA9AEB0).read().read();
    let s = X87::from_f32(b[0]).fmul_m(strength).fmul_m(f32::from_bits(step));
    for (i, o) in [0xC, 0x10, 0x14].into_iter().enumerate() {
        let dir = crate::common::g::<GPtr<f32>>(0xA9AEAC).read().wrapping_index(i as i32).read();
        wr::<f32>(out, o, s.fmul_m(dir).fadd_m(rd::<f32>(out, o)).to_f32());
    }
}

/// `FxInfoRotSpeed_c::GetValue` (0x4A5D40): `FxInterpInfoFloat_c::GetVal`; `out +0x18 = key[0]` (fld/fstp)
/// and +0x1C, +0x20, +0x24 = key[1..3] (dword copies).
#[hook(addr = 0x4A5D40, conv = thiscall, name = "FxInfoRotSpeed_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle movement info")]
pub fn fx_info_rot_speed_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut b = [0u32; 16];
    interp_float_get_val_call(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    wr::<f32>(out, 0x18, X87::from_f32(f32::from_bits(b[0])).to_f32());
    wr::<u32>(out, 0x1C, b[1]);
    wr::<u32>(out, 0x20, b[2]);
    wr::<u32>(out, 0x24, b[3]);
}

/// `FxInfoColour_c::GetValue` (0x4A5F30): `FxInterpInfoU255_c::GetVal`; the four colour bytes of `out`
/// are `_ftol(key[i])` (low byte).
#[hook(addr = 0x4A5F30, conv = thiscall, name = "FxInfoColour_c::GetValue", expect = "8A 41 06 83 EC 40",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle render info")]
pub fn fx_info_colour_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut b = [0f32; 16];
    interp_u255_get_val(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    for (i, v) in b.iter().take(4).enumerate() {
        wr::<u8>(out, i as isize, ftol32(X87::from_f32(*v)) as u8);
    }
}

/// `FxInfoSize_c::Load(FILESTREAM file, int version)` (0x4A6010, thiscall, `ret 8`): `version < 0x69`
/// (signed) → key count +0xE = 2; returns the interp member's `Load(file)` (vtable slot +4).
#[hook(addr = 0x4A6010, conv = thiscall, name = "FxInfoSize_c::Load", expect = "83 7C 24 08 69",
       unverified = "mvpa-fx-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "reads the FX project file")]
pub fn fx_info_size_load(this: GPtr<u8>, file: u32, version: i32) -> u32 {
    if version < 0x69 {
        wr::<u8>(this, 0xE, 2);
    }
    interp_load(this.byte_add(8), file)
}

orig! {
    /// `FxInterpInfo*_c::Load(FILESTREAM)`: vtable slot +4 of the interp member
    pub fn interp_load(this: GPtr<u8>, file: u32) -> u32 = thiscall vslot 0x4 => unported;
}

/// `FxInfoSize_c::GetValue` (0x4A6030): `FxInterpInfo32_c::GetVal`; `out +0xC = key[0]` (fld/fstp),
/// +0x10 = key[1]; with 4 keys (+0xE) +0x14/+0x18 = key[2]/key[3], else 0.
#[hook(addr = 0x4A6030, conv = thiscall, name = "FxInfoSize_c::GetValue", expect = "83 EC 40 56 8B F1",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the particle render info")]
pub fn fx_info_size_get_value(this: GPtr<u8>, curr: u32, mult: u32, _step: u32, total: u32, _use_const: u32, out: GPtr<u8>) {
    let t = key_time(this, curr, mult, total);
    let mut b = [0u32; 16];
    interp_32_get_val(this.byte_add(8), GPtr::from_raw(b.as_mut_ptr()).cast(), t);
    wr::<f32>(out, 0xC, X87::from_f32(f32::from_bits(b[0])).to_f32());
    wr::<u32>(out, 0x10, b[1]);
    if rd::<u8>(this, 0xE) == 4 {
        wr::<u32>(out, 0x14, b[2]);
        wr::<u32>(out, 0x18, b[3]);
    } else {
        wr::<u32>(out, 0x14, 0);
        wr::<u32>(out, 0x18, 0);
    }
}

/// `FxInfoEmRate_c::GetValue(float currTime, float mult, float timeStep, float totalTime, bool
/// useConst, float* out)` (0x4A4B60): `prev = (float)(currTime - timeStep)`; with useConst: `*out =
/// key(currTime)[0] * timeStep`; else when `prev >= 0.0f` (ordered): `*out = GetVal2(0, currTime,
/// timeStep)`; otherwise (wrapping the loop start) `*out = GetVal2(0, totalTime, -prev)` then `*out =
/// GetVal2(0, currTime, prev + timeStep) + *out`.
#[hook(addr = 0x4A4B60, conv = thiscall, name = "FxInfoEmRate_c::GetValue", expect = "83 EC 44 D9 44 24 48",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the caller's emission count")]
pub fn fx_info_em_rate_get_value(this: GPtr<u8>, curr: u32, _mult: u32, step: u32, total: u32, use_const: u32, out: GPtr<u8>) {
    let interp = this.byte_add(8);
    let mut f = Fpu::new();
    f.fld(f32::from_bits(curr));
    f.fsub_m(f32::from_bits(step));
    let prev = f.fstp();
    let mut b = [0f32; 16];
    interp_float_get_val_call(interp, GPtr::from_raw(b.as_mut_ptr()).cast(), curr);
    if use_const as u8 != 0 {
        f.fld(b[0]);
        f.fmul_m(f32::from_bits(step));
        out.cast::<f32>().write(f.fstp());
        return;
    }
    f.fld(prev);
    f.fcomp_m(0.0);
    if f.jp(5) {
        let r = X87::from_f64_exact(interp_float_get_val2(interp, 0, curr, step));
        out.cast::<f32>().write(r.to_f32());
        return;
    }
    let neg = X87::from_f32(prev).fchs().to_f32();
    let r = X87::from_f64_exact(interp_float_get_val2(interp, 0, total, neg.to_bits()));
    out.cast::<f32>().write(r.to_f32());
    let t1 = X87::from_f32(prev).fadd_m(f32::from_bits(step)).to_f32();
    let r = X87::from_f64_exact(interp_float_get_val2(interp, 0, curr, t1.to_bits()));
    let o = out.cast::<f32>();
    o.write(r.fadd_m(o.read()).to_f32());
}

/// `FxInterpInfoFloat_c::GetVal(float* out, float t)` (0x4A8470, thiscall, `ret 8`). Layout: +4 looped,
/// +5 key count (i8), +6 value count (i8, re-read per loop), +8 key times (u16, units of 1/256 s), +0xC
/// per-value key arrays (float*). One key: `out[i] = keys[i][0]`. Looped: `T = times[n-1] / 256`,
/// `t -= _ftol(t / T) * T` (x87, stored f32). The first key j ≥ 1 with `t < times[j] / 256` (x87
/// compare, `jnp`) gives `f = (float)((t - Tp) / (Tj - Tp))` and `out[i] = (v[j] - v[j-1]) * f +
/// v[j-1]`; none: `out[i] = keys[i][n-1]`. Indexing wraps like the original for degenerate counts.
#[hook(addr = 0x4A8470, conv = thiscall, name = "FxInterpInfoFloat_c::GetVal", expect = "51 53 56 8B F1 8A 5E 05",
       unverified = "mvpa-fx-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the caller's key buffer (stack of the caller)")]
pub fn interp_float_get_val(this: GPtr<u8>, out: GPtr<u8>, t: u32) {
    let out = out.cast::<u32>();
    let n = rd::<i8>(this, 5) as i32;
    let vals = rd::<GPtr<GPtr<u32>>>(this, 0xC);
    let count = || rd::<i8>(this, 6) as i32;
    if n == 1 {
        let mut i = 0;
        while i < count() {
            out.wrapping_index(i).write(vals.wrapping_index(i).read().read());
            i += 1;
        }
        return;
    }
    let times = rd::<GPtr<u16>>(this, 8);
    let mut t = f32::from_bits(t);
    if rd::<u8>(this, 4) != 0 {
        let mut f = Fpu::new();
        f.fild(i32::from(times.wrapping_index(n - 1).read()));
        f.fmul_m(k(0x3B80_0000));
        f.fld(t);
        f.fdiv_st(1);
        let q = ftol32(f.pop());
        f.fild(q);
        f.fmul_st(1);
        f.fsubr_m(t);
        t = f.fstp();
    }
    let mut j = 1;
    let mut found = None;
    while j < n {
        let mut f = Fpu::new();
        f.fild(i32::from(times.wrapping_index(j).read()));
        f.fmul_m(k(0x3B80_0000));
        f.fld(t);
        f.fcomp_st(1);
        if !f.jp(5) {
            found = Some((j, f));
            break;
        }
        j += 1;
    }
    let Some((j, mut f)) = found else {
        let mut i = 0;
        while i < count() {
            let v = vals.wrapping_index(i).read();
            out.wrapping_index(i).write(v.wrapping_index(n - 1).read());
            i += 1;
        }
        return;
    };
    f.fild(i32::from(times.wrapping_index(j - 1).read()));
    f.fmul_m(k(0x3B80_0000));
    f.fld(t);
    f.fsub_st(1);
    f.fxch(2);
    f.fsub_st(1);
    f.fdivp(2);
    f.fxch(1);
    let frac = f.fstp();
    let mut i = 0;
    while i < count() {
        let v = vals.wrapping_index(i).read().cast::<f32>();
        let mut g = Fpu::new();
        let a = v.wrapping_index(j - 1).read();
        g.fld(v.wrapping_index(j).read());
        g.fsub_m(a);
        g.fmul_m(frac);
        g.fadd_m(a);
        out.cast::<f32>().wrapping_index(i).write(g.fstp());
        i += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FX_INFO_NOISE_CTOR,
    HOOK_FX_INFO_FORCE_GET_VALUE,
    HOOK_FX_INFO_FRICTION_GET_VALUE,
    HOOK_FX_INFO_WIND_GET_VALUE,
    HOOK_FX_INFO_ROT_SPEED_GET_VALUE,
    HOOK_FX_INFO_COLOUR_GET_VALUE,
    HOOK_FX_INFO_SIZE_LOAD,
    HOOK_FX_INFO_SIZE_GET_VALUE,
    HOOK_FX_INFO_EM_RATE_GET_VALUE,
    HOOK_INTERP_FLOAT_GET_VAL,
];
