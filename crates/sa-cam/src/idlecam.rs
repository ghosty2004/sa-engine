//! PORT-CAM: `CIdleCam` (0x9C bytes; `gIdleCam` at 0xB6FDA0). Ported from the 1.0 US bytes;
//! gta-reversed `IdleCam.cpp` and `docs/re/specs/CAM/CIdleCam.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, TIME_STEP};
use sa_core::{hook, orig};

use crate::common::{Fpu, active_cam, ftol32, g, jne, k, rd, the_camera, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `gIdleCam` (CIdleCam)
    pub static G_IDLE_CAM @ 0xB6FDA0: [u8; 0x9C] = [0; 0x9C];
}

/// `gIdleCam.m_nIdleTickerFrames` (gIdleCam +0x94), written by address in `IdleCamGeneralProcess`.
const IDLE_TICKER: usize = 0xB6FE34;
/// Byte whose bit 0 `IdleCamGeneralProcess` clears (0xC0B184, outside CAM).
const FLAGS_C0B184: usize = 0xC0B184;
/// .data f32 0.1 (0x8CCC8C) added to the height of a ped in state 0x16 / 5 in
/// `GetLookAtPositionOnTarget` (read as a variable, as the bytes).
const LOOK_AT_PED_LOW_Z: usize = 0x8CCC8C;

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CIdleCam::SetTarget(CEntity*)` (0x50A280)
    pub fn set_target(this: GPtr<u8>, e: GPtr<u8>) = thiscall 0x50A280 => unported;
    /// `CIdleCam::Init()` (0x50E6D0)
    pub fn init_call(this: GPtr<u8>) = thiscall 0x50E6D0 => idle_cam_init;
    /// `CIdleCam::Reset(bool)` (0x50A160)
    pub fn reset_call(this: GPtr<u8>, reset_controls: u8) = thiscall 0x50A160 => reset;
}

/// `CIdleCam::CIdleCam()` (0x517760): `Init()`, returns `this`.
#[hook(addr = 0x517760, conv = thiscall, name = "CIdleCam::CIdleCam", expect = "56 8B F1 E8 68 6F FF FF", this = "CIdleCam")]
pub fn idle_cam_constructor(this: GPtr<u8>) -> GPtr<u8> {
    init_call(this);
    this
}

/// `CIdleCam::Init()` (0x50E6D0): the tuning fields, ticker fields +0x90/+0x94 = 0, `Reset(false)`.
#[hook(addr = 0x50E6D0, conv = thiscall, name = "CIdleCam::Init", expect = "B8 00 00 70 41", this = "CIdleCam")]
pub fn idle_cam_init(this: GPtr<u8>) {
    for (o, v) in [
        (0x48, 0x4170_0000),
        (0x68, 0x4170_0000),
        (0x14, 0x44FA_0000),
        (0x28, 0x47AF_C800),
        (0x40, 0x453B_8000),
        (0x44, 0x4080_0000),
        (0x4C, 0x42A0_0000),
        (0x88, 0x3F80_0000),
        (0x8C, 0x453B_8000),
        (0x6C, 0x428C_0000),
        (0x74, 0x447A_0000),
        (0x50, 0xE),
        (0x7C, 0x463B_8000),
        (0x38, 0x459C_4000),
        (0x84, 0x4000_0000),
        (0x90, 0),
        (0x94, 0),
    ] {
        wr::<u32>(this, o, v);
    }
    reset_call(this, 0);
}

/// `CIdleCam::Reset(bool resetControls)` (0x50A160, `ret 4`): positions/angles cleared, the -1.0
/// markers, +0 = 0, +0x58 = 3, +0x70 = 70.0, +0x24 = 1.0; with `resetControls` (byte != 0) pad 0's
/// `LastTimeTouched` (+0x120) = `CTimer::m_snTimeInMilliseconds` (read before `GetPad`).
#[hook(addr = 0x50A160, conv = thiscall, name = "CIdleCam::Reset", expect = "83 EC 0C 33 D2", this = "CIdleCam")]
pub fn reset(this: GPtr<u8>, reset_controls: u8) {
    for o in [4, 8, 0xC, 0x18, 0x1C, 0x20] {
        wr::<u32>(this, o, 0);
    }
    for o in [0x10, 0x2C, 0x34, 0x5C, 0x60, 0x64, 0x3C, 0x80] {
        wr::<u32>(this, o, 0xBF80_0000);
    }
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 0x58, 3);
    wr::<u8>(this, 0x78, 0);
    wr::<u32>(this, 0x70, 0x428C_0000);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x79, 0);
    wr::<u32>(this, 0x24, 0x3F80_0000);
    if reset_controls != 0 {
        let t = TIME_MS.get();
        wr(get_pad(0), 0x120, t);
    }
}

/// `CIdleCam::IdleCamGeneralProcess()` (0x50E690): `(float)ticker (+0x94) <= +0x28` (or NaN) clears bit
/// 0 of 0xC0B184; an active camera mode other than 4 (FOLLOW_PED) clears it too and zeroes
/// `gIdleCam`'s ticker (0xB6FE34, by address, not through `this`).
#[hook(addr = 0x50E690, conv = thiscall, name = "CIdleCam::IdleCamGeneralProcess", expect = "DB 81 94 00 00 00", this = "CIdleCam")]
pub fn idle_cam_general_process(this: GPtr<u8>) {
    let fl = g::<u8>(FLAGS_C0B184);
    if jne(xi(rd(this, 0x94)), x(rd(this, 0x28)), 0x41) {
        fl.write(fl.read() & 0xFE);
    }
    if rd::<u16>(active_cam(the_camera()), 0xC) != 4 {
        fl.write(fl.read() & 0xFE);
        g::<u32>(IDLE_TICKER).write(0);
    }
}

/// `CIdleCam::SetTargetPlayer()` (0x50EB50): `SetTarget(FindPlayerPed(-1))`, +0x78 = 1.
#[hook(addr = 0x50EB50, conv = thiscall, name = "CIdleCam::SetTargetPlayer", expect = "56 6A FF 8B F1", this = "CIdleCam")]
pub fn set_target_player(this: GPtr<u8>) {
    set_target(this, find_player_ped(-1));
    wr::<u8>(this, 0x78, 1);
}

/// `CIdleCam::GetLookAtPositionOnTarget(CEntity* target, CVector* out)` (0x50EAE0, `ret 8`; `this`
/// unused): `out` = the target's position (matrix +0x30, else placement +4); a ped (type bits 3)
/// in state +0x598 == 0x16 or 5 gets `out.z = [0x8CCC8C] + out.z`, any other ped `out.z = out.z + 0.5`
/// (0x858B8C).
#[hook(addr = 0x50EAE0, conv = thiscall, name = "CIdleCam::GetLookAtPositionOnTarget", expect = "8B 4C 24 04 8B 41 14", this = "CIdleCam")]
pub fn get_look_at_position_on_target(_this: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>) {
    let m: GPtr<u8> = rd(target, 0x14);
    let p = if m.is_null() {
        target.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    for o in [0, 4, 8] {
        wr::<u32>(out, o, rd::<u32>(p, o));
    }
    if rd::<u8>(target, 0x36) & 7 != 3 {
        return;
    }
    let st: i32 = rd(target, 0x598);
    let mut f = Fpu::new();
    if st == 0x16 || st == 5 {
        f.fld(g::<f32>(LOOK_AT_PED_LOW_Z).read());
        f.fadd_m(rd(out, 8));
    } else {
        f.fld(rd(out, 8));
        f.fadd_m(k(0x3F00_0000));
    }
    wr(out, 8, f.fstp());
}

/// `CIdleCam::ProcessIdleCamTicker()` (0x50A200): pad 0's `LastTimeTouched` unchanged since +0x90:
/// ticker +0x94 += `_ftol(timeStep * 0.02 * 1000)` (0x858B38, 0x858C4C); else +0x90 = it (a second
/// `GetPad(0)`), ticker 0.
#[hook(addr = 0x50A200, conv = thiscall, name = "CIdleCam::ProcessIdleCamTicker", expect = "56 6A 00 8B F1", this = "CIdleCam")]
pub fn process_idle_cam_ticker(this: GPtr<u8>) {
    let touched: u32 = rd(get_pad(0), 0x120);
    if rd::<u32>(this, 0x90) == touched {
        let n = ftol32(x(TIME_STEP.get()).fmul_m(k(0x3CA3_D70A)).fmul_m(k(0x447A_0000)));
        wr(this, 0x94, rd::<i32>(this, 0x94).wrapping_add(n));
        return;
    }
    let t: u32 = rd(get_pad(0), 0x120);
    wr(this, 0x90, t);
    wr::<u32>(this, 0x94, 0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IDLE_CAM_CONSTRUCTOR,
    HOOK_IDLE_CAM_INIT,
    HOOK_RESET,
    HOOK_IDLE_CAM_GENERAL_PROCESS,
    HOOK_SET_TARGET_PLAYER,
    HOOK_GET_LOOK_AT_POSITION_ON_TARGET,
    HOOK_PROCESS_IDLE_CAM_TICKER,
];
