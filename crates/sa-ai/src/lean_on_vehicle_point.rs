//! WRITER-NATIVE (batch `native-13`): 0x664770, `CTaskGoToVehicleAndLean`'s lean-point helper (called by its
//! `CreateFirstSubTask` 0x664D40 and `ControlSubTask` 0x664E60). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`, returns `out`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CVector CTaskGoToVehicleAndLean::<lean point>(CPed* ped)` (0x664770, thiscall with the result pointer first,
/// `ret 8`).
///
/// The vehicle is +0xC. `side = right . (ped - vehicle)` (`(rz dz + ry dy) + rx dx`; without a matrix the right
/// vector is `(cos h, sin h, 0)` of the heading +0x10). On the right (`side > 0`, ordered): `+0x15 = 1` and the
/// local point `(bbox.max.x + 0.5, 0, 0)`, else `+0x15 = 0` and `(bbox.min.x - 0.5, 0, 0)`; the result is the
/// vehicle matrix times that point (`CMatrix * CVector` on +0x14, null without a matrix as in the original).
#[hook(addr = 0x664770, conv = thiscall, name = "FUN_00664770",
       expect = "83 EC 30 8B 51 0C 0F BF 42 22",
       unverified = "native-13", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "returns the CVector through the hidden pointer (first stack argument)")]
pub fn go_to_vehicle_and_lean_point(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    let info = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(veh, 0x22))).read();
    let col = rd::<GPtr<u8>>(info, 0x14);
    let min_x = rd::<f32>(col, 0);
    let max_x = rd::<f32>(col, 0xC);
    let vp = pos(veh);
    let pp = pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4));
    let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8));
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let (rx, ry, rz) = if !m.is_null() {
        (x(rd::<f32>(m, 0)), x(rd::<f32>(m, 4)), x(rd::<f32>(m, 8)))
    } else {
        let h = rd::<f32>(veh, 0x10);
        (x(h).fcos(), x(h).fsin(), x(0.0))
    };
    let side = rz.fmul(dz).fadd(ry.fmul(dy)).fadd(rx.fmul(dx));
    let lx = if side.fcom(x(0.0)).test_any(0x41) {
        wr::<u8>(this, 0x15, 0);
        x(min_x).fsub_m(0.5)
    } else {
        wr::<u8>(this, 0x15, 1);
        x(max_x).fadd_m(0.5)
    };
    let mut local = [lx.to_f32(), 0.0f32, 0.0f32];
    matrix_mul_vec(out, rd::<GPtr<u8>>(veh, 0x14), GPtr::from_raw(&mut local as *mut [f32; 3]).cast());
    out
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GO_TO_VEHICLE_AND_LEAN_POINT];
