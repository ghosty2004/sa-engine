//! `CGarage::CountCarsInside` (PEDHIT: the player's car near CJ's garage at Grove Street). Ported from the 1.0 US
//! bytes (stub 0x4495F0 → `.HOODLUM` 0x1561400).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd};

orig! {
    /// `CGarage::IsPointInsideGarage(CVector)` (0x448740)
    fn is_point_inside_garage(gp: GPtr<u8>, x: u32, y: u32, z: u32) -> u8 = thiscall 0x448740 => unported;
}

/// `CGarage::CountCarsInside(CVehicle* except)` (stub 0x4495F0 → 0x1561400, thiscall, `ret 4`): the vehicles of
/// the pool (from the last slot down, free slots skipped) other than `except` whose position (matrix or placement)
/// `IsPointInsideGarage` accepts.
#[hook(addr = 0x4495F0, conv = thiscall, name = "thunk_FUN_01561400", expect = "E9 0B 7E 11 01",
       unverified = "pedhit-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn garage_count_cars_inside(this: GPtr<u8>, except: GPtr<u8>) -> i32 {
    let pool = g::<GPtr<u8>>(0xB7_4494).read();
    let mut n = 0;
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() || v == except {
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let p = if m.is_null() { v.byte_add(4) } else { m.byte_add(0x30) };
        if is_point_inside_garage(this, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)) != 0 {
            n += 1;
        }
    }
    n
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GARAGE_COUNT_CARS_INSIDE];
