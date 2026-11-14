//! `CPickups::DoMoneyEffects` (PEDHIT: the money a ped killed by a car drops spins). Ported from the 1.0 US bytes;
//! gta-reversed `Pickups.cpp` (`SetRotateZOnly`) as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{f32_of, g, rd, wr, xi};

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
}

/// `CPlaceable::GetMatrix()` as the original inlines it: the matrix, allocated and filled from the placement when
/// missing.
fn get_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        placeable_allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `CPickups::DoMoneyEffects(CEntity*)` (0x454E80, cdecl): `a = (time & 0x7FF) * 0.0030566407f` (0x859B0C; the
/// `jge` that adds 2^32 to a negative value is never taken), `c = f32(fcos a)`, `s = f32(fsin a)`; the matrix
/// becomes a rotation about z: right = (c, s, 0), forward = (-s, c, 0), up = (0, 0, 1), each field written through
/// its own inlined `GetMatrix()`. The position is kept.
#[hook(addr = 0x454E80, conv = cdecl, name = "CPickups::DoMoneyEffects", expect = "83 EC 08 A1 84 CB B7 00",
       unverified = "pedhit-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "driving",
       note = "FSIN / FCOS of the spin angle (sa_math::x87 models)")]
pub fn pickups_do_money_effects(e: GPtr<u8>) {
    let t = (g::<u32>(0xB7_CB84).read() & 0x7FF) as i32;
    let a = xi(t).fmul_m(0.003_056_640_7);
    let c = f32_of(a.fcos());
    let s = f32_of(a.fsin());
    wr::<f32>(get_matrix(e), 0x0, c);
    wr::<f32>(get_matrix(e), 0x4, s);
    wr::<u32>(get_matrix(e), 0x8, 0);
    wr::<f32>(get_matrix(e), 0x10, f32_of(super::common::x(s).fchs()));
    wr::<f32>(get_matrix(e), 0x14, c);
    wr::<u32>(get_matrix(e), 0x18, 0);
    wr::<u32>(get_matrix(e), 0x20, 0);
    wr::<u32>(get_matrix(e), 0x24, 0);
    wr::<f32>(get_matrix(e), 0x28, 1.0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PICKUPS_DO_MONEY_EFFECTS];
