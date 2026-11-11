//! WRITER-MVP-B (helperB) COLL ports: `tColLighting::GetCurrentLighting` and the `.HOODLUM` byte setter
//! 0x411BF0. Ported from the 1.0 US bytes.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::world::common::{g, wr, x};

/// `tColLighting::GetCurrentLighting(tColLighting, float scale)` (0x59F0C0, cdecl, float in ST0): with
/// `b = CCustomBuildingDNPipeline::m_fDNBalanceParam` (0x8D12C0), `day*scale/15 * (1 - b) + night*scale/15 * b`
/// (day = low nibble, night = high nibble; x87 PC24). Kept original detail: the nibbles are stored into the
/// caller's argument slot (not modelled).
#[hook(addr = 0x59F0C0, conv = cdecl, name = "tColLighting::GetCurrentLighting", expect = "8A 44 24 04 D9 05 C0 12 8D 00",
       unverified = "mvpb-coll-1", route = "L2", hazards = "none", note = "writes its own (caller-owned) first argument slot")]
pub fn col_lighting_get_current_lighting(l: u8, scale: f32) -> X87 {
    let b = x(g::<f32>(0x8D12C0).read());
    let k = f32::from_bits(0x3D88_8889);
    let day = X87::from_i32((l & 0xF) as i32).fmul_m(scale).fmul_m(k);
    let a = x(1.0).fsub(b).fmul(day);
    let night = X87::from_i32((l >> 4) as i32).fmul_m(scale).fmul_m(k).fmul(b);
    a.fadd(night)
}

/// `thunk_FUN_01563610(uint8)` (0x411BF0 -> .HOODLUM 0x1563610, `ret 4`): stores the byte at +0x20.
#[hook(addr = 0x411BF0, conv = thiscall, name = "thunk_FUN_01563610", expect = "E9 1B 1A 15 01",
       unverified = "mvpb-coll-1", route = "L2", hazards = "none")]
pub fn set_byte_20(this: GPtr<u8>, v: u8) {
    wr::<u8>(this, 0x20, v);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COL_LIGHTING_GET_CURRENT_LIGHTING, HOOK_SET_BYTE_20];
