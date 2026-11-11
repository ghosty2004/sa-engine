//! WRITER-RENDER-BIG-2: `operator!=(const CVector&, const CVector&)` (0x509760), a leaf the render
//! passes call. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{jp, rd, x};

/// `operator!=(const CVector& a, const CVector& b)` (0x509760): x, y, z compared in order with `fcomp`
/// (`test ah, 0x44; jp`): any component not equal, or unordered, returns 1, otherwise 0 (EAX is fully
/// 0 or 1; the original also leaves ECX = EAX and EDX = b).
#[hook(addr = 0x509760, conv = cdecl, name = "FUN_00509760", expect = "8B 4C 24 04 D9 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vector_not_equal(a: GPtr<u8>, b: GPtr<u8>) -> u32 {
    for off in [0, 4, 8] {
        if jp(x(rd::<f32>(a, off)), x(rd::<f32>(b, off)), 0x44) {
            return 1;
        }
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VECTOR_NOT_EQUAL];
