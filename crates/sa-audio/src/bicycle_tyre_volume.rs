//! `CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre(float)` (0x4F60B0, STD-13; `#[native]`). Ported from the
//! 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::NativeReg;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

/// The speed → volume curve: six (speed, volume) float pairs at 0x8CC184.
const CURVE: usize = 0x8CC184;

fn curve(i: usize, k: usize) -> f32 {
    GPtr::<f32>::from_addr(CURVE + 8 * i + 4 * k).read()
}

/// `CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre(float speed)` (0x4F60B0, stdcall, `ret 4`; ECX unused), ST0.
/// i = the first of the five segments whose end speed (pair i + 1) is above `speed` (5 when none). Above 1.0 (the
/// double at 0x85A310) or with i = 5: the last volume (0x8CC1B0). Below 0.0 (0x858B50) or with i = 0: pair 1's
/// volume. Otherwise the linear interpolation between pairs i and i + 1 in x87 precision.
///
/// Registered `#[native]` (phase B only): the segment loop jumps back to 0x4F60B2, inside the 5 bytes a hook patches.
#[sa_core::native(addr = 0x4F60B0, conv = stdcall, name = "CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre")]
pub fn get_base_volume_for_bicycle_tyre(speed: f32) -> X87 {
    let mut i = 0usize;
    while i < 5 {
        if !X87::from_f32(curve(i + 1, 0)).fcom(X87::from_f32(speed)).test_any(0x41) {
            break;
        }
        i += 1;
    }
    let s = X87::from_f32(speed);
    let one = X87::from_f64_exact(GPtr::<f64>::from_addr(0x85A310).read());
    if !s.fcom(one).test_any(0x41) || i == 5 {
        return X87::from_f32(GPtr::<f32>::from_addr(0x8CC1B0).read());
    }
    let zero = GPtr::<f32>::from_addr(0x858B50).read();
    if !s.fcom(X87::from_f32(zero)).test_parity(5) || i == 0 {
        return X87::from_f32(curve(1, 1));
    }
    s.fsub_m(curve(i, 0))
        .fdiv(X87::from_f32(curve(i + 1, 0)).fsub_m(curve(i, 0)))
        .fmul(X87::from_f32(curve(i + 1, 1)).fsub_m(curve(i, 1)))
        .fadd_m(curve(i, 1))
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_GET_BASE_VOLUME_FOR_BICYCLE_TYRE];
