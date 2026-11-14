//! `Fx_c::TriggerWaterHydrant` (PEDHIT: a car knocks over a fire hydrant while hitting peds). Ported from the 1.0
//! US bytes.

use sa_core::mem::GPtr;
use sa_core::{native, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};

orig! {
    /// `FxManager_c::CreateFxSystem(const char* name, CVector* point, RwMatrix* parent, bool)` (0x4A9BE0, thiscall)
    fn fx_create_system(fx: GPtr<u8>, name: u32, point: GPtr<u8>, parent: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::PlayAndKill()` (0x4AA3D0)
    fn fx_system_play_and_kill(s: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
}

/// `g_fxMan` (0xA9AE80) and the effect name "water_hydrant" (0x85A70C).
const FX_MAN: usize = 0xA9_AE80;
const WATER_HYDRANT: u32 = 0x85_A70C;

/// `Fx_c::TriggerWaterHydrant(CVector& pos)` (0x4A0D70, thiscall, `ret 4`): nothing unless the squared distance
/// from the camera, `(dx² + dz²) + dy²` (x87), is at most 625.0f (0x85A6E8; NaN passes); then a "water_hydrant"
/// system at `pos` (`CreateFxSystem(name, pos, null, false)`) is played and killed when created.
///
/// The entry jumps into the protection chunk 0x404D09, which loads the camera's target entity as
/// `[[0x15607C4] - [0x55FEC7]]` = `[0xB6F03C]` (evaluated on the image) before the body at 0x4A0D75 picks its
/// position (+0x30) or the camera vector 0xB6F02C. Registered for phase B only (`#[native]`): protection code
/// reads the entry's jmp as a key (`xor ecx, [0x4A0D70]` at 0x401C02).
#[native(addr = 0x4A0D70, conv = thiscall, name = "Fx_c::TriggerWaterHydrant")]
pub fn fx_trigger_water_hydrant(_this: GPtr<u8>, pos: GPtr<u8>) {
    let m = g::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { g::<u8>(0xB6_F02C) };
    let dx = X87::from_f32(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = X87::from_f32(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(pos, 4));
    let dz = X87::from_f32(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(pos, 8));
    let sq = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    if !sq.fcom(X87::from_f32(625.0)).test_any(0x41) {
        return;
    }
    let s = fx_create_system(g::<u8>(FX_MAN), WATER_HYDRANT, pos, GPtr::NULL, 0);
    if !s.is_null() {
        fx_system_play_and_kill(s);
    }
}

/// This module's `#[native]` registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_FX_TRIGGER_WATER_HYDRANT];
