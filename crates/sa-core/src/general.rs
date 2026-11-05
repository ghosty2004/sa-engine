//! `CGeneral::GetRandomNumberInRange` hooks. The arithmetic is sa-math's (MATH-2, L1-verified with
//! `rand()` as an input: `sa_math::general::get_random_number_in_range_*`), which follows the 1.0 US
//! bytes, not gta-reversed's template (catalog/gr-deviations.csv `General.h:104`/`:119`). This module
//! adds what a hook needs: the `rand()` call through `orig!` with its original return address.

use crate::{crt, hook, orig};
use sa_math::x87::X87;

/// Return address of `call rand` in 0x407180 (0x407182 + 5).
pub const RAND_RET_INT: u32 = 0x407187;
/// Return address of `call rand` in 0x41BD90 (0x41BD91 + 5).
pub const RAND_RET_FLOAT: u32 = 0x41BD96;

/// 0x407180 for a given `rand()` result (EAX after the call).
pub fn random_in_range_i32_from(rand: u32, min: i32, max: i32) -> i32 {
    sa_math::general::get_random_number_in_range_i32(rand as i32, min, max)
}

/// 0x41BD90 for a given `rand()` result: the full ST0.
pub fn random_in_range_f32_from(rand: u32, min: f32, max: f32) -> X87 {
    sa_math::general::get_random_number_in_range_f32_x87(rand as i32, min, max)
}

/// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180, cdecl): `[min, max)` for
/// `min < max`; one `rand()` call.
#[hook(
    addr = 0x407180,
    conv = cdecl,
    name = "CGeneral::GetRandomNumberInRange_int32",
    expect = "51 56 E8 97 A9 41 00"
)]
pub fn get_random_number_in_range_i32(min: i32, max: i32) -> i32 {
    let r = orig::at_site(RAND_RET_INT, crt::rand);
    random_in_range_i32_from(r as u32, min, max)
}

/// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90, cdecl, result in ST0).
#[hook(
    addr = 0x41BD90,
    conv = cdecl,
    name = "CGeneral::GetRandomNumberInRange<float>-",
    expect = "51 E8 88 5D 40 00"
)]
pub fn get_random_number_in_range_f32(min: f32, max: f32) -> X87 {
    let r = orig::at_site(RAND_RET_FLOAT, crt::rand);
    random_in_range_f32_from(r as u32, min, max)
}

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn integer_examples() {
        // CAT-14 example: 0..10 with rand() = 3277: original 1 (gta-reversed gives 0).
        assert_eq!(random_in_range_i32_from(3277, 0, 10), 1);
        assert_eq!(random_in_range_i32_from(0, 5, 9), 5);
        assert_eq!(random_in_range_i32_from(0x7FFF, 0, 10), 9);
        // only the low 16 bits of EAX are used
        assert_eq!(random_in_range_i32_from(0x1_0000, 3, 100), 3);
        // max < min gives values in (max, min]
        assert_eq!(random_in_range_i32_from(0x7FFF, 10, 0), 1);
    }

    #[test]
    fn float_examples() {
        assert_eq!(random_in_range_f32_from(0, 1.0, 2.0).to_f32(), 1.0);
        assert_eq!(random_in_range_f32_from(32767, 1.0, 2.0).to_f32(), 2.0);
        // extended range in ST0: no overflow to inf for max - min > f32::MAX
        let v = random_in_range_f32_from(32767, -3e38, 3e38);
        assert!(v.to_f64().is_finite());
    }

    #[test]
    fn owned_build_uses_the_rust_rand() {
        let _g = crt::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crt::srand(1);
        // rand() = 41 → 41/32768 * 100 = 0.125… → 0
        assert_eq!(get_random_number_in_range_i32(0, 100), 0);
        // rand() = 18467 → 18467/32767 * 10 = 5.63…
        let v = get_random_number_in_range_f32(0.0, 10.0).to_f32();
        assert!((5.63..5.64).contains(&v), "{v}");
    }
}
