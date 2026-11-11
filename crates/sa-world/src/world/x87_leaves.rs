//! Single-instruction x87 leaves and the entity area test on the in-world path (batch `mvpl-world-1`). Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use super::common::{g, rd};

/// `CGame::currArea` (0xB72914)
const CURR_AREA: usize = 0xB72914;

/// `FUN_0041bcf0(float)` (0x41BCF0, cdecl): `fld` of the argument, FCOS, result in ST0.
#[hook(addr = 0x41bcf0, conv = cdecl, name = "FUN_0041bcf0", expect = "D9 44 24 04 D9 FF C3",
       unverified = "mvpl-world-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "returns the raw FCOS in ST0: the model carries 53 of the CPU's 64 significand bits")]
pub fn x87_cos(a: u32) -> X87 {
    X87::from_f32(f32::from_bits(a)).fcos()
}

/// `FUN_00406d10(float)` (0x406D10, cdecl): `fld` of the argument, FABS, result in ST0.
#[hook(addr = 0x406d10, conv = cdecl, name = "FUN_00406d10", expect = "D9 44 24 04 D9 E1 C3",
       unverified = "mvpl-world-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn x87_abs(a: u32) -> X87 {
    X87::from_f32(f32::from_bits(a)).fabs()
}

/// `FUN_00407000` (stub 0x407000 → 0x1562960, thiscall on a `CEntity`; called by `CRenderer::SetupEntityVisibility`):
/// AL = 1 when the area code byte (+0x2F) equals `CGame::currArea` (dword compare of the zero-extended byte) or is
/// 13, else 0. AL only: the upper EAX bytes are the caller's.
#[hook(addr = 0x407000, conv = thiscall, name = "thunk_FUN_01562960", expect = "E9 5B B9 15 01",
       unverified = "mvpl-world-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "AL result; the original leaves the caller's upper EAX bytes")]
pub fn entity_is_in_current_area(this: GPtr<u8>) -> u8 {
    let area = rd::<u8>(this, 0x2F);
    u8::from(u32::from(area) == g::<u32>(CURR_AREA).read() || area == 13)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_X87_COS, HOOK_X87_ABS, HOOK_ENTITY_IS_IN_CURRENT_AREA];
