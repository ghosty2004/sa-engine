//! The float `pow` wrapper 0x470000 (batch `boot-veh-1`): `cHandlingDataMgr::LoadHandlingData` (0x5BD830) builds
//! the animation-group flags with it and `CAutomobile::ProcessCarWheelPair` (0x6A4EC0) the wheel speed decay. Ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_math::x87::X87;

/// `float pow(float base, float exponent)` (0x470000, cdecl, catalogue `FUN_00470000`): `fld base; fld exponent;
/// call _CIpow (0x822130); ret`, so the result is the CRT's `_CIpow(base, exponent)` left in ST0 (not rounded to
/// f32; the callers store or `_ftol` it). Byte-identical to the uncatalogued 0x46FFF0.
///
/// The CRT intrinsic runs through `sa_core::orig::x87_binary` (phase A: the original's code; phases B/C:
/// `sa_math::libm::pow`, the ledger line for `pow`); its ST0 comes back as a double, exact for every result the
/// game's 24-bit precision control produces.
#[hook(addr = 0x470000, conv = cdecl, name = "FUN_00470000", expect = "D9 44 24 04 D9 44 24 08 E8 23 21 3B 00",
       unverified = "boot-veh-1", route = "L2", hazards = "x87-transcendental", recs = "boot",
       note = "returns ST0; the CRT _CIpow result is taken through orig::x87_binary (double precision of the stored ST0)")]
pub fn float_pow(base: f32, exponent: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, f64::from(base), f64::from(exponent)))
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FLOAT_POW];
