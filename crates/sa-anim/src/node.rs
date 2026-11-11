//! ANIM: `CAnimBlendNode` (PORT-ANIM-INPUT). Ported from the 1.0 US bytes (gta-reversed
//! `AnimBlendNode.cpp` as a reading aid, `docs/re/specs/ANIM/CAnimBlendNode*.md`). See
//! `docs/arch/anim-input-port.md`.
//!
//! Layout (0x18): +0x0 `m_Theta` (f32), +0x4 `m_InvSin` (f32), +0x8 `m_KFCurr` (int16), +0xA
//! `m_KFPrev` (int16), +0xC `m_KFRemainingTime` (f32), +0x10 `m_Seq` (`CAnimBlendSequence*`), +0x14
//! `m_BlendAssoc` (`CAnimBlendAssociation*`: +0x18 blend amount, +0x28 time step, +0x2E flags byte:
//! bit0 PLAYING, bit1 LOOPED, bit4 IGNORE weight).
//!
//! x87 detail kept: the blend factor and the interpolation fraction are f32 stack slots in `Update*`
//! (the original stores them into its own argument slots, which a port cannot reproduce; they are not
//! read by the callers), some intermediates are rounded to f32, others stay in registers (PC=24).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{rd, wr, x};
use crate::sequence::{self, FLAG_ROT, FLAG_TRANS, K_INV1024, K_INV4096, k_inv60};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CAnimBlendNode::CalcTheta(const CQuaternion*, const CQuaternion*, float* theta, float* invSin)` (0x4D00E0)
    pub fn call_calc_theta(a: GPtr<f32>, b: GPtr<f32>, theta: GPtr<f32>, inv_sin: GPtr<f32>) = cdecl 0x4D00E0 => calc_theta;
    /// `CAnimBlendNode::CalcDeltas()` (0x4D0190)
    pub fn call_calc_deltas(this: GPtr<u8>) = thiscall 0x4D0190 => calc_deltas;
    /// `CAnimBlendNode::CalcDeltasCompressed()` (0x4D0350)
    pub fn call_calc_deltas_compressed(this: GPtr<u8>) = thiscall 0x4D0350 => calc_deltas_compressed;
    /// `CAnimBlendNode::NextKeyFrame()` (0x4D04A0)
    pub fn call_next_key_frame(this: GPtr<u8>) -> u8 = thiscall 0x4D04A0 => next_key_frame;
    /// `CAnimBlendNode::NextKeyFrameCompressed()` (0x4D0570)
    pub fn call_next_key_frame_compressed(this: GPtr<u8>) -> u8 = thiscall 0x4D0570 => next_key_frame_compressed;
    /// `CAnimBlendNode::NextKeyFrameNoCalc()` (0x4CFB90)
    pub fn call_next_key_frame_no_calc(this: GPtr<u8>) -> u8 = thiscall 0x4CFB90 => next_key_frame_no_calc;
    /// `FUN_004CFA90`: `CQuaternion` from `FixedQuat<int16, 4096>` (cdecl `(float* out, const int16* in)`)
    pub fn uncompress_quat(out: GPtr<f32>, q: GPtr<u8>) = cdecl 0x4CFA90 => uncompress_quat_port;
    /// `FUN_004CFB10`: `CVector` from `FixedVector<int16, 1024>` (cdecl `(float* out, const int16* in)`)
    pub fn uncompress_vec(out: GPtr<f32>, v: GPtr<u8>) = cdecl 0x4CFB10 => uncompress_vec_port;
    /// `FUN_004CFA30` (see `hierarchy::compress_quaternion`)
    pub fn compress_quat(out: GPtr<u8>, q: GPtr<f32>) = cdecl 0x4CFA30 => compress_quat_port;
    /// `CQuaternion::Slerp(const CQuaternion& from, const CQuaternion& to, float halfTheta, float invSin, float t)` (0x59C300)
    pub fn quat_slerp(this: GPtr<f32>, from: GPtr<f32>, to: GPtr<f32>, theta: f32, inv_sin: f32, t: f32) = thiscall 0x59C300 => unported;
    /// CRT `_CIacos` (0x822380): argument and result in ST0; not callable through a declared ABI
    pub fn ci_acos_unused() = cdecl 0x822380 => unported;
}

/// `FUN_004CFA90` port (not hooked: catalogue row outside this pass): `out[k] = fild(in[k]) * (1/4096)`.
pub fn uncompress_quat_port(out: GPtr<f32>, q: GPtr<u8>) {
    for k in 0..4 {
        let v = X87::from_i32(i32::from(rd::<i16>(q, 2 * k))).fmul_m(K_INV4096).to_f32();
        out.byte_add(4 * k).write(v);
    }
}

/// `FUN_004CFB10` port (not hooked): `out[k] = fild(in[k]) * (1/1024)`.
pub fn uncompress_vec_port(out: GPtr<f32>, v: GPtr<u8>) {
    for k in 0..3 {
        let f = X87::from_i32(i32::from(rd::<i16>(v, 2 * k))).fmul_m(K_INV1024).to_f32();
        out.byte_add(4 * k).write(f);
    }
}

pub fn compress_quat_port(out: GPtr<u8>, q: GPtr<f32>) {
    crate::hierarchy::compress_quaternion(out, q.cast());
}

#[inline]
fn seq(n: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(n, 0x10)
}
#[inline]
fn assoc(n: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(n, 0x14)
}
#[inline]
fn kf_curr(n: GPtr<u8>) -> i16 {
    rd::<i16>(n, 8)
}
#[inline]
fn kf_prev(n: GPtr<u8>) -> i16 {
    rd::<i16>(n, 0xA)
}
#[inline]
fn rem(n: GPtr<u8>) -> f32 {
    rd::<f32>(n, 0xC)
}

/// Uncompressed key frame `i` of sequence `s` (stride 0x20 with translation, else 0x14).
#[inline]
fn ukf(s: GPtr<u8>, i: i16) -> GPtr<u8> {
    let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x20 } else { 0x14 };
    sequence::frames(s).byte_add(isize::from(i) * stride)
}

/// Compressed key frame `i` (stride 0x10 / 0xA).
#[inline]
fn ckf(s: GPtr<u8>, i: i16) -> GPtr<u8> {
    let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x10 } else { 0xA };
    sequence::frames(s).byte_add(isize::from(i) * stride)
}

/// `fcom` then `test ah, 0x41; jp` / `jnp`: true when `v <= 0` (not NaN).
#[inline]
fn le_zero(v: X87) -> bool {
    !v.fcom(X87::zero()).test_parity(0x41)
}

/// `test ah, 0x41; jne`-skip: true when `v > 0`.
#[inline]
fn gt_zero(v: X87) -> bool {
    v.fcom(X87::zero()) == Cmp::Greater
}

/// The blend factor: `assoc->m_BlendAmount`, times `weight` unless the IGNORE-weight flag is set.
#[inline]
fn blend(n: GPtr<u8>, weight: f32) -> X87 {
    let a = assoc(n);
    let b = x(rd::<f32>(a, 0x18));
    if rd::<u8>(a, 0x2E) & 0x10 != 0 { b } else { b.fmul_m(weight) }
}

/// `CAnimBlendNode::Init()` (0x4CFB70).
#[hook(addr = 0x4CFB70, conv = thiscall, name = "CAnimBlendNode::Init", expect = "33 C0 89 41 14", this = "CAnimBlendNode")]
pub fn init(this: GPtr<u8>) {
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<i16>(this, 0xA, -1);
    wr::<i16>(this, 8, -1);
}

/// The shared key-frame advance of `NextKeyFrame*`: `None` = ran off the end of a non-looped
/// sequence (current = last, remaining time 0, AL = 0); `Some(looped)` otherwise (previous fixed up).
fn advance(this: GPtr<u8>, compressed: bool) -> Option<u8> {
    let s = seq(this);
    let n = i32::from(sequence::num_frames(s));
    let mut looped = 0u8;
    wr::<i16>(this, 0xA, kf_curr(this));
    if le_zero(x(rem(this))) {
        loop {
            let c = kf_curr(this).wrapping_add(1);
            wr::<i16>(this, 8, c);
            if i32::from(c) >= n {
                if rd::<u8>(assoc(this), 0x2E) & 2 != 0 {
                    wr::<i16>(this, 8, 0);
                    looped = 1;
                } else {
                    wr::<i16>(this, 8, c.wrapping_sub(1));
                    wr::<u32>(this, 0xC, 0);
                    return None;
                }
            }
            let t = if compressed {
                X87::from_i32(i32::from(rd::<i16>(ckf(s, kf_curr(this)), 8))).fmul_m(k_inv60())
            } else {
                x(rd::<f32>(ukf(s, kf_curr(this)), 0x10))
            };
            wr::<f32>(this, 0xC, t.fadd_m(rem(this)).to_f32());
            if !le_zero(x(rem(this))) {
                break;
            }
        }
    }
    let p = kf_curr(this).wrapping_sub(1);
    wr::<i16>(this, 0xA, if p < 0 { p.wrapping_add(n as i16) } else { p });
    Some(looped)
}

/// `CAnimBlendNode::NextKeyFrameNoCalc()` (0x4CFB90, gta-reversed: unreachable): the key-frame advance
/// of `NextKeyFrame` without `CalcDeltas`; always the uncompressed stride.
#[hook(addr = 0x4CFB90, conv = thiscall, name = "CAnimBlendNode::NextKeyFrameNoCalc", expect = "55 57 8B 79 10", this = "CAnimBlendNode")]
pub fn next_key_frame_no_calc(this: GPtr<u8>) -> u8 {
    if sequence::num_frames(seq(this)) <= 1 {
        return 0;
    }
    advance(this, false).unwrap_or(0)
}

/// `CAnimBlendNode::NextKeyFrame()` (0x4D04A0): the advance, then `CalcDeltas` (not after running off
/// the end).
#[hook(addr = 0x4D04A0, conv = thiscall, name = "CAnimBlendNode::NextKeyFrame", expect = "55 57 8B 79 10", this = "CAnimBlendNode")]
pub fn next_key_frame(this: GPtr<u8>) -> u8 {
    if sequence::num_frames(seq(this)) <= 1 {
        return 0;
    }
    match advance(this, false) {
        None => 0,
        Some(l) => {
            call_calc_deltas(this);
            l
        }
    }
}

/// `CAnimBlendNode::NextKeyFrameCompressed()` (0x4D0570): as `NextKeyFrame` on compressed frames
/// (`fild time * (1/60)`), then `CalcDeltasCompressed`.
#[hook(addr = 0x4D0570, conv = thiscall, name = "CAnimBlendNode::NextKeyFrameCompressed", expect = "51 55 57 8B 79 10", this = "CAnimBlendNode")]
pub fn next_key_frame_compressed(this: GPtr<u8>) -> u8 {
    if sequence::num_frames(seq(this)) <= 1 {
        return 0;
    }
    match advance(this, true) {
        None => 0,
        Some(l) => {
            call_calc_deltas_compressed(this);
            l
        }
    }
}

/// `prev + (cur - prev) * t`, then times `b` (the `GetCurrentTranslation`/`Update` translation
/// interpolation): dz and dx*t are rounded to f32, dy*t (`keep_y = true`: `Update*`) or dy*t rounded
/// (`GetCurrentTranslation*`), dz*t stays in a register.
fn lerp_trans(out: GPtr<f32>, prev: [f32; 3], cur: [f32; 3], t: X87, b: X87, round_y: bool) {
    let dx = x(cur[0]).fsub_m(prev[0]);
    let dy = x(cur[1]).fsub_m(prev[1]);
    let dz = x(cur[2]).fsub_m(prev[2]).to_f32();
    let tx = dx.fmul(t).to_f32();
    let ty = dy.fmul(t);
    let ty = if round_y { x(ty.to_f32()) } else { ty };
    let tz = t.fmul_m(dz);
    let rx = x(tx).fadd_m(prev[0]).to_f32();
    let ry = ty.fadd_m(prev[1]).to_f32();
    let rz = tz.fadd_m(prev[2]).to_f32();
    out.write(rx);
    out.byte_add(4).write(ry);
    out.byte_add(8).write(rz);
    for k in 0..3 {
        let p = out.byte_add(4 * k);
        p.write(b.fmul_m(p.read()).to_f32());
    }
}

fn vec3(p: GPtr<u8>, off: isize) -> [f32; 3] {
    [rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8)]
}

fn zero3(out: GPtr<f32>) {
    for k in 0..3 {
        out.byte_add(4 * k).write(0.0);
    }
}

/// `CAnimBlendNode::GetCurrentTranslation(CVector& out, float weight)` (0x4CFC50): 0, or with
/// translation and blend > 0 the interpolated translation times the blend.
#[hook(addr = 0x4CFC50, conv = thiscall, name = "CAnimBlendNode::GetCurrentTranslation", expect = "83 EC 18 33 C0", this = "CAnimBlendNode")]
pub fn get_current_translation(this: GPtr<u8>, out: GPtr<f32>, weight: f32) {
    zero3(out);
    let b = blend(this, weight);
    if !gt_zero(b) {
        return;
    }
    let s = seq(this);
    let a = ukf(s, kf_prev(this));
    let c = ukf(s, kf_curr(this));
    let ct = x(rd::<f32>(c, 0x10));
    let t = if ct.fcom(X87::zero()) == Cmp::Equal {
        X87::zero()
    } else {
        ct.fsub_m(rem(this)).fdiv(ct)
    };
    if sequence::flags(s) & FLAG_TRANS == 0 {
        return;
    }
    lerp_trans(out, vec3(a, 0x14), vec3(c, 0x14), t, b, true);
}

/// `CAnimBlendNode::GetEndTranslation(CVector& out, float weight)` (0x4CFD90): 0, or with blend > 0
/// and translation the last frame's translation times the blend.
#[hook(addr = 0x4CFD90, conv = thiscall, name = "CAnimBlendNode::GetEndTranslation", expect = "83 EC 0C 56 8B 74 24 14", this = "CAnimBlendNode")]
pub fn get_end_translation(this: GPtr<u8>, out: GPtr<f32>, weight: f32) {
    zero3(out);
    let b = blend(this, weight);
    if !gt_zero(b) {
        return;
    }
    let s = seq(this);
    let last = ukf(s, sequence::num_frames(s).wrapping_sub(1));
    if sequence::flags(s) & FLAG_TRANS == 0 {
        return;
    }
    for k in 0..3 {
        out.byte_add(4 * k).write(b.fmul_m(rd::<f32>(last, 0x14 + 4 * k)).to_f32());
    }
}

/// `t = (T - rem) / T` with `T = fild(time) * (1/60)`, 0 when the int16 time is 0 (stored as f32).
fn compressed_fraction(this: GPtr<u8>, cur: GPtr<u8>) -> f32 {
    let ti = rd::<i16>(cur, 8);
    if ti == 0 {
        return 0.0;
    }
    let tt = X87::from_i32(i32::from(ti)).fmul_m(k_inv60());
    tt.fsub_m(rem(this)).fdiv(tt).to_f32()
}

/// `CAnimBlendNode::GetCurrentTranslationCompressed(CVector& out, float weight)` (0x4CFE60).
#[hook(addr = 0x4CFE60, conv = thiscall, name = "CAnimBlendNode::GetCurrentTranslationCompressed", expect = "83 EC 28 56 57", this = "CAnimBlendNode")]
pub fn get_current_translation_compressed(this: GPtr<u8>, out: GPtr<f32>, weight: f32) {
    zero3(out);
    let bb = blend(this, weight);
    let b = x(bb.to_f32());
    if !gt_zero(bb) {
        return;
    }
    let s = seq(this);
    let a = ckf(s, kf_prev(this));
    let c = ckf(s, kf_curr(this));
    if sequence::flags(s) & FLAG_TRANS == 0 {
        return;
    }
    let t = compressed_fraction(this, c);
    let mut pa = [0f32; 3];
    let mut pc = [0f32; 3];
    uncompress_vec(GPtr::from_raw(pa.as_mut_ptr()), a.byte_add(0xA));
    uncompress_vec(GPtr::from_raw(pc.as_mut_ptr()), c.byte_add(0xA));
    lerp_trans(out, pa, pc, x(t), b, false);
}

/// `CAnimBlendNode::GetEndTranslationCompressed(CVector& out, float weight)` (0x4D0000).
#[hook(addr = 0x4D0000, conv = thiscall, name = "CAnimBlendNode::GetEndTranslationCompressed", expect = "83 EC 0C 56 8B 74 24 14", this = "CAnimBlendNode")]
pub fn get_end_translation_compressed(this: GPtr<u8>, out: GPtr<f32>, weight: f32) {
    zero3(out);
    let bb = blend(this, weight);
    let b = bb.to_f32();
    if !gt_zero(bb) {
        return;
    }
    let s = seq(this);
    if sequence::flags(s) & FLAG_TRANS == 0 {
        return;
    }
    let last = ckf(s, sequence::num_frames(s).wrapping_sub(1));
    uncompress_vec(out, last.byte_add(0xA));
    for k in 0..3 {
        let p = out.byte_add(4 * k);
        p.write(x(b).fmul_m(p.read()).to_f32());
    }
}

/// `_CIacos` (0x822380) of `d` (an f32 value) as the CRT computes it under the game's CW 0x7F (its
/// CW switch keeps PC = 24): |d| < 1: `atan2(sqrt((1 + d) * (1 - d)), d)` (PC=24 for the arithmetic,
/// FPATAN unrounded); d = ±1: 0 / pi; |d| > 1, ±inf or NaN: the indefinite NaN. FPATAN and pi are modelled
/// at 53 bits (`sa_math::x87`, ledger MATH-2): the stored f32 results are what L2 checks.
fn ci_acos(d: X87) -> X87 {
    let indefinite = X87::from_f64_exact(f64::from_bits(0xFFF8_0000_0000_0000));
    if d.is_nan() {
        return indefinite;
    }
    let v = d.to_f64();
    if v.abs() < 1.0 {
        let s = X87::one().fadd(d).fmul(X87::one().fsub(d)).fsqrt();
        return s.fpatan(d);
    }
    if v == 1.0 {
        return X87::zero();
    }
    if v == -1.0 {
        return X87::from_f64_exact(core::f64::consts::PI);
    }
    // the CRT error path (`__math_exit` / matherr bookkeeping not modelled)
    indefinite
}

/// `CAnimBlendNode::CalcTheta(const CQuaternion* a, const CQuaternion* b, float* theta, float* invSin)`
/// (0x4D00E0, cdecl): `d = ((aw*bw + az*bz) + ay*by) + ax*bx`, clamped to 1.0 when greater (f32);
/// `theta = acos(d)`; `invSin = theta == 0 ? 0 : 1 / sin(theta)` (FSIN of the unrounded acos).
#[hook(addr = 0x4D00E0, conv = cdecl, name = "CAnimBlendNode::CalcTheta", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn calc_theta(a: GPtr<f32>, b: GPtr<f32>, theta: GPtr<f32>, inv_sin: GPtr<f32>) {
    let f = |p: GPtr<f32>, k: isize| x(p.byte_add(4 * k).read());
    let dot = f(a, 3)
        .fmul(f(b, 3))
        .fadd(f(a, 2).fmul(f(b, 2)))
        .fadd(f(a, 1).fmul(f(b, 1)))
        .fadd(f(a, 0).fmul(f(b, 0)));
    let d = if dot.fcom(X87::one()) == Cmp::Greater { 1.0 } else { dot.to_f32() };
    let th = ci_acos(x(d));
    theta.write(th.to_f32());
    if th.fcom(X87::zero()).test_parity(0x44) {
        inv_sin.write(th.fsin().fdivr_m(1.0).to_f32());
    } else {
        inv_sin.write(0.0);
    }
}

/// `CAnimBlendNode::UpdateTime()` (0x4D0160): while playing, `rem -= timeStep`; at <= 0
/// `NextKeyFrameNoCalc` (tail jump); else AL = 0.
#[hook(addr = 0x4D0160, conv = thiscall, name = "CAnimBlendNode::UpdateTime", expect = "8B 41 14 F6 40 2E 01", this = "CAnimBlendNode")]
pub fn update_time(this: GPtr<u8>) -> u8 {
    let a = assoc(this);
    if rd::<u8>(a, 0x2E) & 1 == 0 {
        return 0;
    }
    let r = x(rem(this)).fsub_m(rd::<f32>(a, 0x28));
    wr::<f32>(this, 0xC, r.to_f32());
    if !le_zero(r) {
        return 0;
    }
    call_next_key_frame_no_calc(this)
}

/// `CAnimBlendNode::CalcDeltas()` (0x4D0190): with rotation, `CalcTheta(prev quat, current quat,
/// &m_Theta, &m_InvSin)` (the original passes stack copies; CalcTheta only loads them).
#[hook(addr = 0x4D0190, conv = thiscall, name = "CAnimBlendNode::CalcDeltas", expect = "83 EC 20 56 8B F1", this = "CAnimBlendNode")]
pub fn calc_deltas(this: GPtr<u8>) {
    let s = seq(this);
    if sequence::flags(s) & FLAG_ROT == 0 {
        return;
    }
    let prev = ukf(s, kf_prev(this));
    let cur = ukf(s, kf_curr(this));
    let mut p = [0u32; 4];
    let mut c = [0u32; 4];
    for k in 0..4 {
        p[k] = rd::<u32>(prev, 4 * k as isize);
        c[k] = rd::<u32>(cur, 4 * k as isize);
    }
    call_calc_theta(
        GPtr::from_raw(p.as_mut_ptr()).cast(),
        GPtr::from_raw(c.as_mut_ptr()).cast(),
        this.cast(),
        this.byte_add(4).cast(),
    );
}

/// `CAnimBlendNode::FindKeyFrame(float t)` (0x4D0240): the key frame containing time `t` (looped
/// sequences wrap); remaining time = frame time - rest of `t`; `CalcDeltas`. AL = found.
#[hook(addr = 0x4D0240, conv = thiscall, name = "CAnimBlendNode::FindKeyFrame", expect = "56 8B F1 8B 4E 10", this = "CAnimBlendNode")]
pub fn find_key_frame(this: GPtr<u8>, t: f32) -> u8 {
    let s = seq(this);
    if sequence::num_frames(s) < 1 {
        return 0;
    }
    wr::<i16>(this, 8, 0);
    wr::<i16>(this, 0xA, 0);
    if sequence::num_frames(s) == 1 {
        wr::<u32>(this, 0xC, 0);
        call_calc_deltas(this);
        return 1;
    }
    let mut t = x(t);
    wr::<i16>(this, 8, 1);
    loop {
        let e = kf_curr(this);
        let ft = rd::<f32>(ukf(s, e), 0x10);
        if t.fcom(x(ft)) != Cmp::Greater {
            wr::<f32>(this, 0xC, x(ft).fsub(t).to_f32());
            call_calc_deltas(this);
            return 1;
        }
        t = t.fsub_m(ft);
        if i32::from(e) + 1 >= i32::from(sequence::num_frames(s)) {
            if rd::<u8>(assoc(this), 0x2E) & 2 == 0 {
                call_calc_deltas(this);
                wr::<u32>(this, 0xC, 0);
                return 0;
            }
            wr::<i16>(this, 8, 0);
        }
        let c = kf_curr(this);
        wr::<i16>(this, 0xA, c);
        wr::<i16>(this, 8, c.wrapping_add(1));
    }
}

/// `CAnimBlendNode::CalcDeltasCompressed()` (0x4D0350): with rotation, both quaternions decompressed;
/// when their dot `((aw*bw + az*bz) + ay*by) + ax*bx` is < 0 the current one is negated and written back
/// compressed into the current key frame; `CalcTheta(prev, cur, &m_Theta, &m_InvSin)`.
#[hook(addr = 0x4D0350, conv = thiscall, name = "CAnimBlendNode::CalcDeltasCompressed", expect = "83 EC 30 57 8B F9", this = "CAnimBlendNode")]
pub fn calc_deltas_compressed(this: GPtr<u8>) {
    let s = seq(this);
    if sequence::flags(s) & FLAG_ROT == 0 {
        return;
    }
    let pf = ckf(s, kf_prev(this));
    let cf = ckf(s, kf_curr(this));
    let mut a = [0f32; 4];
    let mut b = [0f32; 4];
    uncompress_quat(GPtr::from_raw(a.as_mut_ptr()), pf);
    uncompress_quat(GPtr::from_raw(b.as_mut_ptr()), cf);
    let dot = x(a[3])
        .fmul_m(b[3])
        .fadd(x(a[2]).fmul_m(b[2]))
        .fadd(x(a[1]).fmul_m(b[1]))
        .fadd(x(a[0]).fmul_m(b[0]));
    if dot.fcom(X87::zero()) == Cmp::Less {
        for v in b.iter_mut() {
            *v = x(*v).fchs().to_f32();
        }
        compress_quat(cf, GPtr::from_raw(b.as_mut_ptr()));
    }
    call_calc_theta(
        GPtr::from_raw(a.as_mut_ptr()),
        GPtr::from_raw(b.as_mut_ptr()),
        this.cast(),
        this.byte_add(4).cast(),
    );
}

/// `CAnimBlendNode::SetupKeyFrameCompressed()` (0x4D0650): current = 1 (0 for a single frame, with
/// remaining time 0), previous = 0, remaining = frame 1's time; `CalcDeltasCompressed`. AL = frames > 0.
#[hook(addr = 0x4D0650, conv = thiscall, name = "CAnimBlendNode::SetupKeyFrameCompressed", expect = "51 8B 41 10 66 83 78 06 01", this = "CAnimBlendNode")]
pub fn setup_key_frame_compressed(this: GPtr<u8>) -> u8 {
    let s = seq(this);
    if sequence::num_frames(s) < 1 {
        return 0;
    }
    wr::<i16>(this, 8, 1);
    wr::<i16>(this, 0xA, 0);
    if sequence::num_frames(s) == 1 {
        wr::<i16>(this, 8, 0);
        wr::<u32>(this, 0xC, 0);
    } else {
        let t = X87::from_i32(i32::from(rd::<i16>(ckf(s, 1), 8))).fmul_m(k_inv60());
        wr::<f32>(this, 0xC, t.to_f32());
    }
    call_calc_deltas_compressed(this);
    1
}

/// The `Update*` time step: while playing `rem -= timeStep`, at <= 0 `next()`; returns the looped flag.
fn step(this: GPtr<u8>, next: fn(GPtr<u8>) -> u8) -> u8 {
    let a = assoc(this);
    if rd::<u8>(a, 0x2E) & 1 == 0 {
        return 0;
    }
    let r = x(rem(this)).fsub_m(rd::<f32>(a, 0x28));
    wr::<f32>(this, 0xC, r.to_f32());
    if le_zero(r) { next(this) } else { 0 }
}

/// `rot = b * Slerp(prev, cur, m_Theta, m_InvSin, t)`.
fn slerp_scaled(this: GPtr<u8>, rot: GPtr<f32>, p: &mut [u32; 4], c: &mut [u32; 4], t: f32, b: f32) {
    quat_slerp(
        rot,
        GPtr::from_raw(p.as_mut_ptr()).cast(),
        GPtr::from_raw(c.as_mut_ptr()).cast(),
        rd::<f32>(this, 0),
        rd::<f32>(this, 4),
        t,
    );
    for k in 0..4 {
        let q = rot.byte_add(4 * k);
        q.write(x(b).fmul_m(q.read()).to_f32());
    }
}

/// `CAnimBlendNode::Update(CVector& trans, CQuaternion& rot, float weight)` (0x4D06C0): advances the
/// key frames (`NextKeyFrame` when the remaining time runs out while playing); with blend > 0 the
/// interpolated translation and `Slerp` rotation, both times the blend. AL = looped.
#[hook(addr = 0x4D06C0, conv = thiscall, name = "CAnimBlendNode::Update", expect = "83 EC 24 33 C0", this = "CAnimBlendNode")]
pub fn update(this: GPtr<u8>, trans: GPtr<f32>, rot: GPtr<f32>, weight: f32) -> u8 {
    zero3(trans);
    for k in 0..4 {
        rot.byte_add(4 * k).write(0.0);
    }
    let looped = step(this, call_next_key_frame);
    let a = assoc(this);
    let mut b = rd::<f32>(a, 0x18);
    if rd::<u8>(a, 0x2E) & 0x10 == 0 {
        b = x(b).fmul_m(weight).to_f32();
    }
    if !gt_zero(x(b)) {
        return looped;
    }
    let s = seq(this);
    let pf = ukf(s, kf_prev(this));
    let cf = ukf(s, kf_curr(this));
    let ct = x(rd::<f32>(cf, 0x10));
    let t = if ct.fcom(X87::zero()) == Cmp::Equal { 0.0 } else { ct.fsub_m(rem(this)).fdiv(ct).to_f32() };
    if sequence::flags(s) & FLAG_TRANS != 0 {
        lerp_trans(trans, vec3(pf, 0x14), vec3(cf, 0x14), x(t), x(b), false);
    }
    if sequence::flags(seq(this)) & FLAG_ROT != 0 {
        // x copied raw, y/z/w through fld/fstp (a signalling NaN is quieted)
        let q = |p: GPtr<u8>, k: isize| {
            if k == 0 { rd::<u32>(p, 0) } else { x(rd::<f32>(p, 4 * k)).to_f32().to_bits() }
        };
        let mut c = [q(cf, 0), q(cf, 1), q(cf, 2), q(cf, 3)];
        let mut p = [q(pf, 0), q(pf, 1), q(pf, 2), q(pf, 3)];
        slerp_scaled(this, rot, &mut p, &mut c, t, b);
    }
    looped
}

/// `CAnimBlendNode::UpdateCompressed(CVector& trans, CQuaternion& rot, float weight)` (0x4D08D0): as
/// `Update` on compressed key frames (`NextKeyFrameCompressed`, decompressed translation and quats).
#[hook(addr = 0x4D08D0, conv = thiscall, name = "CAnimBlendNode::UpdateCompressed", expect = "83 EC 34 53 8B 5C 24 3C", this = "CAnimBlendNode")]
pub fn update_compressed(this: GPtr<u8>, trans: GPtr<f32>, rot: GPtr<f32>, weight: f32) -> u8 {
    zero3(trans);
    for k in 0..4 {
        rot.byte_add(4 * k).write(0.0);
    }
    let looped = step(this, call_next_key_frame_compressed);
    let bb = blend(this, weight);
    let b = bb.to_f32();
    if !gt_zero(bb) {
        return looped;
    }
    let s = seq(this);
    let pf = ckf(s, kf_prev(this));
    let cf = ckf(s, kf_curr(this));
    let t = compressed_fraction(this, cf);
    if sequence::flags(s) & FLAG_TRANS != 0 {
        let mut pa = [0f32; 3];
        let mut pc = [0f32; 3];
        uncompress_vec(GPtr::from_raw(pa.as_mut_ptr()), pf.byte_add(0xA));
        uncompress_vec(GPtr::from_raw(pc.as_mut_ptr()), cf.byte_add(0xA));
        lerp_trans(trans, pa, pc, x(t), x(b), false);
    }
    if sequence::flags(seq(this)) & FLAG_ROT != 0 {
        let mut p = [0u32; 4];
        let mut c = [0u32; 4];
        uncompress_quat(GPtr::from_raw(p.as_mut_ptr()).cast(), pf);
        uncompress_quat(GPtr::from_raw(c.as_mut_ptr()).cast(), cf);
        slerp_scaled(this, rot, &mut p, &mut c, t, b);
    }
    looped
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_NEXT_KEY_FRAME_NO_CALC,
    HOOK_NEXT_KEY_FRAME,
    HOOK_NEXT_KEY_FRAME_COMPRESSED,
    HOOK_GET_CURRENT_TRANSLATION,
    HOOK_GET_END_TRANSLATION,
    HOOK_GET_CURRENT_TRANSLATION_COMPRESSED,
    HOOK_GET_END_TRANSLATION_COMPRESSED,
    HOOK_CALC_THETA,
    HOOK_UPDATE_TIME,
    HOOK_CALC_DELTAS,
    HOOK_FIND_KEY_FRAME,
    HOOK_CALC_DELTAS_COMPRESSED,
    HOOK_SETUP_KEY_FRAME_COMPRESSED,
    HOOK_UPDATE,
    HOOK_UPDATE_COMPRESSED,
];
