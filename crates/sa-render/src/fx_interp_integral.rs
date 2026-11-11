//! `FxInterpInfoFloat_c::GetVal(int, float, float)` (0x4A85C0): the integral of one value's piecewise-linear
//! key curve over a time window (the emission rate integrated over a frame step). Ported from the 1.0 US
//! bytes; every x87 operation in the original's order on [`Fpu`] (the running values stay in registers
//! across the key loop, so no intermediate is rounded to f32 except the key times, which are exact).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{Fpu, k, rd};

/// 1/256 (0x859AA0): key time units (u16) to seconds
const K_TIME_SCALE: u32 = 0x3B80_0000;
/// 0.5 (0x858B8C): the trapezoid midpoint weight
const K_HALF: u32 = 0x3F00_0000;
/// 0.0 (0x858B50): the accumulator's initial value
const K_ZERO: u32 = 0;

/// `FxInterpInfoFloat_c::GetVal(int idx, float t, float dt)` (0x4A85C0, thiscall, `ret 0xC`), result in ST0.
/// Layout: +5 key count `n` (i8), +8 key times (u16, 1/256 s), +0xC per-value key arrays (`float*[]`);
/// `v = keys[idx]`, `T(j) = times[j] / 256`.
///
/// - `n == 1`: `dt * v[0]`.
/// - Otherwise `A = t - dt`, `z = 0.0` (the accumulator, ST at the bottom). The first `e` in `0..n` with
///   `A < T(e)` (x87 compare, `jnp` on `test ah, 5`): `e == 0` → current value `c = v[0]`, else `c =
///   ((A - T(e-1)) / (T(e) - T(e-1))) * (v[e] - v[e-1]) + v[e-1]`. None (or NaN `A`, or `n <= 0`) →
///   return `dt * v[n-1]` (`n == 0` reads `v[-1]`, `n < 0` returns 0.0).
/// - Then for `e` up to `n - 1` (`Tn = T(e)`, `Tp = T(e-1)` stored as f32 in the argument slots; `e == 0`
///   reads `times[-1]`, an original out-of-bounds read):
///   - `Tn == t` → `z += (Tn - A) * ((v[e] - c) * 0.5 + c)` and return `z`;
///   - `Tn < t` → `z += (Tn - A) * ((v[e] - c) * 0.5 + c)`, then `c = v[e]`, `A = Tn`, next key;
///   - `Tn > t` → `w = ((t - Tp) / (Tn - Tp)) * (v[e] - v[e-1]) + v[e-1]`, `z += (t - A) * ((w - c) * 0.5
///     + c)` and return `z`;
///   - unordered (NaN) → next key, unchanged.
/// - Past the last key: `z += (T(n-1) - A) * ((v[n-1] - c) * 0.5 + c)` (this `T(n-1)` is the register
///   value, the same number) and return `z`.
///
/// The original overwrites its own `idx`, `dt` and (last segment) `t` argument slots with `Tp`, `Tn` and
/// `times[n-1]`; they are dead once it returns (`ret 0xC` pops them).
#[hook(addr = 0x4a85c0, conv = thiscall, name = "FxInterpInfoFloat_c::GetVal",
       expect = "8A 41 05 D9 05 50 8B 85 00", unverified = "mvpl-fx-1", route = "L2", hazards = "none",
       recs = "grove-walk",
       note = "x87: the accumulator, the current value and the window start stay in registers across keys; writes only its own argument slots")]
pub fn interp_float_get_val_range(this: GPtr<u8>, idx: i32, t: u32, dt: u32) -> X87 {
    let (t, dt) = (f32::from_bits(t), f32::from_bits(dt));
    let n = i32::from(rd::<i8>(this, 5));
    let keys = || rd::<GPtr<GPtr<f32>>>(this, 0xC).wrapping_index(idx).read();
    let mut f = Fpu::new();
    f.fld(k(K_ZERO));
    if n == 1 {
        f.fstp_st(0);
        f.fld(dt);
        f.fmul_m(keys().read());
        return f.pop();
    }
    let times = rd::<GPtr<u16>>(this, 8);
    let time = |e: i32| i32::from(times.wrapping_index(e).read());
    f.fld(t);
    f.fsub_m(dt);
    // first loop: the first key after the window start
    let mut e = 0;
    let mut found = false;
    while e < n {
        f.fild(time(e));
        f.fmul_m(k(K_TIME_SCALE));
        f.fld_st(1);
        f.fcomp_st(1);
        if !f.jp(5) {
            found = true;
            break;
        }
        e += 1;
        f.fstp_st(0);
    }
    if !found {
        f.fld(t);
    } else if e <= 0 {
        f.fstp_st(0);
        f.fld(keys().wrapping_index(e).read());
    } else {
        let v = keys();
        f.fild(time(e - 1));
        f.fmul_m(k(K_TIME_SCALE));
        f.fld_st(2);
        f.fsub_st(1);
        f.fxch(2);
        f.fsub_st(1);
        f.fdivp(2);
        f.fld(v.wrapping_index(e).read());
        f.fsub_m(v.wrapping_index(e - 1).read());
        f.fmulp(2);
        f.fxch(1);
        f.fadd_m(v.wrapping_index(e - 1).read());
        f.fxch(1);
        f.fstp_st(0);
    }
    // 0x4A862A
    if e == n {
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(dt);
        f.fmul_m(keys().wrapping_index(e - 1).read());
        return f.pop();
    }
    if e < n {
        // second loop (0x4A86A0)
        while e < n {
            let tn = X87::from_i32(time(e)).fmul_m(k(K_TIME_SCALE)).to_f32();
            let tp = X87::from_i32(time(e - 1)).fmul_m(k(K_TIME_SCALE)).to_f32();
            f.fld(tn);
            f.fcomp_m(t);
            if !f.jp(0x44) {
                // Tn == t (0x4A8732)
                let v = keys();
                f.fld(v.wrapping_index(e).read());
                f.fsub_st(1);
                f.fmul_m(k(K_HALF));
                f.fadd_st(1);
                f.fld(tn);
                accumulate(&mut f);
                break;
            }
            f.fld(tn);
            f.fcomp_m(t);
            if !f.jp(5) {
                // Tn < t (0x4A86E9)
                let v = keys();
                f.fld(v.wrapping_index(e).read());
                f.fsub_st(1);
                f.fmul_m(k(K_HALF));
                f.fadd_st(1);
                f.fld(tn);
                f.fsub_st(3);
                f.fmulp(1);
                f.faddp(3);
                f.fstp_st(0);
                f.fstp_st(0);
                f.fld(tn);
                f.fld(v.wrapping_index(e).read());
                e += 1;
                continue;
            }
            f.fld(tn);
            f.fcomp_m(t);
            if !f.jne(0x41) {
                // Tn > t (0x4A874B)
                let v = keys();
                f.fld(t);
                f.fsub_m(tp);
                f.fld(tn);
                f.fsub_m(tp);
                f.fdivp(1);
                f.fld(v.wrapping_index(e).read());
                f.fsub_m(v.wrapping_index(e - 1).read());
                f.fmulp(1);
                f.fadd_m(v.wrapping_index(e - 1).read());
                f.fsub_st(1);
                f.fmul_m(k(K_HALF));
                f.fadd_st(1);
                f.fld(t);
                accumulate(&mut f);
                break;
            }
            e += 1;
        }
    }
    // 0x4A8784
    if e == n {
        let v = keys();
        f.fld(v.wrapping_index(e - 1).read());
        f.fsub_st(1);
        f.fmul_m(k(K_HALF));
        f.fadd_st(1);
        f.fild(time(e - 1));
        f.fmul_m(k(K_TIME_SCALE));
        accumulate(&mut f);
    }
    f.fstp_st(0);
    f.fstp_st(0);
    f.pop()
}

/// 0x4A877E: `[x, m, c, A, z]` → `z += (x - A) * m`, leaving `[c, A, z]`.
fn accumulate(f: &mut Fpu) {
    f.fsub_st(3);
    f.fmulp(1);
    f.faddp(3);
}

/// Phase B/C path of the `orig!` declarations of 0x4A85C0 (ST0 as f64; the value has at most 24
/// significant bits, so the f64 holds it exactly).
pub fn interp_float_get_val_range_host(this: GPtr<u8>, idx: i32, t: u32, dt: u32) -> f64 {
    interp_float_get_val_range(this, idx, t, dt).to_f64()
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERP_FLOAT_GET_VAL_RANGE];
