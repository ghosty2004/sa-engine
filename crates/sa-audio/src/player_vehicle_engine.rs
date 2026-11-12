//! STD-15 (batch `std15-audio-1`): `CAEVehicleAudioEntity::ProcessPlayerVehicleEngine(cVehicleParams&)` (0x4FBB10).
//! Ported from the 1.0 US bytes (gta-reversed's body is restructured; the branch order, the repeated
//! `IsAccInhibitedForLowSpeed` calls, the call order of the volume / frequency getters and the state the shared
//! idle tail stores (EBX = 6) follow the exe). Unverified (docs/verify/HOWTO.md).
//!
//! `cVehicleParams` fields used: +4 vehicle type (0 automobile, 9 bike), +0x10 vehicle, +0x14 transmission,
//! +0x1C speed, +0x20 i16 accel, +0x24 |speed|, +0x28 z over speed, +0x2C speed ratio, +0x30 gas pedal revs ptr,
//! +0x34 previous speed, +0x38 u8 gear, +0x39 handbrake, +0x3C revs ratio, +0x40 wheel spin, +0x44 i16 gears,
//! +0x46 / +0x47 drive wheels on ground (now / last frame), +0x48 wheel states ptr. Entity: +0xA9 i8 state
//! (0 off, 3 accelerating, 4 wheel spin, 5 cruise, 6 idle), +0xAA u8 audio gear, +0xAC cruise count, +0xB0
//! single gear, +0xD0 / +0xD4 inhibit times (ms), +0xDC / +0xE0 the dummy engine bank / slot, +0xE8 + 8 type
//! the engine sounds.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x, GPtr, X87};

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAEVehicleAudioEntity::GetAccelAndBrake(cVehicleParams&)` (0x4F5080)
    fn get_accel_and_brake(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F5080 => unported;
    /// `CAEVehicleAudioEntity::IsAccInhibited(cVehicleParams&)` (0x4F4F70)
    fn is_acc_inhibited(this: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x4F4F70 => unported;
    /// `CAEVehicleAudioEntity::JustFinishedAccelerationLoop()` (0x4F5E50)
    fn just_finished_acceleration_loop(this: GPtr<u8>) -> u8 = thiscall 0x4F5E50 => unported;
    /// `CAEVehicleAudioEntity::CancelVehicleEngineSound(int16)` (0x4F55C0)
    fn cancel_vehicle_engine_sound(this: GPtr<u8>, ty: u32) = thiscall 0x4F55C0 => unported;
    /// `CAEVehicleAudioEntity::UpdateVehicleEngineSound(int16, float speed, float volume)` (0x4F56D0)
    fn update_vehicle_engine_sound(this: GPtr<u8>, ty: u32, speed: u32, vol: u32) -> u32 = thiscall 0x4F56D0 => unported;
    /// `CAEVehicleAudioEntity::RequestNewPlayerCarEngineSound(int16, float speed, float volume)` (0x4F7A50)
    fn request_new_player_car_engine_sound(this: GPtr<u8>, ty: u32, speed: u32, vol: u32) -> u32 = thiscall 0x4F7A50 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn add_audio_event(this: GPtr<u8>, ev: u32, vol: u32) = thiscall 0x4F6420 => unported;
    /// `CAESound::SetFlags(uint16, uint16)` (0x4EF2B0)
    fn sound_set_flags(s: GPtr<u8>, mask: u32, on: u32) = thiscall 0x4EF2B0 => unported;
    /// `CAESound::StopSound()` (0x4EF1C0)
    fn sound_stop(s: GPtr<u8>) = thiscall 0x4EF1C0 => unported;
    /// `CCullZones::DoExtraAirResistanceForPlayer()` (0x72DDD0)
    fn do_extra_air_resistance_for_player() -> u8 = cdecl 0x72DDD0 => unported;
}

const AE_AUDIO_HARDWARE: usize = 0xB5F8B8;
const TIME_MS: usize = 0xB7CB84;
/// `int16` wheel-spin accel threshold
const WHEEL_SPIN_THRESHOLD: usize = 0x8CBC54;
/// `int16` cruise count limit
const CRZ_COUNT_LIMIT: usize = 0x8CBC8C;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

struct Ctx {
    this: GPtr<u8>,
    p: GPtr<u8>,
}

impl Ctx {
    fn accel(&self) -> i16 {
        rd::<i16>(self.p, 0x20)
    }
    fn at_or_below_wheel_spin(&self) -> bool {
        self.accel() <= g::<i16>(WHEEL_SPIN_THRESHOLD).read()
    }
    fn vol(&self, ty: u32) -> u32 {
        crate::vehicle::get_vol_for_player_engine_sound(self.this, self.p, ty)
            .to_f32()
            .to_bits()
    }
    fn freq(&self, ty: u32) -> u32 {
        crate::port3_veh::get_freq_for_player_engine_sound(self.this, self.p, ty)
            .to_f32()
            .to_bits()
    }
    /// vol, freq, `RequestNewPlayerCarEngineSound`
    fn request(&self, ty: u32) {
        let v = self.vol(ty);
        let f = self.freq(ty);
        request_new_player_car_engine_sound(self.this, ty, f, v);
    }
    /// vol, freq, `UpdateVehicleEngineSound`
    fn update(&self, ty: u32) {
        let v = self.vol(ty);
        let f = self.freq(ty);
        update_vehicle_engine_sound(self.this, ty, f, v);
    }
    fn cancel(&self, ty: u32) {
        cancel_vehicle_engine_sound(self.this, ty);
    }
    /// The inlined stop of the engine sound in slot `ty` (+0xE8 + 8 ty): flags 4 off, `StopSound`, forget.
    fn stop_slot(&self, ty: isize) {
        let off = 0xE8 + 8 * ty;
        let s = rd::<GPtr<u8>>(self.this, off);
        if !s.is_null() {
            sound_set_flags(s, 4, 0);
            sound_stop(rd::<GPtr<u8>>(self.this, off));
            wr::<u32>(self.this, off, 0);
        }
    }
    fn set_state(&self, s: u8) {
        wr::<u8>(self.this, 0xA9, s);
    }
    fn acc_inhibited_for_low_speed(&self) -> bool {
        crate::vehicle::is_acc_inhibited_for_low_speed(self.this, self.p) != 0
    }
    /// gear < gears (int16), `0.001 + previous speed > speed` and z over speed > 0.2
    fn cruise_gear_and_speed(&self) -> bool {
        let rg = i16::from(rd::<u8>(self.p, 0x38));
        if rg >= rd::<i16>(self.p, 0x44) {
            return false;
        }
        if x(gf(0x8CBD34))
            .fadd_m(rd::<f32>(self.p, 0x34))
            .fcom(x(rd::<f32>(self.p, 0x1C)))
            .test_any(0x41)
        {
            return false;
        }
        true
    }
    fn z_over_speed_above(&self) -> bool {
        !x(rd::<f32>(self.p, 0x28)).fcom(x(gf(0x8CBD38))).test_any(0x41)
    }
    fn mark_changing_gear(&self) {
        let v = rd::<GPtr<u8>>(self.p, 0x10);
        wr::<u8>(v, 0x42B, rd::<u8>(v, 0x42B) | 0x20);
    }
    /// The audio gear (+0xAA): 1 for single-gear vehicles, else `min(min(max(gear, 1), audio gear), the
    /// transmission's gears - 1)` (signed).
    fn set_audio_gear(&self) {
        if rd::<u8>(self.this, 0xB0) != 0 {
            wr::<u8>(self.this, 0xAA, 1);
            return;
        }
        let rg = i32::from(rd::<u8>(self.p, 0x38)).max(1);
        let au = i32::from(rd::<u8>(self.this, 0xAA));
        let a = if rg < au { rg } else { au };
        let n = i32::from(rd::<u8>(rd::<GPtr<u8>>(self.p, 0x14), 0x4A)) - 1;
        let r = if a < n { a } else { n };
        wr::<u8>(self.this, 0xAA, r as u8);
    }
    /// 0x4FC997: the cruise and idle requests, then idle (EBX = 6)
    fn to_idle_via_off(&self) {
        self.request(5);
        self.to_idle();
    }
    /// 0x4FC9BC
    fn to_idle(&self) {
        self.request(2);
        self.set_state(6);
    }
    /// 0x4FBD9C
    fn to_wheel_spin(&self) {
        self.request(1);
        self.set_state(4);
    }
    /// 0x4FBE4C
    fn to_cruise(&self) {
        self.request(3);
        self.set_state(5);
    }
    /// 0x4FC8FE
    fn to_accel(&self) {
        self.request(4);
        self.set_state(3);
    }
    /// 0x4FC53F
    fn idle_or_wheel_spin(&self) {
        if self.at_or_below_wheel_spin() {
            self.to_idle_via_off();
        } else {
            self.to_wheel_spin();
        }
    }
    fn engine_off(&self, skip9: bool) {
        for t in 0..12u32 {
            if skip9 && t == 9 {
                continue;
            }
            self.cancel(t);
        }
        self.set_state(0);
    }
}

/// `CAEVehicleAudioEntity::ProcessPlayerVehicleEngine(cVehicleParams&)` (0x4FBB10, thiscall, `ret 4`). Without the
/// dummy engine bank loaded it returns at once. It fills the params (`GetAccelAndBrake`, |speed|, the speed ratio
/// and the revs ratio of the current gear clamped to 0..1 (the latter against doubles), gear, handbrake, wheel
/// spin, gears, z over speed (0 at speed 0), the drive-wheel and gas-pedal fields of automobiles / bikes; other
/// types return) and runs the engine-sound state machine (off, accelerating, wheel spin, cruise, idle).
#[hook(addr = 0x4FBB10, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessPlayerVehicleEngine",
       expect = "51 56 8B F1 33 C9 66 8B 8E E0 00 00 00", unverified = "std15-audio-1", route = "L2",
       hazards = "none", recs = "car-drive",
       note = "the idle tail stores the state from EBX (6), as the exe; the cruise test reads the transmission's gear count")]
pub fn process_player_vehicle_engine(this: GPtr<u8>, p: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(p, 0x10);
    if hw_is_sound_bank_loaded(
        GPtr::from_addr(AE_AUDIO_HARDWARE),
        u32::from(rd::<u16>(this, 0xDC)),
        u32::from(rd::<u16>(this, 0xE0)),
    ) == 0
    {
        return;
    }
    get_accel_and_brake(this, p);
    let tr = rd::<GPtr<u8>>(p, 0x14);
    let abs = x(rd::<f32>(p, 0x1C)).fabs();
    wr::<f32>(p, 0x24, abs.to_f32());
    let one = X87::from_f32(gf(0x858624));
    let zero = X87::from_f32(gf(0x858B50));
    let mut r = abs.fdiv_m(rd::<f32>(tr, 0x58));
    if !one.fcom(r).test_parity(0x05) {
        // 1 < r
        if !one.fcom(r).test_parity(0x05) {
            r = one;
        }
    } else if !zero.fcom(r).test_any(0x41) {
        r = zero;
    } else if !one.fcom(r).test_parity(0x05) {
        r = one;
    }
    wr::<f32>(p, 0x2C, r.to_f32());
    let gear = rd::<u8>(veh, 0x4B4);
    wr::<u8>(p, 0x38, gear);
    wr::<u8>(p, 0x39, (rd::<u8>(veh, 0x428) >> 5) & 1);
    let ge = tr.byte_add(isize::from(gear) * 12);
    let cd = rd::<f32>(ge, 8);
    let mut q = abs.fsub_m(cd).fdiv(x(rd::<f32>(ge, 0)).fsub_m(cd));
    let d1 = X87::from_f64_exact(1.0);
    let d0 = X87::from_f64_exact(0.0);
    if q.fcom(d1).test_any(0x41) && !d0.fcom(q).test_any(0x41) {
        q = d0;
    } else if !q.fcom(d1).test_any(0x41) {
        q = d1;
    }
    wr::<f32>(p, 0x3C, q.to_f32());
    if gear == 0 {
        wr::<f32>(p, 0x3C, 0.0);
    }
    wr::<u32>(p, 0x40, rd::<u32>(veh, 0x4BC));
    let reversing = !x(rd::<f32>(p, 0x1C)).fcom(x(gf(0x862CF0))).test_parity(0x05);
    wr::<u16>(p, 0x44, u16::from(rd::<u8>(tr, 0x4A)));
    let spd = rd::<f32>(p, 0x1C);
    if x(spd).fcom(zero).test_parity(0x44) {
        wr::<f32>(p, 0x28, x(rd::<f32>(veh, 0x4C)).fdiv_m(spd).to_f32());
    } else {
        wr::<f32>(p, 0x28, 0.0);
    }
    let (o46, o47, o30, o48, o34) = match rd::<i32>(p, 4) {
        0 => (0x961, 0x962, 0x964, 0x968, 0x8B0),
        9 => (0x805, 0x806, 0x808, 0x80C, 0x7B8),
        _ => return,
    };
    wr::<u8>(p, 0x46, rd::<u8>(veh, o46));
    wr::<u8>(p, 0x47, rd::<u8>(veh, o47));
    wr::<u32>(p, 0x30, veh.addr() as u32 + o30 as u32);
    wr::<u32>(p, 0x48, veh.addr() as u32 + o48 as u32);
    wr::<u32>(p, 0x34, rd::<u32>(veh, o34));
    if rd::<u8>(this, 0xB0) != 0 {
        wr::<u8>(p, 0x38, if gear >= 1 { 1 } else { gear });
        wr::<i16>(p, 0x44, 1);
    }
    let c = Ctx { this, p };
    let engine_on = || rd::<u8>(veh, 0x428) & 0x10 != 0;
    let now = || g::<u32>(TIME_MS).read();
    match rd::<i8>(this, 0xA9) {
        0 => {
            if !engine_on() {
                c.engine_off(true);
                return;
            }
            add_audio_event(this, 0x67, 0);
            let wheel_spin_path = if reversing {
                true
            } else {
                if c.accel() <= 0 {
                    request_new_player_car_engine_sound(this, 2, 1.0f32.to_bits(), (-100.0f32).to_bits());
                    c.set_state(6);
                    return;
                }
                if is_acc_inhibited(this, p) != 0 || c.acc_inhibited_for_low_speed() {
                    true
                } else {
                    if c.acc_inhibited_for_low_speed() {
                        c.to_idle();
                        return;
                    }
                    if c.at_or_below_wheel_spin() {
                        request_new_player_car_engine_sound(this, 2, 1.0f32.to_bits(), (-100.0f32).to_bits());
                        c.set_state(6);
                        return;
                    }
                    if c.cruise_gear_and_speed() && c.z_over_speed_above() {
                        c.mark_changing_gear();
                        c.to_cruise();
                    } else {
                        c.stop_slot(5);
                        c.to_accel();
                    }
                    return;
                }
            };
            if wheel_spin_path {
                if c.at_or_below_wheel_spin() {
                    c.to_idle();
                } else {
                    c.stop_slot(5);
                    c.to_wheel_spin();
                }
            }
        }
        5 => {
            if !engine_on() {
                c.engine_off(false);
                return;
            }
            let end_crz = || {
                wr::<f32>(this, 0xAC, 0.0);
                c.stop_slot(3);
            };
            if reversing {
                c.stop_slot(3);
                wr::<f32>(this, 0xAC, 0.0);
                c.idle_or_wheel_spin();
                return;
            }
            if c.accel() <= 0 {
                end_crz();
                c.to_idle_via_off();
                return;
            }
            if is_acc_inhibited(this, p) != 0 || c.acc_inhibited_for_low_speed() {
                end_crz();
                c.idle_or_wheel_spin();
                return;
            }
            if c.acc_inhibited_for_low_speed() || c.at_or_below_wheel_spin() {
                end_crz();
                c.to_idle_via_off();
                return;
            }
            let prev = rd::<f32>(p, 0x34);
            let speed = rd::<f32>(p, 0x1C);
            let up = !x(gf(0x8CBD30)).fadd_m(prev).fcom(x(speed)).test_parity(0x05)
                && i16::from(rd::<u8>(p, 0x38)) < rd::<i16>(p, 0x44)
                && rd::<u32>(this, 0xD0) >= now()
                && do_extra_air_resistance_for_player() == 0;
            if up {
                end_crz();
                c.set_audio_gear();
                c.request(4);
                wr::<u32>(this, 0xD4, now().wrapping_add(2000));
                c.set_state(3);
                return;
            }
            let slowing = x(prev).fsub_m(gf(0x8CBC90)).fcom(x(speed));
            let crz = x(rd::<f32>(this, 0xAC));
            let n = if slowing.test_parity(0x05) {
                let v = crz.fsub_m(gf(0x858C84));
                if v.fcom(zero).test_any(0x41) {
                    zero
                } else {
                    v
                }
            } else {
                let v = crz.fadd_m(gf(0x858624));
                let lim = f32::from(g::<i16>(CRZ_COUNT_LIMIT).read());
                if v.fcom(x(lim)).test_parity(0x05) {
                    x(lim)
                } else {
                    v
                }
            };
            wr::<f32>(this, 0xAC, n.to_f32());
            c.update(3);
            c.set_state(5);
        }
        6 => {
            if !engine_on() {
                c.engine_off(false);
                return;
            }
            let leave_idle_for_spin = || {
                c.stop_slot(2);
                c.stop_slot(5);
                c.to_wheel_spin();
            };
            if reversing {
                if c.at_or_below_wheel_spin() {
                    c.update(2);
                    c.set_state(6);
                } else {
                    leave_idle_for_spin();
                }
                return;
            }
            if c.accel() <= 0 {
                c.update(2);
                c.set_state(6);
                return;
            }
            if is_acc_inhibited(this, p) != 0 || c.acc_inhibited_for_low_speed() {
                if c.at_or_below_wheel_spin() {
                    c.update(2);
                    c.set_state(6);
                } else {
                    leave_idle_for_spin();
                }
                return;
            }
            if c.acc_inhibited_for_low_speed() || c.at_or_below_wheel_spin() {
                c.update(2);
                c.set_state(6);
                return;
            }
            if c.cruise_gear_and_speed() && c.z_over_speed_above() {
                c.mark_changing_gear();
                c.stop_slot(5);
                c.stop_slot(2);
                c.to_cruise();
                return;
            }
            c.stop_slot(5);
            c.stop_slot(2);
            c.set_audio_gear();
            c.to_accel();
        }
        3 => {
            if !engine_on() {
                c.engine_off(false);
                return;
            }
            if reversing {
                c.cancel(4);
                if c.at_or_below_wheel_spin() {
                    c.to_idle_via_off();
                } else {
                    c.to_wheel_spin();
                }
                return;
            }
            if c.accel() <= 0 {
                c.cancel(4);
                c.to_idle_via_off();
                return;
            }
            if is_acc_inhibited(this, p) != 0 {
                c.cancel(4);
                c.idle_or_wheel_spin();
                return;
            }
            if c.acc_inhibited_for_low_speed()
                || c.acc_inhibited_for_low_speed()
                || c.at_or_below_wheel_spin()
            {
                c.cancel(4);
                c.to_idle_via_off();
                return;
            }
            let up = c.cruise_gear_and_speed()
                && (c.z_over_speed_above() || do_extra_air_resistance_for_player() != 0)
                && rd::<u32>(this, 0xD4) >= now();
            if up {
                c.mark_changing_gear();
                c.cancel(4);
                c.request(3);
                wr::<u32>(this, 0xD0, now().wrapping_add(2000));
                c.set_state(5);
                return;
            }
            if just_finished_acceleration_loop(this) != 0 {
                let au = i32::from(rd::<u8>(this, 0xAA));
                let ng = i32::from(rd::<i16>(p, 0x44));
                let more = au + 2 <= ng
                    || (i32::from(rd::<u8>(p, 0x38)) < ng && do_extra_air_resistance_for_player() == 0);
                let next = (au + 1).min(5) as u8;
                wr::<u8>(this, 0xAA, next);
                if more {
                    c.mark_changing_gear();
                    c.update(4);
                    c.set_state(3);
                } else {
                    if rd::<u8>(this, 0xB0) == 0 {
                        c.mark_changing_gear();
                    }
                    c.cancel(4);
                    c.to_cruise();
                }
                return;
            }
            let f = c.freq(4);
            let v = c.vol(4);
            if rd::<u32>(this, 0x108) == 0 {
                request_new_player_car_engine_sound(this, 4, f, v);
            } else {
                update_vehicle_engine_sound(this, 4, f, v);
            }
            c.set_state(3);
        }
        4 => {
            if !engine_on() {
                c.engine_off(false);
                return;
            }
            let spin_to_idle = || {
                c.stop_slot(1);
                c.to_idle_via_off();
            };
            if reversing {
                if c.at_or_below_wheel_spin() {
                    spin_to_idle();
                } else {
                    c.update(1);
                    c.set_state(4);
                }
                return;
            }
            if c.accel() <= 0 {
                spin_to_idle();
                return;
            }
            if is_acc_inhibited(this, p) != 0 || c.acc_inhibited_for_low_speed() {
                if c.at_or_below_wheel_spin() {
                    spin_to_idle();
                } else {
                    c.update(1);
                    c.set_state(4);
                }
                return;
            }
            if c.acc_inhibited_for_low_speed() || c.at_or_below_wheel_spin() {
                spin_to_idle();
                return;
            }
            if c.cruise_gear_and_speed() && c.z_over_speed_above() {
                c.mark_changing_gear();
                c.stop_slot(1);
                c.to_cruise();
                return;
            }
            c.stop_slot(1);
            c.set_audio_gear();
            c.to_accel();
        }
        _ => {}
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_PLAYER_VEHICLE_ENGINE];

/// `CAEVehicleAudioEntity::GetFreqForIdle(float ratio)` (0x4F5C60, `ret 4`, ECX unused; the callers pass `this`),
/// ST0: [`crate::vehicle::get_freq_for_idle`]. Registered `#[native]` (phases B/C only): its segment loop jumps
/// back to 0x4F5C62, inside the 5 bytes a hook patches. Reached by `GetFreqForPlayerEngineSound` once the player
/// sits in a car.
#[sa_core::native(addr = 0x4F5C60, conv = thiscall, name = "CAEVehicleAudioEntity::GetFreqForIdle")]
pub fn get_freq_for_idle_native(_this: GPtr<u8>, ratio: u32) -> X87 {
    crate::vehicle::get_freq_for_idle(f32::from_bits(ratio))
}

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_GET_FREQ_FOR_IDLE_NATIVE];
