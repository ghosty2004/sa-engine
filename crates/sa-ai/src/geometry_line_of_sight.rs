//! `CPedGeometryAnalyser::GetIsLineOfSightClear(const CVector&, const CVector&, CEntity&)` (0x5F2F00, batch
//! `std12-ai-1`): reached by the ambient peds' route planning. Ported from the 1.0 US bytes (gta-reversed as a reading
//! aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50)
    fn col_line_ctor(l: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => unported;
    /// `CCollision::TestLineOfSight(const CColLine&, const CMatrix&, CColModel&, bool, bool)` (0x417730), AL
    fn test_line_of_sight(l: GPtr<u8>, m: GPtr<u8>, col: GPtr<u8>, see_through: u32, shoot_through: u32) -> u8 = cdecl 0x417730 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560), `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
}

/// `CPedGeometryAnalyser::GetIsLineOfSightClear(const CVector& a, const CVector& b, CEntity& e)` (0x5F2F00, cdecl,
/// full EAX 1 / 0): a `CColLine(a, b)` tested against the entity's collision model (`ms_modelInfoPtrs[mi] + 0x14`)
/// with its matrix (allocated from the simple transform when missing), not see-through, not shoot-through; 1 when
/// nothing is hit.
#[hook(addr = 0x5F2F00, conv = cdecl, name = "CPedGeometryAnalyser::GetIsLineOfSightClear-v3d", expect = "8B 44 24 08 8B 4C 24 04 83 EC 20",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn geometry_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, e: GPtr<u8>) -> u32 {
    let mut line = [0u32; 8];
    let lp = GPtr::from_raw(line.as_mut_ptr()).cast::<u8>();
    col_line_ctor(lp, a, b);
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    u32::from(test_line_of_sight(lp, rd::<GPtr<u8>>(e, 0x14), col, 0, 0) == 0)
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_GEOMETRY_IS_LINE_OF_SIGHT_CLEAR];
