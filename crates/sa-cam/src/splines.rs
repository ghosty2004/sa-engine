//! PORT-CAM-2: the cut-scene camera path splines (`CCamPathSplines`): the two evaluators 0x5B2090
//! (vector) / 0x5B2330 (scalar), `CCamera::LoadPathSplines` and `CCam::Process_FlyBy`. Ported from the
//! 1.0 US bytes; `docs/re/specs/CAM/_unattributed-B.md`, `CCamera.md` and `CCam.md` as reading aids
//! (gta-reversed does not reverse any of them).
//!
//! Spline buffers (`TheCamera` +0x960.. 0xB6F988, four pointers, filled by `LoadPathSplines`): float 0 is
//! the point count `n`; scalar splines (0: FOV, 1: roll) use 4 floats per point, vector splines (2:
//! position, 3: look-at) 10 floats per point (`time, p.xyz, ?.xyz, ?.xyz` around the index the callers
//! keep in the statics below). Times are in seconds, compared in milliseconds (`* 1000.0f`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_STEP_NON_CLIPPED;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, ftol32, g, jp, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `Process_FlyBy`'s FOV static (scalar spline 0 output; copied to the cam's FOV +0xB4)
    pub static FLYBY_FOV @ 0xBC4074: f32 = 0.0;
    /// a second copy of the first FOV key, written once by the first `Process_FlyBy` frame, never read there
    pub static FLYBY_FOV2 @ 0xBC4078: f32 = 0.0;
    /// `Process_FlyBy`'s look-at spline (3) index
    pub static FLYBY_IDX_LOOK @ 0xBC407C: i32 = 0;
    /// its position spline (2) index
    pub static FLYBY_IDX_POS @ 0xBC4080: i32 = 0;
    /// its FOV spline (0) index
    pub static FLYBY_IDX_FOV @ 0xBC4084: i32 = 0;
    /// its roll spline (1) index
    pub static FLYBY_IDX_ROLL @ 0xBC4088: i32 = 0;
    /// `.data` f32 (initial 32.0) both evaluators compare the segment duration (ms) against before
    /// stepping the index once more; no writer known
    pub static SPLINE_STEP_DURATION @ 0x8D0F80: f32 = 32.0;
}

/// 1000.0 (0x858C4C): seconds to milliseconds.
const K_1000: u32 = 0x447A_0000;
/// 1.0 (0x858624), 0.0 (0x858B50), 3.0 (0x858B3C)
const K_ONE: u32 = 0x3F80_0000;
const K_ZERO: u32 = 0x0000_0000;
const K_THREE: u32 = 0x4040_0000;
/// 2^32 (0x858C54): the unsigned-int fix-up after `fild`
const K_2P32: u32 = 0x4F80_0000;
/// 0.02 (0x858B38), 0.0174533 (0x8595EC), pi / 2 (0x858FE4)
const K_0_02: u32 = 0x3CA3_D70A;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_HALF_PI: u32 = 0x3FC9_0FDB;

/// `TheCamera.m_vecCutSceneOffset`-area vector (TheCamera +0x884) the vector evaluator adds.
const CUTSCENE_OFFSET: usize = 0xB6F8AC;
/// `TheCamera` +0x1D (cut scene finished) / +0x33 (fly-by started) bytes `Process_FlyBy` tests.
const CAMERA_CUTSCENE_FINISHED: usize = 0xB6F045;
const CAMERA_FLYBY_STARTED: usize = 0xB6F05B;
/// `TheCamera` +0x140 (position along the cut scene, 0..1).
const CAMERA_POSITION_ALONG_SPLINE: usize = 0xB6F168;
/// `TheCamera` +0x960..: the four spline buffers (0 FOV, 1 roll, 2 position, 3 look-at).
const SPLINE_FOV: usize = 0xB6F988;
const SPLINE_ROLL: usize = 0xB6F98C;
const SPLINE_POS: usize = 0xB6F990;
const SPLINE_LOOK: usize = 0xB6F994;
/// `"%d"` (0x859508) and `", \t"` (0x868B10)
const FMT_INT: usize = 0x859508;
const DELIMS: usize = 0x868B10;

orig! {
    /// `FUN_005b2090` (vector spline evaluator)
    pub fn spline_vector_call(out: GPtr<u8>, spline: GPtr<u8>, time: f32, index: GPtr<i32>) = cdecl 0x5B2090 => spline_vector;
    /// `FUN_005b2330` (scalar spline evaluator)
    pub fn spline_scalar_call(out: GPtr<f32>, spline: GPtr<u8>, time: f32, index: GPtr<i32>) = cdecl 0x5B2330 => spline_scalar;
    /// `CCamera::DeleteCutSceneCamDataMemory()` (0x5B24A0)
    pub fn delete_cut_scene_cam_data_memory_call(this: GPtr<u8>) = thiscall 0x5B24A0 => crate::camctl::delete_cut_scene_cam_data_memory;
    /// `CFileLoader::LoadLine(FILESTREAM)` (0x536F80)
    pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => sa_stream::fileloader::load_line_file;
    /// CRT `sscanf(s, "%d", int*)` (0x8220AD)
    pub fn sscanf_int(s: GPtr<u8>, fmt: GPtr<u8>, out: GPtr<i32>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `strtok` (0x82244B)
    pub fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `atof` (0x823CEE), result in ST0
    pub fn atof(s: GPtr<u8>) -> f64 = cdecl 0x823CEE => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
}

/// `spline[i]` (`[esi+i*4]`, 32-bit index arithmetic).
fn el(spline: GPtr<u8>, i: i32) -> f32 {
    rd::<f32>(spline, (i as isize).wrapping_mul(4))
}

/// An f32 copied through `fld dword` / `fstp dword` (a signalling NaN comes out quiet).
fn q(v: f32) -> f32 {
    x(v).to_f32()
}

/// The clamp both evaluators apply to the segment fraction in ST0: `> 1 -> 1.0`, else `< 0 -> 0.0`
/// (NaN kept), then `time > end -> 1.0`.
fn clamp_fraction(f: &mut Fpu, time: f32, end: f32) {
    f.fcom_m(k(K_ONE));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(k(K_ONE));
    } else {
        f.fcom_m(k(K_ZERO));
        if !f.jp(0x05) {
            f.fstp_st(0);
            f.fld(k(K_ZERO));
        }
    }
    f.fld(time);
    f.fcomp_m(end);
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(k(K_ONE));
    }
}

/// Vector spline evaluator (0x5B2090, cdecl(CVector* out, const float* spline, float timeMs, int32*
/// index)). With `n = _ftol2(spline[0])`, `i = *index`:
/// - `segDur = (spline[i] - spline[i-10]) * 1000` stays on the stack (old index);
///   `end = (f32)(spline[10n-9] * 1000)`;
/// - if `time < end`: `(u32)(i-1)/10 > n` sets `*index = 10n-9` (and blocks the next step); else when
///   `segDur <= SPLINE_STEP_DURATION`, `*index += 10` (again limited to `10n-9`);
/// - `u = (time - spline[i'-10]*1000) / segDur` with the new index, clamped (see [`clamp_fraction`]);
/// - the 12 floats around `i'` are copied to locals (`fld`/`fstp`): P = `i'-9..`, A = `i'-3..`,
///   B = `i'+1..`, C = `i'+4..`; when A == P on all three axes (`test ah,0x44`) the result is the
///   linear `(B - P)*u + P`, else the Bezier `((c*C + d*A)*3 + u^3*B) + (1-u)^3*P` with
///   `c = (f32)((1-u)*u^2)`, `d = u * (f32)((1-u)^2)` (unrounded), `u^3` / `(1-u)^3` stored as f32;
/// - then `out += TheCamera +0x884` (memory operand order: `offset + out`).
///
/// The original reuses its argument slots (spline, time, index) for `end`, `(1-u)^3` and `(1-u)*u^2`;
/// the caller does not read them back.
#[hook(addr = 0x5B2090, conv = cdecl, name = "FUN_005b2090", expect = "83 EC 34 56 8B 74 24 40")]
pub fn spline_vector(out: GPtr<u8>, spline: GPtr<u8>, time: f32, index: GPtr<i32>) {
    let n = ftol32(x(el(spline, 0)));
    let idx = index.read();
    let mut f = Fpu::new();
    f.fld(el(spline, idx));
    f.fsub_m(el(spline, idx.wrapping_sub(10)));
    f.fmul_m(k(K_1000));
    let end = x(el(spline, n.wrapping_mul(10).wrapping_sub(9)))
        .fmul_m(k(K_1000))
        .to_f32();
    let last = n.wrapping_mul(10).wrapping_sub(9);
    if !jp(x(time), x(end), 0x05) {
        let mut step = true;
        if (idx.wrapping_sub(1) as u32) / 10 > n as u32 {
            index.write(last);
            step = false;
        }
        f.fcom_m(SPLINE_STEP_DURATION.get());
        if !f.jp(0x41) && step {
            let i = index.read().wrapping_add(10);
            index.write(i);
            if (i.wrapping_sub(1) as u32) / 10 > n as u32 {
                index.write(last);
            }
        }
    }
    let i = index.read();
    f.fld(el(spline, i.wrapping_sub(10)));
    f.fmul_m(k(K_1000));
    f.fsubr_m(time);
    f.fdivrp(1);
    clamp_fraction(&mut f, time, end);

    let p = [
        q(el(spline, i.wrapping_sub(9))),
        q(el(spline, i.wrapping_sub(8))),
        q(el(spline, i.wrapping_sub(7))),
    ];
    let b = [
        q(el(spline, i.wrapping_add(1))),
        q(el(spline, i.wrapping_add(2))),
        q(el(spline, i.wrapping_add(3))),
    ];
    // `fst` of A.x: the value stays on the stack for the first compare
    f.fld(el(spline, i.wrapping_sub(3)));
    let a0 = f.fst();
    let a = [
        a0,
        q(el(spline, i.wrapping_sub(2))),
        q(el(spline, i.wrapping_sub(1))),
    ];
    let c = [
        q(el(spline, i.wrapping_add(4))),
        q(el(spline, i.wrapping_add(5))),
        q(el(spline, i.wrapping_add(6))),
    ];
    f.fcomp_m(p[0]);
    let cubic = f.jp(0x44) || jp(x(a[1]), x(p[1]), 0x44) || jp(x(a[2]), x(p[2]), 0x44);
    let o = |j: usize| out.byte_add(j as isize * 4).cast::<f32>();
    if cubic {
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(0);
        f.fmul_st(2);
        let t3 = f.fstp();
        f.fld(k(K_ONE));
        f.fsub_st(2);
        f.fld_st(0);
        f.fmul_st(1);
        let s2 = f.fst();
        f.fmul_st(1);
        let s3 = f.fstp();
        f.fmul_st(1);
        let st2 = f.fstp();
        f.fstp_st(0);
        f.fmul_m(s2); // d = u * (1-u)^2, kept on the stack
        for j in 0..2 {
            f.fld(st2);
            f.fmul_m(c[j]);
            f.fld_st(1);
            f.fmul_m(a[j]);
            f.faddp(1);
            f.fmul_m(k(K_THREE));
            f.fld(t3);
            f.fmul_m(b[j]);
            f.faddp(1);
            f.fld(s3);
            f.fmul_m(p[j]);
            f.faddp(1);
            o(j).write(f.fstp());
        }
        f.fmul_m(a[2]);
        f.fld(st2);
        f.fmul_m(c[2]);
        f.faddp(1);
        f.fmul_m(k(K_THREE));
        f.fld(t3);
        f.fmul_m(b[2]);
        f.faddp(1);
        f.fld(s3);
        f.fmul_m(p[2]);
        f.faddp(1);
        o(2).write(f.fstp());
    } else {
        for j in 0..3 {
            f.fld(b[j]);
            f.fsub_m(p[j]);
            f.fmul_st(1);
            f.fadd_m(p[j]);
            o(j).write(f.fstp());
        }
        f.fstp_st(0);
    }
    for j in 0..3 {
        let off = g::<f32>(CUTSCENE_OFFSET + j * 4).read();
        o(j).write(x(off).fadd_m(o(j).read()).to_f32());
    }
}

/// Scalar spline evaluator (0x5B2330, cdecl(float* out, const float* spline, float timeMs, int32*
/// index)), 4 floats per point: as [`spline_vector`] with step 4 and `end = spline[4n-3]*1000`, except
/// that `segDur` is popped after the index update and recomputed with the new index; then with
/// P = `spline[i-3]`, A = `spline[i-1]`, B = `spline[i+1]`, C = `spline[i+2]`: A == P gives
/// `(B - P)*u + P`, else `((B*u + 3*C*(1-u))*u + 3*A*(1-u)^2)*u + (1-u)^3*P`, all on the stack. No offset.
#[hook(addr = 0x5B2330, conv = cdecl, name = "FUN_005b2330", expect = "53 56 8B 74 24 10")]
pub fn spline_scalar(out: GPtr<f32>, spline: GPtr<u8>, time: f32, index: GPtr<i32>) {
    let n = ftol32(x(el(spline, 0)));
    let idx = index.read();
    let mut f = Fpu::new();
    f.fld(el(spline, idx));
    f.fsub_m(el(spline, idx.wrapping_sub(4)));
    f.fmul_m(k(K_1000));
    let last = n.wrapping_mul(4).wrapping_sub(3);
    let end = x(el(spline, last)).fmul_m(k(K_1000)).to_f32();
    if jp(x(time), x(end), 0x05) {
        f.fstp_st(0);
    } else {
        let mut step = true;
        if (idx.wrapping_sub(1) as u32) >> 2 > n as u32 {
            index.write(last);
            step = false;
        }
        f.fcomp_m(SPLINE_STEP_DURATION.get());
        if !f.jp(0x41) && step {
            let i = index.read().wrapping_add(4);
            index.write(i);
            if (i.wrapping_sub(1) as u32) >> 2 > n as u32 {
                index.write(last);
            }
        }
    }
    let i = index.read();
    f.fld(el(spline, i.wrapping_sub(4)));
    f.fmul_m(k(K_1000));
    f.fsubr_m(time);
    f.fld(el(spline, i));
    f.fsub_m(el(spline, i.wrapping_sub(4)));
    f.fmul_m(k(K_1000));
    f.fdivp(1);
    clamp_fraction(&mut f, time, end);

    f.fld(k(K_ONE));
    f.fsub_st(1); // 1-u
    f.fld_st(0);
    f.fmul_st(1); // (1-u)^2
    f.fld_st(0);
    f.fmul_st(2); // (1-u)^3
    f.fld(el(spline, i.wrapping_sub(1))); // A
    f.fld(el(spline, i.wrapping_add(2))); // C
    f.fld_st(1);
    f.fcomp_m(el(spline, i.wrapping_sub(3)));
    if f.jp(0x44) {
        // stack: C, A, (1-u)^3, (1-u)^2, 1-u, u
        f.fld(el(spline, i.wrapping_add(1)));
        f.fmul_st(6);
        f.fxch(1);
        f.fmul_st(5);
        f.fmul_m(k(K_THREE));
        f.faddp(1);
        f.fmul_st(5);
        f.fxch(1);
        f.fmul_st(3);
        f.fmul_m(k(K_THREE));
        f.faddp(1);
        f.fmul_st(4);
        f.fxch(1);
        f.fmul_m(el(spline, i.wrapping_sub(3)));
        f.faddp(1);
        out.write(f.fstp());
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
    } else {
        for _ in 0..5 {
            f.fstp_st(0);
        }
        f.fld(el(spline, i.wrapping_add(1)));
        f.fsub_m(el(spline, i.wrapping_sub(3)));
        f.fmul_st(1);
        f.fadd_m(el(spline, i.wrapping_sub(3)));
        out.write(f.fstp());
        f.fstp_st(0);
    }
}

/// `CCamera::LoadPathSplines(FILESTREAM)` (0x5B24D0, thiscall, `ret 4`): frees the old buffers
/// (`DeleteCutSceneCamDataMemory`), then reads lines with `CFileLoader::LoadLine`, skipping those starting
/// with `#` or empty. Outside a spline (`expect header` flag, initially set) a line is a point count
/// (`sscanf "%d"`): the next of the four buffers (+0x960..; more than four headers end the load) gets
/// `operator new(count*16+4)` (splines 0 and 1) or `count*40+4` (2 and 3), float 0 = `(float)count`.
/// While the count is not zero each line decrements it and every `strtok(", \t")` token is stored as
/// `(f32)atof(token)` at the running write pointer: the token count is not checked against the
/// allocation (original bug, kept). A line starting with `;` (count zero) arms the next header.
#[hook(addr = 0x5B24D0, conv = thiscall, name = "CCamera::LoadPathSplines", expect = "51 53 55 56 8B F1", this = "CCamera")]
pub fn load_path_splines(this: GPtr<u8>, file: GPtr<u8>) {
    delete_cut_scene_cam_data_memory_call(this);
    let mut spline_no: i32 = -1;
    let mut expect_header = true;
    let mut count: i32 = 0;
    let count_ptr = GPtr::<i32>::from_raw(&mut count);
    let mut line = load_line(file);
    if line.is_null() {
        return;
    }
    let mut slot = this.byte_add(0x95C);
    // The original's write pointer starts as the file argument (ESI); it is only used once a header set
    // the count, which also sets the pointer.
    let mut wp: GPtr<u8> = GPtr::NULL;
    loop {
        let c = line.read();
        if c != b'#' && c != 0 {
            if count_ptr.read() != 0 {
                count_ptr.write(count_ptr.read().wrapping_sub(1));
                let mut tok = strtok(line, GPtr::from_addr(DELIMS));
                while !tok.is_null() {
                    wp.cast::<f32>().write(atof(tok) as f32);
                    wp = wp.byte_add(4);
                    tok = strtok(GPtr::NULL, GPtr::from_addr(DELIMS));
                }
            } else if expect_header {
                spline_no += 1;
                slot = slot.byte_add(4);
                if spline_no > 3 {
                    return;
                }
                sscanf_int(line, GPtr::from_addr(FMT_INT), count_ptr);
                let n = count_ptr.read() as u32;
                let size = if spline_no == 0 || spline_no == 1 {
                    n.wrapping_shl(4).wrapping_add(4)
                } else {
                    n.wrapping_mul(40).wrapping_add(4)
                };
                let p = operator_new(size);
                slot.cast::<GPtr<u8>>().write(p);
                p.cast::<f32>().write(X87::from_i32(count_ptr.read()).to_f32());
                wp = slot.cast::<GPtr<u8>>().read().byte_add(4);
                expect_header = false;
            } else if c == b';' {
                expect_header = true;
            }
        }
        line = load_line(file);
        if line.is_null() {
            return;
        }
    }
}

/// The index advance loop `Process_FlyBy` inlines before each evaluator call: while
/// `(spline[i] - spline[1]) * 1000 <= t` (`test ah,0x41; jnp`), `i += step` and store it.
fn advance(spline: GPtr<u8>, idx: &sa_core::mem::Global<i32>, mut i: i32, step: i32, t: f32) {
    let ge = |i: i32| {
        jp(
            x(el(spline, i)).fsub_m(el(spline, 1)).fmul_m(k(K_1000)),
            x(t),
            0x41,
        )
    };
    if ge(i) {
        return;
    }
    loop {
        i = i.wrapping_add(step);
        idx.set(i);
        if ge(i) {
            return;
        }
    }
}

/// `up.x = cos(a)`, `up.z = sin(a)` with `a = roll * 0.0174533 + pi/2` (on the stack; FCOS / FSIN as
/// sa-math's f64 model).
fn roll_up(this: GPtr<u8>, roll: f32) {
    let a = x(roll).fmul_m(k(K_DEG2RAD)).fadd_m(k(K_HALF_PI));
    wr::<f32>(this, 0x1B4, a.fcos().to_f32());
    wr::<f32>(this, 0x1BC, a.fsin().to_f32());
}

fn spline_at(addr: usize) -> GPtr<u8> {
    g::<GPtr<u8>>(addr).read()
}

/// `CCam::Process_FlyBy(const CVector&, float, float, float)` (0x5B25F0, thiscall, `ret 0x10`; the
/// arguments are unused). Nothing when the cut scene finished (TheCamera +0x1D). Up = (0, 0, 1). The
/// first frame (TheCamera +0x33 clear) zeroes the timer +0x90, sets the finish time +0x10 =
/// `_ftol2(pos[10n-9] * 1000)`, the indices (roll / FOV 5, position / look-at 11), the flag, and both
/// FOV statics from `fov[2]`; later frames add `TimeStepNonClipped * 0.02 * 1000` to +0x90. With
/// `t = (f32)(u32)_ftol2(+0x90)` against `(u32)+0x10` (`fild` + 2^32 fix-ups): before the end,
/// TheCamera +0x140 = `t / finish`, each spline's index advances ([`advance`]) and the evaluators give
/// the source (+0x19C), the look-at point (into +0x190), the roll and the FOV static; at or after the
/// end the indices are set to the last segment, the same four calls run, +0x140 = 1.0 and the indices
/// are zeroed. Then +0x148 = look-at point, front (+0x190) = normalised `look-at - source`, up =
/// normalised `front x (up x front)` (`CrossProduct` into a stack temporary), FOV +0xB4 = the static.
#[hook(addr = 0x5B25F0, conv = thiscall, name = "CCam::Process_FlyBy", expect = "83 EC 24 A0 45 F0 B6 00", this = "CCam")]
pub fn process_fly_by(
    this: GPtr<u8>,
    _target: GPtr<u8>,
    _orientation: f32,
    _speed_var: f32,
    _target_speed_var: f32,
) {
    if g::<u8>(CAMERA_CUTSCENE_FINISHED).read() != 0 {
        return;
    }
    let up = this.byte_add(0x1B4);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, 0x3F80_0000);
    let pos_idx = if g::<u8>(CAMERA_FLYBY_STARTED).read() == 0 {
        wr::<u32>(this, 0x90, 0);
        let sp = spline_at(SPLINE_POS);
        let n = ftol32(x(el(sp, 0)));
        wr::<i32>(
            this,
            0x10,
            ftol32(x(el(sp, n.wrapping_mul(10).wrapping_sub(9))).fmul_m(k(K_1000))),
        );
        FLYBY_IDX_ROLL.set(5);
        FLYBY_IDX_FOV.set(5);
        let fov = spline_at(SPLINE_FOV);
        g::<u8>(CAMERA_FLYBY_STARTED).write(1);
        FLYBY_IDX_POS.set(11);
        FLYBY_IDX_LOOK.set(11);
        let v = q(el(fov, 2));
        FLYBY_FOV2.set(v);
        FLYBY_FOV.set(v);
        11
    } else {
        let t = x(TIME_STEP_NON_CLIPPED.get())
            .fmul_m(k(K_0_02))
            .fmul_m(k(K_1000))
            .fadd_m(rd::<f32>(this, 0x90));
        wr::<f32>(this, 0x90, t.to_f32());
        FLYBY_IDX_POS.get()
    };
    let ti = ftol32(x(rd::<f32>(this, 0x90)));
    let mut tv = X87::from_i32(ti);
    if ti < 0 {
        tv = tv.fadd_m(k(K_2P32));
    }
    let t = tv.to_f32();
    let fi = rd::<i32>(this, 0x10);
    let mut finish = X87::from_i32(fi);
    if fi < 0 {
        finish = finish.fadd_m(k(K_2P32));
    }
    let source = this.byte_add(0x19C);
    let front = this.byte_add(0x190);
    let mut roll: f32 = 0.0;
    let roll_ptr = GPtr::<f32>::from_raw(&mut roll);
    if !jp(x(t), finish, 0x05) {
        let sp = spline_at(SPLINE_POS);
        g::<f32>(CAMERA_POSITION_ALONG_SPLINE).write(x(t).fdiv(finish).to_f32());
        advance(sp, &FLYBY_IDX_POS, pos_idx, 10, t);
        spline_vector_call(source, sp, t, FLYBY_IDX_POS.ptr());
        let sl = spline_at(SPLINE_LOOK);
        advance(sl, &FLYBY_IDX_LOOK, FLYBY_IDX_LOOK.get(), 10, t);
        spline_vector_call(front, sl, t, FLYBY_IDX_LOOK.ptr());
        let sr = spline_at(SPLINE_ROLL);
        advance(sr, &FLYBY_IDX_ROLL, FLYBY_IDX_ROLL.get(), 4, t);
        spline_scalar_call(roll_ptr, sr, t, FLYBY_IDX_ROLL.ptr());
        roll_up(this, roll_ptr.read());
        let sf = spline_at(SPLINE_FOV);
        advance(sf, &FLYBY_IDX_FOV, FLYBY_IDX_FOV.get(), 4, t);
        spline_scalar_call(FLYBY_FOV.ptr(), sf, t, FLYBY_IDX_FOV.ptr());
    } else {
        let sp = spline_at(SPLINE_POS);
        let last10 = |s: GPtr<u8>| {
            ftol32(x(el(s, 0)))
                .wrapping_sub(1)
                .wrapping_mul(10)
                .wrapping_add(1)
        };
        let last4 = |s: GPtr<u8>| ftol32(x(el(s, 0))).wrapping_mul(4).wrapping_sub(3);
        FLYBY_IDX_POS.set(last10(sp));
        FLYBY_IDX_LOOK.set(last10(spline_at(SPLINE_LOOK)));
        FLYBY_IDX_ROLL.set(last4(spline_at(SPLINE_ROLL)));
        FLYBY_IDX_FOV.set(last4(spline_at(SPLINE_FOV)));
        spline_vector_call(source, sp, t, FLYBY_IDX_POS.ptr());
        spline_vector_call(front, spline_at(SPLINE_LOOK), t, FLYBY_IDX_LOOK.ptr());
        spline_scalar_call(roll_ptr, spline_at(SPLINE_ROLL), t, FLYBY_IDX_ROLL.ptr());
        roll_up(this, roll_ptr.read());
        spline_scalar_call(FLYBY_FOV.ptr(), spline_at(SPLINE_FOV), t, FLYBY_IDX_FOV.ptr());
        g::<u32>(CAMERA_POSITION_ALONG_SPLINE).write(0x3F80_0000);
        FLYBY_IDX_POS.set(0);
        FLYBY_IDX_LOOK.set(0);
        FLYBY_IDX_ROLL.set(0);
        FLYBY_IDX_FOV.set(0);
    }
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x148 + o, rd::<u32>(front, o));
    }
    let d = [0, 4, 8].map(|o| x(rd::<f32>(front, o)).fsub_m(rd::<f32>(source, o)).to_f32());
    for (j, v) in d.iter().enumerate() {
        wr::<f32>(front, j as isize * 4, *v);
    }
    vector_normalise(front);
    let mut tmp = [0u32; 3];
    let mut right = [0u32; 3];
    let tp = GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    let rp = GPtr::<u32>::from_raw(right.as_mut_ptr()).cast::<u8>();
    let r = cross_product(tp, up, front);
    for o in [0, 4, 8] {
        wr::<u32>(rp, o, rd::<u32>(r, o));
    }
    let r = cross_product(tp, front, rp);
    for o in [0, 4, 8] {
        wr::<u32>(up, o, rd::<u32>(r, o));
    }
    vector_normalise(up);
    wr::<u32>(this, 0xB4, FLYBY_FOV.ptr().cast::<u32>().read());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SPLINE_VECTOR,
    HOOK_SPLINE_SCALAR,
    HOOK_LOAD_PATH_SPLINES,
    HOOK_PROCESS_FLY_BY,
];
