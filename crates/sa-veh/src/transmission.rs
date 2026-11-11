//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `cTransmission` (0x68 bytes: +0 6 gears of
//! (max velocity, change-up velocity, change-down velocity), +0x48 drive type, +0x49 engine type,
//! +0x4A number of gears, +0x58 max gear velocity, +0x60 min gear velocity). Ported from the 1.0 US
//! bytes; gta-reversed `cTransmission.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{f32_of, g, rd, wr, x, xi};

/// `cTransmission*` (thiscall `this`).
type Tr = GPtr<u8>;

/// `cTransmission::cTransmission()` (0x6D0450): `mov eax, ecx; ret` (3 bytes: not hookable, TooShort).
pub fn constructor(tr: Tr) -> Tr {
    tr
}

/// `cTransmission::InitGearRatios()` (0x6D0460). Clears the 6 gears (0x48 bytes), then with
/// n = +0x4A, t = 1.0 (0x858624) / n, a = f32(0x8D3490 * maxVel * t), b = f32(maxVel - a), for gear
/// k = 1..=n (n re-read from the object every iteration, as the original does):
/// `gear[k].max = k * b * t + a`; below n, `gear[k+1].down = 0x871D4C * d + gear[k-1].max` and
/// `gear[k].up = 0x871D50 * d + gear[k-1].max` with d = gear[k].max - gear[k-1].max (x87 value);
/// gear n: `up = maxVel` (bit copy). The statics 0xC1CB34 / 0xC1CB30 receive &gear[k] / &gear[k-1]
/// every iteration. Gear 0 = (minVel, -0.01, minVel) and gear 1's down = -0.01 at the end.
#[hook(addr = 0x6D0460, conv = thiscall, name = "cTransmission::InitGearRatios", expect = "83 EC 10 8B D1", this = "cTransmission")]
pub fn init_gear_ratios(tr: Tr) {
    for k in 0..0x12 {
        wr::<u32>(tr, 4 * k, 0);
    }
    let n0 = rd::<u8>(tr, 0x4A);
    let t = f32_of(xi(i32::from(n0)).fdivr_m(g::<f32>(0x858624).read()));
    let a = f32_of(x(g::<f32>(0x8D3490).read()).fmul_m(rd::<f32>(tr, 0x58)).fmul_m(t));
    let bb = f32_of(x(rd::<f32>(tr, 0x58)).fsub_m(a));
    if n0 >= 1 {
        let mut k: u8 = 1;
        loop {
            let cur = tr.byte_add(isize::from(k) * 12);
            let prev = cur.byte_add(-12);
            g::<u32>(0xC1CB34).write(cur.addr() as u32);
            g::<u32>(0xC1CB30).write(prev.addr() as u32);
            let v = xi(i32::from(k)).fmul_m(bb).fmul_m(t).fadd_m(a);
            cur.cast::<f32>().write(f32_of(v));
            let n = rd::<u8>(tr, 0x4A);
            let d = v.fsub_m(prev.cast::<f32>().read());
            if k < n {
                let down = x(g::<f32>(0x871D4C).read())
                    .fmul(d)
                    .fadd_m(prev.cast::<f32>().read());
                wr::<f32>(cur, 0x14, f32_of(down));
                let up = x(g::<f32>(0x871D50).read())
                    .fmul(d)
                    .fadd_m(prev.cast::<f32>().read());
                wr::<f32>(cur, 4, f32_of(up));
            } else {
                wr::<u32>(cur, 4, rd::<u32>(tr, 0x58));
            }
            let n = rd::<u8>(tr, 0x4A);
            k = k.wrapping_add(1);
            if k > n {
                break;
            }
        }
    }
    let min = rd::<u32>(tr, 0x60);
    wr::<u32>(tr, 0, min);
    wr::<u32>(tr, 4, 0xBC23_D70A);
    wr::<u32>(tr, 8, min);
    wr::<u32>(tr, 0x14, 0xBC23_D70A);
}

pub const HOOKS: &[HookReg] = &[HOOK_INIT_GEAR_RATIOS];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
