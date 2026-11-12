//! PORT-VEH-4 (batch `veh4-1`): `CAutomobile::NitrousControl`. Ported from the 1.0 US bytes; gta-reversed
//! as a reading aid (the bytes win).
//!
//! The body has a protection detour after the status test (`nop; jmp 0x401CCB` at 0x6A3EAD): the fragment
//! is the relocated `mov ecx, [esi + 0x460]` and jumps straight back to 0x6A3EB3, so it is ordinary code.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x};

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetCarGunFired()` (0x53FF90): int16 in AX
    fn pad_get_car_gun_fired(pad: GPtr<u8>) -> i16 = thiscall 0x53FF90 => unported;
    /// `CPad::GetLookLeft()` (0x53FDD0)
    fn pad_get_look_left(pad: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    /// `CPad::GetLookRight()` (0x53FE10)
    fn pad_get_look_right(pad: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    /// `CPad::GetLookBehindForCar()` (0x53FE70)
    fn pad_get_look_behind_for_car(pad: GPtr<u8>) -> u8 = thiscall 0x53FE70 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CVehicle::RemoveUpgrade(int32 upgradeId)` (0x6D3630)
    fn vehicle_remove_upgrade(v: GPtr<u8>, id: u32) = thiscall 0x6D3630 => unported;
    /// `CAutomobile::DoNitroEffect(float power)` (0x6A3BD0)
    fn do_nitro_effect(v: GPtr<u8>, power: f32) = thiscall 0x6A3BD0 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C).
fn time_step() -> f32 {
    g::<f32>(0xB7_CB5C).read()
}

/// The inlined `StopNitroEffect`: kills and clears both exhaust fx systems (+0x978, +0x97C).
fn stop_nitro_effect(this: GPtr<u8>) {
    for o in [0x978, 0x97C] {
        let fx = rd::<GPtr<u8>>(this, o);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(this, o, 0);
        }
    }
}

/// Sets the boost count (+0x48A) and stops the nitro fx (the shared tail at 0x6A3EE5).
fn set_count_and_stop(this: GPtr<u8>, count: u8) {
    wr::<u8>(this, 0x48A, count);
    stop_nitro_effect(this);
}

/// `CAutomobile::NitrousControl(int8 boosts)` (0x6A3EA0).
///
/// `boosts > 0`: handling flag +0x38C bit 0x80000 set, the nitro level +0x8A4 = 1.0, count +0x48A =
/// `boosts`, fx stopped. `boosts < 0`: the flag cleared, level 1.0, count 0, fx stopped.
/// `boosts == 0`: with a full level (+0x8A4 bits == 1.0) and boosts left, a player-driven car (status 0,
/// a player driver's pad) fires the nitro on `GetCarGunFired` unless a look button is held (status-3
/// cars fire at once): level = -1e-6 and, below 101 boosts, one boost used; then the fx are stopped.
/// Otherwise a negative level burns down by `timeStep * 0.001` (at -1 it becomes +1e-6; with no boosts
/// left the flag is cleared, upgrade slot 15 removed and the level reset to 1.0) and the effect power is
/// `|gas| * 0.5 + 0.5` for gas > 0, else 0.5; a non-negative level refills by
/// `timeStep * 0.001 * max(1 - gas, 0.25)` up to 1.0 and the power is `(1 - level) * 0.5`.
///
/// Kept as in the original: the 101+ boost case still sets the level to -1e-6 (infinite nitro), and the
/// burn compares the unrounded ST0 against -1.0 after storing it.
#[hook(addr = 0x6A3EA0, conv = thiscall, name = "CAutomobile::NitrousControl", this = "CAutomobile",
       expect = "56 8B F1 8A 46 36",
       unverified = "veh4-1", route = "L2", hazards = "heap-writes",
       recs = "driving",
       note = "kills the exhaust FxSystems (+0x978/+0x97C) and may remove upgrade slot 15; the body's `nop; jmp 0x401CCB` detour is a relocated `mov ecx, [esi+0x460]`")]
pub fn automobile_nitrous_control(this: GPtr<u8>, boosts: u32) {
    let boosts = boosts as u8 as i8;
    let mut pad = GPtr::<u8>::NULL;
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let driver = rd::<GPtr<u8>>(this, 0x460);
        if ped_is_player(driver) != 0 {
            pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
        }
    }
    if boosts > 0 {
        wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) | 0x8_0000);
        wr::<u32>(this, 0x8A4, 0x3F80_0000);
        set_count_and_stop(this, boosts as u8);
        return;
    }
    if boosts < 0 {
        wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) & 0xFFF7_FFFF);
        wr::<u32>(this, 0x8A4, 0x3F80_0000);
        set_count_and_stop(this, 0);
        return;
    }
    if rd::<u32>(this, 0x8A4) == 0x3F80_0000 && (rd::<u8>(this, 0x48A) as i8) > 0 {
        if rd::<u8>(this, 0x36) & 0xF8 != 0x18
            && (pad.is_null()
                || pad_get_car_gun_fired(pad) == 0
                || pad_get_look_left(pad) != 0
                || pad_get_look_right(pad) != 0
                || pad_get_look_behind_for_car(pad) != 0)
        {
            stop_nitro_effect(this);
            return;
        }
        let count = rd::<u8>(this, 0x48A);
        wr::<u32>(this, 0x8A4, 0xB586_37BD);
        if (count as i8) >= 0x65 {
            stop_nitro_effect(this);
            return;
        }
        set_count_and_stop(this, count.wrapping_sub(1));
        return;
    }
    let level = rd::<f32>(this, 0x8A4);
    // `fcomp qword 0.0; test ah, 5; jp`: falls through only when level < 0
    if !jp5(x(level), x(0.0)) {
        let st = x(time_step()).fmul_m(f32::from_bits(0x3A83_126F)).fsubr_m(level);
        wr::<f32>(this, 0x8A4, f32_of(st));
        if !jp5(st, x(-1.0)) {
            let count = rd::<u8>(this, 0x48A);
            wr::<u32>(this, 0x8A4, 0x3586_37BD);
            if count == 0 {
                wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) & 0xFFF7_FFFF);
                vehicle_remove_upgrade(this, 0xF);
                wr::<u32>(this, 0x8A4, 0x3F80_0000);
            }
        }
        let gas = rd::<f32>(this, 0x49C);
        // `test ah, 0x41; jne`: gas > 0 falls through
        if !jne_mask(x(gas), x(0.0), 0x41) {
            let p = x(gas).fabs().fmul_m(0.5).fadd_m(0.5);
            do_nitro_effect(this, f32_of(p));
        } else {
            do_nitro_effect(this, 0.5);
        }
        return;
    }
    let mut t = x(1.0).fsub_m(rd::<f32>(this, 0x49C));
    // `fld 0.25; fcomp st(1); test ah, 0x41; jne`: 0.25 > t falls through (t = 0.25)
    if !jne_mask(x(0.25), t, 0x41) {
        t = x(0.25);
    }
    let st = x(time_step()).fmul_m(f32::from_bits(0x3A83_126F)).fmul(t).fadd_m(rd::<f32>(this, 0x8A4));
    wr::<f32>(this, 0x8A4, f32_of(st));
    if !jne_mask(st, x(1.0), 0x41) {
        wr::<u32>(this, 0x8A4, 0x3F80_0000);
    }
    let p = x(1.0).fsub_m(rd::<f32>(this, 0x8A4)).fmul_m(0.5);
    do_nitro_effect(this, f32_of(p));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_NITROUS_CONTROL];
