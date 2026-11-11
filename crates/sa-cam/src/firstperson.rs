//! PORT-CAM-3 (helper A): `CCam::Process_1stPerson` (0x517EA0), the in-vehicle first-person ("bumper")
//! camera. Ported from the 1.0 US bytes (gta-reversed reverses it; its semantics are a reading aid, the
//! bytes win). Same technique as [`crate::followped`]: a frame array at the original's `B`-relative
//! offsets (B = ESP after `push ebx`; the orientation argument slot `B+0x70` holds the matrix pointer
//! later) and the [`Fpu`] machine.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::camcol::{allocate_matrix, simple_transform_update_matrix, vector_sub};
use crate::camctl::set_motion_blur_call;
use crate::cammodes::{get_water_level, multiply_3x3, ped_get_bone_position};
use crate::camproc::{cross_product, get_pad, vec_add, vec_mul_s, vector_normalise};
use crate::common::{Fpu, THE_CAMERA, ftol32, g, k, rd, wr};
use crate::followped::{copy3, d, entity_pos, vector_add_assign};

orig! {
    /// `CCam::Process_WheelCam(const CVector&, float, float, float)` (0x512110, AL); the floats as dwords
    pub fn cam_process_wheel_cam(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) -> u8 = thiscall 0x512110 => unported;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150)
    pub fn bike_calculate_lean_matrix(b: GPtr<u8>) = thiscall 0x6B7150 => unported;
    /// `CVehicle::IsUpsideDown()` (0x6D1D90, AL)
    pub fn vehicle_is_upside_down(v: GPtr<u8>) -> u8 = thiscall 0x6D1D90 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0); the float as a dword
    pub fn vector_scale_assign(v: GPtr<u8>, s: u32) -> GPtr<u8> = thiscall 0x40FEF0 => unported;
    /// `CWorld::GetIsLineOfSightClear(...)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// `CCam::m_bFirstPersonRunThisFrame`-ish byte (0xB6EC20), the upside-down rise (0xB7004C), the
/// wheel-cam timer (0xB6F18C), the last wheel-cam time (0x8CCD14) and its 3000 ms interval (0x8CCD10).
const FIRST_PERSON_RUN: usize = 0xB6EC20;
const UPSIDE_RISE: usize = 0xB7004C;
const WHEEL_TIMER: usize = 0xB6F18C;
const WHEEL_LAST: usize = 0x8CCD14;
const WHEEL_INTERVAL: usize = 0x8CCD10;
const TIME_MS: usize = 0xB7CB84;
const TIME_STEP: usize = 0xB7CB5C;
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTimeCycle::m_CurrentColours` water red / green / blue and the under-water blur tweaks (cap 10.0,
/// alpha 20).
const WATER_R: usize = 0xB7C508;
const WATER_G: usize = 0xB7C50C;
const WATER_B: usize = 0xB7C510;
const UW_COLOUR_CAP: usize = 0x8CC7A8;
const UW_ALPHA: usize = 0x8CC7A4;

const K_0: u32 = 0; // [0x858B50]
const K_0_5: u32 = 0x3F00_0000; // [0x858B8C]
const K_0_03: u32 = 0x3CF5_C28F; // [0x858B10]
const K_0_3: u32 = 0x3E99_999A; // [0x858C24]
const K_2P32: u32 = 0x4F80_0000; // [0x858C54]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_0_08: u32 = 0x3DA3_D70A; // [0x859018]
const K_0_62: u32 = 0x3F1E_B852; // [0x8631EC]

/// `CCam::Process_1stPerson(const CVector& target, float orientation, float, float)` (0x517EA0,
/// thiscall, `ret 0x10`). Order of calls, re-reads and x87 operations as in the bytes:
/// - FOV 70, 0xB6EC20 = 1; no RwObject on the target: return at once (m_bResetStatics kept);
/// - reset: beta = beta2 (+0xA8) = orientation, alpha 0 (a ped target: both betas = heading + pi/2),
///   0xB7004C = 0xB6F18C = 0; a ped target then returns (clearing m_bResetStatics);
/// - the wheel camera every 3 s (`(float)(u32)now - last >= 3000`, or a bike with the 0x614 flag / a
///   running timer) when the pad's +0xA and +0xE are 0: `Process_WheelCam` true returns (timer 50 when
///   the flag is set, else `timer -= ts` and the flag set; m_bResetStatics kept), false resets both;
/// - source = `Multiply3x3(matrix, (0, seat.y + 0.08, seat.z + 0.62)) + pos` (FOV 60; bikes use the lean
///   matrix +0x5CC, boats raise z by 0.5, a bike with a driver uses the driver's bone 4 + ts * moveSpeed
///   + 0.15 unless the line from there to `source + (front x up)` is blocked);
/// - the upside-down rise (+-0.03 per frame, 0 .. 0.5) added to z; front / up from the matrix,
///   re-orthonormalised (`up = normalise(normalise(front x up) x front)`);
/// - under water (level - 0.3 above the source): `SetMotionBlur` with the water colour scaled to at most
///   length 10 (`_ftol2` each), alpha [0x8CC7A4], type 2.
#[hook(addr = 0x517EA0, conv = thiscall, name = "CCam::Process_1stPerson", expect = "83 EC 5C 55 56 8B F1", this = "CCam")]
pub fn process_1st_person(
    this: GPtr<u8>,
    target: GPtr<u8>,
    orientation: u32,
    speed_var: u32,
    speed_var_wanted: u32,
) {
    let mut fr = [0u32; 0x1F];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    g::<u8>(FIRST_PERSON_RUN).write(1);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    wr::<u32>(this, 0xB4, 0x428C_0000);
    if rd::<u32>(t, 0x18) == 0 {
        return;
    }
    if rd::<u8>(this, 0xA) != 0 {
        wr::<u32>(this, 0xBC, orientation);
        wr::<u32>(this, 0xAC, 0);
        wr::<u32>(this, 0xA8, orientation);
        if rd::<u8>(t, 0x36) & 7 == 3 {
            f.fld(rd(t, 0x558));
            wr::<u32>(this, 0xAC, 0);
            f.fadd_m(k(K_HALF_PI));
            wr::<f32>(this, 0xBC, f.fstp());
            f.fld(rd(t, 0x558));
            f.fadd_m(k(K_HALF_PI));
            wr::<f32>(this, 0xA8, f.fstp());
        }
        g::<u32>(UPSIDE_RISE).write(0);
        g::<u32>(WHEEL_TIMER).write(0);
    }
    let veh = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(veh, 0x36) & 7 == 3 {
        wr::<u8>(this, 0xA, 0);
        return;
    }
    // 0x517F49: the wheel camera
    let now = g::<u32>(TIME_MS).read();
    f.fild(now as i32);
    if (now as i32) < 0 {
        f.fadd_m(k(K_2P32));
    }
    wr::<f32>(b, 0xC, f.fstp());
    f.fld(d(WHEEL_LAST));
    f.fcomp_m(rd(b, 0xC));
    if !f.jne(0x41) {
        g::<u32>(WHEEL_LAST).write(0);
    }
    f.fld(rd(b, 0xC));
    f.fsub_m(d(WHEEL_LAST));
    wr::<f32>(b, 0x10, f.fst());
    f.fcomp_m(d(WHEEL_INTERVAL));
    let try_wheel = !f.jp(0x5) || {
        rd::<i32>(veh, 0x590) == 9
            && (rd::<u8>(veh, 0x614) & 0x80 != 0 || {
                f.fld(d(WHEEL_TIMER));
                f.fcomp_m(k(K_0));
                !f.jne(0x41)
            })
            && {
                f.fld(rd(b, 0x10));
                f.fcomp_m(d(WHEEL_INTERVAL));
                if !f.jne(0x1) {
                    g::<u32>(WHEEL_LAST).write(rd::<u32>(b, 0xC));
                }
                true
            }
    };
    if try_wheel && rd::<i16>(get_pad(0), 0xA) == 0 && rd::<i16>(get_pad(0), 0xE) == 0 {
        if cam_process_wheel_cam(this, target, orientation, speed_var, speed_var_wanted) != 0 {
            let v = rd::<GPtr<u8>>(this, 0x21C);
            if rd::<u8>(v, 0x614) & 0x80 != 0 {
                g::<u32>(WHEEL_TIMER).write(0x4248_0000);
                return;
            }
            f.fld(d(WHEEL_TIMER));
            f.fsub_m(d(TIME_STEP));
            g::<f32>(WHEEL_TIMER).write(f.fstp());
            let v = rd::<GPtr<u8>>(this, 0x21C);
            wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 0x80);
            return;
        }
        g::<u32>(WHEEL_TIMER).write(0);
        let v = rd::<GPtr<u8>>(this, 0x21C);
        let fl = rd::<u8>(v, 0x614) & 0x7F;
        g::<u32>(WHEEL_LAST).write(0);
        wr::<u8>(v, 0x614, fl);
    }
    main_part(this, b, f);
}

/// 0x51809F (after `push edi`: `[esp+X]` = `B+X-4`): source, front / up, the under-water blur.
fn main_part(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let v = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<GPtr<u8>>(v, 0x14).is_null() {
        allocate_matrix(v);
        simple_transform_update_matrix(v.byte_add(4), rd::<GPtr<u8>>(v, 0x14));
    }
    let mut veh = rd::<GPtr<u8>>(this, 0x21C);
    let mut m = rd::<GPtr<u8>>(v, 0x14);
    wr(b, 0x70, m);
    if rd::<i32>(veh, 0x590) == 9 {
        bike_calculate_lean_matrix(veh);
        veh = rd::<GPtr<u8>>(this, 0x21C);
        m = veh.byte_add(0x5CC);
        wr(b, 0x70, m);
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(veh, 0x22) as isize as usize).read();
    let seat = if rd::<i32>(mi, 0x3C) == 5 {
        rd::<GPtr<u8>>(mi, 0x5C)
    } else {
        rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x30)
    };
    wr::<u32>(b, 0x14, rd::<u32>(seat, 0));
    wr::<u32>(b, 0x18, rd::<u32>(seat, 4));
    f.fld(rd(b, 0x18));
    let sz = rd::<u32>(seat, 8);
    f.fadd_m(k(K_0_08));
    wr::<u32>(b, 0x1C, sz);
    wr::<f32>(b, 0x18, f.fstp());
    f.fld(rd(b, 0x1C));
    f.fadd_m(k(K_0_62));
    wr::<u32>(b, 0x14, 0);
    wr::<f32>(b, 0x1C, f.fstp());
    wr::<u32>(this, 0xB4, 0x4270_0000);
    let r = multiply_3x3(b.byte_add(0x50), m, b.byte_add(0x14));
    let src = this.byte_add(0x19C);
    copy3(src, r);
    let pos = entity_pos(rd::<GPtr<u8>>(this, 0x21C));
    f.fld(rd(src, 0));
    f.fadd_m(rd(pos, 0));
    wr::<f32>(src, 0, f.fstp());
    f.fld(rd(pos, 4));
    f.fadd_m(rd(src, 4));
    wr::<f32>(src, 4, f.fstp());
    f.fld(rd(pos, 8));
    f.fadd_m(rd(src, 8));
    wr::<f32>(src, 8, f.fstp());
    let veh = rd::<GPtr<u8>>(this, 0x21C);
    let kind = rd::<i32>(veh, 0x590);
    if kind == 5 {
        f.fld(rd(this, 0x1A4));
        f.fadd_m(k(K_0_5));
        wr::<f32>(this, 0x1A4, f.fstp());
    } else if kind == 9 && !rd::<GPtr<u8>>(veh, 0x460).is_null() {
        bike_driver_eye(this, b, veh, f);
    }
    // 0x5182E8: the upside-down rise
    let up_down = vehicle_is_upside_down(rd::<GPtr<u8>>(this, 0x21C));
    f.fld(d(UPSIDE_RISE));
    if up_down != 0 {
        f.fcomp_m(k(K_0_5));
        if !f.jp(0x5) {
            f.fld(d(UPSIDE_RISE));
            f.fadd_m(k(K_0_03));
            g::<f32>(UPSIDE_RISE).write(f.fstp());
        } else {
            g::<u32>(UPSIDE_RISE).write(0x3F00_0000);
        }
    } else {
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            f.fld(d(UPSIDE_RISE));
            f.fsub_m(k(K_0_03));
            g::<f32>(UPSIDE_RISE).write(f.fstp());
        } else {
            g::<u32>(UPSIDE_RISE).write(0);
        }
    }
    f.fld(d(UPSIDE_RISE));
    f.fadd_m(rd(this, 0x1A4));
    wr::<f32>(this, 0x1A4, f.fstp());
    let m = rd::<GPtr<u8>>(b, 0x70);
    let front = this.byte_add(0x190);
    let up = this.byte_add(0x1B4);
    copy3(front, m.byte_add(0x10));
    vector_normalise(front);
    copy3(up, rd::<GPtr<u8>>(b, 0x70).byte_add(0x20));
    vector_normalise(up);
    let r = cross_product(b.byte_add(0x5C), front, up);
    copy3(b.byte_add(0x2C), r);
    vector_normalise(b.byte_add(0x2C));
    let r = cross_product(b.byte_add(0x5C), b.byte_add(0x2C), front);
    copy3(up, r);
    vector_normalise(up);
    // 0x518404: under water
    wr::<u32>(b, 0x10, 0);
    let wl = get_water_level(
        rd::<u32>(src, 0),
        rd::<u32>(src, 4),
        rd::<u32>(src, 8),
        b.byte_add(0x10),
        1,
        GPtr::NULL,
    );
    if wl != 0 {
        f.fld(rd(b, 0x10));
        f.fsub_m(k(K_0_3));
        f.fcomp_m(rd(this, 0x1A4));
        if !f.jne(0x41) {
            under_water_blur(f);
        }
    }
    wr::<u8>(this, 0xA, 0);
}

/// 0x5181D5: a bike with a driver: the eye at the driver's bone 4 moved by `ts * moveSpeed`, + 0.15 in z;
/// when the line from `source + (front x up) * [0x8CCD0C]` back to `source - ...` is blocked, the source
/// becomes the vehicle position with that eye height + 0.62.
fn bike_driver_eye(this: GPtr<u8>, b: GPtr<u8>, veh: GPtr<u8>, f: &mut Fpu) {
    let src = this.byte_add(0x19C);
    wr::<u32>(b, 0x20, 0);
    wr::<u32>(b, 0x24, 0);
    wr::<u32>(b, 0x28, 0);
    ped_get_bone_position(rd::<GPtr<u8>>(veh, 0x460), b.byte_add(0x20), 4, 1);
    let ts = g::<u32>(TIME_STEP).read();
    let mv = rd::<GPtr<u8>>(this, 0x21C).byte_add(0x44);
    let r = vec_mul_s(b.byte_add(0x58), mv, ts);
    vector_add_assign(b.byte_add(0x20), r);
    f.fld(rd(b, 0x28));
    f.fadd_m(d(0x8CC7B4));
    wr::<f32>(this, 0x1A4, f.fstp());
    cross_product(b.byte_add(0x38), this.byte_add(0x190), this.byte_add(0x1B4));
    vector_scale_assign(b.byte_add(0x38), g::<u32>(0x8CCD0C).read());
    vec_add(b.byte_add(0x50), src, b.byte_add(0x38));
    vector_sub(b.byte_add(0x44), src, b.byte_add(0x38));
    if get_is_line_of_sight_clear(b.byte_add(0x50), b.byte_add(0x44), 1, 0, 0, 0, 0, 0, 0) != 0 {
        return;
    }
    let pos = entity_pos(rd::<GPtr<u8>>(this, 0x21C));
    f.fld(rd(b, 0x28));
    copy3(src, pos);
    f.fadd_m(d(0x8CC7B4));
    f.fadd_m(k(K_0_62));
    wr::<f32>(this, 0x1A4, f.fstp());
}

/// 0x518452: `SetMotionBlur(r, g, b, alpha, 2)` from the water colour, scaled by `10 / |c|` when
/// `|c| > 10` (`fld b; fmul st, st(1)`, `fld g; fmul st, st(1)`, `fmul r` on the scale), each `_ftol2`.
fn under_water_blur(f: &mut Fpu) {
    f.fld(d(WATER_B));
    f.fmul_m(d(WATER_B));
    f.fld(d(WATER_G));
    f.fmul_m(d(WATER_G));
    f.faddp(1);
    f.fld(d(WATER_R));
    f.fmul_m(d(WATER_R));
    f.faddp(1);
    f.fsqrt();
    f.fcom_m(d(UW_COLOUR_CAP));
    let alpha = g::<i32>(UW_ALPHA).read();
    let (bi, gi, ri);
    if !f.jne(0x41) {
        f.fdivr_m(d(UW_COLOUR_CAP));
        f.fld(d(WATER_B));
        f.fmul_st(1);
        bi = ftol32(f.pop());
        f.fld(d(WATER_G));
        f.fmul_st(1);
        gi = ftol32(f.pop());
        f.fmul_m(d(WATER_R));
        ri = ftol32(f.pop());
    } else {
        f.fstp_st(0);
        f.fld(d(WATER_B));
        bi = ftol32(f.pop());
        f.fld(d(WATER_G));
        gi = ftol32(f.pop());
        f.fld(d(WATER_R));
        ri = ftol32(f.pop());
    }
    set_motion_blur_call(GPtr::from_addr(THE_CAMERA), ri, gi, bi, alpha, 2);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_1ST_PERSON];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
