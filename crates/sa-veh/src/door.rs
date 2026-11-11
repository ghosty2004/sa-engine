//! CDoor and CBouncingPanel (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! `CDoor` (0x18): +0 open angle f32, +4 closed angle f32, +8 dirn (i16), +0xA axis u8, +0xB state u8,
//! +0xC angle f32, +0x10 previous angle f32, +0x14 angular velocity f32.
//! `CBouncingPanel` (0x20): +0 frame id i16, +2 axis u16, +4 angle limit f32, +8 rotation CVector,
//! +0x14 position CVector.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

/// `fld dword` + `fabs` (the sign of a NaN is cleared too).
fn fabs(v: f32) -> X87 {
    x(f32::from_bits(v.to_bits() & 0x7FFF_FFFF))
}

/// The 0.0 constant at 0x858B50.
fn zero_const() -> f32 {
    g::<f32>(0x858B50).read()
}

/// `CDoor::Open(float ratio)` (0x6F4790): previous angle = angle; `ratio < 1.0` (ordered): angle =
/// `ratio * openAngle` and, when that is 0.0, the angular velocity is zeroed; otherwise (>= 1.0 or NaN)
/// angle = openAngle and, if the low byte of dirn is not negative, the state becomes 1.
#[hook(addr = 0x6F4790, conv = thiscall, name = "CDoor::Open", expect = "D9 44 24 04 8B 41 0C", this = "CDoor")]
pub fn open(door: GPtr<u8>, ratio: f32) {
    let angle = rd::<u32>(door, 0xC);
    wr::<u32>(door, 0x10, angle);
    if x(ratio).fcom(x(g::<f32>(0x858624).read())).test_parity(0x05) {
        wr::<u32>(door, 0xC, rd::<u32>(door, 0));
        if rd::<u8>(door, 8) as i8 >= 0 {
            wr::<u8>(door, 0xB, 1);
        }
        return;
    }
    let v = x(ratio).fmul_m(rd::<f32>(door, 0));
    wr::<f32>(door, 0xC, v.to_f32());
    if !v.fcom(x(zero_const())).test_parity(0x44) {
        wr::<u32>(door, 0x14, 0);
    }
}

/// `CDoor::GetAngleOpenRatio()` (0x6F47E0): openAngle == 0.0 -> 0.0, else `angle / openAngle` left in
/// ST0 at register precision.
#[hook(addr = 0x6F47E0, conv = thiscall, name = "CDoor::GetAngleOpenRatio", expect = "D9 01 D8 1D 50 8B 85 00", this = "CDoor")]
pub fn get_angle_open_ratio(door: GPtr<u8>) -> X87 {
    let open = rd::<f32>(door, 0);
    if !x(open).fcom(x(zero_const())).test_parity(0x44) {
        return x(zero_const());
    }
    x(rd::<f32>(door, 0xC)).fdiv_m(open)
}

/// `CDoor::IsClosed()` (0x6F4800): `closedAngle == angle`. Only AL is defined (AH = the FPU status
/// word's high byte from `fnstsw ax`).
#[hook(addr = 0x6F4800, conv = thiscall, name = "CDoor::IsClosed", expect = "D9 41 0C D9 41 04", this = "CDoor")]
pub fn is_closed(door: GPtr<u8>) -> u8 {
    u8::from(
        !x(rd::<f32>(door, 4))
            .fcom(x(rd::<f32>(door, 0xC)))
            .test_parity(0x44),
    )
}

/// `CDoor::IsFullyOpen()` (0x6F4820): `|openAngle| - 0.5 (qword 0x859060) <= |angle|`. Only AL is
/// defined (AH = the FPU status word's high byte).
#[hook(addr = 0x6F4820, conv = thiscall, name = "CDoor::IsFullyOpen", expect = "D9 41 0C D9 E1", this = "CDoor")]
pub fn is_fully_open(door: GPtr<u8>) -> u8 {
    let angle = fabs(rd::<f32>(door, 0xC));
    let open = fabs(rd::<f32>(door, 0));
    let half = X87::from_f64_exact(g::<f64>(0x859060).read());
    u8::from(!open.fsub(half).fcom(angle).test_parity(0x41))
}

/// `CBouncingPanel::ResetPanel()` (0x6F4910): frame id = -1.
#[hook(addr = 0x6F4910, conv = thiscall, name = "CBouncingPanel::ResetPanel", expect = "66 C7 01 FF FF", this = "CBouncingPanel")]
pub fn reset_panel(panel: GPtr<u8>) {
    wr::<u16>(panel, 0, 0xFFFF);
}

/// `CBouncingPanel::SetPanel(int16 frameId, int16 axis, float angleLimit)` (0x6F4920, prologue tail in
/// the .HOODLUM fragment at 0x401AA5): stores the three arguments and zeroes both vectors (+8..+0x20).
/// The angle limit is copied as raw bits. Protection code at 0x401635 reads the entry bytes (0x6F4924)
/// as a key (listed in sa-hook's keycells).
#[hook(addr = 0x6F4920, conv = thiscall, name = "CBouncingPanel::SetPanel", expect = "83 EC 0C E9 7D D1 D0 FF", this = "CBouncingPanel")]
pub fn set_panel(panel: GPtr<u8>, frame_id: u32, axis: u32, angle_limit: u32) {
    wr::<u16>(panel, 0, frame_id as u16);
    wr::<u16>(panel, 2, axis as u16);
    wr::<u32>(panel, 4, angle_limit);
    for k in 0..6 {
        wr::<u32>(panel, 8 + 4 * k, 0);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_OPEN,
    HOOK_GET_ANGLE_OPEN_RATIO,
    HOOK_IS_CLOSED,
    HOOK_IS_FULLY_OPEN,
    HOOK_RESET_PANEL,
    HOOK_SET_PANEL,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
