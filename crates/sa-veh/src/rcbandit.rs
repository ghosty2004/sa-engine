//! FRONTIER-2 turn 11, lane B (batch `frontier2-b25`): the RC bandit's wheel hits, `CAutomobile::RcbanditCheckHitWheels`
//! (0x6B45E0) and `CAutomobile::RcbanditCheck1CarWheels` (0x6B3F70). Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::gf;
use sa_core::mem::GPtr;
use sa_core::bindings::{atexit, matrix_assign, matrix_invert, matrix_mul_vec};
use sa_core::field::ptr_of as vp;
use sa_core::hook;
use sa_math::fpu::ge_or_nan;
use sa_math::vector::CVector;
use sa_stream::bindings::get_wheel_posn;
use sa_world::bindings::test_sphere_box;
use sa_math::fpu::ftol32;
use sa_math::x87::X87;
use sa_world::bindings::crt_floor;
use sa_world::world::common::entity_pos;
use sa_world::world::mvpb_world::advance_scan_code;

use crate::common::{f32_of, g, model_info, rd, wr, x};
use crate::mvpd_v1::read_vec;

/// 2.0 (0x858CA0), 0.02 (0x858B38), 60.0 (0x858B34); `CWorld::ms_aRepeatSectors` (0xB992B8, 16 x 16, 12 bytes).
const K_REACH: usize = 0x85_8CA0;
const K_SECTOR_SCALE: usize = 0x85_8B38;
const K_SECTOR_OFFSET: usize = 0x85_8B34;
const REPEAT_SECTORS: usize = 0xB9_92B8;

/// The function-static `CMatrix` of RcbanditCheck1CarWheels (0xC1C220) and its guard (0xC1C268 bit 0).
const S_MATRIX: usize = 0xC1_C220;
const S_MATRIX_GUARD: usize = 0xC1_C268;
/// 0.0 (0x858B50), 10.0 (0x85862C), 1.0 (0x858624), 0.25 (0x858C84).
const K_ZERO: usize = 0x85_8B50;
const K_NEAR: usize = 0x85_862C;
const K_ONE: usize = 0x85_8624;
const K_QUARTER: usize = 0x85_8C84;

/// `|a - b|` as the bytes take it (`fcomp 0.0; test ah, 5; jp` keeps the difference when `>= 0` or NaN, else
/// recomputes and negates it).
fn abs_diff(a: f32, b: f32) -> X87 {
    let d = x(a).fsub_m(b);
    if ge_or_nan(d, x(gf(K_ZERO))) { d } else { d.fchs() }
}

/// `CAutomobile::RcbanditCheck1CarWheels(CPtrList& list)` (0x6B3F70, thiscall, `ret 4`; AX the result).
///
/// The static matrix is constructed on first use (attach pointer / owns flag zeroed, `atexit(0x856A10)`). Every
/// automobile (+0x590 == 0) of the list other than this one and not an RC bandit (model 441) that this scan has not
/// seen (+0x2C, then marked) and lies within 10 of this one in x and in y (`|this - it|`, each `< 10`) has its four
/// wheels tested while compressed (+0x7E4 ratio `< 1.0`) or the vehicle is under physics (status 2): the wheel's
/// position (`GetWheelPosn(i, .., false)`) in this car's frame (`Invert(this matrix)` copied into the static, times
/// its matrix times the wheel), radius `wheel scale (+0x40) * 0.25`, against this car's col model
/// (`TestSphereBox`); a hit returns 1, the end of the list 0. The next node is kept in the caller's argument slot (not
/// modelled).
#[hook(addr = 0x6B3F70, conv = thiscall, name = "CAutomobile::RcbanditCheck1CarWheels", this = "CAutomobile",
       expect = "83 EC 40 53 55 56 57", unverified = "frontier2-b25", route = "L2", hazards = "none",
       note = "marks the scan code of the vehicles it visits; writes the static matrix 0xC1C220; AX only")]
pub fn rcbandit_check_1_car_wheels(this: GPtr<u8>, list: GPtr<u8>) -> u32 {
    let col = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(this, 0x22))), 0x14);
    let guard = g::<u32>(S_MATRIX_GUARD);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
        g::<u32>(S_MATRIX + 0x40).write(0);
        g::<u32>(S_MATRIX + 0x44).write(0);
        atexit(0x85_6A10);
    }
    let sm = GPtr::<u8>::from_addr(S_MATRIX);
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let v = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let code = g::<u16>(0xB7_CD78).read();
        if v == this || rd::<u32>(v, 0x590) != 0 || rd::<u16>(v, 0x22) == 0x1B9 || rd::<u16>(v, 0x2C) == code {
            continue;
        }
        wr::<u16>(v, 0x2C, code);
        let tp = entity_pos(this);
        let op = entity_pos(v);
        let near = x(gf(K_NEAR));
        if ge_or_nan(abs_diff(rd::<f32>(tp, 0), rd::<f32>(op, 0)), near) || ge_or_nan(abs_diff(rd::<f32>(tp, 4), rd::<f32>(op, 4)), near) {
            continue;
        }
        let mi = model_info(i32::from(rd::<i16>(v, 0x22)));
        for i in 0..4 {
            let compressed = !ge_or_nan(x(rd::<f32>(v, 0x7E4 + 4 * i)), x(gf(K_ONE)));
            if !compressed && rd::<u8>(v, 0x36) & 0xF8 != 0x10 {
                continue;
            }
            let mut wheel = CVector::default();
            get_wheel_posn(mi, i as i32, vp(&mut wheel), 0);
            matrix_assign(sm, matrix_invert(rd::<GPtr<u8>>(this, 0x14), sm));
            let mut t1 = CVector::default();
            let w = matrix_mul_vec(vp(&mut t1), rd::<GPtr<u8>>(v, 0x14), vp(&mut wheel));
            let mut t2 = CVector::default();
            let c = read_vec(matrix_mul_vec(vp(&mut t2), sm, w));
            let mut sphere = [c.x, c.y, c.z, f32_of(x(rd::<f32>(mi, 0x40)).fmul_m(gf(K_QUARTER))), 0.0];
            if test_sphere_box(GPtr::from_raw(sphere.as_mut_ptr()), col.cast()) != 0 {
                return 1;
            }
        }
    }
    0
}

/// One sector bound: `s = v * 0.02 + 60` (stored f32 too); `_ftol(floor(s))` (the double of the unrounded `s`) is
/// tested against the limit (`> 0` for a lower bound, `< 119` for an upper one); inside it the bound is
/// `_ftol(floor(f32 s))`, else the limit.
fn bound(v: X87, lower: bool) -> i32 {
    let s = v.fmul_m(gf(K_SECTOR_SCALE)).fadd_m(gf(K_SECTOR_OFFSET));
    let sf = f32_of(s);
    let first = ftol32(X87::from_f64_exact(crt_floor(s.to_f64())));
    let inside = if lower { first > 0 } else { first < 0x77 };
    if !inside {
        return if lower { 0 } else { 0x77 };
    }
    ftol32(X87::from_f64_exact(crt_floor(f64::from(sf))))
}

/// `CAutomobile::RcbanditCheckHitWheels()` (0x6B45E0, thiscall, plain `ret`; AX the result).
///
/// The sector rectangle of `position ± 2` (the far bounds and the near y stored as f32 first, the near x from the
/// unrounded difference; clamped to 0..119 as [`bound`] tests), the scan code advanced, then for every sector (y
/// outer, x inner) `RcbanditCheck1CarWheels(repeat sector vehicles)`: the first non-zero AX returns 1, else 0.
#[hook(addr = 0x6B45E0, conv = thiscall, name = "CAutomobile::RcbanditCheckHitWheels", this = "CAutomobile",
       expect = "55 8B EC 83 E4 F8", unverified = "frontier2-b25", route = "L2", hazards = "none",
       note = "advances the world scan code; AX only (EAX bits 16..31 differ)")]
pub fn rcbandit_check_hit_wheels(this: GPtr<u8>) -> u32 {
    let p = entity_pos(this);
    let r = gf(K_REACH);
    let x0v = x(rd::<f32>(p, 0)).fsub_m(r);
    let x1 = f32_of(x(rd::<f32>(p, 0)).fadd_m(r));
    let y0 = f32_of(x(rd::<f32>(p, 4)).fsub_m(r));
    let y1 = f32_of(x(rd::<f32>(p, 4)).fadd_m(r));
    let sx0 = bound(x0v, true);
    let sy0 = bound(x(y0), true);
    let sx1 = bound(x(x1), false);
    let sy1 = bound(x(y1), false);
    advance_scan_code();
    for sy in sy0..=sy1 {
        for sx in sx0..=sx1 {
            let cell = (sx & 15) + (sy & 15) * 16;
            let list = GPtr::<u8>::from_addr(REPEAT_SECTORS + cell as usize * 12);
            if rcbandit_check_1_car_wheels(this, list) & 0xFFFF != 0 {
                return 1;
            }
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_RCBANDIT_CHECK_1_CAR_WHEELS, HOOK_RCBANDIT_CHECK_HIT_WHEELS];
