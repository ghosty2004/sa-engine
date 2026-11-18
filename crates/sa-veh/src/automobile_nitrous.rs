//! PORT-VEH-4 (batch `veh4-1`): `CAutomobile::NitrousControl`. Ported from the 1.0 US bytes; gta-reversed
//! as a reading aid (the bytes win).
//!
//! The body has a protection detour after the status test (`nop; jmp 0x401CCB` at 0x6A3EAD): the fragment
//! is the relocated `mov ecx, [esi + 0x460]` and jumps straight back to 0x6A3EB3, so it is ordinary code.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, jne_mask, jp5, rd, wr, x};

use sa_pop::bindings::get_look_behind_for_car as pad_get_look_behind_for_car;
use sa_pop::bindings::pad_get_look_left;
use sa_pop::bindings::pad_get_look_right;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::fx_kill;
use sa_world::bindings::get_pad_from_player;
use sa_world::bindings::pad_get_car_gun_fired;
use sa_core::bindings::{fx_play, fx_stop, matrix_mul_vec};
use sa_world::bindings::{fx_create_system, fx_get_play_status, fx_set_const_time, fx_set_local_particles, get_water_level};
use sa_core::field::{gf, ptr_of as vp};
use sa_math::vector::CVector;
use crate::common::model_info;
use crate::mvpd_v1::read_vec;
orig! {
    /// `CVehicle::RemoveUpgrade(int32 upgradeId)` (0x6D3630)
    fn vehicle_remove_upgrade(v: GPtr<u8>, id: u32) = thiscall 0x6D3630 => unported;
}

use sa_pop::carctrl3::time_step;
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
/// `g_fxMan` (FxManager_c), the string "nitro", -1.0f.
const FX_MAN: usize = 0xA9_AE80;
const S_NITRO: usize = 0x87_1114;
const K_MINUS_ONE: usize = 0x85_8C1C;

/// The exhaust's water test: `p = m_matrix * e`; under water when `GetWaterLevel(p, &level, true, null)` finds a
/// level and `!(level < p.z)` (`fcomp; test ah, 1; jne`).
fn exhaust_under_water(this: GPtr<u8>, e: &mut CVector) -> bool {
    let mut out = CVector::default();
    let p = read_vec(matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(e)));
    let mut level = 0f32;
    get_water_level(p.x, p.y, p.z, vp(&mut level), 1, GPtr::NULL) != 0 && !x(level).fcom(x(p.z)).c0()
}

/// One exhaust's fx system at `slot`: none yet, with `create` (the first exhaust's water flag clear and a frame
/// matrix) `CreateFxSystem("nitro", e, rwm, true)`, stored, then local particles and `Play()` (the original returns
/// right after a created second system). An existing one gets `SetConstTime(1, |power|)`, then
/// `Stop()` when playing (status 0) under water, else `Play()` when stopped (status 1, re-read) above it.
fn exhaust_fx(this: GPtr<u8>, slot: isize, create: bool, e: &mut CVector, rwm: GPtr<u8>, power: f32, wet: bool) {
    let fx = rd::<GPtr<u8>>(this, slot);
    if fx.is_null() {
        if create && !rwm.is_null() {
            let fx = fx_create_system(GPtr::from_addr(FX_MAN), GPtr::from_addr(S_NITRO), vp(e), rwm, 1);
            wr::<GPtr<u8>>(this, slot, fx);
            if !fx.is_null() {
                fx_set_local_particles(fx, 1);
                fx_play(rd::<GPtr<u8>>(this, slot));
            }
        }
        return;
    }
    fx_set_const_time(fx, 1, power.abs());
    if fx_get_play_status(rd::<GPtr<u8>>(this, slot)) == 0 && wet {
        fx_stop(rd::<GPtr<u8>>(this, slot));
    } else if fx_get_play_status(rd::<GPtr<u8>>(this, slot)) == 1 && !wet {
        fx_play(rd::<GPtr<u8>>(this, slot));
    }
}

/// `CAutomobile::DoNitroEffect(float power)` (0x6A3BD0, thiscall, `ret 4`).
///
/// The exhaust dummy (vehicle struct +0x48) and, for a `DOUBLE_EXHAUST` handling (+0xCC bit 0x2000), its mirror
/// (x * -1.0 as an f32 store). With bTouchingWater (+0x40 bit 27) each is tested for being under water (the
/// mirror only then, unlike gta-reversed). `rwm` is the RW frame's modelling matrix (frame + 0x10) or null. The
/// first fx system (+0x978) follows the first exhaust; the second (+0x97C) is created only when the first exhaust
/// is dry (the bytes test the first flag) and then follows the second.
#[hook(addr = 0x6A3BD0, conv = thiscall, name = "CAutomobile::DoNitroEffect", this = "CAutomobile",
       expect = "83 EC 2C 53 56 8B F1", unverified = "frontier2-b25", route = "L2", hazards = "heap-writes",
       note = "creates / plays / stops the exhaust fx systems")]
pub fn do_nitro_effect(this: GPtr<u8>, power: f32) {
    let e1p = rd::<GPtr<u8>>(model_info(rd::<i16>(this, 0x22) as i32), 0x5C).byte_add(0x48);
    let mut e1 = read_vec(e1p);
    let wet_water = rd::<u32>(this, 0x40) & 0x0800_0000 != 0;
    let wet1 = wet_water && exhaust_under_water(this, &mut e1);
    let double = || rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x2000 != 0;
    let mut e2 = CVector::default();
    let mut wet2 = false;
    if double() {
        e2 = CVector::new(f32_of(x(e1.x).fmul_m(gf(K_MINUS_ONE))), e1.y, e1.z);
        if rd::<u32>(this, 0x40) & 0x0800_0000 != 0 {
            wet2 = exhaust_under_water(this, &mut e2);
        }
    }
    let obj = rd::<GPtr<u8>>(this, 0x18);
    let rwm = if obj.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(obj, 4).byte_add(0x10) };
    exhaust_fx(this, 0x978, !wet1, &mut e1, rwm, power, wet1);
    if double() {
        exhaust_fx(this, 0x97C, !wet1, &mut e2, rwm, power, wet2);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_NITROUS_CONTROL, HOOK_DO_NITRO_EFFECT];
