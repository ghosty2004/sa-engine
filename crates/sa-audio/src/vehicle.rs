//! `CAEVehicleAudioEntity` (PORT-AUDIO).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//!
//! `cVehicleParams` (the `params` argument): +0x10 vehicle, +0x1C speed, +0x22 int16, +0x39 byte,
//! +0x40 float, +0x46 byte. The `s_Config` tuning values (0x8CBC24.., `.data`) are read from memory.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{
    AE_SOUND_MANAGER, ae_random_int, audio_engine, audio_hardware, ftol32, g, rd, wr, x, xi,
};
use crate::entities::load_sound_bank;
use crate::utility::call_audio_log10;
use sa_core::timer::TIME_MS;

sa_core::globals! {
    list GLOBALS;
    /// `CAEVehicleAudioEntity::s_pPlayerAttachedForRadio` (`CPed*`)
    pub static PLAYER_ATTACHED_FOR_RADIO @ 0xB6B98C: u32 = 0;
    /// `CAEVehicleAudioEntity::s_pVehicleAudioSettingsForRadio` (`tVehicleAudioSettings*`)
    pub static VEHICLE_AUDIO_SETTINGS_FOR_RADIO @ 0xB6B99C: u32 = 0;
    /// `CAEVehicleAudioEntity::s_pPlayerDriver` (`CPed*`)
    pub static PLAYER_DRIVER @ 0xB6B990: u32 = 0;
    /// `CAEVehicleAudioEntity::s_HelicoptorsDisabled`
    pub static HELICOPTORS_DISABLED @ 0xB6B994: u8 = 0;
    /// `CAEVehicleAudioEntity::s_NextDummyEngineSlot`
    pub static NEXT_DUMMY_ENGINE_SLOT @ 0xB6B998: u16 = 0;
    /// `CAEVehicleAudioEntity::s_DummyEngineSlots[10]` (`{ int16 bankSlot; int16 refCount; }`)
    pub static DUMMY_ENGINE_SLOTS @ 0xB6B9A0: [u16; 20] = [0; 20];
}

/// `s_Config.PlayerEngine.AC.InhibitForLowSpeedLimit` (float, `.data`)
const AC_INHIBIT_FOR_LOW_SPEED_LIMIT: usize = 0x8CBCD0;
/// `s_Config.PlayerEngine.AC.LoopInterval` (int16, `.data`)
const AC_LOOP_INTERVAL: usize = 0x8CBCBC;
/// `s_Config.PlayerEngine.AC.LoopFrameCnt` (int16, `.data`)
const AC_LOOP_FRAME_CNT: usize = 0x8CBCC0;
/// Gas pedal audio step up (float, `.data`)
const GAS_PEDAL_STEP_UP: usize = 0x8CBC24;
/// Gas pedal audio step down (float, `.data`)
const GAS_PEDAL_STEP_DOWN: usize = 0x8CBC28;
/// `GetFreqForIdle` curve: five (x, y) float pairs (`.data`)
const IDLE_FREQ_CURVE: usize = 0x8CC164;
/// `GetBaseVolumeForBicycleTyre` curve: five (x, y) float pairs (`.data`)
const BICYCLE_TYRE_VOLUME_CURVE: usize = 0x8CC18C;
/// 1.0 (double, `.rdata`)
const DBL_1: usize = 0x85A310;
/// 0.0f (`.rdata`)
const FLT_0: usize = 0x858B50;
/// `s_Config` "non-drive wheel skid enabled" byte (`.data`)
const NON_DRIVE_SKID_ENABLED: usize = 0x8CBD81;
/// Non-drive wheel skid factors (`.data`): spinning, sliding
const NON_DRIVE_SKID_SPIN_MULT: usize = 0x8CBD8C;
const NON_DRIVE_SKID_SLIDE_MULT: usize = 0x8CBD84;
/// Horn pattern table: 0x2C-byte rows of on/off bytes, indexed `pattern * 0x2C - counter`
const HORN_PATTERNS: usize = 0x862B9C;
/// `GetFlyingMetalVolume` constants (`.rdata`): speed^2 clamp, scale, log threshold
const FLYING_METAL_MAX_SPEED_SQ: usize = 0x862CE8;
const FLYING_METAL_SCALE: usize = 0x862CE4;
const FLYING_METAL_MIN: usize = 0x862CE0;
/// -100.0f, 10.0f, 1.0f, 0.04f, 0.75f, 0.2f (`.rdata`)
const FLT_M100: usize = 0x859014;
const FLT_10: usize = 0x85862C;
const FLT_1: usize = 0x858624;
const FLT_0_04: usize = 0x858CEC;
const FLT_0_75: usize = 0x858F34;
const FLT_0_2: usize = 0x858CC4;
/// Dummy engine tuning (`.data`): idle ratio divisor, rev ratio offset, idle / rev base frequency,
/// "accelerate-ish" factor
const DUMMY_IDLE_RATIO_DIV: usize = 0x8CBBF0;
const DUMMY_REV_RATIO_OFFSET: usize = 0x8CBBF4;
const DUMMY_IDLE_FREQ_BASE: usize = 0x8CBBF8;
const DUMMY_REV_FREQ_BASE: usize = 0x8CBC0C;
const DUMMY_FREQ_FLAG_MULT: usize = 0x8CBC48;
/// Frequency ranges computed by the static initialisers (`.bss`): idle (0xB6BA20), rev (0xB6BA28)
const DUMMY_IDLE_FREQ_RANGE: usize = 0xB6BA20;
const DUMMY_REV_FREQ_RANGE: usize = 0xB6BA28;
/// Five-point curves (`.data`) used for audio type 2 (`GetPiecewiseLinear(fwd, 5, ..)`)
const DUMMY_IDLE_FWD_CURVE: usize = 0x8CC0EC;
const DUMMY_REV_FWD_CURVE: usize = 0x8CC13C;
/// Dummy engine volume tuning (`.data`): idle / rev base volume, flag offset, "+0x4C8" offset
const DUMMY_IDLE_VOL_BASE: usize = 0x8CBC00;
const DUMMY_REV_VOL_BASE: usize = 0x8CBC14;
const DUMMY_VOL_FLAG_OFFSET: usize = 0x8CBC44;
const DUMMY_VOL_TRAILER_OFFSET: usize = 0x8CBC40;
/// Volume ranges computed by the static initialisers (`.bss`): idle (0xB6BA24), rev (0xB6BA2C)
const DUMMY_IDLE_VOL_RANGE: usize = 0xB6BA24;
const DUMMY_REV_VOL_RANGE: usize = 0xB6BA2C;
/// Five-point curves (`.data`) for audio type 2 volumes
const DUMMY_IDLE_VOL_CURVE: usize = 0x8CC0C4;
const DUMMY_REV_VOL_CURVE: usize = 0x8CC114;
/// `.rdata` floats used by the dummy volumes: model-0x1C9 offset, log factor (type 2), fwd threshold
const FLT_8CA4: usize = 0x858CA4;
const FLT_8BA4: usize = 0x858BA4;
const FLT_2CD0: usize = 0x862CD0;
/// `s_Config` "drive wheel skid enabled" byte (`.data`) and the gas-pedal skid factor
const DRIVE_SKID_ENABLED: usize = 0x8CBD80;
const DRIVE_SKID_GAS_MULT: usize = 0x8CBD88;
/// 0.4f, 1.666..f (`.rdata`)
const FLT_0_4: usize = 0x858EE8;
const FLT_1_667: usize = 0x85F0A8;
/// Player engine volume tuning: ranges from the static initialisers (`.bss`) and offsets (`.data`)
const PE_VOL_RANGE_1: usize = 0xB6BA64;
const PE_VOL_RANGE_2: usize = 0xB6BA44;
const PE_VOL_SINGLE_GEAR: usize = 0xB6BA3C;
const PE_VOL_RANGE_3: usize = 0xB6BA54;
const PE_VOL_BASE_3: usize = 0xB6B9D8;
const PE_VOL_RANGE_4: usize = 0xB6BA38;
const PE_VOL_BASE_1: usize = 0x8CBCA8;
const PE_VOL_BASE_2: usize = 0x8CBC78;
const PE_VOL_BASE_4: usize = 0x8CBC68;
const PE_VOL_EXTRA_MULT: usize = 0x8CBC4C;
/// `CReplay::Mode` (byte)
const REPLAY_MODE: usize = 0xA43088;
/// 255.0f (`.rdata`)
const FLT_255: usize = 0x859AAC;
/// `CWeather::UnderWaterness`
const WEATHER_UNDERWATERNESS: usize = 0xC8132C;
/// Boat-over-water constants: `.rdata` 1.333 (speed scale), 1e-5 (silence threshold), 0.2 / 0.8
/// (speed curve), 0.5 (underwater threshold); `.data` volume offsets (type 6 / others) and the
/// underwater factors
const BOAT_SPEED_SCALE: usize = 0x85F09C;
const BOAT_MIN_SPEED: usize = 0x858C14;
const BOAT_FREQ_MULT: usize = 0x862D00;
const BOAT_FREQ_BASE: usize = 0x858C98;
const FLT_0_5: usize = 0x858B8C;
const BOAT_VOL_TYPE6: usize = 0x8CBEDC;
const BOAT_VOL_OTHER: usize = 0x8CBED8;
const BOAT_UNDERWATER_FREQ: usize = 0x8CBEEC;
const BOAT_UNDERWATER_VOL: usize = 0x8CBEF0;
/// Reverse gear tuning (`.data`): speed factor / base, volume offset; 0.0 (double, `.rdata`)
const REVERSE_SPEED_MULT: usize = 0x8CBD28;
const REVERSE_SPEED_BASE: usize = 0x8CBD24;
const REVERSE_VOL_OFFSET: usize = 0x8CBD2C;
const DBL_0: usize = 0x859EF8;
/// `m_pAudioEventVolumes` (`int8*`, event volume table)
const AUDIO_EVENT_VOLUMES: usize = 0xBD00F8;
/// Event volume offset (`.data`, 1.5)
const EVENT_VOLUME_OFFSET: usize = 0x8CBD50;
/// 100.0f (`.rdata`)
const FLT_100: usize = 0x858628;
/// 0.99 (double, `.rdata`)
const DBL_0_99: usize = 0x862CD8;
/// 5.0f (`.rdata`)
const FLT_5: usize = 0x858C80;

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    pub fn is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAudioEngine::StartRadio(tVehicleAudioSettings*)` (0x507DF0)
    pub fn engine_start_radio(engine: GPtr<u8>, settings: GPtr<u8>) = thiscall 0x507DF0 => unported;
    /// `CAudioEngine::StopRadio(tVehicleAudioSettings*, bool)` (0x506F70)
    /// `CAEAudioUtility::GetPiecewiseLinear(float, int16, const float (*)[2])` (0x4D9D90)
    pub fn get_piecewise_linear(v: u32, n: u32, table: GPtr<u8>) -> f32 = cdecl 0x4D9D90 => unported;
    /// `CAESoundManager::CancelSoundsInBankSlot(int16 slot, bool)` (0x4EFC60)
    pub fn cancel_sounds_in_bank_slot(sm: GPtr<u8>, slot: u32, full: u32) = thiscall 0x4EFC60 => unported;
    /// `CAESound::SetFlags(uint16 flags, uint16 enabled)` (0x4EF2B0)
    pub fn sound_set_flags(sound: GPtr<u8>, flags: u32, enabled: u32) = thiscall 0x4EF2B0 => unported;
    /// `CAESound::StopSound()` (0x4EF1C0)
    pub fn sound_stop_sound(sound: GPtr<u8>) = thiscall 0x4EF1C0 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    pub fn player_get_pad_from_player(ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0)
    pub fn pad_get_accelerate(pad: GPtr<u8>) -> u32 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080)
    pub fn pad_get_brake(pad: GPtr<u8>) -> u32 = thiscall 0x540080 => unported;
    /// `CAEVehicleAudioEntity::UpdateBoatSound(int16 engineState, int16 bankSlot, int16 sfx, float speed,
    /// float volume)` (0x4F9E90)
    pub fn update_boat_sound(this: GPtr<u8>, state: u32, slot: u32, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F9E90 => unported;
    /// `CAEVehicleAudioEntity::PlayReverseSound(int16 sfx, float speed, float volume)` (0x4F87D0)
    pub fn play_reverse_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F87D0 => unported;
    /// `CAEVehicleAudioEntity::PlayTrainBrakeSound(int16 sfx, float speed, float volume)` (0x4FA630)
    pub fn play_train_brake_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4FA630 => unported;
    /// `CAEVehicleAudioEntity::CancelVehicleEngineSound(int16)` (0x4F55C0)
    pub fn cancel_vehicle_engine_sound_call(this: GPtr<u8>, ty: u32) = thiscall 0x4F55C0 => cancel_vehicle_engine_sound;
    /// `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver()` (0x4F5BA0)
    pub fn player_about_to_exit_vehicle_as_driver(this: GPtr<u8>) = thiscall 0x4F5BA0 => unported;
    /// `CAEVehicleAudioEntity::JustWreckedVehicle()` (0x4FD0B0)
    pub fn just_wrecked_vehicle(this: GPtr<u8>) = thiscall 0x4FD0B0 => unported;
    /// `CAEVehicleAudioEntity::JustGotInVehicleAsDriver()` (0x4F5700)
    pub fn just_got_in_vehicle_as_driver(this: GPtr<u8>) = thiscall 0x4F5700 => unported;
    /// `CAEVehicleAudioEntity::JustGotOutOfVehicleAsDriver()` (0x4FCF40)
    pub fn just_got_out_of_vehicle_as_driver_call(this: GPtr<u8>) = thiscall 0x4FCF40 => just_got_out_of_vehicle_as_driver;
    /// `CAEVehicleAudioEntity::TurnOnRadioForVehicle()` (0x4F5B20)
    pub fn turn_on_radio_for_vehicle_call(this: GPtr<u8>) = thiscall 0x4F5B20 => turn_on_radio_for_vehicle;
    /// `CAEVehicleAudioEntity::TurnOffRadioForVehicle()` (0x4F5B60)
    pub fn turn_off_radio_for_vehicle_call(this: GPtr<u8>) = thiscall 0x4F5B60 => turn_off_radio_for_vehicle;
    /// `CAEVehicleAudioEntity::ProcessVehicle(CPhysical*)` (0x501E10)
    pub fn process_vehicle(this: GPtr<u8>, veh: u32) = thiscall 0x501E10 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CAEAudioUtility::FindVehicleOfPlayer()` (0x4D9E10)
    pub fn find_vehicle_of_player() -> u32 = cdecl 0x4D9E10 => unported;
    pub fn engine_stop_radio(engine: GPtr<u8>, settings: GPtr<u8>, duck: u32) = thiscall 0x506F70 => unported;
}

/// Radio types 0, 2, 3 (`+0x9B`, signed) have a radio the player hears.
fn has_radio(this: GPtr<u8>) -> bool {
    matches!(rd::<i8>(this, 0x9B), 0 | 2 | 3)
}

/// The inlined five-point curve of `GetFreqForIdle` / `GetBaseVolumeForBicycleTyre`: `i` = first point
/// with `x > v` (5 if none, NaN included); `v > 1.0` or `i == 5` gives the last y, `v < 0.0` or `i == 0`
/// the first y, else the x87 interpolation `(v - x0) / (x1 - x0) * (y1 - y0) + y0`.
fn five_point_curve(curve: usize, v: f32) -> X87 {
    let px = |i: usize| g::<f32>(curve + i * 8).read();
    let py = |i: usize| g::<f32>(curve + i * 8 + 4).read();
    let mut i = 0usize;
    while i < 5 {
        if !x(px(i)).fcom(x(v)).test_any(0x41) {
            break;
        }
        i += 1;
    }
    let one = X87::from_f64_exact(f64::from_bits(g::<u64>(DBL_1).read()));
    if !x(v).fcom(one).test_any(0x41) || i == 5 {
        return x(py(4));
    }
    if !x(v).fcom(x(g::<f32>(FLT_0).read())).test_parity(0x05) || i == 0 {
        return x(py(0));
    }
    let num = x(v).fsub_m(px(i - 1));
    let den = x(px(i)).fsub_m(px(i - 1));
    let dy = x(py(i)).fsub_m(py(i - 1));
    num.fdiv(den).fmul(dy).fadd_m(py(i - 1))
}

/// `CAEVehicleAudioEntity::GetFreqForIdle(float)` (0x4F5C60, static): the idle frequency curve.
/// Not hooked: its own loop branches back to entry +2 (0x4F5C7C -> 0x4F5C62), inside the patch.
pub fn get_freq_for_idle(v: f32) -> X87 {
    five_point_curve(IDLE_FREQ_CURVE, v)
}

/// `CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre(float)` (0x4F60B0, static): the tyre volume curve.
/// Not hooked: its own loop branches back to entry +2 (0x4F60CC -> 0x4F60B2), inside the patch.
pub fn get_base_volume_for_bicycle_tyre(v: f32) -> X87 {
    five_point_curve(BICYCLE_TYRE_VOLUME_CURVE, v)
}

/// `CAEVehicleAudioEntity::Constructor()` (0x4F63E0): vtable 0x862CEC, the embedded twin-loop entity
/// (+0x184, vtable 0x85F438, entity +0x194) and a few fields cleared; returns `this`.
#[hook(addr = 0x4F63E0, conv = thiscall, name = "CAEVehicleAudioEntity::Constructor", expect = "8B C1 33 C9 89 48 10")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0, 0x862CEC);
    wr::<u32>(this, 0x194, 0);
    wr::<u32>(this, 0x184, 0x85F438);
    wr::<u16>(this, 0x20C, 0);
    wr::<u32>(this, 0x224, 0);
    wr::<u32>(this, 0x228, 0);
    wr::<u8>(this, 0xA4, 0);
    wr::<u32>(this, 4, 0);
    this
}

/// `CAEVehicleAudioEntity::UpdateVehicleEngineSound(int16 type, float speed, float volume)` (0x4F56D0):
/// the engine sound of `type` (+0xE8 + 8 * type) gets volume `volume + m_EventVolume` (+0xD8) and the
/// speed. Returns EAX = the sign-extended type, as the exe (it never sets a bool).
#[hook(addr = 0x4F56D0, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateVehicleEngineSound", expect = "0F BF 44 24 04")]
pub fn update_vehicle_engine_sound(this: GPtr<u8>, ty: u32, speed: u32, volume: f32) -> u32 {
    let idx = ty as u16 as i16 as i32;
    let vol = x(volume).fadd_m(rd::<f32>(this, 0xD8));
    let s = rd::<u32>(this, 0xE8 + idx as isize * 8);
    if s != 0 {
        let s: GPtr<u8> = g(s as usize);
        wr::<f32>(s, 0x14, vol.to_f32());
        wr::<u32>(s, 0x1C, speed);
    }
    idx as u32
}

/// `CAEVehicleAudioEntity::IsAccInhibitedForLowSpeed(cVehicleParams&)` (0x4F4FF0): not single-gear
/// (+0xB0) and speed below the config limit.
/// Not hooked: callers 0x4FBDE9 / 0x4FBF55 / 0x4FC29B / 0x4FC7F8 reuse ECX (`this`) for the next call.
pub fn is_acc_inhibited_for_low_speed(this: GPtr<u8>, params: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0xB0) != 0 {
        return 0;
    }
    let c = x(rd::<f32>(params, 0x1C)).fcom(x(g::<f32>(AC_INHIBIT_FOR_LOW_SPEED_LIMIT).read()));
    // fcomp; test ah, 5; jp -> false
    (!c.test_parity(0x05)) as u8
}

/// `CAEVehicleAudioEntity::IsAccInhibited(cVehicleParams&)` (0x4F4F70): true unless the engine bank
/// (+0xDE) is loaded in slot 0x28, params +0x22 <= 0, +0x39 clear, +0x40 <= 5.0 (or unordered) and +0x46 set.
#[hook(addr = 0x4F4F70, conv = thiscall, name = "CAEVehicleAudioEntity::IsAccInhibited", expect = "33 C0 66 8B 81 DE 00 00 00")]
pub fn is_acc_inhibited(this: GPtr<u8>, params: GPtr<u8>) -> u8 {
    if is_sound_bank_loaded(audio_hardware(), rd::<u16>(this, 0xDE) as u32, 0x28) == 0 {
        return 1;
    }
    if rd::<i16>(params, 0x22) > 0 || rd::<u8>(params, 0x39) != 0 {
        return 1;
    }
    let c = x(rd::<f32>(params, 0x40)).fcom(x(g::<f32>(FLT_5).read()));
    if !c.test_any(0x41) {
        return 1;
    }
    if rd::<u8>(params, 0x46) == 0 {
        return 1;
    }
    0
}

/// `CAEVehicleAudioEntity::JustFinishedAccelerationLoop()` (0x4F5E50): counts frames since the AC loop
/// (+0x14C) up to the config count; then true (and resets the counter) when both play positions
/// (+0x148, +0x14A) are valid, the AC sound (+0x108) exists and `pos > length - LoopInterval`.
#[hook(addr = 0x4F5E50, conv = thiscall, name = "CAEVehicleAudioEntity::JustFinishedAccelerationLoop", expect = "33 C0 66 8B 81 4C 01 00 00")]
pub fn just_finished_acceleration_loop(this: GPtr<u8>) -> u8 {
    let frames = rd::<i16>(this, 0x14C);
    if frames < g::<i16>(AC_LOOP_FRAME_CNT).read() {
        wr::<i16>(this, 0x14C, frames.wrapping_add(1));
        return 0;
    }
    let pos = rd::<i16>(this, 0x148);
    if pos < 0 || rd::<i16>(this, 0x14A) < 0 {
        return 0;
    }
    let s = rd::<u32>(this, 0x108);
    if s == 0 {
        return 0;
    }
    let len = rd::<i16>(g::<u8>(s as usize), 0x70) as i32;
    let limit = len.wrapping_sub(g::<i16>(AC_LOOP_INTERVAL).read() as i32);
    if pos as i32 <= limit {
        return 0;
    }
    wr::<u16>(this, 0x14C, 0);
    1
}

/// `CAEVehicleAudioEntity::GetSirenState(bool& siren, bool& fastSiren, cVehicleParams&)` (0x4F62A0):
/// `siren` = has siren (+0xC8), not wrecked (+0xA8), the vehicle's siren flag (+0x42D bit 7) and status
/// not abandoned; `fastSiren` (written only when `siren` ends up set) = horn counter (+0x514) set and
/// model != 0x1A7.
#[hook(addr = 0x4F62A0, conv = thiscall, name = "CAEVehicleAudioEntity::GetSirenState", expect = "8A 81 A8 00 00 00")]
pub fn get_siren_state(this: GPtr<u8>, siren: GPtr<u8>, fast: GPtr<u8>, params: GPtr<u8>) {
    if rd::<u8>(this, 0xA8) != 0 || rd::<u8>(this, 0xC8) == 0 {
        siren.write(0);
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if (rd::<u8>(veh, 0x42D) as i8) >= 0 {
        siren.write(0);
        return;
    }
    siren.write(1);
    if rd::<u8>(veh, 0x36) & 0xF8 == 0x20 {
        siren.write(0);
        return;
    }
    let on = rd::<u32>(veh, 0x514) != 0 && rd::<u16>(veh, 0x22) != 0x1A7;
    fast.write(on as u8);
}

/// `CAEVehicleAudioEntity::GetVehicleNonDriveWheelSkidValue(CVehicle*, tWheelState, cTransmission&,
/// float velocity)` (0x4F6000): 0.0 unless enabled; spinning (3): `min(|v| * 5, 1) * spin mult` above
/// |v| = 0.04 (else 0 * mult); sliding (2): `min(|v|, 1) * 0.75 * slide mult`, * 0.2 more for vehicle
/// audio type 2 (+0x80); other states 0.0.
/// The caller at 0x4F9062 / 0x4F907D keeps EDX across the call: hooked with `preserve = "edx"` (STD-12; the
/// function was reached in every real-time Grove Street run with traffic and blocked phase B while unregistered).
#[hook(addr = 0x4F6000, conv = thiscall, name = "CAEVehicleAudioEntity::GetVehicleNonDriveWheelSkidValue", expect = "A0 81 BD 8C 00 D9 05 50 8B 85 00",
       preserve = "edx", unverified = "std12-audio-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk,driving")]
pub fn get_vehicle_non_drive_wheel_skid_value(
    this: GPtr<u8>,
    _veh: GPtr<u8>,
    state: i32,
    _tr: GPtr<u8>,
    vel: f32,
) -> X87 {
    let zero = x(g::<f32>(FLT_0).read());
    if g::<u8>(NON_DRIVE_SKID_ENABLED).read() == 0 {
        return zero;
    }
    let one = || x(g::<f32>(FLT_1).read());
    let abs = f32::from_bits(vel.to_bits() & 0x7FFF_FFFF);
    match state {
        3 => {
            if x(abs).fcom(x(g::<f32>(FLT_0_04).read())).test_any(0x41) {
                return zero.fmul_m(g::<f32>(NON_DRIVE_SKID_SPIN_MULT).read());
            }
            let mut v = x(abs).fmul_m(g::<f32>(FLT_5).read());
            if !one().fcom(v).test_parity(0x05) {
                v = one();
            }
            v.fmul_m(g::<f32>(NON_DRIVE_SKID_SPIN_MULT).read())
        }
        2 => {
            let mut v = x(abs);
            if !one().fcom(v).test_parity(0x05) {
                v = one();
            }
            let mut v = v
                .fmul_m(g::<f32>(FLT_0_75).read())
                .fmul_m(g::<f32>(NON_DRIVE_SKID_SLIDE_MULT).read());
            if rd::<u8>(this, 0x80) == 2 {
                v = v.fmul_m(g::<f32>(FLT_0_2).read());
            }
            v
        }
        _ => zero,
    }
}

/// `CAEVehicleAudioEntity::GetVehicleDriveWheelSkidValue(CVehicle*, tWheelState, float gasPedal,
/// cTransmission&, float velocity)` (0x4F5F30): 0.0 unless enabled; state 1: `(gas - 0.4) * 1.667 *
/// 0.75 * mult` above 0.4 (else 0 * mult); state 2 / 3 as the non-drive wheel value without the audio
/// type factor; other states 0.0.
/// The caller at 0x4F9094 keeps EDX across the call: hooked with `preserve = "edx"` (STD-12, as the non-drive value).
#[hook(addr = 0x4F5F30, conv = thiscall, name = "CAEVehicleAudioEntity::GetVehicleDriveWheelSkidValue", expect = "A0 80 BD 8C 00 D9 05 50 8B 85 00",
       preserve = "edx", unverified = "std12-audio-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk,driving")]
pub fn get_vehicle_drive_wheel_skid_value(
    _this: GPtr<u8>,
    _veh: GPtr<u8>,
    state: i32,
    gas: f32,
    _tr: GPtr<u8>,
    vel: f32,
) -> X87 {
    let zero = x(g::<f32>(FLT_0).read());
    if g::<u8>(DRIVE_SKID_ENABLED).read() == 0 {
        return zero;
    }
    let one = || x(g::<f32>(FLT_1).read());
    let abs = f32::from_bits(vel.to_bits() & 0x7FFF_FFFF);
    match state {
        1 => {
            let c = g::<f32>(FLT_0_4).read();
            let v = if x(gas).fcom(x(c)).test_any(0x41) {
                zero
            } else {
                x(gas)
                    .fsub_m(c)
                    .fmul_m(g::<f32>(FLT_1_667).read())
                    .fmul_m(g::<f32>(FLT_0_75).read())
            };
            v.fmul_m(g::<f32>(DRIVE_SKID_GAS_MULT).read())
        }
        2 => {
            let mut v = x(abs);
            if !one().fcom(v).test_parity(0x05) {
                v = one();
            }
            v.fmul_m(g::<f32>(FLT_0_75).read())
                .fmul_m(g::<f32>(NON_DRIVE_SKID_SLIDE_MULT).read())
        }
        3 => {
            if x(abs).fcom(x(g::<f32>(FLT_0_04).read())).test_any(0x41) {
                return zero.fmul_m(g::<f32>(NON_DRIVE_SKID_SPIN_MULT).read());
            }
            let mut v = x(abs).fmul_m(g::<f32>(FLT_5).read());
            if !one().fcom(v).test_parity(0x05) {
                v = one();
            }
            v.fmul_m(g::<f32>(NON_DRIVE_SKID_SPIN_MULT).read())
        }
        _ => zero,
    }
}

/// `CAEVehicleAudioEntity::StaticInitialise()` (0x5B99F0): the ten dummy engine slots = (-1, 0), banks
/// 0x8A -> slot 0x13, 0x0D -> 0x12, 0x4A -> 0x11, then the radio / driver / slot / helicopter statics
/// cleared.
#[hook(addr = 0x5B99F0, conv = cdecl, name = "CAEVehicleAudioEntity::StaticInitialise", expect = "53 83 CA FF B8 A2 B9 B6 00")]
pub fn static_initialise() {
    let slots = DUMMY_ENGINE_SLOTS.ptr().cast::<u16>();
    for i in 0..10isize {
        slots.byte_add(i * 4).write(0xFFFF);
        slots.byte_add(i * 4 + 2).write(0);
    }
    load_sound_bank(audio_hardware(), 0x8A, 0x13);
    load_sound_bank(audio_hardware(), 0x0D, 0x12);
    load_sound_bank(audio_hardware(), 0x4A, 0x11);
    VEHICLE_AUDIO_SETTINGS_FOR_RADIO.set(0);
    PLAYER_ATTACHED_FOR_RADIO.set(0);
    PLAYER_DRIVER.set(0);
    NEXT_DUMMY_ENGINE_SLOT.set(0);
    HELICOPTORS_DISABLED.set(0);
}

/// `CAEVehicleAudioEntity::GetFlyingMetalVolume(CPhysical*)` (0x4F6150): `min(|velocity|^2, 0.5625)`
/// (x87 `(x*x + y*y) + z*z`, NaN -> 0.5625) * 1.777..; -100.0 below 1e-10, else
/// `AudioLog10(f32(v)) * 10`. `this` is unused.
/// `AudioLog10`'s full 64-bit FYL2X result is multiplied directly, as the original does ([`audio_log10`]).
#[hook(addr = 0x4F6150, conv = thiscall, name = "CAEVehicleAudioEntity::GetFlyingMetalVolume", expect = "8B 4C 24 04 D9 41 58")]
pub fn get_flying_metal_volume(_this: GPtr<u8>, phys: GPtr<u8>) -> X87 {
    let sq = || {
        let (vx, vy, vz) = (
            x(rd::<f32>(phys, 0x50)),
            x(rd::<f32>(phys, 0x54)),
            x(rd::<f32>(phys, 0x58)),
        );
        vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
    };
    let max = g::<f32>(FLYING_METAL_MAX_SPEED_SQ).read();
    let v = if sq().fcom(x(max)).test_parity(0x05) {
        x(max)
    } else {
        sq()
    };
    let v = v.fmul_m(g::<f32>(FLYING_METAL_SCALE).read());
    let stored = v.to_f32();
    if !v.fcom(x(g::<f32>(FLYING_METAL_MIN).read())).test_parity(0x05) {
        return x(g::<f32>(FLT_M100).read());
    }
    call_audio_log10(stored).fmul_m(g::<f32>(FLT_10).read())
}

/// `CAEVehicleAudioEntity::GetHornState(bool* out, cVehicleParams&)` (0x4F61E0): no horn for wrecked
/// (+0xA8); with a driver-ish word (+0x45C, not 0/-1) only in status 5; then horn counter (+0x514) > 0,
/// always on for status 0 (player), otherwise the counter is capped at 44, a pattern is drawn with
/// `GetRandomNumberInRange(0, 7)` (the audio RNG; gta-reversed uses CGeneral) when it is exactly 44,
/// and the pattern table decides.
#[hook(addr = 0x4F61E0, conv = thiscall, name = "CAEVehicleAudioEntity::GetHornState-", expect = "8A 81 A8 00 00 00")]
pub fn get_horn_state(this: GPtr<u8>, out: GPtr<u8>, params: GPtr<u8>) {
    if rd::<u8>(this, 0xA8) != 0 {
        out.write(0);
        return;
    }
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let v = veh();
    let w = rd::<u16>(v, 0x45C);
    if w != 0 && w != 0xFFFF && rd::<u8>(v, 0x36) & 0xF8 != 0x28 {
        out.write(0);
        return;
    }
    let counter = rd::<u32>(v, 0x514);
    if counter == 0 {
        out.write(0);
        return;
    }
    if rd::<u8>(v, 0x36) & 0xF8 == 0 {
        out.write(1);
        return;
    }
    if counter > 0x2C {
        wr::<u32>(v, 0x514, 0x2C);
    }
    if rd::<u32>(veh(), 0x514) == 0x2C {
        let r = ae_random_int(0, 7);
        wr::<u8>(veh(), 0x518, r as u8);
    }
    let v = veh();
    let idx = (rd::<u8>(v, 0x518) as u32)
        .wrapping_mul(0x2C)
        .wrapping_sub(rd::<u32>(v, 0x514));
    let on = g::<u8>((HORN_PATTERNS as u32).wrapping_add(idx) as usize).read();
    out.write((on != 0) as u8);
}

/// The shared clamp of the dummy frequency ratio: `t < 0` -> 0.0, `t >= 1` or NaN -> 1.0.
fn clamp01(t: X87) -> X87 {
    let one = x(g::<f32>(FLT_1).read());
    let zero = x(g::<f32>(FLT_0).read());
    if !t.fcom(one).test_parity(0x05) {
        // t < 1
        if !zero.fcom(t).test_any(0x41) {
            return zero;
        }
    }
    if t.fcom(one).test_parity(0x05) {
        return one;
    }
    t
}

/// The common tail: `(flag 0x40 of vehicle +0x42B ? f32(mult * freq) : freq) * st0`.
fn dummy_freq_tail(this: GPtr<u8>, st0: X87, freq: f32) -> X87 {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let mut f = freq;
    if rd::<u8>(veh, 0x42B) & 0x40 != 0 {
        f = x(g::<f32>(DUMMY_FREQ_FLAG_MULT).read()).fmul_m(f).to_f32();
    }
    st0.fmul_m(f)
}

/// `CAEVehicleAudioEntity::GetFrequencyForDummyIdle(float ratio, float fwd)` (0x4F5310): 1.0 for model
/// 0x1C9; else `clamp01(ratio / div) * range + base` (stored as f32), times the forward curve for audio
/// type 2 (+0xA9) else 1.0, with the vehicle-flag factor.
#[hook(addr = 0x4F5310, conv = thiscall, name = "CAEVehicleAudioEntity::GetFrequencyForDummyIdle", expect = "56 8B F1 8B 46 04")]
pub fn get_frequency_for_dummy_idle(this: GPtr<u8>, ratio: f32, fwd: u32) -> X87 {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u16>(veh, 0x22) == 0x1C9 {
        return x(g::<f32>(FLT_1).read());
    }
    let t = clamp01(x(ratio).fdiv_m(g::<f32>(DUMMY_IDLE_RATIO_DIV).read()));
    let freq = x(g::<f32>(DUMMY_IDLE_FREQ_RANGE).read())
        .fmul(t)
        .fadd_m(g::<f32>(DUMMY_IDLE_FREQ_BASE).read())
        .to_f32();
    let st0 = if rd::<u8>(this, 0xA9) == 2 {
        x(get_piecewise_linear(fwd, 5, g(DUMMY_IDLE_FWD_CURVE)))
    } else {
        x(g::<f32>(FLT_1).read())
    };
    dummy_freq_tail(this, st0, freq)
}

/// `CAEVehicleAudioEntity::GetFrequencyForDummyRev(float ratio, float fwd)` (0x4F54F0):
/// `clamp01((ratio - off) / (1 - off)) * range + base` (stored as f32), times the forward curve for
/// audio type 2 (+0xA9) when `fwd <= 0.99` (or NaN) else 1.0, with the vehicle-flag factor.
#[hook(addr = 0x4F54F0, conv = thiscall, name = "CAEVehicleAudioEntity::GetFrequencyForDummyRev", expect = "D9 44 24 04 56")]
pub fn get_frequency_for_dummy_rev(this: GPtr<u8>, ratio: f32, fwd: f32) -> X87 {
    let off = g::<f32>(DUMMY_REV_RATIO_OFFSET).read();
    let num = x(ratio).fsub_m(off);
    let den = x(g::<f32>(FLT_1).read()).fsub_m(off);
    let t = clamp01(num.fdiv(den));
    let freq = x(g::<f32>(DUMMY_REV_FREQ_RANGE).read())
        .fmul(t)
        .fadd_m(g::<f32>(DUMMY_REV_FREQ_BASE).read())
        .to_f32();
    let d = X87::from_f64_exact(f64::from_bits(g::<u64>(DBL_0_99).read()));
    let st0 = if rd::<u8>(this, 0xA9) == 2 && x(fwd).fcom(d).test_any(0x41) {
        x(get_piecewise_linear(fwd.to_bits(), 5, g(DUMMY_REV_FWD_CURVE)))
    } else {
        x(g::<f32>(FLT_1).read())
    };
    dummy_freq_tail(this, st0, freq)
}

/// The common tail of the dummy volumes: `vol` -= flag offset (vehicle +0x42B bit 0x40), += trailer
/// offset when vehicle +0x4C8 is set (each stored as f32); returns `(st0 + m_EventVolume) + vol`.
fn dummy_vol_tail(this: GPtr<u8>, st0: X87, vol: f32) -> X87 {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let mut v = vol;
    if rd::<u8>(veh, 0x42B) & 0x40 != 0 {
        v = x(v).fsub_m(g::<f32>(DUMMY_VOL_FLAG_OFFSET).read()).to_f32();
    }
    if rd::<u32>(veh, 0x4C8) != 0 {
        v = x(g::<f32>(DUMMY_VOL_TRAILER_OFFSET).read()).fadd_m(v).to_f32();
    }
    st0.fadd_m(rd::<f32>(this, 0xA0)).fadd_m(v)
}

/// `AudioLog10(f32(GetPiecewiseLinear(fwd, 5, curve))) * [0x858BA4]`
fn dummy_curve_db(fwd: u32, curve: usize) -> X87 {
    let pl = get_piecewise_linear(fwd, 5, g(curve));
    call_audio_log10(pl).fmul_m(g::<f32>(FLT_8BA4).read())
}

/// `CAEVehicleAudioEntity::GetVolumeForDummyIdle(float ratio, float fwd)` (0x4F51F0): model 0x1C9 gives
/// `base - [0x858CA4]`; else `clamp01(ratio / div) * range + base` (f32) plus, for audio type 2
/// (+0xA9), the dB of the forward curve, otherwise `AudioLog10(fwd) * 10` when `fwd <= [0x862CD0]`
/// (or NaN) else 0.0; then the common tail.
/// `AudioLog10`'s full 64-bit FYL2X result is multiplied directly, as the original does ([`audio_log10`]).
#[hook(addr = 0x4F51F0, conv = thiscall, name = "CAEVehicleAudioEntity::GetVolumeForDummyIdle", expect = "56 8B F1 8B 46 04")]
pub fn get_volume_for_dummy_idle(this: GPtr<u8>, ratio: f32, fwd: f32) -> X87 {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u16>(veh, 0x22) == 0x1C9 {
        return x(g::<f32>(DUMMY_IDLE_VOL_BASE).read()).fsub_m(g::<f32>(FLT_8CA4).read());
    }
    let t = clamp01(x(ratio).fdiv_m(g::<f32>(DUMMY_IDLE_RATIO_DIV).read()));
    let vol = x(g::<f32>(DUMMY_IDLE_VOL_RANGE).read())
        .fmul(t)
        .fadd_m(g::<f32>(DUMMY_IDLE_VOL_BASE).read())
        .to_f32();
    let st0 = if rd::<u8>(this, 0xA9) == 2 {
        dummy_curve_db(fwd.to_bits(), DUMMY_IDLE_VOL_CURVE)
    } else if x(fwd).fcom(x(g::<f32>(FLT_2CD0).read())).test_any(0x41) {
        call_audio_log10(fwd).fmul_m(g::<f32>(FLT_10).read())
    } else {
        x(g::<f32>(FLT_0).read())
    };
    dummy_vol_tail(this, st0, vol)
}

/// `CAEVehicleAudioEntity::GetVolumeForDummyRev(float ratio, float fwd)` (0x4F53D0):
/// `clamp01((ratio - off) / (1 - off)) * range + base` (f32) plus, for audio type 2 (+0xA9), the dB of
/// the forward curve when `fwd <= 0.99` (or NaN) else 0.0, otherwise `AudioLog10(f32(1 - fwd)) * 10`;
/// then the common tail.
/// `AudioLog10`'s full 64-bit FYL2X result is multiplied directly, as the original does ([`audio_log10`]).
#[hook(addr = 0x4F53D0, conv = thiscall, name = "CAEVehicleAudioEntity::GetVolumeForDummyRev", expect = "D9 44 24 04 56 D8 25 F4 BB 8C 00")]
pub fn get_volume_for_dummy_rev(this: GPtr<u8>, ratio: f32, fwd: f32) -> X87 {
    let off = g::<f32>(DUMMY_REV_RATIO_OFFSET).read();
    let num = x(ratio).fsub_m(off);
    let den = x(g::<f32>(FLT_1).read()).fsub_m(off);
    let t = clamp01(num.fdiv(den));
    let vol = x(g::<f32>(DUMMY_REV_VOL_RANGE).read())
        .fmul(t)
        .fadd_m(g::<f32>(DUMMY_REV_VOL_BASE).read())
        .to_f32();
    let st0 = if rd::<u8>(this, 0xA9) == 2 {
        let d = X87::from_f64_exact(f64::from_bits(g::<u64>(DBL_0_99).read()));
        if x(fwd).fcom(d).test_any(0x41) {
            dummy_curve_db(fwd.to_bits(), DUMMY_REV_VOL_CURVE)
        } else {
            x(g::<f32>(FLT_0).read())
        }
    } else {
        let a = x(g::<f32>(FLT_1).read()).fsub_m(fwd).to_f32();
        call_audio_log10(a).fmul_m(g::<f32>(FLT_10).read())
    };
    dummy_vol_tail(this, st0, vol)
}

/// `CAEVehicleAudioEntity::GetVolForPlayerEngineSound(cVehicleParams&, int16 engineState)` (0x4F5D00):
/// per state 1..=5 a linear volume (state 5 = the state-2 value, returned without the tail); any other
/// state returns the argument dword read as a float, as the bytes. Tail: -flag offset (vehicle +0x42B
/// bit 0x40), + m_EventVolume (+0xA0), + trailer offset (vehicle +0x4C8), + 3 * extra (+0x248) when the
/// extra flag (+0x244) is set and 0 <= extra <= 1.
/// Not hooked: ten callers (0x4FBE51, 0x4FC06C, ..) keep ECX (`this`) for the next call.
pub fn get_vol_for_player_engine_sound(this: GPtr<u8>, params: GPtr<u8>, state: u32) -> X87 {
    let gf = |a: usize| g::<f32>(a).read();
    let mut v = match (state as u16 as i16 as i32).wrapping_sub(1) {
        0 => x(gf(PE_VOL_RANGE_1))
            .fmul_m(rd::<f32>(params, 0x3C))
            .fadd_m(gf(PE_VOL_BASE_1)),
        1 => {
            let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
            if rd::<u16>(veh, 0x22) == 0x1C9 {
                x(gf(PE_VOL_BASE_2)).fsub_m(gf(FLT_8CA4))
            } else {
                x(gf(PE_VOL_RANGE_2))
                    .fmul_m(rd::<f32>(params, 0x2C))
                    .fadd_m(gf(PE_VOL_BASE_2))
            }
        }
        2 => {
            if rd::<u8>(this, 0xB0) != 0 {
                x(gf(PE_VOL_SINGLE_GEAR))
            } else {
                x(gf(PE_VOL_RANGE_3))
                    .fmul_m(rd::<f32>(this, 0xAC))
                    .fadd_m(gf(PE_VOL_BASE_3))
            }
        }
        3 => {
            if rd::<u8>(this, 0xB0) != 0 {
                x(gf(PE_VOL_SINGLE_GEAR))
            } else {
                xi(rd::<u8>(this, 0xAA) as i32)
                    .fmul_m(gf(PE_VOL_RANGE_4))
                    .fadd_m(gf(PE_VOL_BASE_4))
            }
        }
        // The original calls itself through its address (state 2) and returns that value directly.
        4 => return get_vol_for_player_engine_sound(this, params, 2),
        _ => x(f32::from_bits(state)),
    };
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u8>(veh, 0x42B) & 0x40 != 0 {
        v = v.fsub_m(gf(DUMMY_VOL_FLAG_OFFSET));
    }
    v = v.fadd_m(rd::<f32>(this, 0xA0));
    if rd::<u32>(veh, 0x4C8) != 0 {
        v = v.fadd_m(gf(DUMMY_VOL_TRAILER_OFFSET));
    }
    if rd::<u8>(this, 0x244) != 0 {
        let e = rd::<f32>(this, 0x248);
        // fcomp 1.0; test ah, 0x41; jp -> skip (e > 1 or NaN); fcomp 0.0; test ah, 1; jne -> skip (e < 0)
        if !x(e).fcom(x(gf(FLT_1))).test_parity(0x41) && !x(e).fcom(x(gf(FLT_0))).test_any(0x01) {
            v = v.fadd(x(gf(PE_VOL_EXTRA_MULT)).fmul_m(e));
        }
    }
    v
}

/// `CAEVehicleAudioEntity::CancelVehicleEngineSound(int16 type)` (0x4F55C0): for types 0..12 the engine
/// sound (+0xE8 + 8 * type, read before the range check as the bytes) is stopped (flag 4 cleared first)
/// and forgotten; for the AC sound (4) the play position (+0x148) moves to +0x154, the stop time (+0x150)
/// is now, both positions become -1 and +0x14E = `_ftol(clamp01(pos / length) * 100)` (0 when the old
/// length is <= 0).
#[hook(addr = 0x4F55C0, conv = thiscall, name = "CAEVehicleAudioEntity::CancelVehicleEngineSound", expect = "53 55 66 8B 6C 24 0C")]
pub fn cancel_vehicle_engine_sound(this: GPtr<u8>, ty: u32) {
    let t = ty as u16 as i16;
    let slot = this.byte_add(0xE8 + t as isize * 8);
    let snd = rd::<u32>(slot, 0);
    let len: i16 = if snd != 0 {
        rd::<i16>(g::<u8>(snd as usize), 0x70)
    } else {
        -1
    };
    if !(0..0xC).contains(&t) {
        return;
    }
    if snd != 0 {
        sound_set_flags(g(snd as usize), 4, 0);
        sound_stop_sound(g(rd::<u32>(slot, 0) as usize));
        wr::<u32>(slot, 0, 0);
    }
    if t != 4 {
        return;
    }
    let pos = rd::<i16>(this, 0x148);
    wr::<i16>(this, 0x154, pos);
    wr::<u32>(this, 0x150, TIME_MS.get());
    wr::<u16>(this, 0x148, 0xFFFF);
    wr::<u16>(this, 0x14A, 0xFFFF);
    if len <= 0 {
        wr::<u16>(this, 0x14E, 0);
        return;
    }
    let r = clamp01(xi(pos as i32).fdiv(xi(len as i32))).fmul_m(g::<f32>(FLT_100).read());
    wr::<u16>(this, 0x14E, ftol32(r) as u16);
}

/// `CAEVehicleAudioEntity::RequestBankSlot(int16 bankId)` (0x4F4D10, static): a dummy engine slot
/// already holding the bank gains a reference (returns its index + 7); otherwise, if any slot is free
/// (count <= 0), the first free one from `s_NextDummyEngineSlot` (round robin, `% 10`) gets the bank:
/// its bank slot (index + 7) is cancelled and loaded, count = 1. Returns EAX = index + 7, or 0xFFFF
/// when none is free.
#[hook(addr = 0x4F4D10, conv = cdecl, name = "CAEVehicleAudioEntity::RequestBankSlot", expect = "53 55 8B 6C 24 0C")]
pub fn request_bank_slot(bank: u32) -> u32 {
    let base = DUMMY_ENGINE_SLOTS.ptr().addr() as u32;
    let bank_of = |i: i32| g::<u16>(base.wrapping_add((i as u32).wrapping_mul(4)) as usize);
    let count_of = |i: i32| g::<i16>(base.wrapping_add((i as u32).wrapping_mul(4)).wrapping_add(2) as usize);
    let mut free: i32 = -1;
    for i in 0..10 {
        if bank_of(i).read() == bank as u16 {
            let c = count_of(i);
            c.write(c.read().wrapping_add(1));
            return (i + 7) as u32;
        }
        if count_of(i).read() <= 0 {
            free = i;
        }
    }
    if free as i16 == -1 {
        return 0xFFFF;
    }
    let next = NEXT_DUMMY_ENGINE_SLOT.get() as i16 as i32;
    for k in 0..10 {
        let c = (k + next) % 10;
        let ci = c as i16 as i32;
        if count_of(ci).read() <= 0 {
            count_of(ci).write(count_of(ci).read().wrapping_add(1));
            NEXT_DUMMY_ENGINE_SLOT.set(((ci + 1) % 10) as u16);
            free = c;
            break;
        }
    }
    let bank_slot = free.wrapping_add(7) as u32;
    cancel_sounds_in_bank_slot(g(AE_SOUND_MANAGER), bank_slot, 1);
    load_sound_bank(audio_hardware(), bank, bank_slot);
    let fi = free as i16 as i32;
    bank_of(fi).write(bank as u16);
    count_of(fi).write(1);
    (fi + 7) as u32
}

/// The replay pedal clamp of `GetAccelAndBrake`: `v < 0` -> 0.0, `v > 1` -> 1.0, else (NaN too) `v`.
fn clamp_pedal(v: f32) -> X87 {
    let one = x(g::<f32>(FLT_1).read());
    let zero = x(g::<f32>(FLT_0).read());
    // fld 1.0; fcomp v; test ah, 5; jnp -> (1 < v) skip the low clamp
    if one.fcom(x(v)).test_parity(0x05) && !zero.fcom(x(v)).test_any(0x41) {
        return zero;
    }
    if one.fcom(x(v)).test_parity(0x05) {
        x(v)
    } else {
        one
    }
}

/// `CAEVehicleAudioEntity::GetAccelAndBrake(cVehicleParams&)` (0x4F5080): during replay playback the
/// vehicle's gas / brake pedals (+0x49C / +0x4A0) clamped to 0..1 times 255 (`_ftol`); otherwise the
/// player driver's pad values when the driver exists, the entity is not exiting (+0xA7), the ped state
/// (+0x530) is not 0x3F and it is alive; else 0. Stored as int16 at params +0x20 / +0x22.
#[hook(addr = 0x4F5080, conv = thiscall, name = "CAEVehicleAudioEntity::GetAccelAndBrake", expect = "80 3D 88 30 A4 00 01")]
pub fn get_accel_and_brake(this: GPtr<u8>, params: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
        let k = g::<f32>(FLT_255).read();
        let a = ftol32(clamp_pedal(rd::<f32>(veh, 0x49C)).fmul_m(k));
        wr::<u16>(params, 0x20, a as u16);
        let b = ftol32(clamp_pedal(rd::<f32>(veh, 0x4A0)).fmul_m(k));
        wr::<u16>(params, 0x22, b as u16);
        return;
    }
    let p = PLAYER_DRIVER.get();
    if p != 0
        && rd::<u8>(this, 0xA7) == 0
        && rd::<u32>(g::<u8>(p as usize), 0x530) != 0x3F
        && ped_is_alive(g(p as usize)) != 0
    {
        let pad = player_get_pad_from_player(g(PLAYER_DRIVER.get() as usize));
        let a = pad_get_accelerate(pad);
        wr::<u16>(params, 0x20, a as u16);
        let b = pad_get_brake(pad);
        wr::<u16>(params, 0x22, b as u16);
        return;
    }
    wr::<u16>(params, 0x20, 0);
    wr::<u16>(params, 0x22, 0);
}

/// `CAEVehicleAudioEntity::ProcessBoatMovingOverWater(cVehicleParams&)` (0x4FA0C0): speed ratio
/// `f32(min(|speed|, 0.75) * 1.333)` (NaN -> 0.75); volume -100 unless the boat touches water
/// (vehicle +0x5AC bit 0) and the ratio >= 1e-5 (or NaN), then `AudioLog10(ratio) * [0x858BA4]` + the
/// type-6 / other offset; frequency `f32(ratio * 0.2 + 0.8)`; with `UnderWaterness >= 0.5` (or NaN)
/// frequency *= 0.185 and volume += 6 (each stored as f32); then `UpdateBoatSound(6, 2, 3, ..)`.
/// `AudioLog10`'s full 64-bit FYL2X result is multiplied directly, as the original does ([`audio_log10`]).
#[hook(addr = 0x4FA0C0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessBoatMovingOverWater", expect = "51 8B 44 24 08 D9 40 1C")]
pub fn process_boat_moving_over_water(this: GPtr<u8>, params: GPtr<u8>) {
    let gf = |a: usize| g::<f32>(a).read();
    let abs = f32::from_bits(rd::<u32>(params, 0x1C) & 0x7FFF_FFFF);
    let mut v = x(abs);
    if v.fcom(x(gf(FLT_0_75))).test_parity(0x05) {
        v = x(gf(FLT_0_75));
    }
    let ratio = v.fmul_m(gf(BOAT_SPEED_SCALE)).to_f32();
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let mut vol: f32 = f32::from_bits(0xC2C8_0000);
    if rd::<u8>(veh, 0x5AC) & 1 != 0 && x(ratio).fcom(x(gf(BOAT_MIN_SPEED))).test_parity(0x05) {
        let off = if rd::<u8>(this, 0x80) == 6 {
            BOAT_VOL_TYPE6
        } else {
            BOAT_VOL_OTHER
        };
        vol = call_audio_log10(ratio).fmul_m(gf(FLT_8BA4)).fadd_m(gf(off)).to_f32();
    }
    let mut freq = x(ratio)
        .fmul_m(gf(BOAT_FREQ_MULT))
        .fadd_m(gf(BOAT_FREQ_BASE))
        .to_f32();
    if !x(gf(WEATHER_UNDERWATERNESS)).fcom(x(gf(FLT_0_5))).test_any(0x01) {
        freq = x(gf(BOAT_UNDERWATER_FREQ)).fmul_m(freq).to_f32();
        vol = x(gf(BOAT_UNDERWATER_VOL)).fadd_m(vol).to_f32();
    }
    update_boat_sound(this, 6, 2, 3, freq.to_bits(), vol.to_bits());
}

/// `CAEVehicleAudioEntity::ProcessReverseGear(cVehicleParams&)` (0x4F8DF0): with the engine on (vehicle
/// +0x428 bit 0x10) and not (gas >= 0 or NaN while +0x4B4 is set): the reverse ratio is `speed /
/// transmission max reverse velocity` with drive wheels on the ground (+0x961), else the stored gas pedal
/// audio (+0x964, first scaled by 0.4 when +0x962 is set); its absolute value gives `speed = f32(0.2 * t
/// + 0.75)` and, above 0.0, `volume = f32(AudioLog10(f32 t) * [0x858BA4] - 6)`; sfx 0x13 while
/// reversing (gas < 0) else 0x14. Otherwise sfx -1, speed 1.0, volume -100. Then `PlayReverseSound`.
/// `AudioLog10`'s full 64-bit FYL2X result is multiplied directly, as the original does ([`audio_log10`]).
#[hook(addr = 0x4F8DF0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessReverseGear", expect = "83 EC 08 8B 54 24 0C")]
pub fn process_reverse_gear(this: GPtr<u8>, params: GPtr<u8>) {
    let gf = |a: usize| g::<f32>(a).read();
    let mut sfx: u32 = 0xFFFF_FFFF;
    let mut vol: u32 = 0xC2C8_0000;
    let mut speed: u32 = 0x3F80_0000;
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let go = rd::<u8>(veh, 0x428) & 0x10 != 0
        && (!x(rd::<f32>(veh, 0x49C)).fcom(x(gf(FLT_0))).test_parity(0x05) || rd::<u8>(veh, 0x4B4) == 0);
    if go {
        let t = if rd::<u8>(veh, 0x961) > 0 {
            let tr: GPtr<u8> = g(rd::<u32>(params, 0x14) as usize);
            x(rd::<f32>(params, 0x1C)).fdiv_m(rd::<f32>(tr, 0x60))
        } else {
            if rd::<u8>(veh, 0x962) > 0 {
                wr::<f32>(veh, 0x964, x(rd::<f32>(veh, 0x964)).fmul_m(gf(FLT_0_4)).to_f32());
            }
            x(rd::<f32>(veh, 0x964))
        };
        let t = if t.fcom(x(gf(FLT_0))).test_parity(0x05) {
            t
        } else {
            t.fchs()
        };
        let t_f32 = t.to_f32();
        let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
        sfx = if !x(rd::<f32>(veh, 0x49C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
            0x13
        } else {
            0x14
        };
        speed = x(gf(REVERSE_SPEED_MULT))
            .fmul(t)
            .fadd_m(gf(REVERSE_SPEED_BASE))
            .to_f32()
            .to_bits();
        let d = X87::from_f64_exact(f64::from_bits(g::<u64>(DBL_0).read()));
        if !t.fcom(d).test_any(0x41) {
            vol = call_audio_log10(t_f32)
                .fmul_m(gf(FLT_8BA4))
                .fadd_m(gf(REVERSE_VOL_OFFSET))
                .to_f32()
                .to_bits();
        }
    }
    play_reverse_sound(this, sfx, speed, vol);
}

/// `m_EventVolume` (+0xD8) = `f32(fild((int8)m_pAudioEventVolumes[0x4D]) - 1.5)`.
fn set_event_volume_from_table(this: GPtr<u8>) {
    let tab: GPtr<u8> = g(g::<u32>(AUDIO_EVENT_VOLUMES).read() as usize);
    let v = xi(rd::<i8>(tab, 0x4D) as i32).fsub_m(g::<f32>(EVENT_VOLUME_OFFSET).read());
    wr::<f32>(this, 0xD8, v.to_f32());
}

/// `CAEVehicleAudioEntity::JustGotOutOfVehicleAsDriver()` (0x4FCF40): clears the exiting flag (+0xA7);
/// by vehicle audio type (+0x80): 0 / 1 reset the event volume, the two gear factors (+0x23C, +0x240 =
/// 1.0) and cancel the engine sounds (all but 2 for engine state 6, which becomes 1; else all, with a
/// non-zero state becoming 10), stop the reverse sound and clear +0xAA; 4 / 5 / 6 cancel all twelve
/// engine sounds and reset +0x22C = -1.0, +0x230 = -100.0; 8 stops the train brake sound; 9 resets the
/// event volume. `s_pPlayerDriver` is cleared in every case.
#[hook(addr = 0x4FCF40, conv = thiscall, name = "CAEVehicleAudioEntity::JustGotOutOfVehicleAsDriver", expect = "51 53 56 8B F1")]
pub fn just_got_out_of_vehicle_as_driver(this: GPtr<u8>) {
    wr::<u8>(this, 0xA7, 0);
    match rd::<u8>(this, 0x80) {
        0 | 1 => {
            set_event_volume_from_table(this);
            wr::<u32>(this, 0x23C, 0x3F80_0000);
            wr::<u32>(this, 0x240, 0x3F80_0000);
            if rd::<u8>(this, 0xA9) == 6 {
                for i in 0..12u32 {
                    if i != 2 {
                        cancel_vehicle_engine_sound_call(this, i);
                    }
                }
                wr::<u8>(this, 0xA9, 1);
            } else {
                for i in 0..12u32 {
                    cancel_vehicle_engine_sound_call(this, i);
                }
                if rd::<u8>(this, 0xA9) != 0 {
                    wr::<u8>(this, 0xA9, 10);
                }
            }
            play_reverse_sound(this, 0xFFFF_FFFF, 0x3F80_0000, 0xC2C8_0000);
            wr::<u8>(this, 0xAA, 0);
        }
        4..=6 => {
            for i in 0..12u32 {
                cancel_vehicle_engine_sound_call(this, i);
            }
            wr::<u32>(this, 0x22C, 0xBF80_0000);
            wr::<u32>(this, 0x230, 0xC2C8_0000);
        }
        8 => play_train_brake_sound(this, 0xFFFF_FFFF, 0x3F80_0000, 0xC2C8_0000),
        9 => set_event_volume_from_table(this),
        _ => {}
    }
    PLAYER_DRIVER.set(0);
}

/// `CAEVehicleAudioEntity::Service()` (0x502280): only when initialised (+0xA4). A player driver
/// (+0xA5) that is dead or in state 0x3F (and not already exiting, +0xA7) triggers
/// `PlayerAboutToExitVehicleAsDriver`; wrecked status (0x28) triggers `JustWreckedVehicle` once (+0xA8).
/// `driver` = the vehicle's driver (+0x460) is a player, `player` = it is the player's vehicle; the
/// driver / passenger transitions (+0xA5 / +0xA6) call `JustGotOutOfVehicleAsDriver`,
/// `JustGotInVehicleAsDriver` and turn the radio on / off exactly as the exe's branch layout; then
/// +0xA5 = driver, +0xA6 = player passenger and `ProcessVehicle(entity)`.
#[hook(addr = 0x502280, conv = thiscall, name = "CAEVehicleAudioEntity::Service", expect = "51 56 8B F1 8A 86 A4 00 00 00")]
pub fn service(this: GPtr<u8>) {
    if rd::<u8>(this, 0xA4) == 0 {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u8>(this, 0xA5) != 0 {
        let d = PLAYER_DRIVER.get();
        if d != 0 && rd::<u8>(this, 0xA7) == 0 {
            let dp: GPtr<u8> = g(d as usize);
            if rd::<u32>(dp, 0x530) == 0x3F || ped_is_alive(dp) == 0 {
                player_about_to_exit_vehicle_as_driver(this);
            }
        }
    }
    let wrecked = rd::<u8>(g::<u8>(rd::<u32>(this, 4) as usize), 0x36) & 0xF8 == 0x28;
    if wrecked && rd::<u8>(this, 0xA8) == 0 {
        just_wrecked_vehicle(this);
    }
    wr::<u8>(this, 0xA8, wrecked as u8);
    let drv = rd::<u32>(veh, 0x460);
    let bl = drv != 0 && ped_is_player(g(drv as usize)) != 0;
    let f = find_vehicle_of_player() == rd::<u32>(this, 4);
    let a5 = |t: GPtr<u8>| rd::<u8>(t, 0xA5) != 0;
    let a6 = |t: GPtr<u8>| rd::<u8>(t, 0xA6) != 0;
    // Labels as in the exe: A 0x502335, B 0x502362, C 0x50237A, E 0x502381, D 0x502385, F 0x5023A8.
    let e: bool;
    let mut at_b = false;
    if !f || bl {
        e = false;
        if bl {
            if !a5(this) {
                just_got_in_vehicle_as_driver(this);
            }
            at_b = true;
        }
    } else {
        e = true;
    }
    let mut at_c = false;
    if !at_b {
        // A
        if a5(this) {
            just_got_out_of_vehicle_as_driver_call(this);
        }
        if e && !a6(this) && !a5(this) {
            at_c = true;
        }
    }
    let mut at_d = false;
    if !at_c {
        // B
        if !bl {
            at_d = true;
        } else if !a5(this) && !a6(this) {
            at_c = true;
        }
    }
    if at_c {
        turn_on_radio_for_vehicle_call(this);
        // E
        at_d = !bl;
    }
    if at_d && !f && (a5(this) || a6(this)) {
        turn_off_radio_for_vehicle_call(this);
    }
    // F
    wr::<u8>(this, 0xA5, bl as u8);
    wr::<u8>(this, 0xA6, e as u8);
    process_vehicle(this, rd::<u32>(this, 4));
}

/// `CAEVehicleAudioEntity::UpdateGasPedalAudio(CVehicle*, int32 vehicleType)` (0x4F5EB0): moves the
/// stored gas pedal value (automobile +0x964, bike type 9 +0x808) towards `|gas pedal|` (+0x49C) by
/// at most the config steps; other types untouched. `this` is unused.
#[hook(addr = 0x4F5EB0, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateGasPedalAudio", expect = "8B 44 24 04 D9 80 9C 04 00 00")]
pub fn update_gas_pedal_audio(_this: GPtr<u8>, veh: GPtr<u8>, ty: i32) {
    // fld; fabs; fstp [esp+4]: |v| rounded to f32
    let v = f32::from_bits(rd::<u32>(veh, 0x49C) & 0x7FFF_FFFF);
    let off = match ty {
        0 => 0x964,
        9 => 0x808,
        _ => return,
    };
    let p = veh.byte_add(off).cast::<f32>();
    let cur = p.read();
    if !x(v).fcom(x(cur)).test_any(0x41) {
        // v > cur: min(cur + up, v)
        let t = x(g::<f32>(GAS_PEDAL_STEP_UP).read()).fadd_m(cur);
        let r = if t.fcom(x(v)).test_parity(0x05) {
            v
        } else {
            t.to_f32()
        };
        p.write(r);
    } else {
        // v <= cur (or unordered): max(cur - down, v)
        let t = x(cur).fsub_m(g::<f32>(GAS_PEDAL_STEP_DOWN).read());
        let r = if t.fcom(x(v)).test_any(0x41) {
            v
        } else {
            t.to_f32()
        };
        p.write(r);
    }
}

/// `CAEVehicleAudioEntity::TurnOnRadioForVehicle()` (0x4F5B20): records the driver (vehicle +0x460) and
/// the settings (+0x80) for the radio, and starts it for radio types 0/2/3.
#[hook(addr = 0x4F5B20, conv = thiscall, name = "CAEVehicleAudioEntity::TurnOnRadioForVehicle", expect = "8B 41 04 8B 90 60 04 00 00")]
pub fn turn_on_radio_for_vehicle(this: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    PLAYER_ATTACHED_FOR_RADIO.set(rd::<u32>(veh, 0x460));
    let settings = this.byte_add(0x80);
    VEHICLE_AUDIO_SETTINGS_FOR_RADIO.set(settings.addr() as u32);
    if has_radio(this) {
        engine_start_radio(audio_engine(), settings);
    }
}

/// `CAEVehicleAudioEntity::TurnOffRadioForVehicle()` (0x4F5B60): stops the radio for types 0/2/3, then
/// clears both radio statics.
#[hook(addr = 0x4F5B60, conv = thiscall, name = "CAEVehicleAudioEntity::TurnOffRadioForVehicle", expect = "0F BE 81 9B 00 00 00")]
pub fn turn_off_radio_for_vehicle(this: GPtr<u8>) {
    if has_radio(this) {
        engine_stop_radio(audio_engine(), this.byte_add(0x80), 0);
    }
    PLAYER_ATTACHED_FOR_RADIO.set(0);
    VEHICLE_AUDIO_SETTINGS_FOR_RADIO.set(0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_VEHICLE_NON_DRIVE_WHEEL_SKID_VALUE,
    HOOK_GET_VEHICLE_DRIVE_WHEEL_SKID_VALUE,
    HOOK_CONSTRUCTOR,
    HOOK_STATIC_INITIALISE,
    HOOK_UPDATE_VEHICLE_ENGINE_SOUND,
    HOOK_IS_ACC_INHIBITED,
    HOOK_JUST_FINISHED_ACCELERATION_LOOP,
    HOOK_GET_SIREN_STATE,
    HOOK_UPDATE_GAS_PEDAL_AUDIO,
    HOOK_SERVICE,
    HOOK_JUST_GOT_OUT_OF_VEHICLE_AS_DRIVER,
    HOOK_GET_ACCEL_AND_BRAKE,
    HOOK_CANCEL_VEHICLE_ENGINE_SOUND,
    HOOK_REQUEST_BANK_SLOT,
    HOOK_GET_FREQUENCY_FOR_DUMMY_IDLE,
    HOOK_GET_FREQUENCY_FOR_DUMMY_REV,
    HOOK_GET_HORN_STATE,
    HOOK_TURN_ON_RADIO_FOR_VEHICLE,
    HOOK_TURN_OFF_RADIO_FOR_VEHICLE,
    HOOK_GET_FLYING_METAL_VOLUME,
    HOOK_GET_VOLUME_FOR_DUMMY_IDLE,
    HOOK_GET_VOLUME_FOR_DUMMY_REV,
    HOOK_PROCESS_BOAT_MOVING_OVER_WATER,
    HOOK_PROCESS_REVERSE_GEAR,
];
