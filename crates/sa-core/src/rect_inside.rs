//! STD-7: `CRect::IsPointInside` (0x4042D0) and `CBoundingBox::CBoundingBox(CVector, CVector)` (0x470100), ported from the 1.0 US bytes with the original's x87
//! evaluation (sums and differences kept in ST0, compared against the f32 point). Unverified.

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

fn ld(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}

/// `CRect::IsPointInside(const CVector2D& p, float tolerance)` (0x4042D0, thiscall, `ret 8`; CRect: left
/// +0, bottom +4, right +8, top +0xC): false when `left - tol > p.x`, `tol + right < p.x`, `top - tol >
/// p.y` or `tol + bottom < p.y` (each in x87 precision; a NaN comparison never rejects), else true.
/// AL only (AH holds the last `fnstsw`).
#[hook(addr = 0x4042D0, conv = thiscall, name = "CRect::IsPointInside-Tolerance", expect = "D9 01 8B 54 24 04 D8 64 24 08",
       unverified = "std7-math-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn rect_is_point_inside(this: GPtr<u8>, p: GPtr<u8>, tol: f32) -> u8 {
    let t = X87::from_f32(tol);
    let (px, py) = (X87::from_f32(ld(p, 0)), X87::from_f32(ld(p, 4)));
    if !X87::from_f32(ld(this, 0)).fsub(t).fcom(px).test_any(0x41)
        || !t.fadd_m(ld(this, 8)).fcom(px).test_parity(5)
        || !X87::from_f32(ld(this, 0xC)).fsub(t).fcom(py).test_any(0x41)
        || !t.fadd_m(ld(this, 4)).fcom(py).test_parity(5)
    {
        return 0;
    }
    1
}

// `CBoundingBox::SetMinMax` (0x470100) is registered once, by sa-script's misc.rs (PORT-SCRIPT-4).

pub const HOOKS: &[HookReg] = &[HOOK_RECT_IS_POINT_INSIDE];
