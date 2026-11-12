//! WRITER-NATIVE (batch `native-29`): `CTaskSimpleClimb::TestForClimb` (0x6803A0), called by every player jump.
//! Ported from the 1.0 US bytes; gta-reversed (`TaskSimpleClimb.cpp`) as a reading aid. x87 arithmetic follows the
//! original's instruction order. `ScanToGrab` (0x67FD30) and the rest of the climb task are WRITER-CLIMB's.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimpleClimb::ScanToGrab(CPed*, CVector& pos, float& angle, uint8& surface, bool launch, bool, bool,
    /// CVector* posOverride)` (0x67FD30, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn climb_scan_to_grab(ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, launch: u32, a: u32, b: u32, over: GPtr<u8>) -> GPtr<u8> = cdecl 0x67FD30 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
}

/// `CTaskSimpleClimb::ms_fAtEdgeOffsetHorz` (0x8D2F24), `ms_fAtEdgeOffsetVert` (0xC18F78)
const EDGE_HORZ: usize = 0x8D_2F24;
const EDGE_VERT: usize = 0xC1_8F78;

/// `CEntity* CTaskSimpleClimb::TestForClimb(CPed* ped, CVector& climbPos, float& climbAngle, uint8& surfaceType,
/// bool launch)` (0x6803A0, cdecl).
///
/// `ScanToGrab(ped, climbPos, climbAngle, surfaceType, launch, false, false, null)`; nothing found: null. The
/// handhold (a vehicle, ped or object's local position and angle turned into world space through its matrix,
/// allocating one when missing; the angle plus `atan2(-forward.x, forward.y)`, else plus its heading) is moved to
/// the edge (`x - sin(a) * [0x8D2F24]`, `y + cos(a) * [0x8D2F24]`, `z + [0xC18F78]`) and scanned again
/// (`ScanToGrab(ped, &tmp, &angle, &surface, false, true, false, &edge)`): something there means the edge is
/// blocked, `climbAngle = -9999.9` and null; otherwise the first entity.
#[hook(addr = 0x6803A0, conv = cdecl, name = "CTaskSimpleClimb::TestForClimb",
       expect = "8B 44 24 14 8B 4C 24 10",
       unverified = "native-29", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "fsin / fcos / fpatan")]
pub fn climb_test_for_climb(ped: GPtr<u8>, pos: GPtr<u8>, angle: GPtr<u8>, surface: GPtr<u8>, launch: u32) -> GPtr<u8> {
    let e = climb_scan_to_grab(ped, pos, angle, surface, launch, 0, 0, GPtr::NULL);
    if e.is_null() {
        return e;
    }
    let mut a_local: f32 = rd::<f32>(angle, 0);
    let mut wp = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
    let wpp: GPtr<u8> = GPtr::from_raw(wp.as_mut_ptr()).cast();
    let ty = rd::<u8>(e, 0x36) & 7;
    let a = if ty > 1 && ty < 5 {
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut out = [0u32; 3];
        matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(e, 0x14), wpp);
        wp = out;
        let m = rd::<GPtr<u8>>(e, 0x14);
        let h = if !m.is_null() {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
        } else {
            x(rd::<f32>(e, 0x10))
        };
        h.fadd_m(a_local)
    } else {
        x(a_local)
    };
    let horz = g::<f32>(EDGE_HORZ).read();
    let mut edge = [
        a.fsin().fmul_m(horz).fsubr_m(f32::from_bits(wp[0])).to_f32(),
        a.fcos().fmul_m(horz).fadd_m(f32::from_bits(wp[1])).to_f32(),
        x(g::<f32>(EDGE_VERT).read()).fadd_m(f32::from_bits(wp[2])).to_f32(),
    ];
    let mut tmp = [0u32; 3];
    let mut surf: u32 = 0;
    let hit = climb_scan_to_grab(
        ped,
        GPtr::from_raw(tmp.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut a_local as *mut f32).cast(),
        GPtr::from_raw(&mut surf as *mut u32).cast(),
        0,
        1,
        0,
        GPtr::from_raw(edge.as_mut_ptr()).cast(),
    );
    if hit.is_null() {
        return e;
    }
    wr::<u32>(angle, 0, 0xC61C_3F9A);
    GPtr::NULL
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CLIMB_TEST_FOR_CLIMB];
