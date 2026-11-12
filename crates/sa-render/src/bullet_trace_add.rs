//! `CBulletTraces::AddTrace(from, to, radius, disappearTime, alpha)` (0x723750, batch `samp2-fx-1`) with the
//! inlined `PlayFrontEndSoundForTrace`. Ported from the 1.0 US bytes (gta-reversed's version computes the
//! lifetime from the trace index; the original uses the number of existing traces). Unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, x, xi};

orig! {
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volume, float speed)` (0x506EA0, thiscall)
    fn audio_report_frontend_event(ae: GPtr<u8>, event: u32, volume: f32, speed: f32) = thiscall 0x506EA0 => unported;
}

/// `CBulletTraces::aTraces` (16 x 0x2C at 0xC7C748): start +0, end +0xC, exists +0x18, creation time +0x1C,
/// lifetime +0x20, radius +0x24, alpha +0x28.
const TRACES: usize = 0xC7_C748;
/// `TheCamera.m_mCameraMatrix` rows: right (0xB6F99C), forward (0xB6F9AC), up (0xB6F9BC).
const CAM_RIGHT: usize = 0xB6_F99C;
const CAM_FWD: usize = 0xB6_F9AC;
const CAM_UP: usize = 0xB6_F9BC;

fn cam_pos() -> [f32; 3] {
    let m = g::<u32>(0xB6_F03C).read();
    let p = if m != 0 { m as usize + 0x30 } else { 0xB6_F02C };
    g::<[f32; 3]>(p).read()
}

fn row(a: usize) -> [f32; 3] {
    g::<[f32; 3]>(a).read()
}

/// `fild` of an unsigned dword (2^32 added when the signed value is negative).
fn u32_x(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 { f.fadd_m(k(0x4F80_0000)) } else { f }
}

/// `CBulletTraces::AddTrace(const CVector& from, const CVector& to, float radius, uint32 disappearTime,
/// uint8 alpha)` (0x723750, cdecl).
///
/// Lifetime from the number of existing traces n: `n >= 10`: `ftol(time * 0.25)`, `n >= 5`: `ftol(time *
/// 0.5)` (time as unsigned), else `time`. The first free slot (if any) gets from, to, the current time,
/// alpha, exists, radius, lifetime. Then the fly-by sound: both points relative to the camera position
/// (`[0xB6F03C] + 0x30`, else 0xB6F02C) projected on the camera matrix rows; only when `toY * fromY < 0`
/// (fromY kept at 80 bits): `t = |fromY| / (|toY| + |fromY|)`, the crossing point `p = from + (to - from) *
/// t` in the x/z plane, `d = sqrt(pz² + px²)` (stored over the `to` argument slot, stack only); for `d <
/// 2`: volume `20 * log10(1 - d/2)` (-100 when that is 0), event 0x1E / 0x1D (`p.x < 0`) or 0x1C / 0x1B,
/// the first of each pair when `fromY > 0`, `ReportFrontendAudioEvent(event, volume, 1.0)`.
#[hook(addr = 0x723750, conv = cdecl, name = "CBulletTraces::AddTrace-", expect = "83 EC 1C 33 C9",
       unverified = "samp2-fx-1", route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "weapon-combat",
       note = "fyl2x volume; writes its own `to` argument slot")]
pub fn add_trace(from: GPtr<f32>, to: GPtr<f32>, radius: u32, time: u32, alpha: u32) {
    let exists = |i: usize| g::<u8>(TRACES + i * 0x2C + 0x18).read() != 0;
    let n = (0..16).filter(|&i| exists(i)).count();
    let life = if n >= 10 {
        ftol32(u32_x(time).fmul_m(k(0x3E80_0000))) as u32
    } else if n >= 5 {
        ftol32(u32_x(time).fmul_m(0.5)) as u32
    } else {
        time
    };
    if let Some(i) = (0..16).find(|&i| !exists(i)) {
        let t = TRACES + i * 0x2C;
        g::<[f32; 3]>(t).write(from.cast::<[f32; 3]>().read());
        g::<[f32; 3]>(t + 0xC).write(to.cast::<[f32; 3]>().read());
        g::<u32>(t + 0x1C).write(sa_core::timer::TIME_MS.get());
        g::<u8>(t + 0x28).write(alpha as u8);
        g::<u8>(t + 0x18).write(1);
        g::<u32>(t + 0x24).write(radius);
        g::<u32>(t + 0x20).write(life);
    }
    let f: [f32; 3] = from.cast::<[f32; 3]>().read();
    let to_: [f32; 3] = to.cast::<[f32; 3]>().read();
    let rel = |p: [f32; 3]| {
        let c = cam_pos();
        (x(p[0]).fsub_m(c[0]), x(p[1]).fsub_m(c[1]), x(p[2]).fsub_m(c[2]))
    };
    let (r, fw, up) = (row(CAM_RIGHT), row(CAM_FWD), row(CAM_UP));
    let (dx, dy, dz) = rel(f);
    let from_x = dz.fmul_m(r[2]).fadd(dy.fmul_m(r[1])).fadd(dx.fmul_m(r[0])).to_f32();
    let (dx, dy, dz) = rel(f);
    let from_z = x(up[0]).fmul(dx).fadd(x(up[2]).fmul(dz)).fadd(x(up[1]).fmul(dy)).to_f32();
    let (dx, dy, dz) = rel(f);
    let from_y = dx.fmul_m(fw[0]).fadd(x(fw[2]).fmul(dz)).fadd(x(fw[1]).fmul(dy));
    let (dx, dy, dz) = rel(to_);
    let to_x = dy.fmul_m(r[1]).fadd(dz.fmul_m(r[2])).fadd(dx.fmul_m(r[0])).to_f32();
    let (dx, dy, dz) = rel(to_);
    let to_z = x(up[2]).fmul(dz).fadd(x(up[1]).fmul(dy)).fadd(x(up[0]).fmul(dx)).to_f32();
    let (dx, dy, dz) = rel(to_);
    let to_y = x(fw[2]).fmul(dz).fadd(x(fw[1]).fmul(dy)).fadd(x(fw[0]).fmul(dx)).to_f32();
    if x(to_y).fmul(from_y).fcom(x(0.0)).test_parity(5) {
        return;
    }
    let abs = |v: X87| if !v.fcom(x(0.0)).test_parity(5) { v.fchs() } else { v };
    let a = abs(from_y);
    let sum = abs(x(to_y)).fadd(abs(from_y));
    let t = a.fdiv(sum);
    let ddx = x(to_x).fsub_m(from_x);
    let dzt = x(x(to_z).fsub_m(from_z).to_f32()).fmul(t).to_f32();
    let px = ddx.fmul(t).fadd_m(from_x);
    let pz = x(dzt).fadd_m(from_z);
    let d = pz.fmul(pz).fadd(px.fmul(px)).fsqrt();
    let d32 = d.to_f32();
    if d.fcom(x(k(0x4000_0000))).test_parity(5) {
        return;
    }
    let neg_x = !px.fcom(x(0.0)).test_parity(5);
    let v = x(d32).fmul_m(0.5).fsubr_m(1.0);
    let volume = if v.fcom(x(0.0)).test_parity(0x44) {
        X87::lg2().fyl2x(v).fmul_m(k(0x41A0_0000)).to_f32()
    } else {
        k(0xC2C8_0000)
    };
    let above = !from_y.fcom(X87::from_f64_exact(0.0)).test_any(0x41);
    let event = match (neg_x, above) {
        (true, true) => 0x1E,
        (true, false) => 0x1D,
        (false, true) => 0x1C,
        (false, false) => 0x1B,
    };
    audio_report_frontend_event(g(0xB6_BC90), event, volume, 1.0);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_TRACE];
