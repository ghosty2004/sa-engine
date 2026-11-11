//! PORT-AUDIO-3 hand ports, helper A: the remaining CAEVehicleAudioEntity bodies (see `docs/arch/audio-port.md`, PORT-AUDIO-3).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids (the exe wins).
//!
//! Conventions: every stack argument word is passed through as `u32` (int16 / bool arguments keep the
//! caller's upper bytes, as the original's `push` of the whole slot); floats are rounded where the
//! original stores them (`fstp dword`). The vehicle entity's temp sound is `this + 8` (a member, not a
//! stack local); the `Play*Sound` functions that build a stack `CAESound` read its never-written words
//! through `peek` (see [`stack_sound`]).

#[allow(unused_imports)]
use sa_core::abi::HookReg;
#[allow(unused_imports)]
use sa_core::{hook, orig};

#[allow(unused_imports)]
use crate::common::{GPtr, X87, g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
}

/// `AEAudioHardware`, `AESoundManager`, `AudioEngine`
const HW: usize = 0xB5F8B8;
const SM: usize = 0xB62CB0;
const ENGINE: usize = 0xB6BC90;
/// `CAEVehicleAudioEntity::s_pPlayerAttachedForRadio` / `s_pVehicleAudioSettingsForRadio`
const PLAYER_ATTACHED_FOR_RADIO: usize = 0xB6B98C;
const VEHICLE_AUDIO_SETTINGS_FOR_RADIO: usize = 0xB6B99C;
/// `CAEVehicleAudioEntity::s_HelicoptorsDisabled`
const HELICOPTORS_DISABLED: usize = 0xB6B994;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// 1.0f as bits
const ONE: u32 = 0x3F80_0000;

orig! {
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    fn is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAudioEngine::StopRadio(tVehicleAudioSettings*, bool)` (0x506F70)
    fn engine_stop_radio(engine: GPtr<u8>, settings: GPtr<u8>, duck: u32) = thiscall 0x506F70 => unported;
    /// `CAEAudioUtility::GetRandomNumberInRange(int, int)` (0x4D9C10)
    fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
    /// `CAESound::Initialise(...)` (0x4EFE50, 14 argument dwords)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, id: u32, entity: u32, px: u32, py: u32, pz: u32, volume: u32, max_distance: u32, speed: u32, time_scale: u32, ignored_cycles: u32, flags: u32, speed_variance: u32, play_pos: u32) = thiscall 0x4EFE50 => crate::sound::initialise;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAESound::SetFlags(uint16 flags, uint16 enabled)` (0x4EF2B0)
    fn sound_set_flags(sound: GPtr<u8>, flags: u32, enabled: u32) = thiscall 0x4EF2B0 => crate::sound::set_flags;
    /// `CAESound::StopSound()` (0x4EF1C0)
    fn sound_stop_sound(sound: GPtr<u8>) = thiscall 0x4EF1C0 => crate::sound::stop_sound;
    /// `CAETwinLoopSoundEntity::Initialise(int16 slot, int16 sfx1, int16 sfx2, CAEAudioEntity*, int16, int16, int16, int16)` (0x4F28A0)
    fn twin_initialise(twin: GPtr<u8>, slot: u32, sfx1: u32, sfx2: u32, ent: u32, min_time: u32, max_time: u32, a: u32, b: u32) = thiscall 0x4F28A0 => unported;
    /// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960)
    fn twin_stop_sound_and_forget(twin: GPtr<u8>) = thiscall 0x4F2960 => crate::entities::twin_loop_stop_sound_and_forget;
    /// `CAETwinLoopSoundEntity::UpdateTwinLoopSound(CVector pos, float volume, float speed)` (0x4F29A0)
    fn twin_update(twin: GPtr<u8>, px: u32, py: u32, pz: u32, volume: u32, speed: u32) = thiscall 0x4F29A0 => unported;
    /// `CAETwinLoopSoundEntity::PlayTwinLoopSound(CVector pos, float volume, float speed, float maxDistance, float timeScale, eSoundEnvironment)` (0x4F2CB0)
    fn twin_play(twin: GPtr<u8>, px: u32, py: u32, pz: u32, volume: u32, speed: u32, max_distance: u32, time_scale: u32, flags: u32) = thiscall 0x4F2CB0 => unported;
    /// `CAEVehicleAudioEntity::GetSirenState(bool&, bool&, cVehicleParams&)` (0x4F62A0)
    fn get_siren_state(this: GPtr<u8>, siren: GPtr<u8>, fast: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4F62A0 => crate::vehicle::get_siren_state;
    /// `CAEVehicleAudioEntity::GetHornState(bool*, cVehicleParams&)` (0x4F61E0)
    fn get_horn_state(this: GPtr<u8>, out: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4F61E0 => crate::vehicle::get_horn_state;
    /// `CAEVehicleAudioEntity::PlayHornOrSiren(bool horn, bool siren, bool fastSiren, cVehicleParams&)` (0x4F99D0)
    fn play_horn_or_siren_call(this: GPtr<u8>, horn: u32, siren: u32, fast: u32, params: GPtr<u8>) = thiscall 0x4F99D0 => play_horn_or_siren;
    /// `CAEVehicleAudioEntity::JustWreckedVehicle()` (0x4FD0B0)
    fn just_wrecked_vehicle(this: GPtr<u8>) = thiscall 0x4FD0B0 => unported;
    /// `CAEVehicleAudioEntity::ProcessPlayerSeaPlane(cVehicleParams&)` (0x4FF320)
    fn pa_process_player_sea_plane(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FF320 => unported;
    /// `CAEVehicleAudioEntity::ProcessDummySeaPlane(cVehicleParams&)` (0x4FF7C0)
    fn pa_process_dummy_sea_plane(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FF7C0 => process_dummy_sea_plane;
    /// `CAEVehicleAudioEntity::ProcessPlayerJet(cVehicleParams&)` (0x501650)
    fn pa_process_player_jet(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x501650 => process_player_jet;
    /// `CAEVehicleAudioEntity::ProcessPlayerProp(cVehicleParams&)` (0x4FD290)
    fn pa_process_player_prop(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FD290 => unported;
    /// `CAEVehicleAudioEntity::ProcessDummyJet(cVehicleParams&)` (0x501960)
    fn pa_process_dummy_jet(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x501960 => process_dummy_jet;
    /// `CAEVehicleAudioEntity::ProcessDummyProp(cVehicleParams&)` (0x4FD8F0)
    fn pa_process_dummy_prop(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FD8F0 => unported;
    /// `CAEVehicleAudioEntity::ProcessAIProp(cVehicleParams&)` (0x4FDFD0)
    fn pa_process_ai_prop(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FDFD0 => process_ai_prop;
    /// `CAEVehicleAudioEntity::ProcessPlayerHeli(cVehicleParams&)` (0x4FE420)
    fn pa_process_player_heli(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FE420 => unported;
    /// `CAEVehicleAudioEntity::ProcessAIHeli(cVehicleParams&)` (0x4FEE20)
    fn pa_process_ai_heli(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FEE20 => process_ai_heli;
    /// `CAEVehicleAudioEntity::ProcessDummyHeli(cVehicleParams&)` (0x4FE940)
    fn pa_process_dummy_heli(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4FE940 => process_dummy_heli;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn get_wheel_status(dm: GPtr<u8>, wheel: u32) -> u32 = thiscall 0x6C21B0 => unported;
    /// `SurfaceInfos_c` audio surface tests (`g_surfaceInfos` 0xB79538): 0x55EA50, 0x55EB10 (group 1),
    /// 0x55EA90, 0x55EA70, 0x55EAD0 (group 2; 0x55EAD0 = `IsAudioWater`)
    fn surface_55ea50(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA50 => unported;
    fn surface_55eb10(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EB10 => unported;
    fn surface_55ea90(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA90 => unported;
    fn surface_55ea70(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EA70 => unported;
    fn surface_55ead0(infos: GPtr<u8>, s: u32) -> u8 = thiscall 0x55EAD0 => unported;
    /// `CAEVehicleAudioEntity::GetVehicleDriveWheelSkidValue(CVehicle*, tWheelState, float, cTransmission&, float)` (0x4F5F30), ST0
    fn drive_wheel_skid_value_st0(this: GPtr<u8>, veh: u32, state: u32, gas: u32, tr: u32, vel: u32) -> f64 = thiscall 0x4F5F30 => unported;
    /// `CAEVehicleAudioEntity::GetVehicleNonDriveWheelSkidValue(CVehicle*, tWheelState, cTransmission&, float)` (0x4F6000), ST0
    fn non_drive_wheel_skid_value_st0(this: GPtr<u8>, veh: u32, state: u32, tr: u32, vel: u32) -> f64 = thiscall 0x4F6000 => unported;
    /// `CAEVehicleAudioEntity::PlayFlatTyreSound` (0x4F8650)
    fn play_flat_tyre_sound_call(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F8650 => play_flat_tyre_sound;
    /// `CAEVehicleAudioEntity::PlayRoadNoiseSound` (0x4F84D0)
    fn play_road_noise_sound_call(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F84D0 => play_road_noise_sound;
    /// `CAEVehicleAudioEntity::PlaySkidSound` (0x4F8360)
    fn play_skid_sound_call(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F8360 => play_skid_sound;
    /// `CAESound::SetPosition(CVector)` (0x4EF880)
    fn sound_set_position(sound: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4EF880 => crate::sound::set_position;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, entity: GPtr<u8>) = thiscall 0x4EF820 => crate::sound::register_with_physical_entity;
    /// `CAEVehicleAudioEntity::GetAircraftNearPosition(CVector* out)` (0x4F96A0): returns `out`
    fn get_aircraft_near_position(this: GPtr<u8>, out: GPtr<u8>) -> u32 = thiscall 0x4F96A0 => unported;
    /// `CAEVehicleAudioEntity::Process*(cVehicleParams&)` bodies `ProcessVehicle` dispatches to
    fn pv_special_vehicle(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x501AB0 => process_special_vehicle;
    fn pv_boat_engine(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5003A0 => process_boat_engine;
    fn pv_rain(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F92C0 => process_rain_on_vehicle;
    fn pv_boat_moving_over_water(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FA0C0 => crate::vehicle::process_boat_moving_over_water;
    fn pv_train_track_sound(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FA3F0 => process_train_track_sound;
    fn pv_dummy_train_engine(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500710 => process_dummy_train_engine;
    fn pv_player_train_brakes(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500AB0 => process_player_train_brakes;
    fn pv_update_gas_pedal_audio(this: GPtr<u8>, veh: u32, ty: u32) = thiscall 0x4F5EB0 => unported;
    fn pv_road_noise(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F8B00 => process_vehicle_road_noise;
    fn pv_skidding(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F8F10 => process_vehicle_skidding;
    fn pv_siren_alarm_horn(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5002C0 => process_vehicle_siren_alarm_horn;
    fn pv_player_vehicle_engine(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FBB10 => unported;
    fn pv_dummy_vehicle_engine(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x501480 => unported;
    fn pv_engine_damage(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FAE20 => unported;
    fn pv_flat_tyre(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F8940 => process_vehicle_flat_tyre;
    fn pv_player_bicycle(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500040 => process_player_bicycle;
    fn pv_dummy_bicycle(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FFDC0 => process_dummy_bicycle;
    fn pv_aircraft(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x501C50 => process_aircraft;
    fn pv_moving_parts(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FB260 => process_moving_parts;
    fn pv_reverse_gear(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4F8DF0 => crate::vehicle::process_reverse_gear;
    fn pv_nitro(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FB070 => unported;
    fn pv_player_combine(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500CE0 => process_player_combine;
}

/// `CAEVehicleAudioEntity::ProcessVehicle(CPhysical*)` (0x501E10): builds a `cVehicleParams` (0x4C
/// bytes) on its stack at entry ESP - 0x4C: +0 = vehicle +0x594, +4 = vehicle type (+0x590), +8 byte 0,
/// +0xC 0, +0x10 the vehicle, +0x14 transmission (handling +0x2C, or 0 without handling +0x384), +0x18
/// model - 400, +0x1C speed (train status 0x10: `+0x3C8 * 0.02`, else `(v.z*f.z + v.y*f.y) + v.x*f.x`
/// of the move speed and the matrix forward), +0x20 / +0x22 words 0, +0x24 0, +0x2C..+0x34 0, +0x38 /
/// +0x39 bytes 0, +0x3C / +0x40 0. Bytes +9..+0xB, +0x28..+0x2B, +0x3A / +0x3B and +0x44..+0x4B are
/// never written: they keep the original's stack residue (read through `peek`). Then by audio type
/// (+0x80): 0 automobile, 1 bike, 2 bicycle, 3 boat, 4 heli, 5 plane, 6 seaplane, 8 train, 9 special
/// (the exe's jump table 0x50224C; the call sequences in the body), 7 / others nothing. Automobile and
/// bikes store the params speed (as left by the callees) at vehicle +0x8B0 / +0x7B8.
#[hook(addr = 0x501E10, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessVehicle", expect = "83 EC 4C 53 33 DB", peek = 0x4C)]
pub fn process_vehicle(this: GPtr<u8>, veh: GPtr<u8>) {
    let mut buf = [0u32; 0x4C / 4];
    for (i, w) in buf.iter_mut().enumerate() {
        *w = sa_core::abi::peek_stack(0x501E10, -0x4C + 4 * i as i32).unwrap_or(0);
    }
    let p = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(p, 0, 0xFFFF_FFFF);
    wr::<u32>(p, 4, 0xFFFF_FFFF);
    wr::<u8>(p, 8, 0);
    for off in [0xCisize, 0x14, 0x18, 0x1C] {
        wr::<u32>(p, off, 0);
    }
    wr::<u16>(p, 0x20, 0);
    wr::<u16>(p, 0x22, 0);
    for off in [0x24isize, 0x2C, 0x30, 0x34] {
        wr::<u32>(p, off, 0);
    }
    wr::<u8>(p, 0x38, 0);
    wr::<u8>(p, 0x39, 0);
    wr::<u32>(p, 0x3C, 0);
    wr::<u32>(p, 0x40, 0);
    wr::<u32>(p, 0x10, veh.addr() as u32);
    let h = rd::<u32>(veh, 0x384);
    wr::<u32>(p, 0x14, if h != 0 { h.wrapping_add(0x2C) } else { 0 });
    wr::<i32>(p, 0x18, (rd::<i16>(veh, 0x22) as i32).wrapping_sub(400));
    let speed = if rd::<u8>(veh, 0x36) & 0xF8 == 0x10 {
        x(rd::<f32>(veh, 0x3C8)).fmul_m(gf(0x858B38))
    } else {
        let m: GPtr<u8> = g(rd::<u32>(veh, 0x14) as usize);
        x(rd::<f32>(veh, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(rd::<f32>(veh, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(rd::<f32>(veh, 0x44)).fmul_m(rd::<f32>(m, 0x10)))
    };
    wr::<f32>(p, 0x1C, speed.to_f32());
    let vtype = rd::<u32>(veh, 0x590);
    wr::<u32>(p, 4, vtype);
    wr::<u32>(p, 0, rd::<u32>(veh, 0x594));
    let v = veh.addr() as u32;
    let wrecked = || rd::<u8>(this, 0xA8) == 1;
    let player = || rd::<u8>(this, 0xA5) != 0;
    let moving_train = || rd::<u8>(g::<u8>(rd::<u32>(p, 0x10) as usize), 0x36) & 0xF8 == 0x10;
    match rd::<i8>(this, 0x80) {
        0 => {
            pv_update_gas_pedal_audio(this, v, vtype);
            if wrecked() {
                return;
            }
            pv_road_noise(this, p);
            if player() {
                pv_reverse_gear(this, p);
            }
            if !moving_train() {
                pv_skidding(this, p);
            }
            if !moving_train() {
                pv_flat_tyre(this, p);
            }
            pv_siren_alarm_horn(this, p);
            if player() {
                pv_player_vehicle_engine(this, p);
            } else {
                pv_dummy_vehicle_engine(this, p);
            }
            if !moving_train() {
                pv_engine_damage(this, p);
            }
            pv_rain(this, p);
            if rd::<u32>(g::<u8>(rd::<u32>(p, 0x10) as usize), 0x590) == 0 {
                pv_nitro(this, p);
            }
            pv_moving_parts(this, p);
            if rd::<u16>(g::<u8>(rd::<u32>(p, 0x10) as usize), 0x22) == 0x214 {
                pv_player_combine(this, p);
            }
            wr::<u32>(veh, 0x8B0, rd::<u32>(p, 0x1C));
        }
        1 => {
            if wrecked() {
                return;
            }
            pv_update_gas_pedal_audio(this, v, vtype);
            pv_road_noise(this, p);
            if !moving_train() {
                pv_skidding(this, p);
            }
            pv_siren_alarm_horn(this, p);
            if player() {
                pv_player_vehicle_engine(this, p);
            } else {
                pv_dummy_vehicle_engine(this, p);
            }
            if !moving_train() {
                pv_engine_damage(this, p);
            }
            if !moving_train() {
                pv_flat_tyre(this, p);
            }
            pv_rain(this, p);
            wr::<u32>(veh, 0x7B8, rd::<u32>(p, 0x1C));
        }
        2 => {
            if wrecked() {
                return;
            }
            pv_update_gas_pedal_audio(this, v, vtype);
            pv_skidding(this, p);
            pv_siren_alarm_horn(this, p);
            if player() {
                pv_player_bicycle(this, p);
            } else {
                pv_dummy_bicycle(this, p);
            }
            wr::<u32>(veh, 0x7B8, rd::<u32>(p, 0x1C));
        }
        3 => {
            pv_boat_engine(this, p);
            pv_rain(this, p);
            pv_boat_moving_over_water(this, p);
        }
        4 => {
            pv_update_gas_pedal_audio(this, v, vtype);
            pv_aircraft(this, p);
            pv_rain(this, p);
        }
        5 => {
            pv_update_gas_pedal_audio(this, v, vtype);
            pv_aircraft(this, p);
            pv_rain(this, p);
            pv_moving_parts(this, p);
        }
        6 => {
            pv_aircraft(this, p);
            pv_rain(this, p);
            pv_boat_moving_over_water(this, p);
        }
        8 => {
            pv_train_track_sound(this, p);
            if player() {
                pv_dummy_train_engine(this, p);
                pv_player_train_brakes(this, p);
            } else {
                pv_dummy_train_engine(this, p);
            }
        }
        9 => pv_special_vehicle(this, p),
        _ => {}
    }
}

/// `AEAudioEventVolumes[event]` (int8 table through the pointer at 0xBD00F8), `fild` + `fstp` to f32.
fn event_volume(event: u32) -> f32 {
    let t = g::<u32>(0xBD00F8).read();
    crate::common::xi(g::<i8>(t.wrapping_add(event) as usize).read() as i32).to_f32()
}

/// `CAEVehicleAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4FB6C0): nothing for a
/// null sound. Event 0x5D (flying metal): with a physical entity (+8), volume (+0x14) = `f32(
/// GetFlyingMetalVolume(physical) + event volume 0x5D)`, done. Event 0x63 with a live position: the
/// sound moves to the vehicle and is stopped when `UnderWaterness >= 0.5`. Then: a finished sound
/// (`curPlayPos == -1`) is forgotten in the first of +0x158 / +0x160 / +0x168 / +0x170 / +0x178 /
/// +0x17C / +0x180 / the twelve engine sounds that holds it; a live one moves to the vehicle (aircraft
/// types 4 / 5 / 6 with the engine sound +0x100: `GetAircraftNearPosition`). Types 0 / 1 with the AC
/// sound (+0x108): +0x14A = +0x148, +0x148 = curPlayPos.
/// `GetFlyingMetalVolume` runs as its Rust port (its full-width X87 result, as the original's ST0).
#[hook(addr = 0x4FB6C0, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateParameters", expect = "83 EC 0C 53 55")]
pub fn update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    if sound.is_null() {
        return;
    }
    let s = sound.addr() as u32;
    let ev = rd::<u32>(sound, 0xC);
    if ev == 0x5D {
        let phys = rd::<u32>(sound, 8);
        if phys == 0 {
            return;
        }
        let ev_vol = event_volume(0x5D);
        let v = crate::vehicle::get_flying_metal_volume(this, g(phys as usize)).fadd_m(ev_vol);
        wr::<f32>(sound, 0x14, v.to_f32());
        return;
    }
    let pos16 = pos as u16;
    if ev == 0x63 && pos16 != 0xFFFF {
        let p = veh_pos(this);
        sound_set_position(sound, p[0], p[1], p[2]);
        if !x(gf(0xC8132C)).fcom(x(gf(0x858B8C))).test_any(0x01) {
            sound_stop_sound(sound);
        }
    }
    if pos16 == 0xFFFF {
        let mut done = false;
        for off in [0x158isize, 0x160, 0x168, 0x170, 0x178, 0x17C, 0x180] {
            if rd::<u32>(this, off) == s {
                wr::<u32>(this, off, 0);
                done = true;
                break;
            }
        }
        if !done && let Some(i) = (0..12isize).find(|&i| rd::<u32>(this, 0xE8 + i * 8) == s) {
            wr::<u32>(this, 0xE8 + i * 8, 0);
        }
    } else {
        let ty = rd::<u8>(this, 0x80);
        let mut buf = [0u32; 3];
        let p = if matches!(ty, 4..=6) && rd::<u32>(this, 0x100) == s {
            let out = get_aircraft_near_position(this, GPtr::from_raw(buf.as_mut_ptr().cast::<u8>()));
            let o: GPtr<u8> = g(out as usize);
            [rd::<u32>(o, 0), rd::<u32>(o, 4), rd::<u32>(o, 8)]
        } else {
            veh_pos(this)
        };
        sound_set_position(sound, p[0], p[1], p[2]);
    }
    let ty = rd::<u8>(this, 0x80);
    if (ty == 0 || ty == 1) && rd::<u32>(this, 0x108) == s {
        wr::<u16>(this, 0x14A, rd::<u16>(this, 0x148));
        wr::<u16>(this, 0x148, pos16);
    }
}

/// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, CPhysical*)` (0x4F7580): with banks 0x8A (slot
/// 0x13) and 0x27 (slot 2) loaded, event 0x5D and a physical: the temp sound (+8) plays sfx 0xC (slot
/// 0x13) at the physical's position, volume `f32(GetFlyingMetalVolume(physical) + event volume 0x5D)`,
/// 1.0 x3, zeros; +0x20 = 0.85, flags word +0x5E = 0x84, registered with the physical, event +0x14 =
/// 0x5D, requested. `GetFlyingMetalVolume` runs as its Rust port (full-width X87, as the ST0).
#[hook(addr = 0x4F7580, conv = thiscall, name = "CAEVehicleAudioEntity::AddAudioEvent-1", expect = "53 56 57 6A 13")]
pub fn add_audio_event_physical(this: GPtr<u8>, event: u32, phys: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 || is_sound_bank_loaded(g(HW), 0x27, 2) == 0 {
        return;
    }
    if event != 0x5D {
        return;
    }
    let ev_vol = event_volume(0x5D);
    if phys.is_null() {
        return;
    }
    let vol = crate::vehicle::get_flying_metal_volume(this, phys)
        .fadd_m(ev_vol)
        .to_f32();
    let p = entity_pos(phys);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        0x13,
        0xC,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(this, 0x20, 0x3F59_999A);
    wr::<u16>(this, 0x5E, 0x84);
    sound_register_with_physical_entity(snd, phys);
    wr::<u32>(this, 0x14, 0x5D);
    sm_request_new_sound(g(SM), snd);
}

/// `g_surfaceInfos`
const SURFACE_INFOS: usize = 0xB79538;
/// 0.0f / 1.0f / 20.0f / -100.0f (`.rdata`)
const FLT_0: usize = 0x858B50;
const FLT_1: usize = 0x858624;
const FLT_20: usize = 0x858BA4;
const FLT_M100: usize = 0x859014;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// A `.rdata` double as an x87 operand.
fn gd(a: usize) -> X87 {
    X87::from_f64_exact(f64::from_bits(g::<u64>(a).read()))
}

/// The audio surface groups of the vehicle's contact surface (vehicle byte +0xBA): 1 = 0x55EA50 /
/// 0x55EB10, 2 = 0x55EA90 / 0x55EA70 / 0x55EAD0, 0 = neither (tested in this order, as the bytes).
fn surface_group(params: GPtr<u8>) -> u8 {
    let s = || rd::<u8>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0xBA) as u32;
    let si = || g::<u8>(SURFACE_INFOS);
    if surface_55ea50(si(), s()) != 0 || surface_55eb10(si(), s()) != 0 {
        return 1;
    }
    if surface_55ea90(si(), s()) != 0 || surface_55ea70(si(), s()) != 0 || surface_55ead0(si(), s()) != 0 {
        return 2;
    }
    0
}

/// `CAEVehicleAudioEntity::ProcessVehicleFlatTyre(cVehicleParams&)` (0x4F8940): a burst wheel touching
/// the ground (automobile type 0, or type 9 with model 0x1C9: `GetWheelStatus(i) == 1` and vehicle
/// +0x7F4 + 4i > 0 for wheels 0..3; bike types 1 / 2: byte +0x65C + i == 1 and +0x730 + 4i > 0 for wheels
/// 0..1) gives `t = min(|speed| / (max velocity * 0.3), 1)`, sfx 4, speed `f32(0.4 * t + 1.0)` and, when
/// t > 0, volume `f32(AudioLog10(f32 t) * 20 - 9)` (else -100); otherwise sfx -1, speed 1.0, volume -100.
/// Then `PlayFlatTyreSound`. (The original also stores `f32 t` into its own argument slot.)
#[hook(addr = 0x4F8940, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessVehicleFlatTyre", expect = "83 EC 08 53 55")]
pub fn process_vehicle_flat_tyre(this: GPtr<u8>, params: GPtr<u8>) {
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let pos = |off: isize| !x(rd::<f32>(veh(), off)).fcom(x(gf(FLT_0))).test_any(0x41);
    let car_burst =
        || (0..4u32).any(|i| get_wheel_status(veh().byte_add(0x5A0), i) == 1 && pos(0x7F4 + i as isize * 4));
    let burst = match rd::<i8>(this, 0x80) {
        0 => car_burst(),
        1 | 2 => (0..2isize).any(|i| rd::<u8>(veh(), 0x65C + i) == 1 && pos(0x730 + i * 4)),
        9 => rd::<u16>(veh(), 0x22) == 0x1C9 && car_burst(),
        _ => false,
    };
    let (mut sfx, mut speed, mut vol) = (0xFFFF_FFFFu32, ONE, 0xC2C8_0000u32);
    if burst {
        let tr: GPtr<u8> = g(rd::<u32>(params, 0x14) as usize);
        let mut t = x(rd::<f32>(params, 0x1C))
            .fabs()
            .fdiv(x(rd::<f32>(tr, 0x58)).fmul_m(gf(0x858C24)));
        if !x(gf(FLT_1)).fcom(t).test_parity(0x05) {
            t = x(gf(FLT_1));
        }
        let tf = t.to_f32();
        sfx = 4;
        speed = x(gf(0x8CBD18)).fmul(t).fadd(gd(0x85A310)).to_f32().to_bits();
        if !t.fcom(gd(0x859EF8)).test_any(0x41) {
            vol = crate::utility::audio_log10(tf)
                .fmul_m(gf(FLT_20))
                .fadd_m(gf(0x8CBD1C))
                .to_f32()
                .to_bits();
        }
    }
    play_flat_tyre_sound_call(this, sfx, speed, vol);
}

/// `CAEVehicleAudioEntity::ProcessVehicleRoadNoise(cVehicleParams&)` (0x4F8B00): wheels on the ground
/// (type 0: vehicle +0x960, type 9: +0x804, else 4) and a transmission (trailer models 435 / 450 / 584 /
/// 591 skip that check) and `|speed| > 0`: `ratio = min(2|speed|, 1)`; speed = `f32(max(dist / 72, 1)
/// * 0.5 + 0.75)` with `dist` = camera-to-vehicle distance (`sqrt((dz^2 + dx^2) + dy^2)`); surface group
/// 1: sfx 0x15, speed *= 1.3, base `[0xB6B9E4] - 12`; group 2: sfx 0x16, speed *= 1.0, base `4.5 - 12`;
/// else sfx 0x15, base -12; volume `f32(AudioLog10(ratio) * 20 + base)` when ratio > 8e-5, else -100.
/// Otherwise sfx -1, speed 1.0, volume -100. Then `PlayRoadNoiseSound`.
#[hook(addr = 0x4F8B00, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessVehicleRoadNoise", expect = "83 EC 0C 53 55")]
pub fn process_vehicle_road_noise(this: GPtr<u8>, params: GPtr<u8>) {
    let (mut sfx, mut speed, mut vol) = (0xFFFF_FFFFu32, ONE, 0xC2C8_0000u32);
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let wheels = match rd::<u32>(params, 4) {
        0 => rd::<u8>(veh, 0x960),
        9 => rd::<u8>(veh, 0x804),
        _ => 4,
    };
    let trailer = matches!(rd::<i16>(veh, 0x22), 435 | 450 | 584 | 591);
    let s = x(rd::<f32>(params, 0x1C)).fabs();
    if (trailer || rd::<u32>(params, 0x14) != 0) && wheels != 0 && !s.fcom(x(gf(FLT_0))).test_any(0x41) {
        let s2 = s.fadd(s);
        let ratio = if x(gf(FLT_1)).fcom(s2).test_parity(0x05) {
            s2.to_f32()
        } else {
            gf(FLT_1)
        };
        let cm = g::<u32>(0xB6F03C).read();
        let c: GPtr<u8> = if cm != 0 {
            g(cm as usize + 0x30)
        } else {
            g(0xB6F02C)
        };
        let p = veh_pos(this);
        let d = |i: usize| x(f32::from_bits(p[i])).fsub_m(rd::<f32>(c, i as isize * 4));
        let (dx, dy, dz) = (d(0), d(1), d(2));
        let mut q = dz
            .fmul(dz)
            .fadd(dx.fmul(dx))
            .fadd(dy.fmul(dy))
            .fsqrt()
            .fdiv_m(gf(0x8CBD10));
        if !q.fcom(gd(0x85A310)).test_parity(0x05) {
            q = gd(0x85A310);
        }
        let mut sp = q.fmul(gd(0x859060)).fadd(gd(0x862CF8)).to_f32();
        let base = match surface_group(params) {
            1 => {
                sp = x(gf(0x8CBD04)).fmul_m(sp).to_f32();
                sfx = 0x15;
                x(gf(0xB6B9E4)).fadd_m(gf(0x8CBD00)).to_f32()
            }
            2 => {
                sfx = 0x16;
                sp = x(gf(0x8CBD0C)).fmul_m(sp).to_f32();
                x(gf(0x8CBD08)).fadd_m(gf(0x8CBD00)).to_f32()
            }
            _ => {
                sfx = 0x15;
                gf(0x8CBD00)
            }
        };
        speed = sp.to_bits();
        if !x(ratio).fcom(x(gf(0x862CF4))).test_any(0x41) {
            vol = crate::utility::audio_log10(ratio)
                .fmul_m(gf(FLT_20))
                .fadd_m(base)
                .to_f32()
                .to_bits();
        }
    }
    play_road_noise_sound_call(this, sfx, speed, vol);
}

/// `CAEVehicleAudioEntity::ProcessVehicleSkidding(cVehicleParams&)` (0x4F8F10): only types 0
/// (automobile: 4 wheels, states +0x968, contact +0x7F4, gas +0x964, wheels on ground +0x960) and 9
/// (bike: 2 wheels, +0x80C, +0x730, +0x808, +0x804). `all` = neither wheel state [1] nor (automobile)
/// [3] is 2. With wheels on the ground and a non-zero time step, every wheel with a non-zero state and
/// contact > 0 (or NaN) adds its skid value (drive / non-drive by the transmission's drive type 4 / F /
/// R; when `all`, front wheels 0 / 2 in state 2 are skipped); an unknown drive type adds the previous
/// value still on the x87 stack (0.0 at first). For a positive sum: type 2 (+0x80) gives sfx 0 and its
/// own speed / volume offset; otherwise the surface group picks sfx 6 / 8 / 0x18 (speed curves, offsets
/// -12 / -9 / 0; type 1 scales the default speed by `[0x858F08]`) and types 4 / 5 add `[0x8CBCF4]` to
/// the offset. `avg = f32(sum / wheels)`; above 1e-5 the volume is `f32(([0xB6B9E0] + offset) +
/// AudioLog10(avg) * 20)`, kept unless below -100; otherwise sfx -1 / volume -100. Then
/// `PlaySkidSound(sfx, speed, volume)`.
#[hook(addr = 0x4F8F10, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessVehicleSkidding", expect = "83 EC 20 D9 05 50 8B 85 00")]
pub fn process_vehicle_skidding(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let (states, contact, gas, on_ground, n) = match rd::<u32>(params, 4) {
        0 => (0x968isize, 0x7F4isize, 0x964isize, 0x960isize, 4u8),
        9 => (0x80C, 0x730, 0x808, 0x804, 2),
        _ => return,
    };
    let all = if n == 4 {
        rd::<u32>(veh, states + 4) != 2 && rd::<u32>(veh, states + 0xC) != 2
    } else {
        rd::<u32>(veh, states + 4) != 2
    };
    let gas_w = rd::<u32>(veh, gas);
    let mut sum: f32 = 0.0;
    let (mut sfx, mut speed, mut off) = (0xFFFF_FFFFu32, gf(FLT_1), 0.0f32);
    let mut z = x(gf(FLT_0));
    let ts = x(gf(0xB7CB64)).fcom(x(gf(FLT_0)));
    if rd::<u8>(veh, on_ground) != 0 && ts.test_parity(0x44) {
        for i in 0..n {
            let st = rd::<u32>(veh, states + i as isize * 4);
            if st == 0 {
                continue;
            }
            if !x(rd::<f32>(veh, contact + i as isize * 4))
                .fcom(x(gf(FLT_0)))
                .test_parity(0x41)
            {
                continue;
            }
            if all && (i == 0 || i == 2) && st == 2 {
                continue;
            }
            let tr = rd::<u32>(params, 0x14);
            let vel = rd::<u32>(params, 0x1C);
            let v = veh.addr() as u32;
            let drive = match rd::<u8>(g::<u8>(tr as usize), 0x48) {
                0x34 => Some(true),
                0x46 => Some(i == 0 || i == 2),
                0x52 => Some(i == 1 || i == 3),
                _ => None,
            };
            if let Some(d) = drive {
                z = X87::from_f64_exact(if d {
                    drive_wheel_skid_value_st0(this, v, st, gas_w, tr, vel)
                } else {
                    non_drive_wheel_skid_value_st0(this, v, st, tr, vel)
                });
            }
            sum = x(sum).fadd(z).to_f32();
        }
        if !x(sum).fcom(x(gf(FLT_0))).test_any(0x41) {
            let ty = rd::<u8>(this, 0x80);
            if ty == 2 {
                speed = x(gf(0x8CBCF0)).fmul_m(sum).fadd_m(gf(0x8CBCEC)).to_f32();
                sfx = 0;
                off = gf(0x8CBCF8);
            } else {
                match surface_group(params) {
                    1 => {
                        sfx = 6;
                        off = f32::from_bits(0xC140_0000);
                        speed = x(gf(0x8CBCE0)).fmul_m(sum).fadd_m(gf(0x8CBCDC)).to_f32();
                    }
                    2 => {
                        sfx = 8;
                        off = f32::from_bits(0xC110_0000);
                        speed = x(gf(0x8CBCD8)).fmul_m(sum).fadd_m(gf(0x8CBCD4)).to_f32();
                    }
                    _ => {
                        sfx = 0x18;
                        speed = x(gf(0x8CBCE8)).fmul_m(sum).fadd_m(gf(0x8CBCE4)).to_f32();
                        if ty == 1 {
                            speed = x(speed).fmul_m(gf(0x858F08)).to_f32();
                        }
                    }
                }
                let ty = rd::<u8>(this, 0x80);
                if ty == 5 || ty == 4 {
                    off = x(gf(0x8CBCF4)).fadd_m(off).to_f32();
                }
            }
        }
    }
    let avg = x(sum).fdiv(X87::from_i32(n as i32)).to_f32();
    let mut vol = 0xC2C8_0000u32;
    let mut keep = false;
    if !x(avg).fcom(x(gf(0x858C14))).test_any(0x41) {
        let l = crate::utility::audio_log10(avg).fmul_m(gf(FLT_20));
        let v = x(gf(0xB6B9E0)).fadd_m(off).fadd(l).to_f32();
        if x(v).fcom(x(gf(FLT_M100))).test_parity(0x05) {
            vol = v.to_bits();
            keep = true;
        }
    }
    if !keep {
        sfx = 0xFFFF_FFFF;
    }
    play_skid_sound_call(this, sfx, speed.to_bits(), vol);
}

/// The entity's position (matrix +0x30 when it has one, else placement +4) as three dwords.
fn entity_pos(ent: GPtr<u8>) -> [u32; 3] {
    let m = rd::<u32>(ent, 0x14);
    let p: GPtr<u8> = if m != 0 {
        g(m as usize + 0x30)
    } else {
        ent.byte_add(4)
    };
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// The vehicle entity (`this + 4`) position.
fn veh_pos(this: GPtr<u8>) -> [u32; 3] {
    entity_pos(g(rd::<u32>(this, 4) as usize))
}

/// `volume + m_EventVolume` (+0xD8) rounded to f32 (`fld arg; fadd [esi+0xD8]; fstp`).
fn with_event_volume(this: GPtr<u8>, volume: u32) -> u32 {
    x(f32::from_bits(volume))
        .fadd_m(rd::<f32>(this, 0xD8))
        .to_f32()
        .to_bits()
}

/// A stack `CAESound` (0x74 bytes) whose words start as the original's stack residue: `off` is the
/// local's offset from the entry ESP (negative), read through the hook's `peek` frame; a direct Rust
/// call (no frame) gets zeros. Only the words `Initialise` and the caller write are later copied by
/// `RequestNewSound` (`CAESound::operator=`), so the residue is never observable; it is kept anyway.
fn stack_sound(hook: u32, off: i32) -> [u32; 0x74 / 4] {
    let mut buf = [0u32; 0x74 / 4];
    for (i, w) in buf.iter_mut().enumerate() {
        *w = sa_core::abi::peek_stack(hook, off + 4 * i as i32).unwrap_or(0);
    }
    buf
}

/// `CAEVehicleAudioEntity::StaticService()` (0x4F4EC0): empty (`ret`). Not hooked: a 1-byte body
/// (sa-hook `TooShort(1)`: the patch would cover the padding after `ret`).
pub fn static_service() {}

/// `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver()` (0x4F5BA0): marks the entity as exiting
/// (+0xA7 = 1, always); for a player driver (+0xA5) stops the radio (radio types 0/2/3, `StopRadio(+0x80,
/// 0)`) and clears both radio statics.
#[hook(addr = 0x4F5BA0, conv = thiscall, name = "CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver", expect = "90 E9 1E 00 F1 FF")]
pub fn player_about_to_exit_vehicle_as_driver(this: GPtr<u8>) {
    let driver = rd::<u8>(this, 0xA5);
    wr::<u8>(this, 0xA7, 1);
    if driver == 0 {
        return;
    }
    if matches!(rd::<i8>(this, 0x9B), 0 | 2 | 3) {
        engine_stop_radio(g(ENGINE), this.byte_add(0x80), 0);
    }
    g::<u32>(PLAYER_ATTACHED_FOR_RADIO).write(0);
    g::<u32>(VEHICLE_AUDIO_SETTINGS_FOR_RADIO).write(0);
}

/// `CAEVehicleAudioEntity::ProcessRainOnVehicle(cVehicleParams&)` (0x4F92C0): with bank 0x69 in slot 6
/// and a rain volume `[0x8CC30C]` above -100: the counter +0xB2 steps; once it reaches `[0x8CBD94]` it is
/// reset and the temp sound (+8) plays sfx `GetRandomNumberInRange(12, 15)` (slot 6, the vehicle's
/// position, volume `f32([0xB6B9EC] + [0x8CC30C])`, 1.0 x3, zeros), +0x28 = 0.1, +0x20 = 1.0, event
/// +0x14 = 0x8E, requested.
#[hook(addr = 0x4F92C0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessRainOnVehicle", expect = "51 53 56 57 6A 06")]
pub fn process_rain_on_vehicle(this: GPtr<u8>, _params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), 0x69, 6) == 0 {
        return;
    }
    let base = g::<f32>(0x8CC30C).read();
    if x(base).fcom(x(g::<f32>(0x859014).read())).test_any(0x41) {
        return;
    }
    let c = rd::<i16>(this, 0xB2).wrapping_add(1);
    wr::<i16>(this, 0xB2, c);
    let vol = x(g::<f32>(0xB6B9EC).read()).fadd(x(base)).to_f32();
    if c < g::<i16>(0x8CBD94).read() {
        return;
    }
    wr::<u16>(this, 0xB2, 0);
    let sfx = ae_random_int(12, 15) as u32;
    let p = veh_pos(this);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        6,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(this, 0x28, 0x3DCC_CCCD);
    wr::<u32>(this, 0x20, ONE);
    wr::<u32>(this, 0x14, 0x8E);
    sm_request_new_sound(g(SM), snd);
}

/// `CAEVehicleAudioEntity::UpdateBoatSound(int16 type, int16 bankSlot, int16 sfx, float speed, float
/// volume)` (0x4F9E90): nothing unless the audio type (+0x80) is 6, `type` is 6 or the engine bank slot
/// (+0xE0) is set. Volume += m_EventVolume. An existing engine sound (+0xE8 + 8 * type) gets the volume
/// and speed; otherwise types 2 (bank +0xDC in slot +0xE0, max distance `[0x8CBEE0]`), 3 (bank 0x8A, slot
/// 0x13, `[0x8CBEE4]`) and 6 (bank 0x27, slot 2, `[0x8CBEE8]`) start one through the temp sound (+8) when
/// the bank is loaded (the `bankSlot` / `sfx` argument words passed through), flags word 4.
#[hook(addr = 0x4F9E90, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateBoatSound", expect = "53 56 8B F1 90")]
pub fn update_boat_sound(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, speed: u32, volume: u32) {
    let t = ty as u16;
    if rd::<u8>(this, 0x80) != 6 && t != 6 && rd::<u16>(this, 0xE0) == 0xFFFF {
        return;
    }
    let vol = with_event_volume(this, volume);
    let idx = t as i16 as isize;
    let cur = rd::<u32>(this, 0xE8 + idx * 8);
    if cur != 0 {
        wr::<u32>(g::<u8>(cur as usize), 0x14, vol);
        let s = rd::<u32>(this, 0xE8 + idx * 8);
        wr::<u32>(g::<u8>(s as usize), 0x1C, speed);
        return;
    }
    let (bank, bslot, dist) = match idx {
        2 => (
            rd::<u16>(this, 0xDC) as u32,
            rd::<u16>(this, 0xE0) as u32,
            0x8CBEE0,
        ),
        3 => (0x8A, 0x13, 0x8CBEE4),
        6 => (0x27, 2, 0x8CBEE8),
        _ => return,
    };
    if is_sound_bank_loaded(g(HW), bank, bslot) == 0 {
        return;
    }
    let p = veh_pos(this);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(this, 0x1C, vol);
    wr::<u32>(this, 0x20, g::<u32>(dist).read());
    wr::<u32>(this, 0x24, speed);
    wr::<u16>(this, 0x5E, 4);
    let got = sm_request_new_sound(g(SM), snd);
    wr::<u32>(this, 0xE8 + idx * 8, got);
}

/// `CAEVehicleAudioEntity::UpdateGenericVehicleSound(int16 type, int16 bankSlot, int16 bankId, int16 sfx,
/// float speed, float volume, float maxDistance)` (0x4FAD40): volume += m_EventVolume; an existing engine
/// sound (+0xE8 + 8 * type) gets volume and speed; otherwise, with the bank loaded, the temp sound (+8)
/// starts it (volume, max distance, speed, flags word 4) and its handle is stored.
#[hook(addr = 0x4FAD40, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateGenericVehicleSound", expect = "E9 39 72 F0 FF D9 44 24 18")]
#[allow(clippy::too_many_arguments)]
pub fn update_generic_vehicle_sound(
    this: GPtr<u8>,
    ty: u32,
    slot: u32,
    bank: u32,
    sfx: u32,
    speed: u32,
    volume: u32,
    dist: u32,
) {
    let idx = ty as u16 as i16 as isize;
    let vol = with_event_volume(this, volume);
    let cur = rd::<u32>(this, 0xE8 + idx * 8);
    if cur != 0 {
        wr::<u32>(g::<u8>(cur as usize), 0x14, vol);
        let s = rd::<u32>(this, 0xE8 + idx * 8);
        wr::<u32>(g::<u8>(s as usize), 0x1C, speed);
        return;
    }
    if is_sound_bank_loaded(g(HW), bank, slot) == 0 {
        return;
    }
    let p = veh_pos(this);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(this, 0x24, speed);
    wr::<u32>(this, 0x20, dist);
    wr::<u32>(this, 0x1C, vol);
    wr::<u16>(this, 0x5E, 4);
    let got = sm_request_new_sound(g(SM), snd);
    wr::<u32>(this, 0xE8 + idx * 8, got);
}

/// `CAEVehicleAudioEntity::ProcessVehicleSirenAlarmHorn(cVehicleParams&)` (0x5002C0): three stack bools
/// (horn, siren, fast siren; only their low bytes are written, the rest is the stack residue the
/// argument words carry) cleared; `GetSirenState`; without a siren the horn comes from the alarm (vehicle
/// +0x45C not 0 / -1 and status not wrecked: +0x514 = now + 750 once now passed it, horn = +0x514 <
/// now + 375, unsigned) or else `GetHornState`. Then `PlayHornOrSiren(horn, siren, fast, params)` and
/// +0xBE / +0xBF / +0xC0 = the three bytes.
#[hook(addr = 0x5002C0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessVehicleSirenAlarmHorn", expect = "83 EC 0C 56 57", peek = 0xC)]
pub fn process_vehicle_siren_alarm_horn(this: GPtr<u8>, params: GPtr<u8>) {
    const H: u32 = 0x5002C0;
    // [esp0-0xC] horn, [esp0-8] siren, [esp0-4] fast siren
    let mut w = [
        sa_core::abi::peek_stack(H, -0xC).unwrap_or(0),
        sa_core::abi::peek_stack(H, -8).unwrap_or(0),
        sa_core::abi::peek_stack(H, -4).unwrap_or(0),
    ];
    for v in w.iter_mut() {
        *v &= !0xFF;
    }
    let p = GPtr::from_raw(w.as_mut_ptr().cast::<u8>());
    get_siren_state(this, p.byte_add(4), p.byte_add(8), params);
    if rd::<u8>(p, 4) == 0 {
        let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
        let a = rd::<u16>(veh, 0x45C);
        if a != 0 && a != 0xFFFF && rd::<u8>(veh, 0x36) & 0xF8 != 0x28 {
            let now = g::<u32>(TIME_MS).read();
            if now > rd::<u32>(veh, 0x514) {
                wr::<u32>(veh, 0x514, now.wrapping_add(0x2EE));
            }
            let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
            let on = rd::<u32>(veh, 0x514) < now.wrapping_add(0x177);
            wr::<u8>(p, 0, on as u8);
        } else {
            get_horn_state(this, p, params);
        }
    }
    play_horn_or_siren_call(this, w[0], w[1], w[2], params);
    wr::<u8>(this, 0xBE, w[0] as u8);
    wr::<u8>(this, 0xBF, w[1] as u8);
    wr::<u8>(this, 0xC0, w[2] as u8);
}

/// `CAEVehicleAudioEntity::ProcessAircraft(cVehicleParams&)` (0x501C50): nothing unless bank 0x8A is in
/// slot 0x13 (the bank id is decoded by the protection chunk at 0x4013B1: `[0x156062C] - [0x52A00D] - 1`
/// = 0x8A). Jets = models 519, 520, 577, 592 (jump table 0x501DB8). By audio type (+0x80, signed): 4
/// (heli) - not wrecked (+0xA8): with helicopters disabled or this one (+0xBD) `JustWreckedVehicle`, else
/// player (+0xA5) heli, AI heli (status 0x18) or dummy heli; 5 (plane) - not wrecked: player jet / prop,
/// dummy jet, AI prop (status 0x18 or vehicle byte +0x424 >= 0) or dummy prop; 6 (seaplane) - not
/// wrecked: player / dummy seaplane.
#[hook(addr = 0x501C50, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessAircraft", expect = "56 6A 13 8B F1")]
pub fn process_aircraft(this: GPtr<u8>, params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let jet = matches!(rd::<i16>(veh, 0x22), 519 | 520 | 577 | 592);
    let wrecked = || rd::<u8>(this, 0xA8) != 0;
    let player = || rd::<u8>(this, 0xA5) != 0;
    let status = || rd::<u8>(veh, 0x36) & 0xF8;
    match rd::<i8>(this, 0x80) {
        4 => {
            if wrecked() {
                return;
            }
            if g::<u8>(HELICOPTORS_DISABLED).read() != 0 || rd::<u8>(this, 0xBD) != 0 {
                just_wrecked_vehicle(this);
            } else if player() {
                pa_process_player_heli(this, params);
            } else if status() == 0x18 {
                pa_process_ai_heli(this, params);
            } else {
                pa_process_dummy_heli(this, params);
            }
        }
        5 => {
            if wrecked() {
                return;
            }
            if player() {
                if jet {
                    pa_process_player_jet(this, params);
                } else {
                    pa_process_player_prop(this, params);
                }
            } else if jet {
                pa_process_dummy_jet(this, params);
            } else if status() != 0x18 && rd::<i8>(veh, 0x424) < 0 {
                pa_process_dummy_prop(this, params);
            } else {
                pa_process_ai_prop(this, params);
            }
        }
        6 => {
            if wrecked() {
                return;
            }
            if player() {
                pa_process_player_sea_plane(this, params);
            } else {
                pa_process_dummy_sea_plane(this, params);
            }
        }
        _ => {}
    }
}

/// `CAEVehicleAudioEntity::PlaySkidSound(int16 sfx, float speed, float volume)` (0x4F8360): volume +=
/// m_EventVolume. Same sfx as +0x156: the twin loop (+0x184) is updated unless it is -1. A new sfx: the
/// twin loop is stopped when initialised (+0x20C), +0x156 = sfx, and unless -1 the twin loop is
/// initialised (slot 0x13, sfx, sfx + 1, this, 200, 1000, -1, -1) and played (max distance
/// `[0x8CBCFC]`, time scale 1.0, flags 0). The stack `CAESound` the original builds is only destroyed
/// (its physical pointer cleared first): no observable effect.
#[hook(addr = 0x4F8360, conv = thiscall, name = "CAEVehicleAudioEntity::PlaySkidSound", expect = "6A FF E9 3A 96 F0 FF")]
pub fn play_skid_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) {
    play_twin_loop(this, 0x13, 0x8CBCFC, sfx, speed, volume);
}

/// The shared body of `PlaySkidSound` / `PlayTrainBrakeSound` (after the train's bank check).
fn play_twin_loop(this: GPtr<u8>, bank_slot: u32, dist: usize, sfx: u32, speed: u32, volume: u32) {
    let vol = with_event_volume(this, volume);
    let twin = this.byte_add(0x184);
    if rd::<u16>(this, 0x156) == sfx as u16 {
        if sfx as u16 != 0xFFFF {
            let p = veh_pos(this);
            twin_update(twin, p[0], p[1], p[2], vol, speed);
        }
        return;
    }
    if rd::<u16>(this, 0x20C) != 0 {
        twin_stop_sound_and_forget(twin);
    }
    wr::<u16>(this, 0x156, sfx as u16);
    if sfx as u16 == 0xFFFF {
        return;
    }
    twin_initialise(
        twin,
        bank_slot,
        sfx,
        sfx.wrapping_add(1),
        this.addr() as u32,
        200,
        1000,
        0xFFFF_FFFF,
        0xFFFF_FFFF,
    );
    let p = veh_pos(this);
    twin_play(twin, p[0], p[1], p[2], vol, speed, g::<u32>(dist).read(), ONE, 0);
}

/// `CAEVehicleAudioEntity::PlayTrainBrakeSound(int16 sfx, float speed, float volume)` (0x4FA630): nothing
/// unless the engine bank (+0xDE) is set and loaded in slot 0x28; then as `PlaySkidSound` with twin-loop
/// slot 0x28 and max distance `[0x8CBEF8]`.
#[hook(addr = 0x4FA630, conv = thiscall, name = "CAEVehicleAudioEntity::PlayTrainBrakeSound", expect = "6A FF 68 A8 C3 83 00")]
pub fn play_train_brake_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) {
    let bank = rd::<u16>(this, 0xDE);
    if bank == 0xFFFF || is_sound_bank_loaded(g(HW), bank as u32, 0x28) == 0 {
        return;
    }
    play_twin_loop(this, 0x28, 0x8CBEF8, sfx, speed, volume);
}

/// The shared body of `PlayRoadNoiseSound` / `PlayFlatTyreSound` / `PlayReverseSound`: `cur` = the
/// current sfx word, `slot` = the sound pointer; volume += m_EventVolume. Same sfx: the sound (if any)
/// gets speed and volume unless the sfx is -1. A new sfx: the old sound is stopped (flag 4 cleared) and
/// forgotten, `cur` = sfx and unless -1 a stack `CAESound` (at entry ESP - 0x80) is initialised (slot
/// 0x13, sfx, this, the vehicle's position, volume 0.0, 1.0 x3, zeros), then volume, speed, max distance
/// (`dist`, `None` = left at 1.0) and flags 4 are set and it is requested.
#[allow(clippy::too_many_arguments)]
fn play_simple_sound(
    hook: u32,
    this: GPtr<u8>,
    cur: isize,
    slot: isize,
    dist: Option<usize>,
    sfx: u32,
    speed: u32,
    volume: u32,
) {
    let mut buf = stack_sound(hook, -0x80);
    let local = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(local, 8, 0);
    let vol = with_event_volume(this, volume);
    if rd::<u16>(this, cur) == sfx as u16 {
        if sfx as u16 != 0xFFFF {
            let s = rd::<u32>(this, slot);
            if s != 0 {
                wr::<u32>(g::<u8>(s as usize), 0x1C, speed);
                let s = rd::<u32>(this, slot);
                wr::<u32>(g::<u8>(s as usize), 0x14, vol);
            }
        }
        return;
    }
    let s = rd::<u32>(this, slot);
    if s != 0 {
        sound_set_flags(g(s as usize), 4, 0);
        sound_stop_sound(g(rd::<u32>(this, slot) as usize));
        wr::<u32>(this, slot, 0);
    }
    wr::<u16>(this, cur, sfx as u16);
    if sfx as u16 == 0xFFFF {
        return;
    }
    let p = veh_pos(this);
    sound_initialise(
        local,
        0x13,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        0,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(local, 0x14, vol);
    if let Some(d) = dist {
        wr::<u32>(local, 0x1C, speed);
        wr::<u16>(local, 0x56, 4);
        wr::<u32>(local, 0x18, g::<u32>(d).read());
    } else {
        wr::<u32>(local, 0x1C, speed);
        wr::<u16>(local, 0x56, 4);
    }
    let got = sm_request_new_sound(g(SM), local);
    wr::<u32>(this, slot, got);
}

/// `CAEVehicleAudioEntity::PlayRoadNoiseSound(int16 sfx, float speed, float volume)` (0x4F84D0): see
/// [`play_simple_sound`]; sfx +0x15C, sound +0x160, max distance `[0x8CBD14]`.
#[hook(addr = 0x4F84D0, conv = thiscall, name = "CAEVehicleAudioEntity::PlayRoadNoiseSound", expect = "6A FF 68 08 C3 83 00", peek = 0x80)]
pub fn play_road_noise_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) {
    play_simple_sound(0x4F84D0, this, 0x15C, 0x160, Some(0x8CBD14), sfx, speed, volume);
}

/// `CAEVehicleAudioEntity::PlayFlatTyreSound(int16 sfx, float speed, float volume)` (0x4F8650): see
/// [`play_simple_sound`]; sfx +0x164, sound +0x168, max distance `[0x8CBD20]`.
#[hook(addr = 0x4F8650, conv = thiscall, name = "CAEVehicleAudioEntity::PlayFlatTyreSound", expect = "6A FF 68 28 C3 83 00", peek = 0x80)]
pub fn play_flat_tyre_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) {
    play_simple_sound(0x4F8650, this, 0x164, 0x168, Some(0x8CBD20), sfx, speed, volume);
}

/// `CAEVehicleAudioEntity::PlayReverseSound(int16 sfx, float speed, float volume)` (0x4F87D0): see
/// [`play_simple_sound`]; sfx +0x16C, sound +0x170, max distance left at `Initialise`'s 1.0.
#[hook(addr = 0x4F87D0, conv = thiscall, name = "CAEVehicleAudioEntity::PlayReverseSound", expect = "6A FF 68 48 C3 83 00", peek = 0x80)]
pub fn play_reverse_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) {
    play_simple_sound(0x4F87D0, this, 0x16C, 0x170, None, sfx, speed, volume);
}

/// `CAEVehicleAudioEntity::StartVehicleEngineSound(int16 type, float speed, float volume)` (0x4F7F20):
/// volume += m_EventVolume; an existing engine sound (+0xE8 + 8 * type) gets volume and speed; else
/// type 1 (sfx 0, max distance `[0x8CBC30]`) and 2 (sfx 1, `[0x8CBC2C]`) start one from a stack
/// `CAESound` (entry ESP - 0x80: slot = engine bank slot +0xE0, this, the vehicle's position, volume 0.0,
/// 1.0 x3, zeros; then volume, max distance, speed, flags 4) and store its handle.
#[hook(addr = 0x4F7F20, conv = thiscall, name = "CAEVehicleAudioEntity::StartVehicleEngineSound", expect = "6A FF 68 C8 C2 83 00", peek = 0x80)]
pub fn start_vehicle_engine_sound(this: GPtr<u8>, ty: u32, speed: u32, volume: u32) {
    let mut buf = stack_sound(0x4F7F20, -0x80);
    let local = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(local, 8, 0);
    let idx = ty as u16 as i16 as isize;
    let vol = with_event_volume(this, volume);
    let cur = rd::<u32>(this, 0xE8 + idx * 8);
    if cur != 0 {
        wr::<u32>(g::<u8>(cur as usize), 0x14, vol);
        wr::<u32>(g::<u8>(cur as usize), 0x1C, speed);
        return;
    }
    let (sfx, dist) = match idx {
        1 => (0, 0x8CBC30),
        2 => (1, 0x8CBC2C),
        _ => return,
    };
    let dist = g::<u32>(dist).read();
    let p = veh_pos(this);
    let slot = rd::<u16>(this, 0xE0) as u32;
    sound_initialise(
        local,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        0,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(local, 0x14, vol);
    wr::<u32>(local, 0x18, dist);
    wr::<u32>(local, 0x1C, speed);
    wr::<u16>(local, 0x56, 4);
    let got = sm_request_new_sound(g(SM), local);
    wr::<u32>(this, 0xE8 + idx * 8, got);
}

orig! {
    /// `CAEVehicleAudioEntity::GetFreqForIdle(float)` (0x4F5C60, called with `this` in ECX), ST0
    fn get_freq_for_idle_st0(this: GPtr<u8>, v: u32) -> f64 = thiscall 0x4F5C60 => unported;
}

/// `min(ratio, 1.0)` as the original loads it: the stored f32 ratio when `<= 1.0` (or NaN), else 1.0.
fn ratio_le1(ratio: f32) -> X87 {
    if x(ratio).fcom(gd(0x85A310)).test_any(0x41) {
        x(ratio)
    } else {
        gd(0x85A310)
    }
}

/// `CAEVehicleAudioEntity::GetFreqForPlayerEngineSound(cVehicleParams&, int16 engineState)` (0x4F8070):
/// `t` from params +0x28 (above 0: `-(min(v, 0.2) / 0.2 * 0.15)`, else `max(v, -0.2) / -0.2 * 0.15`,
/// with `0.2 / 0.2` / `-0.2` for the clamped values), negated again when `speed < -0.001`; `ratio =
/// f32(params +0x24 / transmission +0x10)`; `A = f32(t * min(ratio, 1))`; bikes (type 1, +0x80): `L =
/// f32((|sin(vehicle +0x648)| * 0.12 (+0.1 with vehicle +0x614 bit 0x20)) * min(ratio, 1))`, else 0. By
/// state: 1..5 the per-state curves (state 2 uses `GetFreqForIdle(params +0x2C)` and model 0x1C9 gives
/// 1.0), times `(A + 1) * (L + 1)`; other states the argument dword read as a float. Then the vehicle
/// flag factor (+0x42B bit 0x40) and, with +0x244 set and `0 <= +0x248 <= 1`, `* (0.12 * e + 1)`.
#[hook(addr = 0x4F8070, conv = thiscall, name = "CAEVehicleAudioEntity::GetFreqForPlayerEngineSound", expect = "83 EC 08 8B 54 24 0C")]
pub fn get_freq_for_player_engine_sound(this: GPtr<u8>, params: GPtr<u8>, state: u32) -> X87 {
    let neg = !x(rd::<f32>(params, 0x1C)).fcom(x(gf(0x862CF0))).test_parity(0x05);
    let v = rd::<f32>(params, 0x28);
    let c40 = gf(0x8CBD40);
    let c3c = gf(0x8CBD3C);
    let c44 = gf(0x8CBD44);
    let mut t = if !x(v).fcom(x(gf(FLT_0))).test_any(0x41) {
        let n = if x(v).fcom(x(c40)).test_parity(0x05) {
            x(c40)
        } else {
            x(v)
        };
        n.fdiv_m(c40).fmul_m(c3c).fchs()
    } else {
        let n = if x(v).fcom(x(c44)).test_any(0x41) {
            x(c44)
        } else {
            x(v)
        };
        n.fdiv_m(c44).fmul_m(c3c)
    };
    if neg {
        t = t.fchs();
    }
    let tr: GPtr<u8> = g(rd::<u32>(params, 0x14) as usize);
    let ratio = x(rd::<f32>(params, 0x24)).fdiv(x(rd::<f32>(tr, 0x10))).to_f32();
    let a = t.fmul(ratio_le1(ratio)).to_f32();
    let mut l: f32 = 0.0;
    if rd::<u8>(this, 0x80) == 1 {
        let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
        let mut s = x(rd::<f32>(veh, 0x648)).fsin();
        if !s.fcom(x(gf(FLT_0))).test_parity(0x05) {
            s = s.fchs();
        }
        s = s.fmul_m(gf(0x8CBD6C));
        if rd::<u8>(veh, 0x614) & 0x20 != 0 {
            s = s.fadd_m(gf(0x8CBD70));
        }
        l = s.fmul(ratio_le1(ratio)).to_f32();
    }
    let tail = |f: X87| f.fmul(x(a).fadd_m(gf(FLT_1))).fmul(x(l).fadd_m(gf(FLT_1)));
    let px = || rd::<f32>(params, 0x40);
    let mut f = match (state as u16 as i16 as i32).wrapping_sub(1) {
        0 => {
            if !x(px()).fcom(x(gf(0x858C80))).test_any(0x41) {
                tail(
                    x(px())
                        .fsub_m(gf(0x858C80))
                        .fmul_m(gf(0x858CC4))
                        .fmul_m(gf(0xB6BA68))
                        .fadd_m(gf(0x8CBCB0))
                        .fadd_m(gf(0x8CBCA4)),
                )
            } else {
                tail(
                    x(px())
                        .fmul_m(gf(0x858CC4))
                        .fmul_m(gf(0x8CBCAC))
                        .fadd_m(gf(0x8CBCA4)),
                )
            }
        }
        1 => {
            let idle = X87::from_f64_exact(get_freq_for_idle_st0(this, rd::<u32>(params, 0x2C)));
            let f = tail(idle.fmul_m(gf(0xB6BA40)).fadd_m(gf(0x8CBC70)));
            if rd::<u16>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x22) == 0x1C9 {
                x(gf(FLT_1))
            } else {
                f
            }
        }
        2 => {
            let f = x(px())
                .fmul_m(gf(0x858CC4))
                .fmul_m(gf(0x8CBCB8))
                .fadd(x(gf(0xB6BA50)).fmul_m(rd::<f32>(this, 0xAC)));
            let c = if rd::<u8>(this, 0xB0) != 0 {
                0xB6BA58
            } else {
                0x8CBC80
            };
            tail(f.fadd_m(gf(c)))
        }
        3 => {
            let f = x(px()).fmul_m(gf(0x858CC4)).fmul_m(gf(0x8CBCB8));
            let f = if rd::<u8>(this, 0xB0) != 0 {
                f.fadd_m(gf(0x8CBC6C))
            } else {
                f.fadd_m(gf(0x8CC1C0 + rd::<u8>(this, 0xAA) as usize * 4))
                    .fadd_m(gf(0x8CBC60))
            };
            tail(f)
        }
        4 => tail(
            x(gf(0xB6BA5C))
                .fmul_m(rd::<f32>(params, 0x2C))
                .fadd_m(gf(0x8CBC94)),
        ),
        _ => x(f32::from_bits(state)),
    };
    let veh: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u8>(veh, 0x42B) & 0x40 != 0 {
        f = f.fmul_m(gf(0x8CBC48));
    }
    if rd::<u8>(this, 0x244) != 0 {
        let e = rd::<f32>(this, 0x248);
        if !x(e).fcom(x(gf(FLT_1))).test_parity(0x41) && !x(e).fcom(x(gf(FLT_0))).test_any(0x01) {
            f = f.fmul(x(gf(0x8CBC50)).fmul_m(e).fadd_m(gf(FLT_1)));
        }
    }
    f
}

orig! {
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    fn sound_stop_sound_and_forget(sound: GPtr<u8>) = thiscall 0x4EF850 => unported;
}

/// Stops and forgets the sound at `this + off` when there is one.
fn forget(this: GPtr<u8>, off: isize) {
    let s = rd::<u32>(this, off);
    if s != 0 {
        sound_stop_sound_and_forget(g(s as usize));
        wr::<u32>(this, off, 0);
    }
}

/// `IsSoundBankLoaded(0x4A, 0x11) || IsSoundBankLoaded(0x8A, 0x13)`
fn horn_bank_loaded() -> bool {
    is_sound_bank_loaded(g(HW), 0x4A, 0x11) != 0 || is_sound_bank_loaded(g(HW), 0x8A, 0x13) != 0
}

/// The temp sound (+8) initialised at the vehicle's position with 1.0 x3 and zeros.
fn init_temp_sound(this: GPtr<u8>, slot: u32, sfx: u32, vol: u32) -> GPtr<u8> {
    let p = veh_pos(this);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    snd
}

/// `CAEVehicleAudioEntity::PlayHornOrSiren(bool horn, bool siren, bool fastSiren, cVehicleParams&)`
/// (0x4F99D0). Event volumes 0x5F (horn), 0x60 (generic horn), 0x61 (sirens) as f32.
/// Horn, horn type +0x90 == 0: a new press (`horn` and not +0xBE) stops +0x178, sets +0xC4 = volume
/// 0x60 and (bank 0x4A / 0x11 or 0x8A / 0x13 loaded, else the function returns) plays sfx 0 (slot 0x11,
/// +0x24 = 1.0); otherwise +0xC4 fades by `[0x8CBECC]` down to -100 and the playing horn takes it or is
/// stopped at -100. Horn type 1..9: a new press sets +0xC4 = volume 0x5F + +0xA0 and plays sfx = type
/// (speed +0x94) unless +0x178 is playing (any other type returns); releasing stops it. Both horn
/// sounds: +0x58 = `[0x8CBEC4]`, flags 4, max distance 2.5, handle in +0x178. Siren (`siren && !fast`,
/// previous +0xBF / +0xC0): starts +0x17C (model 0x1A7: bank 0x4F slot 0x28 sfx 3, else slot 0x11 sfx
/// 0xA; volume 0x61, +0x58 = `[0x8CBEC8]`, speed 1.0, flags 4, max distance 3.0) or stops it; fast
/// siren likewise with +0x180, slot 0x11, sfx 0xB. A missing bank or a sound already playing returns
/// from the whole function, as the bytes.
#[hook(addr = 0x4F99D0, conv = thiscall, name = "CAEVehicleAudioEntity::PlayHornOrSiren", expect = "83 EC 0C A1 F8 00 BD 00")]
pub fn play_horn_or_siren(this: GPtr<u8>, horn: u32, siren: u32, fast: u32, params: GPtr<u8>) {
    let (horn, siren, fast) = (horn as u8 != 0, siren as u8 != 0, fast as u8 != 0);
    let horn_vol = event_volume(0x5F);
    let generic_vol = event_volume(0x60);
    let siren_vol = event_volume(0x61);
    let ty = rd::<i8>(this, 0x90);
    let was = rd::<u8>(this, 0xBE) != 0;
    let mut start_horn = None;
    if ty == 0 {
        if horn && !was {
            forget(this, 0x178);
            wr::<f32>(this, 0xC4, generic_vol);
            if !horn_bank_loaded() {
                return;
            }
            start_horn = Some(ONE);
        } else {
            let m100 = gf(FLT_M100);
            let mut c = x(rd::<f32>(this, 0xC4)).fsub_m(gf(0x8CBECC));
            if c.fcom(x(m100)).test_any(0x41) {
                c = x(m100);
            }
            let cf = c.to_f32();
            wr::<f32>(this, 0xC4, cf);
            let s = rd::<u32>(this, 0x178);
            if s != 0 {
                if !c.fcom(x(m100)).test_any(0x41) {
                    wr::<f32>(g::<u8>(s as usize), 0x14, cf);
                } else {
                    sound_stop_sound_and_forget(g(s as usize));
                    wr::<u32>(this, 0x178, 0);
                }
            }
        }
    } else if horn {
        if !was {
            if ty <= 0 || ty > 9 {
                return;
            }
            wr::<f32>(this, 0xC4, x(horn_vol).fadd_m(rd::<f32>(this, 0xA0)).to_f32());
            if !horn_bank_loaded() || rd::<u32>(this, 0x178) != 0 {
                return;
            }
            start_horn = Some(x(rd::<f32>(this, 0x94)).to_f32().to_bits());
        }
    } else if was {
        forget(this, 0x178);
    }
    if let Some(speed) = start_horn {
        let sfx = ty as i16 as u16 as u32;
        init_temp_sound(this, 0x11, sfx, rd::<u32>(this, 0xC4));
        wr::<u32>(this, 0x24, speed);
        wr::<u32>(this, 0x58, g::<u32>(0x8CBEC4).read());
        wr::<u16>(this, 0x5E, 4);
        wr::<u32>(this, 0x20, 0x4020_0000);
        let got = sm_request_new_sound(g(SM), this.byte_add(8));
        wr::<u32>(this, 0x178, got);
    }
    let sv = siren_vol.to_bits();
    let cur = siren && !fast;
    let prev = rd::<u8>(this, 0xBF) != 0 && rd::<u8>(this, 0xC0) == 0;
    if cur {
        if !prev {
            if is_sound_bank_loaded(g(HW), 0x4A, 0x11) == 0 || rd::<u32>(this, 0x17C) != 0 {
                return;
            }
            let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
            if rd::<u16>(veh, 0x22) == 0x1A7 {
                if is_sound_bank_loaded(g(HW), 0x4F, 0x28) == 0 {
                    return;
                }
                init_temp_sound(this, 0x28, 3, sv);
            } else {
                init_temp_sound(this, 0x11, 0xA, sv);
            }
            wr::<u32>(this, 0x58, g::<u32>(0x8CBEC8).read());
            wr::<u32>(this, 0x24, ONE);
            wr::<u16>(this, 0x5E, 4);
            wr::<u32>(this, 0x20, 0x4040_0000);
            let got = sm_request_new_sound(g(SM), this.byte_add(8));
            wr::<u32>(this, 0x17C, got);
        }
    } else if prev {
        forget(this, 0x17C);
    }
    let was_fast = rd::<u8>(this, 0xC0) != 0;
    if fast {
        if was_fast || is_sound_bank_loaded(g(HW), 0x4A, 0x11) == 0 || rd::<u32>(this, 0x180) != 0 {
            return;
        }
        init_temp_sound(this, 0x11, 0xB, sv);
        wr::<u32>(this, 0x58, g::<u32>(0x8CBEC8).read());
        wr::<u32>(this, 0x24, ONE);
        wr::<u16>(this, 0x5E, 4);
        wr::<u32>(this, 0x20, 0x4040_0000);
        let got = sm_request_new_sound(g(SM), this.byte_add(8));
        wr::<u32>(this, 0x180, got);
    } else if was_fast {
        forget(this, 0x180);
    }
}

orig! {
    /// `CAEVehicleAudioEntity::UpdateGenericVehicleSound` (0x4FAD40)
    fn update_generic_vehicle_sound_call(this: GPtr<u8>, ty: u32, slot: u32, bank: u32, sfx: u32, speed: u32, volume: u32, dist: u32) = thiscall 0x4FAD40 => update_generic_vehicle_sound;
}

/// `CAEVehicleAudioEntity::ProcessMovingParts(cVehicleParams&)` (0x4FB260): models 443, 486, 406, 524,
/// 592, 530 only. `t = (u16 +0x86C - u16 +0x86E) / [0x8CC050]`; `a = min(|t|, 1)`; +0x238 moves towards
/// `a` by at most `[0x8CC048]` up / `[0x8CC04C]` down (f32). Model 524: nothing more. Others: volume
/// `f32(AudioLog10(+0x238) * 20 + base)` with (speed, base) from `.data` by model and sign of t (t > 0
/// or not); above -100 `UpdateGenericVehicleSound(0xB, slot, bank, sfx, speed, volume, 1.5)` (models
/// 443 / 486 / 406 / 592: slot 0x13, bank 0x8A, sfx 0xF; 530: slot = word +0xE0, bank 0x39, sfx 2),
/// else the sound +0x140 is stopped (flag 4 cleared) and forgotten.
#[hook(addr = 0x4FB260, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessMovingParts", expect = "51 8B 54 24 08")]
pub fn process_moving_parts(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let model = rd::<u16>(veh, 0x22);
    if !matches!(model, 0x1BB | 0x1E6 | 0x196 | 0x20C | 0x250 | 0x212) {
        return;
    }
    let diff = (rd::<u16>(veh, 0x86C) as i32).wrapping_sub(rd::<u16>(veh, 0x86E) as i32);
    let t = crate::common::xi(diff).fdiv_m(gf(0x8CC050));
    let mut a = t.fabs();
    if !x(gf(FLT_1)).fcom(a).test_parity(0x05) {
        a = x(gf(FLT_1));
    } else if !x(gf(FLT_0)).fcom(a).test_any(0x41) {
        a = x(gf(FLT_0));
    }
    let cur = rd::<f32>(this, 0x238);
    let new = if a.fcom(x(cur)).test_any(0x41) {
        let n = x(cur).fsub_m(gf(0x8CC04C)).to_f32();
        if a.fcom(x(n)).test_any(0x41) { x(n) } else { a }
    } else {
        let n = x(gf(0x8CC048)).fadd_m(cur).to_f32();
        if a.fcom(x(n)).test_parity(0x05) { x(n) } else { a }
    };
    let new = new.to_f32();
    wr::<f32>(this, 0x238, new);
    let model = rd::<i16>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x22);
    let (slot, bank, sfx, pos_c, neg_c) = match model {
        443 => (0x13, 0x8A, 0xF, (0x8CC06C, 0x8CC064), (0x8CC070, 0x8CC068)),
        486 => (0x13, 0x8A, 0xF, (0x8CC07C, 0x8CC074), (0x8CC080, 0x8CC078)),
        406 => (0x13, 0x8A, 0xF, (0x8CC08C, 0x8CC084), (0x8CC090, 0x8CC088)),
        592 => (0x13, 0x8A, 0xF, (0x8CC09C, 0x8CC094), (0x8CC0A0, 0x8CC098)),
        530 => (
            rd::<u16>(this, 0xE0) as u32,
            0x39,
            2,
            (0x8CC05C, 0x8CC054),
            (0x8CC060, 0x8CC058),
        ),
        _ => return,
    };
    let (sp, base) = if !t.fcom(gd(0x859EF8)).test_any(0x41) {
        pos_c
    } else {
        neg_c
    };
    let speed = g::<u32>(sp).read();
    let v = crate::utility::audio_log10(new)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(base));
    if !v.fcom(x(gf(FLT_M100))).test_any(0x41) {
        update_generic_vehicle_sound_call(
            this,
            0xB,
            slot,
            bank,
            sfx,
            speed,
            v.to_f32().to_bits(),
            0x3FC0_0000,
        );
    } else {
        let s = rd::<u32>(this, 0x140);
        if s != 0 {
            sound_set_flags(g(s as usize), 4, 0);
            sound_stop_sound(g(rd::<u32>(this, 0x140) as usize));
            wr::<u32>(this, 0x140, 0);
        }
    }
}

orig! {
    /// `CAEVehicleAudioEntity::CancelVehicleEngineSound(int16)` (0x4F55C0)
    fn cancel_vehicle_engine_sound(this: GPtr<u8>, ty: u32) = thiscall 0x4F55C0 => crate::vehicle::cancel_vehicle_engine_sound;
}

/// `fild` of a dword the original treats as unsigned: `+ 2^32` (`[0x858C54]`) when negative.
fn fild_u32(v: u32) -> X87 {
    let f = X87::from_i32(v as i32);
    if (v as i32) < 0 { f.fadd_m(gf(0x858C54)) } else { f }
}

/// `CAEVehicleAudioEntity::RequestNewPlayerCarEngineSound(int16 type, float speed, float volume)`
/// (0x4F7A50): -1 without an engine bank slot (+0xE0) or, for types 1 / 2, with bank +0xDC not in that
/// slot, for types 3..5 with bank +0xDE not in slot 0x28. Volume += m_EventVolume; an existing engine
/// sound of the type is cancelled (not for type 5). A stack `CAESound` (entry ESP - 0x80; only +8
/// cleared) is set up by type: 1 / 2 slot +0xE0, sfx 0 / 1, max distance `[0x8CBC30]` / `[0x8CBC2C]`;
/// 3 / 5 slot 0x28, sfx 1 / 2, `[0x8CBC38]` / `[0x8CBC3C]` (flags 4); 4 (acceleration) slot 0x28, sfx 0,
/// `[0x8CBC34]`, flags 0x24, +0x14C = 0 and, within `[0x8CBCCC]` ms of the last stop (+0x150), a play
/// position `_ftol(clamp(+0x14E * (1 - min(max(elapsed, 0), interval) / interval), 0, [0x8CBCC8]))`
/// (+0x14C = `[0x8CBCC0]` + 1 above 10); any other type requests the sound as it is (stack residue).
/// The handle goes to +0xE8 + 8 * type; returns 0. AX only: the callers (all in
/// `ProcessPlayerVehicleEngine`) never read the result.
#[hook(addr = 0x4F7A50, conv = thiscall, name = "CAEVehicleAudioEntity::RequestNewPlayerCarEngineSound", expect = "6A FF 68 A8 C2 83 00", peek = 0x80)]
pub fn request_new_player_car_engine_sound(
    this: GPtr<u8>,
    ty: u32,
    speed: u32,
    volume: u32,
) -> sa_core::abi::Ax {
    let mut buf = stack_sound(0x4F7A50, -0x80);
    let local = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(local, 8, 0);
    let fail = sa_core::abi::Ax(0xFFFF);
    let slot = rd::<u16>(this, 0xE0);
    if slot == 0xFFFF {
        return fail;
    }
    let idx = ty as u16 as i16 as i32;
    let loaded = match idx {
        1 | 2 => is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDC) as u32, slot as u32) != 0,
        3..=5 => is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDE) as u32, 0x28) != 0,
        _ => true,
    };
    if !loaded {
        return fail;
    }
    let vol = with_event_volume(this, volume);
    let e = 0xE8 + idx as isize * 8;
    if rd::<u32>(this, e) != 0 && ty as u16 != 5 {
        cancel_vehicle_engine_sound(this, ty);
    }
    let init = |slot: u32, sfx: u32| {
        let p = veh_pos(this);
        sound_initialise(
            local,
            slot,
            sfx,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            0,
            ONE,
            ONE,
            ONE,
            0,
            0,
            0,
            0,
        );
    };
    let simple = |slot: u32, sfx: u32, dist: usize| {
        init(slot, sfx);
        wr::<u32>(local, 0x14, vol);
        wr::<u32>(local, 0x1C, speed);
        wr::<u32>(local, 0x18, g::<u32>(dist).read());
        wr::<u16>(local, 0x56, 4);
    };
    match idx {
        1 => simple(rd::<u16>(this, 0xE0) as u32, 0, 0x8CBC30),
        2 => simple(rd::<u16>(this, 0xE0) as u32, 1, 0x8CBC2C),
        3 => simple(0x28, 1, 0x8CBC38),
        5 => simple(0x28, 2, 0x8CBC3C),
        4 => {
            init(0x28, 0);
            wr::<u32>(local, 0x1C, speed);
            wr::<u32>(local, 0x18, g::<u32>(0x8CBC34).read());
            wr::<u32>(local, 0x14, vol);
            wr::<u16>(this, 0x14C, 0);
            wr::<u16>(local, 0x56, 0x24);
            let interval = g::<u32>(0x8CBCCC).read();
            let stop = rd::<u32>(this, 0x150);
            let now = g::<u32>(TIME_MS).read();
            if interval.wrapping_add(stop) > now {
                let d = fild_u32(now.wrapping_sub(stop)).to_f32();
                let iv = fild_u32(interval);
                let min_d = |d: f32| {
                    if x(d).fcom(iv).test_parity(0x05) { iv } else { x(d) }
                };
                let r = if min_d(d).fcom(gd(0x859EF8)).test_parity(0x05) {
                    min_d(d)
                } else {
                    gd(0x859EF8)
                };
                let k = x(gf(FLT_1)).fsub(r.fdiv(iv)).to_f32();
                let a = X87::from_i32(g::<i16>(0x8CBCC8).read() as i32);
                let b = || X87::from_i32(rd::<i16>(this, 0x14E) as i32).fmul_m(k);
                let q = if b().fcom(a).test_parity(0x05) { a } else { b() };
                let p = if !x(gf(FLT_0)).fcom(q).test_any(0x41) {
                    x(gf(FLT_0))
                } else if b().fcom(a).test_parity(0x05) {
                    a
                } else {
                    b()
                };
                let pos = crate::common::ftol32(p);
                wr::<u16>(local, 0x5C, pos as u16);
                if !p.fcom(x(gf(0x85862C))).test_any(0x41) {
                    wr::<u16>(this, 0x14C, g::<u32>(0x8CBCC0).read().wrapping_add(1) as u16);
                }
            }
        }
        _ => {}
    }
    let got = sm_request_new_sound(g(SM), local);
    wr::<u32>(this, e, got);
    sa_core::abi::Ax(0)
}

orig! {
    /// `CAEVehicleAudioEntity::GetVolumeForDummyIdle(float, float)` (0x4F51F0), ST0
    fn vol_dummy_idle_st0(this: GPtr<u8>, ratio: u32, fwd: u32) -> f64 = thiscall 0x4F51F0 => unported;
    /// `CAEVehicleAudioEntity::GetFrequencyForDummyIdle(float, float)` (0x4F5310), ST0
    fn freq_dummy_idle_st0(this: GPtr<u8>, ratio: u32, fwd: u32) -> f64 = thiscall 0x4F5310 => unported;
    /// `CAEVehicleAudioEntity::GetVolumeForDummyRev(float, float)` (0x4F53D0), ST0
    fn vol_dummy_rev_st0(this: GPtr<u8>, ratio: u32, fwd: u32) -> f64 = thiscall 0x4F53D0 => unported;
    /// `CAEVehicleAudioEntity::GetFrequencyForDummyRev(float, float)` (0x4F54F0), ST0
    fn freq_dummy_rev_st0(this: GPtr<u8>, ratio: u32, fwd: u32) -> f64 = thiscall 0x4F54F0 => unported;
    /// `CAEVehicleAudioEntity::UpdateVehicleEngineSound(int16, float, float)` (0x4F56D0)
    fn update_vehicle_engine_sound(this: GPtr<u8>, ty: u32, speed: u32, volume: u32) = thiscall 0x4F56D0 => unported;
    /// `CAEVehicleAudioEntity::StartVehicleEngineSound(int16, float, float)` (0x4F7F20)
    fn start_vehicle_engine_sound_call(this: GPtr<u8>, ty: u32, speed: u32, volume: u32) = thiscall 0x4F7F20 => start_vehicle_engine_sound;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420, protection entry: not hooked)
    fn add_audio_event_volume(this: GPtr<u8>, event: u32, volume: u32) = thiscall 0x4F6420 => unported;
}

/// The x87 value returned in ST0 by an original `_st0` call.
fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `f = min(step + cur, 1.0)` as the dummy transitions compute it (`fcom 1.0; test ah, 5; jnp`), f32.
fn step_to_one(step: usize, cur: f32) -> f32 {
    let v = x(gf(step)).fadd_m(cur);
    if v.fcom(x(gf(FLT_1))).test_parity(0x05) {
        gf(FLT_1)
    } else {
        v.to_f32()
    }
}

/// The engine sound at `this + off` (if any) gets speed `freq` (f32) and volume `vol`.
fn set_engine_sound(this: GPtr<u8>, off: isize, freq: X87, vol: f32) {
    let s = rd::<u32>(this, off);
    if s != 0 {
        wr::<f32>(g::<u8>(s as usize), 0x1C, freq.to_f32());
        wr::<f32>(g::<u8>(s as usize), 0x14, vol);
    }
}

/// `SetFlags(4, 0)`, `StopSound` and forget the sound at `this + off` when there is one.
fn stop_engine_sound(this: GPtr<u8>, off: isize) {
    let s = rd::<u32>(this, off);
    if s != 0 {
        sound_set_flags(g(s as usize), 4, 0);
        sound_stop_sound(g(rd::<u32>(this, off) as usize));
        wr::<u32>(this, off, 0);
    }
}

/// `CAEVehicleAudioEntity::ProcessDummyStateTransition(int16 newState, float ratio, cVehicleParams&)`
/// (0x4FCA10): +0x144 = now. From state 1 (+0xA9): to 1 the idle fade (+0x23C, step `[0x8CBBE4]`)
/// updates the idle sound (+0xF8) and the rev fade (+0x240, step `[0x8CBBE8]`) the rev sound (+0xF0)
/// until 0.99, then stops it; to 2 both fades restart at 0, the idle sound is updated and a rev sound
/// started (type 1); else state 0, both stopped. From state 2: to 1 the mirror image (type 2 started);
/// to 2 the rev fade (`[0x8CBBEC]`) updates engine sound 1, the idle fade (`[0x8CBBE0]`) engine sound 2
/// until 0.99 (vehicle +0x42B |= 0x20 for `[0x862D30] < fade < 0.75`), then the idle sound stops; else
/// state 0, both stopped. From states 0 / 10: to 1 / 2 (after `AddAudioEvent(0x67, 0.0)` from 0) both
/// fades = 1.0 and engine sound 2 (idle) / 1 (rev) starts; else state 0. Other states become 0. All
/// sounds through the dummy volume / frequency curves (the originals, ST0) and m_EventVolume.
#[hook(addr = 0x4FCA10, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyStateTransition", expect = "A1 84 CB B7 00")]
pub fn process_dummy_state_transition(this: GPtr<u8>, new_state: u32, ratio: u32, params: GPtr<u8>) {
    wr::<u32>(this, 0x144, g::<u32>(TIME_MS).read());
    let ns = new_state as u16;
    let ev = |v: X87| v.fadd_m(rd::<f32>(this, 0xD8)).to_f32();
    let bits = |v: X87| v.to_f32().to_bits();
    match rd::<u8>(this, 0xA9) {
        1 => match ns {
            1 => {
                wr::<u8>(this, 0xA9, 1);
                let l = step_to_one(0x8CBBE4, rd::<f32>(this, 0x23C));
                wr::<f32>(this, 0x23C, l);
                let v = st0(vol_dummy_idle_st0(this, ratio, l.to_bits())).to_f32();
                let f = st0(freq_dummy_idle_st0(this, ratio, rd::<u32>(this, 0x23C)));
                set_engine_sound(this, 0xF8, f, ev(x(v)));
                let l2 = step_to_one(0x8CBBE8, rd::<f32>(this, 0x240));
                wr::<f32>(this, 0x240, l2);
                if x(l2).fcom(gd(0x862CD8)).test_parity(0x05) {
                    stop_engine_sound(this, 0xF0);
                    return;
                }
                let v = st0(vol_dummy_rev_st0(this, ratio, l2.to_bits())).to_f32();
                let f = st0(freq_dummy_rev_st0(this, ratio, rd::<u32>(this, 0x240)));
                set_engine_sound(this, 0xF0, f, ev(x(v)));
            }
            2 => {
                wr::<u8>(this, 0xA9, 2);
                wr::<u32>(this, 0x240, 0);
                let v = st0(vol_dummy_idle_st0(this, ratio, 0)).to_f32();
                let f = st0(freq_dummy_idle_st0(this, ratio, rd::<u32>(this, 0x240)));
                set_engine_sound(this, 0xF8, f, ev(x(v)));
                wr::<u32>(this, 0x23C, 0);
                let v = bits(st0(vol_dummy_rev_st0(this, ratio, 0)));
                let f = bits(st0(freq_dummy_rev_st0(this, ratio, rd::<u32>(this, 0x23C))));
                start_vehicle_engine_sound_call(this, 1, f, v);
            }
            _ => {
                wr::<u8>(this, 0xA9, 0);
                stop_engine_sound(this, 0xF8);
                stop_engine_sound(this, 0xF0);
            }
        },
        2 => match ns {
            1 => {
                wr::<u8>(this, 0xA9, 1);
                wr::<u32>(this, 0x240, 0);
                let v = st0(vol_dummy_rev_st0(this, ratio, 0)).to_f32();
                let f = st0(freq_dummy_rev_st0(this, ratio, rd::<u32>(this, 0x240)));
                set_engine_sound(this, 0xF0, f, ev(x(v)));
                wr::<u32>(this, 0x23C, 0);
                let v = bits(st0(vol_dummy_idle_st0(this, ratio, 0)));
                let f = bits(st0(freq_dummy_idle_st0(this, ratio, rd::<u32>(this, 0x23C))));
                start_vehicle_engine_sound_call(this, 2, f, v);
            }
            2 => {
                wr::<u8>(this, 0xA9, 2);
                let l = step_to_one(0x8CBBEC, rd::<f32>(this, 0x23C));
                wr::<f32>(this, 0x23C, l);
                let v = bits(st0(vol_dummy_rev_st0(this, ratio, l.to_bits())));
                let f = bits(st0(freq_dummy_rev_st0(this, ratio, rd::<u32>(this, 0x23C))));
                update_vehicle_engine_sound(this, 1, f, v);
                let l2 = step_to_one(0x8CBBE0, rd::<f32>(this, 0x240));
                wr::<f32>(this, 0x240, l2);
                if x(l2).fcom(gd(0x862CD8)).test_parity(0x05) {
                    stop_engine_sound(this, 0xF8);
                    return;
                }
                if !x(l2).fcom(gd(0x862D30)).test_any(0x41) && !x(l2).fcom(gd(0x862CF8)).test_parity(0x05) {
                    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
                    wr::<u8>(veh, 0x42B, rd::<u8>(veh, 0x42B) | 0x20);
                }
                let v = bits(st0(vol_dummy_idle_st0(this, ratio, rd::<u32>(this, 0x240))));
                let f = bits(st0(freq_dummy_idle_st0(this, ratio, rd::<u32>(this, 0x240))));
                update_vehicle_engine_sound(this, 2, f, v);
            }
            _ => {
                wr::<u8>(this, 0xA9, 0);
                stop_engine_sound(this, 0xF0);
                stop_engine_sound(this, 0xF8);
            }
        },
        cur @ (0 | 10) => match ns {
            1 | 2 => {
                if cur == 0 {
                    add_audio_event_volume(this, 0x67, 0);
                }
                wr::<u8>(this, 0xA9, ns as u8);
                wr::<u32>(this, 0x240, ONE);
                wr::<u32>(this, 0x23C, ONE);
                if ns == 1 {
                    let v = bits(st0(vol_dummy_idle_st0(this, ratio, ONE)));
                    let f = bits(st0(freq_dummy_idle_st0(this, ratio, ONE)));
                    start_vehicle_engine_sound_call(this, 2, f, v);
                } else {
                    let v = bits(st0(vol_dummy_rev_st0(this, ratio, ONE)));
                    let f = bits(st0(freq_dummy_rev_st0(this, ratio, ONE)));
                    start_vehicle_engine_sound_call(this, 1, f, v);
                }
            }
            _ => wr::<u8>(this, 0xA9, 0),
        },
        _ => wr::<u8>(this, 0xA9, 0),
    }
}

/// `CAEVehicleAudioEntity::PlayAircraftSound(int16 type, int16 bankSlot, int16 sfx, float volume, float
/// speed)` (0x4F93C0): an existing engine sound (+0xE8 + 8 * type) gets volume and speed. Otherwise a
/// stack `CAESound` (entry ESP - 0x80; +8 cleared) is initialised (bankSlot, sfx, this, the vehicle's
/// position, volume 0.0, 1.0 x3, zeros) with max distance by type: 1 -> 6.0, 2 / 4 -> 4.0, 3 -> 1.0
/// (time scale +0x50 = 0.4), 5 -> 4.5, 7 -> 50.0 at position (0, 1, 0); type 6 and types outside 1..7
/// are not initialised (the stack residue is requested). Then volume, speed, flags 4, `RequestNewSound`
/// and the handle stored.
#[hook(addr = 0x4F93C0, conv = thiscall, name = "CAEVehicleAudioEntity::PlayAircraftSound", expect = "6A FF 68 68 C3 83 00", peek = 0x80)]
pub fn play_aircraft_sound(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, volume: u32, speed: u32) {
    let mut buf = stack_sound(0x4F93C0, -0x80);
    let local = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(local, 8, 0);
    let idx = ty as u16 as i16 as isize;
    let cur = rd::<u32>(this, 0xE8 + idx * 8);
    if cur != 0 {
        wr::<u32>(g::<u8>(cur as usize), 0x14, volume);
        let s = rd::<u32>(this, 0xE8 + idx * 8);
        wr::<u32>(g::<u8>(s as usize), 0x1C, speed);
        return;
    }
    let init = |p: [u32; 3]| {
        sound_initialise(
            local,
            slot,
            sfx,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            0,
            ONE,
            ONE,
            ONE,
            0,
            0,
            0,
            0,
        );
    };
    match idx {
        1 => {
            init(veh_pos(this));
            wr::<u32>(local, 0x18, 0x40C0_0000);
        }
        2 | 4 => {
            init(veh_pos(this));
            wr::<u32>(local, 0x18, 0x4080_0000);
        }
        3 => {
            init(veh_pos(this));
            wr::<u32>(local, 0x18, ONE);
            wr::<u32>(local, 0x50, 0x3ECC_CCCD);
        }
        5 => {
            init(veh_pos(this));
            wr::<u32>(local, 0x18, 0x4090_0000);
        }
        7 => {
            init([0, ONE, 0]);
            wr::<u32>(local, 0x18, 0x4248_0000);
        }
        _ => {}
    }
    wr::<u32>(local, 0x14, volume);
    wr::<u32>(local, 0x1C, speed);
    wr::<u16>(local, 0x56, 4);
    let got = sm_request_new_sound(g(SM), local);
    wr::<u32>(this, 0xE8 + idx * 8, got);
}

/// `CAEVehicleAudioEntity::PlayBicycleSound(int16 type, int16 bankSlot, int16 sfx, float volume, float
/// speed)` (0x4F9710): nothing without an engine bank slot (+0xE0) or with the type's bank missing (1:
/// +0xDC in +0xE0, 2: +0xDE in 0x28, 3: 0x27 in 2). Type 3 (unless +0x100 plays): pick
/// `GetRandomNumberInRange(0, 3)` of the (sfx, play position) pairs at 0x8CC1D8 and play it from a stack
/// `CAESound` (slot 2, the vehicle's position, volume, 1.0 x3, zeros; then volume, speed, flags 0x24,
/// play position) into +0x100. Other types: an existing engine sound gets volume and speed; types 1 / 2
/// start one (bankSlot, sfx, volume 0.0; max distance 2.5 / 1.5; volume, speed, flags 4).
#[hook(addr = 0x4F9710, conv = thiscall, name = "CAEVehicleAudioEntity::PlayBicycleSound", expect = "6A FF 68 88 C3 83 00", peek = 0x80)]
pub fn play_bicycle_sound(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, volume: u32, speed: u32) {
    let mut buf = stack_sound(0x4F9710, -0x80);
    let local = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(local, 8, 0);
    let e0 = rd::<u16>(this, 0xE0);
    if e0 == 0xFFFF {
        return;
    }
    let idx = ty as u16 as i16 as isize;
    let ok = match idx {
        1 => is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDC) as u32, e0 as u32) != 0,
        2 => is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDE) as u32, 0x28) != 0,
        3 => is_sound_bank_loaded(g(HW), 0x27, 2) != 0,
        _ => true,
    };
    if !ok {
        return;
    }
    if idx == 3 {
        if rd::<u32>(this, 0x100) != 0 {
            return;
        }
        let r = ae_random_int(0, 3) as i16 as usize;
        let pick_sfx = g::<u16>(0x8CC1D8 + r * 4).read() as u32;
        let pick_pos = g::<u16>(0x8CC1DA + r * 4).read();
        let p = veh_pos(this);
        sound_initialise(
            local,
            2,
            pick_sfx,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            volume,
            ONE,
            ONE,
            ONE,
            0,
            0,
            0,
            0,
        );
        wr::<u32>(local, 0x14, volume);
        wr::<u32>(local, 0x1C, speed);
        wr::<u16>(local, 0x56, 0x24);
        wr::<u16>(local, 0x5C, pick_pos);
        let got = sm_request_new_sound(g(SM), local);
        wr::<u32>(this, 0x100, got);
        return;
    }
    let cur = rd::<u32>(this, 0xE8 + idx * 8);
    if cur != 0 {
        wr::<u32>(g::<u8>(cur as usize), 0x14, volume);
        let s = rd::<u32>(this, 0xE8 + idx * 8);
        wr::<u32>(g::<u8>(s as usize), 0x1C, speed);
        return;
    }
    let dist = match idx {
        1 => 0x4020_0000,
        2 => 0x3FC0_0000,
        _ => return,
    };
    let p = veh_pos(this);
    sound_initialise(
        local,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        0,
        ONE,
        ONE,
        ONE,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(local, 0x18, dist);
    wr::<u32>(local, 0x1C, speed);
    wr::<u32>(local, 0x14, volume);
    wr::<u16>(local, 0x56, 4);
    let got = sm_request_new_sound(g(SM), local);
    wr::<u32>(this, 0xE8 + idx * 8, got);
}

orig! {
    /// `CAEVehicleAudioEntity::PlayAircraftSound` (0x4F93C0)
    fn play_aircraft_sound_call(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, volume: u32, speed: u32) = thiscall 0x4F93C0 => play_aircraft_sound;
}

/// `min(max(v, 0), 1)` as `fld 1; fcomp v; test ah, 5; jnp` then `fld 0; fcomp v; test ah, 0x41; jne`.
fn clamp01_f32(v: f32) -> f32 {
    if !x(gf(FLT_1)).fcom(x(v)).test_parity(0x05) {
        gf(FLT_1)
    } else if !x(gf(FLT_0)).fcom(x(v)).test_any(0x41) {
        0.0
    } else {
        v
    }
}

/// `CAEVehicleAudioEntity::ProcessPlayerCombine(cVehicleParams&)` (0x500CE0): with bank 0x45 in slot
/// 0x28: engine on (vehicle +0x428 bit 0x10): `t = clamp01(f32(speed / [0x8CBF88]))`,
/// `UpdateGenericVehicleSound(0xB, 0x28, 0x45, 3, 1.0, f32(AudioLog10(t) * 20 + [0x8CBF84]), 2.0)`;
/// engine off: the sound +0x140 is stopped (flag 4 cleared) and forgotten.
#[hook(addr = 0x500CE0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessPlayerCombine", expect = "56 6A 28 8B F1")]
pub fn process_player_combine(this: GPtr<u8>, params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), 0x45, 0x28) == 0 {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if rd::<u8>(veh, 0x428) & 0x10 != 0 {
        let t = clamp01_f32(x(rd::<f32>(params, 0x1C)).fdiv_m(gf(0x8CBF88)).to_f32());
        let vol = crate::utility::audio_log10(t)
            .fmul_m(gf(FLT_20))
            .fadd_m(gf(0x8CBF84))
            .to_f32();
        update_generic_vehicle_sound_call(this, 0xB, 0x28, 0x45, 3, ONE, vol.to_bits(), 0x4000_0000);
    } else {
        stop_engine_sound(this, 0x140);
    }
}

/// `CAEVehicleAudioEntity::ProcessDummySeaPlane(cVehicleParams&)` (0x4FF7C0): engine off (vehicle +0x428
/// bit 0x10 clear): all twelve engine sounds cancelled. Else, with bank 0x8A in slot 0x13: `v = vehicle
/// +0x5A4 * [0x862D1C]`, above 1 -> 1, below 0.2 -> 0 (skipping the next step), else `(v - 0.2) *
/// [0x8595F0]`; then `v = max(v, [0x862D08])` (double) stored as f32; `PlayAircraftSound(3, 0x13,
/// 0x11, f32(AudioLog10(v) * 20 + [0x859000]), 1.0)`. The stack `CMatrix` the original destroys is
/// never set up (no observable effect).
#[hook(addr = 0x4FF7C0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummySeaPlane", expect = "6A FF 68 A8 C4 83 00")]
pub fn process_dummy_sea_plane(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if rd::<u8>(veh, 0x428) & 0x10 == 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let v = x(rd::<f32>(veh, 0x5A4)).fmul_m(gf(0x862D1C));
    let v = if v.fcom(x(gf(FLT_1))).test_any(0x41) {
        if v.fcom(x(gf(0x858CC4))).test_parity(0x05) {
            Some(v)
        } else {
            None
        }
    } else {
        Some(x(gf(FLT_1)))
    };
    let v = match v {
        Some(v) => v.fsub_m(gf(0x858CC4)).fmul_m(gf(0x8595F0)),
        None => x(gf(FLT_0)),
    };
    let v = if v.fcom(gd(0x862D08)).test_any(0x41) {
        gd(0x862D08)
    } else {
        v
    };
    let vf = v.to_f32();
    let vol = crate::utility::audio_log10(vf)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x859000))
        .to_f32();
    play_aircraft_sound_call(this, 3, 0x13, 0x11, vol.to_bits(), ONE);
}

orig! {
    /// `CAEVehicleAudioEntity::RequestBankSlot(int16)` (0x4F4D10, static)
    fn request_bank_slot(bank: u32) -> u32 = cdecl 0x4F4D10 => crate::vehicle::request_bank_slot;
    /// `CAEAudioUtility::GetPiecewiseLinear(float, int16, const float (*)[2])` (0x4D9D90)
    fn get_piecewise_linear(v: u32, n: u32, table: u32) -> f32 = cdecl 0x4D9D90 => unported;
    /// `CAEVehicleAudioEntity::ProcessGenericJet(bool, cVehicleParams&, float, float, float, float, float)` (0x4FF900)
    fn process_generic_jet_call(this: GPtr<u8>, on: u32, params: GPtr<u8>, f: u32, accel: u32, brake: u32, a: u32, b: u32) = thiscall 0x4FF900 => process_generic_jet;
}

/// The x87 form of [`clamp01_f32`] (on a value still on the FPU stack).
fn clamp01_x(v: X87) -> X87 {
    if !x(gf(FLT_1)).fcom(v).test_parity(0x05) {
        x(gf(FLT_1))
    } else if !x(gf(FLT_0)).fcom(v).test_any(0x41) {
        x(gf(FLT_0))
    } else {
        v
    }
}

/// The dummy engine bank of a generic dummy (RC car / golf cart / hovercraft prologue): without a bank
/// slot (+0xE0) one is requested (`RequestBankSlot(+0xDC)`, none free -> false); a slot whose bank is not
/// loaded yet: still loading (the slot's `s_DummyEngineSlots` entry holds +0xDC) -> false, else a new
/// slot is requested, +0xA9 = 0 (only with `reset_state`) -> false.
fn dummy_engine_bank_ready_ex(this: GPtr<u8>, reset_state: bool) -> bool {
    if rd::<u16>(this, 0xE0) == 0xFFFF {
        let s = request_bank_slot(rd::<u16>(this, 0xDC) as u32) as u16;
        wr::<u16>(this, 0xE0, s);
        if s == 0xFFFF {
            return false;
        }
    }
    let slot = rd::<u16>(this, 0xE0);
    if is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDC) as u32, slot as u32) != 0 {
        return true;
    }
    let i = slot.wrapping_sub(7) as i16;
    if (0..10).contains(&i) && g::<u16>(0xB6B9A0 + i as usize * 4).read() == rd::<u16>(this, 0xDC) {
        return false;
    }
    let s = request_bank_slot(rd::<u16>(this, 0xDC) as u32);
    wr::<u16>(this, 0xE0, s as u16);
    if reset_state {
        wr::<u8>(this, 0xA9, 0);
    }
    false
}

/// [`dummy_engine_bank_ready_ex`] resetting +0xA9.
fn dummy_engine_bank_ready(this: GPtr<u8>) -> bool {
    dummy_engine_bank_ready_ex(this, true)
}

/// `CAEVehicleAudioEntity::ProcessDummyRCCar(cVehicleParams&)` (0x500DC0): the dummy bank must be ready
/// ([`dummy_engine_bank_ready`]); wrecked (+0xA8): all twelve engine sounds cancelled. Else `t =
/// clamp01(|speed / max velocity|)` (f32), volume / speed = the 4-point curves 0x8CC0A4 / 0x8CBF8C at t,
/// `UpdateGenericVehicleSound(2, +0xE0, +0xDC, 0, speed, volume, 1.0)`. (AL, which `ProcessSpecialVehicle`
/// sets before the call, is not read.)
#[hook(addr = 0x500DC0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyRCCar", expect = "51 56 8B F1 66 83 BE E0 00 00 00 FF")]
pub fn process_dummy_rc_car(this: GPtr<u8>, params: GPtr<u8>) {
    if !dummy_engine_bank_ready(this) {
        return;
    }
    if rd::<u8>(this, 0xA8) != 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    let tr: GPtr<u8> = g(rd::<u32>(params, 0x14) as usize);
    let t = clamp01_x(x(rd::<f32>(params, 0x1C)).fdiv_m(rd::<f32>(tr, 0x58)).fabs()).to_f32();
    let vol = get_piecewise_linear(t.to_bits(), 4, 0x8CC0A4);
    let speed = get_piecewise_linear(t.to_bits(), 4, 0x8CBF8C);
    let slot = rd::<u16>(this, 0xE0) as u32;
    let bank = rd::<u16>(this, 0xDC) as u32;
    update_generic_vehicle_sound_call(this, 2, slot, bank, 0, speed.to_bits(), vol.to_bits(), ONE);
}

/// `CAEVehicleAudioEntity::ProcessDummyJet(cVehicleParams&)` (0x501960): `a = _ftol(gas * 255)`, `b =
/// _ftol(brake * 255)`, `m = max(a, b)` (int16), `f = clamp01(m / 255)`; vehicle byte +0x424 >= 0:
/// `f = f * 0.3 + 0.7`, else `f = +0x9C4 * [0x862D18]`. Then `f > 1 -> 1`, `f < 0.2 -> 0` (stored
/// directly), else `(f - 0.2) * 1.25`; engine on = +0x428 bit 0x10 or +0x424 >= 0;
/// `ProcessGenericJet(on, params, f, a / 255, b / 255, 0.0, 1.0)` (the bool word passed as AL only).
#[hook(addr = 0x501960, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyJet", expect = "8B 44 24 04 83 EC 08")]
pub fn process_dummy_jet(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let k255 = gf(0x859AAC);
    let fwd = rd::<i8>(veh, 0x424) >= 0;
    let a = crate::common::ftol32(x(rd::<f32>(veh, 0x49C)).fmul_m(k255)) as i16;
    let b = crate::common::ftol32(x(rd::<f32>(veh, 0x4A0)).fmul_m(k255)) as i16;
    let m = if a > b { a } else { b };
    let k = gf(0x859A3C);
    let f = clamp01_x(X87::from_i32(m as i32).fmul_m(k));
    let f = if fwd {
        f.fmul_m(gf(0x858C24)).fadd_m(gf(0x858CB0))
    } else {
        x(rd::<f32>(veh, 0x9C4)).fmul_m(gf(0x862D18))
    };
    let f = if f.fcom(x(gf(FLT_1))).test_any(0x41) {
        if f.fcom(x(gf(0x858CC4))).test_parity(0x05) {
            Some(f)
        } else {
            None
        }
    } else {
        Some(x(gf(FLT_1)))
    };
    let f = match f {
        Some(f) => f.fsub_m(gf(0x858CC4)).fmul_m(gf(0x8595F0)).to_f32(),
        None => 0.0,
    };
    let on = rd::<u8>(veh, 0x428) & 0x10 != 0 || fwd;
    let brake = X87::from_i32(b as i32).fmul_m(k).to_f32();
    let accel = X87::from_i32(a as i32).fmul_m(k).to_f32();
    process_generic_jet_call(
        this,
        on as u32,
        params,
        f.to_bits(),
        accel.to_bits(),
        brake.to_bits(),
        0,
        ONE,
    );
}

/// `CAEVehicleAudioEntity::ProcessDummyGolfCart(cVehicleParams&)` (0x501270): the dummy bank must be
/// ready ([`dummy_engine_bank_ready`]). Engine on (vehicle +0x428 bit 0x10) and not wrecked: `t =
/// clamp01(|speed / max velocity|)` (0 without a transmission or for a vehicle in status 0x20); below
/// `[0x859068]` the sound +0xF0 is stopped and forgotten; else `p = GetPiecewiseLinear(t, 4, 0x8CBFD8)`,
/// `UpdateGenericVehicleSound(1, +0xE0, +0xDC, 0, f32([0xB6BA9C] * t + [0x8CBFD0]), f32(AudioLog10(p) *
/// 20 + [0x8CBFCC]), 2.0)`. Otherwise all twelve engine sounds are cancelled. (AL, which
/// `ProcessSpecialVehicle` sets before the call, is not read.)
#[hook(addr = 0x501270, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyGolfCart", expect = "51 56 8B F1 66 83 BE E0 00 00 00 FF")]
pub fn process_dummy_golf_cart(this: GPtr<u8>, params: GPtr<u8>) {
    if !dummy_engine_bank_ready(this) {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if rd::<u8>(veh, 0x428) & 0x10 == 0 || rd::<u8>(this, 0xA8) != 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    let tr = rd::<u32>(params, 0x14);
    let mut t = if tr != 0 {
        clamp01_x(
            x(rd::<f32>(params, 0x1C))
                .fdiv_m(rd::<f32>(g::<u8>(tr as usize), 0x58))
                .fabs(),
        )
        .to_f32()
    } else {
        0.0
    };
    if rd::<u8>(g::<u8>(rd::<u32>(this, 4) as usize), 0x36) & 0xF8 == 0x20 {
        t = 0.0;
    }
    if !x(t).fcom(gd(0x859068)).test_parity(0x05) {
        stop_engine_sound(this, 0xF0);
        return;
    }
    let p = get_piecewise_linear(t.to_bits(), 4, 0x8CBFD8);
    let vol = crate::utility::audio_log10(p)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x8CBFCC))
        .to_f32();
    let speed = x(gf(0xB6BA9C)).fmul_m(t).fadd_m(gf(0x8CBFD0)).to_f32();
    let slot = rd::<u16>(this, 0xE0) as u32;
    let bank = rd::<u16>(this, 0xDC) as u32;
    update_generic_vehicle_sound_call(
        this,
        1,
        slot,
        bank,
        0,
        speed.to_bits(),
        vol.to_bits(),
        0x4000_0000,
    );
}

orig! {
    fn ps_rc_heli(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FAA80 => process_dummy_rc_heli;
    fn ps_rc_plane(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x4FA7C0 => process_dummy_rc_plane;
    fn ps_rc_car(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500DC0 => process_dummy_rc_car;
    fn ps_hovercraft(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x500F50 => process_dummy_hovercraft;
    fn ps_golf_cart(this: GPtr<u8>, p: GPtr<u8>) = thiscall 0x501270 => process_dummy_golf_cart;
}

/// `CAEVehicleAudioEntity::ProcessSpecialVehicle(cVehicleParams&)` (0x501AB0): by model (jump table
/// 0x501BA8): trailers 435 / 450 / 584 / 591 road noise + rain; RC Bandit / Tiger 441 / 564
/// `ProcessDummyRCCar`; Caddy 457 road noise, skidding, flat tyre, siren / alarm / horn,
/// `ProcessDummyGolfCart`, rain and, unless the vehicle's status is 0x10, engine damage; RC Baron 464
/// `ProcessDummyRCPlane`; RC Raider / Goblin 465 / 501 `ProcessDummyRCHeli`; Vortex 539 hovercraft, rain,
/// siren / alarm / horn; others nothing. The original sets AL = +0xA5 before the RC car and golf cart
/// calls; neither callee reads it.
#[hook(addr = 0x501AB0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessSpecialVehicle", expect = "56 57 8B 7C 24 0C")]
pub fn process_special_vehicle(this: GPtr<u8>, p: GPtr<u8>) {
    let model = rd::<i16>(g::<u8>(rd::<u32>(p, 0x10) as usize), 0x22);
    match model {
        435 | 450 | 584 | 591 => {
            pv_road_noise(this, p);
            pv_rain(this, p);
        }
        441 | 564 => ps_rc_car(this, p),
        457 => {
            pv_road_noise(this, p);
            pv_skidding(this, p);
            pv_flat_tyre(this, p);
            pv_siren_alarm_horn(this, p);
            ps_golf_cart(this, p);
            pv_rain(this, p);
            if rd::<u8>(g::<u8>(rd::<u32>(p, 0x10) as usize), 0x36) & 0xF8 != 0x10 {
                pv_engine_damage(this, p);
            }
        }
        464 => ps_rc_plane(this, p),
        465 | 501 => ps_rc_heli(this, p),
        539 => {
            ps_hovercraft(this, p);
            pv_rain(this, p);
            pv_siren_alarm_horn(this, p);
        }
        _ => {}
    }
}

orig! {
    /// `CAEVehicleAudioEntity::UpdateTrainSound(int16, int16, int16, float, float)` (0x4FA1C0)
    fn update_train_sound(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, speed: u32, volume: u32) = thiscall 0x4FA1C0 => unported;
}

/// `|v|` as `fld v; fcomp 0; fld v; test ah, 5; jp; fchs` (negated only below 0; NaN / -0.0 kept).
fn abs_x87(v: f32) -> X87 {
    if x(v).fcom(x(gf(FLT_0))).test_parity(0x05) {
        x(v)
    } else {
        x(v).fchs()
    }
}

/// `CAEVehicleAudioEntity::ProcessTrainTrackSound(cVehicleParams&)` (0x4FA3F0): the dummy bank must be
/// ready (without resetting +0xA9). Tuning by bank (+0xDC 0x85: volume base `[0x8CBF38]`, speed scale
/// `[0xB6BA84]`, speed base `[0x8CBF2C]`; else `[0x8CBF14]`, `[0xB6BA78]`, `[0x8CBF08]`). Derailed
/// (vehicle +0x5B9 bit 0): t = 0, volume -100. Else `s = |+0x5A4|`: above 1 -> t = 1; at least 1e-5 ->
/// t = f32(s); then volume = `f32(AudioLog10(t) * 20 + base)`; below 1e-5: t = s, volume -100. `n` =
/// carriages (the +0x5D0 chain length); `UpdateTrainSound(2, +0xE0, 2, f32((t * scale + base) *
/// [0x8CBF54 + 4 * (n % 5)]), volume)`.
#[hook(addr = 0x4FA3F0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessTrainTrackSound", expect = "83 EC 0C 57 8B F9")]
pub fn process_train_track_sound(this: GPtr<u8>, params: GPtr<u8>) {
    if !dummy_engine_bank_ready_ex(this, false) {
        return;
    }
    let (vb, sc, sb) = if rd::<u16>(this, 0xDC) == 0x85 {
        (0x8CBF38, 0xB6BA84, 0x8CBF2C)
    } else {
        (0x8CBF14, 0xB6BA78, 0x8CBF08)
    };
    let mut vol = gf(vb);
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let v = || rd::<f32>(veh, 0x5A4);
    let t = if rd::<u8>(veh, 0x5B9) & 1 != 0 {
        vol = gf(FLT_M100);
        x(gf(FLT_0))
    } else {
        // The original's first range test (`1 < |v|` or `0 <= |v|`) always reaches the same code.
        let tf = if !x(gf(FLT_1)).fcom(abs_x87(v())).test_parity(0x05) {
            Some(gf(FLT_1))
        } else {
            let a = abs_x87(v());
            if a.fcom(x(gf(0x858C14))).test_parity(0x05) {
                Some(a.to_f32())
            } else {
                vol = gf(FLT_M100);
                None
            }
        };
        match tf {
            Some(t) => {
                vol = crate::utility::audio_log10(t)
                    .fmul_m(gf(FLT_20))
                    .fadd_m(vol)
                    .to_f32();
                x(t)
            }
            None => abs_x87(v()),
        }
    };
    let mut n: u16 = 0;
    let mut e = veh.addr() as u32;
    loop {
        e = rd::<u32>(g::<u8>(e as usize), 0x5D0);
        n = n.wrapping_add(1);
        if e == 0 {
            break;
        }
    }
    let r = (n as i16 as i32) % 5;
    let speed = t
        .fmul_m(gf(sc))
        .fadd_m(gf(sb))
        .fmul_m(gf(0x8CBF54 + r as usize * 4))
        .to_f32();
    update_train_sound(
        this,
        2,
        rd::<u16>(this, 0xE0) as u32,
        2,
        speed.to_bits(),
        vol.to_bits(),
    );
}

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: u32) -> u32 = cdecl 0x53FB70 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0)
    fn pad_get_accelerate(pad: u32) -> u32 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080)
    fn pad_get_brake(pad: u32) -> u32 = thiscall 0x540080 => unported;
    /// `CAEVehicleAudioEntity::PlayTrainBrakeSound` (0x4FA630)
    fn play_train_brake_sound_call(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4FA630 => play_train_brake_sound;
}

/// `CAEVehicleAudioEntity::ProcessPlayerTrainBrakes(cVehicleParams&)` (0x500AB0): pad 0; `v` = train
/// speed (+0x5A4; 0 when derailed, +0x5B9 bit 0), negated unless +0x5B8 bit 0x40; `t = min(|v|, 1)`
/// (f32). Status 0 (player): `|v| < [0x858CDC]`: pedal = 0 (accelerate and brake still read);
/// moving forward (`v > 0`): pedal = brake; else pedal = accelerate (both pads read in the original's
/// order). Other statuses leave the pedal local unwritten: its stack residue (entry ESP - 8, `peek`).
/// Derailed or brakes off (+0x5B8 bit 3 clear): `PlayTrainBrakeSound(-1, 1.0, -100)`. Else speed =
/// `f32([0xB6BA88] * t + [0x8CBF44])`, `r = f32(GetPiecewiseLinear(t, 4, 0x8CC1E8) * (pedal / 255))`;
/// `r < -100`: sfx -1 / volume -100, else sfx 0 / volume `f32(AudioLog10(r) * 20 + [0x8CBF3C])`.
#[hook(addr = 0x500AB0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessPlayerTrainBrakes", expect = "83 EC 08 55 56", peek = 8)]
pub fn process_player_train_brakes(this: GPtr<u8>, params: GPtr<u8>) {
    let mut pedal = f32::from_bits(sa_core::abi::peek_stack(0x500AB0, -8).unwrap_or(0));
    let pad = pad_get_pad(0);
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let mut v = if rd::<u8>(veh, 0x5B9) & 1 != 0 {
        x(gf(FLT_0))
    } else {
        x(rd::<f32>(veh, 0x5A4))
    };
    if rd::<u8>(veh, 0x5B8) & 0x40 == 0 {
        v = v.fchs();
    }
    let zero = || x(gf(FLT_0));
    let abs = |v: X87| {
        if v.fcom(zero()).test_parity(0x05) {
            v
        } else {
            v.fchs()
        }
    };
    let t = if !x(gf(FLT_1)).fcom(abs(v)).test_parity(0x05) || x(gf(FLT_0)).fcom(abs(v)).test_any(0x41) {
        if x(gf(FLT_1)).fcom(abs(v)).test_parity(0x05) {
            if v.fcom(zero()).test_parity(0x05) {
                v.to_f32()
            } else {
                v.fchs().to_f32()
            }
        } else {
            gf(FLT_1)
        }
    } else {
        0.0
    };
    if rd::<u8>(veh, 0x36) & 0xF8 == 0 {
        if !abs(v).fcom(x(gf(0x858CDC))).test_parity(0x05) {
            pedal = 0.0;
            pad_get_accelerate(pad);
            pad_get_brake(pad);
        } else if !v.fcom(zero()).test_any(0x41) {
            let b = pad_get_brake(pad) as u16 as i16;
            pedal = X87::from_i32(b as i32).to_f32();
            pad_get_accelerate(pad);
        } else {
            pad_get_brake(pad);
            let a = pad_get_accelerate(pad) as u16 as i16;
            pedal = X87::from_i32(a as i32).to_f32();
        }
    }
    if rd::<u8>(veh, 0x5B9) & 1 != 0 || rd::<u8>(veh, 0x5B8) & 8 == 0 {
        play_train_brake_sound_call(this, 0xFFFF_FFFF, ONE, 0xC2C8_0000);
        return;
    }
    let speed = x(gf(0xB6BA88)).fmul_m(t).fadd_m(gf(0x8CBF44)).to_f32();
    let p = x(get_piecewise_linear(t.to_bits(), 4, 0x8CC1E8));
    let r = p.fmul(x(pedal).fmul_m(gf(0x859A3C))).to_f32();
    if !x(r).fcom(x(gf(FLT_M100))).test_parity(0x05) {
        play_train_brake_sound_call(this, 0xFFFF_FFFF, speed.to_bits(), 0xC2C8_0000);
        return;
    }
    let vol = crate::utility::audio_log10(r)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x8CBF3C))
        .to_f32();
    play_train_brake_sound_call(this, 0, speed.to_bits(), vol.to_bits());
}

/// `CAEVehicleAudioEntity::ProcessDummyTrainEngine(cVehicleParams&)` (0x500710): pad 0; the dummy bank
/// must be ready (without resetting +0xA9). Tuning by bank (+0xDC 0x85: volume base `[0x8CBF30]`,
/// speed scale `[0xB6BA80]`, speed base `[0x8CBF24]`, slope factor `[0x8CBF50]`; else `[0xB6BA00]`,
/// `[0xB6BA78]`, `[0x8CBF08]`, `[0x8CBF4C]`). Engine off (vehicle +0x5B8 bit 3 clear): the sound +0xF0
/// is stopped and forgotten. Else slope `s = clamp((fy + fx) * 0 + fz)` of the matrix forward between
/// `[0x862D60]` and `[0x862D58]`, `k = f32(factor * s)`; `V` = train speed (+0x5A4; 0 derailed), negated
/// (as f32) unless +0x5B8 bit 0x40; `t` = 0 derailed, else `min(|V|, 1)` (the dword copied when V >=
/// 0); bank 0x85: volume = `f32(AudioLog10(t) * 20 + base)`; status 0 (player): volume +=
/// `pedal / 255 * [0x8CBF10]` (pedal = accelerate - brake when `|V| < [0x858CDC]`, else accelerate
/// moving forward or -brake). `UpdateTrainSound(1, +0xE0, 0, f32((t * scale + base) * (k + 1)),
/// volume)`.
#[hook(addr = 0x500710, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyTrainEngine", expect = "83 EC 14 53 56")]
pub fn process_dummy_train_engine(this: GPtr<u8>, params: GPtr<u8>) {
    let pad = pad_get_pad(0);
    if !dummy_engine_bank_ready_ex(this, false) {
        return;
    }
    let b85 = rd::<u16>(this, 0xDC) == 0x85;
    let (mut vol, scale, base, factor) = if b85 {
        (gf(0x8CBF30), gf(0xB6BA80), gf(0x8CBF24), gf(0x8CBF50))
    } else {
        (gf(0xB6BA00), gf(0xB6BA78), gf(0x8CBF08), gf(0x8CBF4C))
    };
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let flags = rd::<u8>(veh, 0x5B8);
    if flags & 8 == 0 {
        stop_engine_sound(this, 0xF0);
        return;
    }
    let m: GPtr<u8> = g(rd::<u32>(veh, 0x14) as usize);
    let s = x(rd::<f32>(m, 0x14))
        .fadd_m(rd::<f32>(m, 0x10))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 0x18));
    let (lo, hi) = (gd(0x862D60), gd(0x862D58));
    let s = if !s.fcom(lo).test_any(0x41) && s.fcom(hi).test_parity(0x05) {
        hi
    } else if s.fcom(lo).test_any(0x41) {
        lo
    } else {
        s
    };
    let k = x(factor).fmul(s).to_f32();
    let derailed = rd::<u8>(veh, 0x5B9) & 1 != 0;
    let mut vb: u32 = if derailed { 0 } else { rd::<u32>(veh, 0x5A4) };
    if flags & 0x40 == 0 {
        vb = x(f32::from_bits(vb)).fchs().to_f32().to_bits();
    }
    let v = f32::from_bits(vb);
    let zero = || x(gf(FLT_0));
    let abs = || {
        if x(v).fcom(zero()).test_parity(0x05) {
            x(v)
        } else {
            x(v).fchs()
        }
    };
    let t: u32 = if derailed {
        0
    } else if !x(gf(FLT_1)).fcom(abs()).test_parity(0x05) || x(gf(FLT_0)).fcom(abs()).test_any(0x41) {
        if !x(gf(FLT_1)).fcom(abs()).test_parity(0x05) {
            ONE
        } else if !x(v).fcom(zero()).test_parity(0x05) {
            x(v).fchs().to_f32().to_bits()
        } else {
            vb
        }
    } else {
        0
    };
    let t = f32::from_bits(t);
    if b85 {
        vol = crate::utility::audio_log10(t)
            .fmul_m(gf(FLT_20))
            .fadd_m(vol)
            .to_f32();
    }
    if rd::<u8>(veh, 0x36) & 0xF8 == 0 {
        let d = if !abs().fcom(x(gf(0x858CDC))).test_parity(0x05) {
            let a = pad_get_accelerate(pad) as u16 as i16 as i32;
            let b = pad_get_brake(pad) as u16 as i16 as i32;
            a.wrapping_sub(b)
        } else if !x(v).fcom(zero()).test_any(0x41) {
            pad_get_accelerate(pad) as u16 as i16 as i32
        } else {
            (pad_get_brake(pad) as u16 as i16 as i32).wrapping_neg()
        };
        vol = X87::from_i32(d)
            .fmul_m(gf(0x859A3C))
            .fmul_m(gf(0x8CBF10))
            .fadd_m(vol)
            .to_f32();
    }
    let speed = x(t)
        .fmul_m(scale)
        .fadd_m(base)
        .fmul(x(k).fadd_m(gf(FLT_1)))
        .to_f32();
    update_train_sound(
        this,
        1,
        rd::<u16>(this, 0xE0) as u32,
        0,
        speed.to_bits(),
        vol.to_bits(),
    );
}

orig! {
    /// `CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre(float)` (0x4F60B0, called with `this`), ST0
    fn bicycle_tyre_volume_st0(this: GPtr<u8>, v: u32) -> f64 = thiscall 0x4F60B0 => unported;
    /// `CAEVehicleAudioEntity::PlayBicycleSound` (0x4F9710)
    fn play_bicycle_sound_call(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, volume: u32, speed: u32) = thiscall 0x4F9710 => play_bicycle_sound;
}

/// The common body of the bicycle processors: `t = |speed / max velocity|` clamped to 1 (f32, 0 when
/// not above 0); `w` = wheels on the ground (vehicle byte +0x804) as f32; chain volume `f32(
/// AudioLog10(f32(w * 0.5 * t)) * 20 + m_EventVolume + [0x8CBEBC])`, chain speed `f32((w * k * c48 +
/// k * t + [0x8CBD5C]) * (|sin(+0x648)| * [0x8CBD60] + 1))` (`k = [0xB6BA6C]`); tyre volume `tv =
/// GetBaseVolumeForBicycleTyre(t)` with tyre contact (+0x834), else 0; `PlayBicycleSound(1, +0xE0, 0,
/// chain volume, chain speed)`, `PlayBicycleSound(2, +0xE0, 1, f32(AudioLog10(tv) * 20 + m_EventVolume
/// + [0x8CBEC0]), f32([0xB6BA70] * t + [0x8CBD64]))`; when the tyre lost contact since the last frame
/// (`flag` static set): `PlayBicycleSound(3, +0xE0, 1, chain volume, 1.0)`. `flag` = contact.
fn bicycle_body(this: GPtr<u8>, params: GPtr<u8>, flag: usize) {
    let tr: GPtr<u8> = g(rd::<u32>(params, 0x14) as usize);
    let r = x(rd::<f32>(params, 0x1C)).fdiv_m(rd::<f32>(tr, 0x58));
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let wheels = rd::<u8>(veh(), 0x804) as i32;
    let a = if r.fcom(x(gf(FLT_0))).test_parity(0x05) {
        r
    } else {
        r.fchs()
    };
    let t = if !a.fcom(gd(0x85A310)).test_parity(0x05) {
        if r.fcom(x(gf(FLT_0))).test_parity(0x05) {
            r
        } else {
            r.fchs()
        }
    } else {
        gd(0x85A310)
    };
    let t = if !t.fcom(x(gf(FLT_0))).test_any(0x41) {
        t.to_f32()
    } else {
        0.0
    };
    let w = X87::from_i32(wheels).to_f32();
    let arg = x(w).fmul_m(gf(0x858C84)).fmul_m(t).to_f32();
    let chain_vol = crate::utility::audio_log10(arg)
        .fmul_m(gf(FLT_20))
        .fadd_m(rd::<f32>(this, 0xD8))
        .fadd_m(gf(0x8CBEBC))
        .to_f32();
    let k = gf(0xB6BA6C);
    let s1 = x(w)
        .fmul_m(k)
        .fmul_m(gf(0x858C48))
        .fadd(x(k).fmul_m(t))
        .fadd_m(gf(0x8CBD5C));
    let mut sn = x(rd::<f32>(veh(), 0x648)).fsin();
    if !sn.fcom(x(gf(FLT_0))).test_parity(0x05) {
        sn = sn.fchs();
    }
    let contact = rd::<u8>(veh(), 0x834) != 0;
    let chain_speed = s1.fmul(sn.fmul_m(gf(0x8CBD60)).fadd_m(gf(FLT_1))).to_f32();
    let tv = if contact {
        st0(bicycle_tyre_volume_st0(this, t.to_bits())).to_f32()
    } else {
        0.0
    };
    let tyre_vol = crate::utility::audio_log10(tv)
        .fmul_m(gf(FLT_20))
        .fadd_m(rd::<f32>(this, 0xD8))
        .fadd_m(gf(0x8CBEC0))
        .to_f32();
    let tyre_speed = x(gf(0xB6BA70)).fmul_m(t).fadd_m(gf(0x8CBD64)).to_f32();
    let slot = || rd::<u16>(this, 0xE0) as u32;
    play_bicycle_sound_call(this, 1, slot(), 0, chain_vol.to_bits(), chain_speed.to_bits());
    play_bicycle_sound_call(this, 2, slot(), 1, tyre_vol.to_bits(), tyre_speed.to_bits());
    if !contact && g::<u8>(flag).read() != 0 {
        play_bicycle_sound_call(this, 3, slot(), 1, chain_vol.to_bits(), ONE);
    }
    g::<u8>(flag).write(contact as u8);
}

/// `CAEVehicleAudioEntity::ProcessDummyBicycle(cVehicleParams&)` (0x4FFDC0): the dummy bank must be
/// ready ([`dummy_engine_bank_ready`]), then [`bicycle_body`] with the static flag 0xB6BAC8.
#[hook(addr = 0x4FFDC0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyBicycle", expect = "83 EC 0C 56 8B F1")]
pub fn process_dummy_bicycle(this: GPtr<u8>, params: GPtr<u8>) {
    if !dummy_engine_bank_ready(this) {
        return;
    }
    bicycle_body(this, params, 0xB6BAC8);
}

/// `CAEVehicleAudioEntity::ProcessPlayerBicycle(cVehicleParams&)` (0x500040): nothing unless bank +0xDC
/// is loaded in slot +0xE0. Above 5.0 at vehicle +0x818, a sign change of +0x820 against the last frame
/// (`[0xB6BAD0] * +0x820 < 0`) sets the static `[0xB6BACC]` to 6.0 when it is exactly 0; then
/// `[0xB6BACC] = max([0xB6BACC] - 0.7, 0)` (NaN kept); [`bicycle_body`] with the static flag 0xB6BAC9;
/// `[0xB6BAD0]` = +0x820.
#[hook(addr = 0x500040, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessPlayerBicycle", expect = "83 EC 0C 56 8B F1")]
pub fn process_player_bicycle(this: GPtr<u8>, params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDC) as u32, rd::<u16>(this, 0xE0) as u32) == 0 {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let st = g::<f32>(0xB6BACC);
    if !x(rd::<f32>(veh, 0x818)).fcom(x(gf(0x858C80))).test_any(0x41)
        && !x(gf(0xB6BAD0))
            .fmul_m(rd::<f32>(veh, 0x820))
            .fcom(x(gf(FLT_0)))
            .test_parity(0x05)
        && !x(st.read()).fcom(x(gf(FLT_0))).test_parity(0x44)
    {
        st.write(6.0);
    }
    let v = x(st.read()).fsub_m(gf(0x858CB0));
    if x(gf(FLT_0)).fcom(v).test_any(0x41) {
        st.write(v.to_f32());
    } else {
        st.write(0.0);
    }
    bicycle_body(this, params, 0xB6BAC9);
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    g::<u32>(0xB6BAD0).write(rd::<u32>(veh, 0x820));
}

orig! {
    /// `CAEVehicleAudioEntity::GetAccelAndBrake(cVehicleParams&)` (0x4F5080)
    fn get_accel_and_brake(this: GPtr<u8>, params: GPtr<u8>) = thiscall 0x4F5080 => crate::vehicle::get_accel_and_brake;
}

/// The throttle ratio of the dummy aircraft: `p > 1 -> 1`, `p < 0.2 -> 0` (stored directly), else
/// `(p - 0.2) * 1.25`; then at least `[0x862D08]` (double).
fn aircraft_throttle(p: f32) -> X87 {
    let v = if x(p).fcom(x(gf(FLT_1))).test_any(0x41) {
        if x(p).fcom(x(gf(0x858CC4))).test_parity(0x05) {
            Some(x(p))
        } else {
            None
        }
    } else {
        Some(x(gf(FLT_1)))
    };
    let v = match v {
        Some(v) => v.fsub_m(gf(0x858CC4)).fmul_m(gf(0x8595F0)),
        None => x(gf(FLT_0)),
    };
    if v.fcom(gd(0x862D08)).test_any(0x41) {
        gd(0x862D08)
    } else {
        v
    }
}

/// The RC prologue shared by `ProcessDummyRCPlane` / `ProcessDummyRCHeli`: false when the dummy bank
/// is not ready or the vehicle is wrecked (all twelve engine sounds cancelled).
fn rc_ready(this: GPtr<u8>) -> bool {
    if !dummy_engine_bank_ready(this) {
        return false;
    }
    if rd::<u8>(this, 0xA8) != 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return false;
    }
    true
}

/// The RC pedal term: 0.1 accelerating, -0.05 braking (wins), else 0; the pedals come from
/// `GetAccelAndBrake` for the player's own RC vehicle (`s_pPlayerDriver` and +0xA5), else are cleared.
fn rc_pedal_term(this: GPtr<u8>, params: GPtr<u8>) -> f32 {
    if g::<u32>(0xB6B990).read() != 0 && rd::<u8>(this, 0xA5) != 0 {
        get_accel_and_brake(this, params);
    } else {
        wr::<u16>(params, 0x20, 0);
        wr::<u16>(params, 0x22, 0);
    }
    let mut l = 0.0f32;
    if rd::<i16>(params, 0x20) > 0 {
        l = f32::from_bits(0x3DCC_CCCD);
    }
    if rd::<i16>(params, 0x22) > 0 {
        l = f32::from_bits(0xBD4C_CCCD);
    }
    l
}

/// The speed smoothing of the RC aircraft: +0x22C (initialised to `s` while negative) moves towards
/// `s` by at most `[0x862D04]`; then `PlayAircraftSound(2, +0xE0, 0, volume, s)`.
fn rc_finish(this: GPtr<u8>, s: f32, vol: f32) {
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, s);
    }
    let cur = rd::<f32>(this, 0x22C);
    let s = if x(s).fcom(x(cur)).test_any(0x01) {
        let n = x(cur).fsub_m(gf(0x862D04));
        if x(s).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            s
        }
    } else {
        let n = x(cur).fadd_m(gf(0x862D04));
        if x(s).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            s
        }
    };
    wr::<f32>(this, 0x22C, s);
    play_aircraft_sound_call(
        this,
        2,
        rd::<u16>(this, 0xE0) as u32,
        0,
        vol.to_bits(),
        s.to_bits(),
    );
}

/// `CAEVehicleAudioEntity::ProcessDummyRCPlane(cVehicleParams&)` (0x4FA7C0): bank / wreck prologue
/// ([`rc_ready`]); `p = f32(vehicle +0x9C4 * [0x862D18])`; pedal term; pitch `L2 = f32((fy + fx) * 0 +
/// fz) * -1` and roll `L0 = (rx + ry) * 0 + rz` (times -1 when negative, f32) of the matrix; volume
/// `f32(AudioLog10(f32 throttle(p)) * 20 + [0xB6BA04])`, speed `f32(L0 * 0.1 + L2 * 0.15 + pedal + 1)`;
/// [`rc_finish`].
#[hook(addr = 0x4FA7C0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyRCPlane", expect = "83 EC 10 56 8B F1")]
pub fn process_dummy_rc_plane(this: GPtr<u8>, params: GPtr<u8>) {
    if !rc_ready(this) {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let p = x(rd::<f32>(veh, 0x9C4)).fmul_m(gf(0x862D18)).to_f32();
    let l1 = rc_pedal_term(this, params);
    let m: GPtr<u8> = g(rd::<u32>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x14) as usize);
    let l2 = x(rd::<f32>(m, 0x14))
        .fadd_m(rd::<f32>(m, 0x10))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 0x18))
        .fmul(gd(0x862D10))
        .to_f32();
    let mut l0 = x(rd::<f32>(m, 4))
        .fadd_m(rd::<f32>(m, 0))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 8))
        .to_f32();
    if !x(l0).fcom(x(gf(FLT_0))).test_parity(0x05) {
        l0 = x(l0).fmul(gd(0x862D10)).to_f32();
    }
    let v = aircraft_throttle(p).to_f32();
    let vol = crate::utility::audio_log10(v)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0xB6BA04))
        .to_f32();
    let s = x(l0)
        .fmul_m(gf(0x858B1C))
        .fadd(x(l2).fmul_m(gf(0x858FCC)))
        .fadd_m(l1)
        .fadd_m(gf(FLT_1))
        .to_f32();
    rc_finish(this, s, vol);
}

/// `CAEVehicleAudioEntity::ProcessDummyRCHeli(cVehicleParams&)` (0x4FAA80): as the RC plane with `p =
/// f32(vehicle +0x84C * [0x862D1C])`, tilt `z = (1 - ((ux + uy) * 0 + uz)) * 0.5 / [0x8CBF6C]` of the
/// matrix up vector (+0x20..; at least 1 -> 1.0, f32), `v = f32(throttle(p))`, volume `f32(AudioLog10(v)
/// * 20 + [0x8CBF68])`, speed `f32(v * [0x862D00] + z * 0.2 + pedal + 0.8)`; [`rc_finish`].
#[hook(addr = 0x4FAA80, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyRCHeli", expect = "83 EC 0C 56 8B F1")]
pub fn process_dummy_rc_heli(this: GPtr<u8>, params: GPtr<u8>) {
    if !rc_ready(this) {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let p = x(rd::<f32>(veh, 0x84C)).fmul_m(gf(0x862D1C)).to_f32();
    let l1 = rc_pedal_term(this, params);
    let m: GPtr<u8> = g(rd::<u32>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x14) as usize);
    let z = x(gf(FLT_1))
        .fsub(
            x(rd::<f32>(m, 0x24))
                .fadd_m(rd::<f32>(m, 0x20))
                .fmul_m(gf(FLT_0))
                .fadd_m(rd::<f32>(m, 0x28)),
        )
        .fmul_m(gf(0x858B8C))
        .fdiv_m(gf(0x8CBF6C));
    let lz = if z.fcom(x(gf(FLT_1))).test_parity(0x05) {
        gf(FLT_1)
    } else {
        z.to_f32()
    };
    let v = aircraft_throttle(p).to_f32();
    let vol = crate::utility::audio_log10(v)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x8CBF68))
        .to_f32();
    let s = x(v)
        .fmul_m(gf(0x862D00))
        .fadd(x(lz).fmul_m(gf(0x858CC4)))
        .fadd_m(l1)
        .fadd_m(gf(0x858C98))
        .to_f32();
    rc_finish(this, s, vol);
}

orig! {
    /// `CAEVehicleAudioEntity::UpdateBoatSound` (0x4F9E90)
    fn update_boat_sound_call(this: GPtr<u8>, ty: u32, slot: u32, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F9E90 => update_boat_sound;
}

/// `CAEVehicleAudioEntity::ProcessBoatEngine(cVehicleParams&)` (0x5003A0): the dummy bank must be
/// ready (without resetting +0xA9). Engine off (vehicle +0x428 bit 0x10) or wrecked: engine sounds
/// +0xF8 / +0x100 stopped and forgotten. Else the player's pedals (`GetAccelAndBrake` with
/// `s_pPlayerDriver` and +0xA5); `P = min(1 - ((ux + uy) * 0 + uz), 1)` (f32) of the matrix up vector;
/// `c = clamp01(+0x5A4 * [0x858B3C])`; `P = f32([0x8CBED4] * P + (k * c + [0x858CC8]))` (`k` =
/// `[0x862D50]` with +0x5AC bit 1, else `[0xB6BA74]`); `A = f32(c * [0x858B3C] - [0x858B3C])`; the
/// smoothed speed +0x234 / volume +0x230 (set to the new values while negative) move P and A towards
/// themselves by at most `[0x858B38]` / `[0x858B3C]`. Underwater (`UnderWaterness >= 0.5`): boat sound
/// 2 (+0xE0, sfx 0) silent and 3 (slot 0x13, sfx 0x1C) at P, `A + [0x858B44]`; else 2 at P, A and 3
/// silent. +0x230 = A, +0x234 = P.
#[hook(addr = 0x5003A0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessBoatEngine", expect = "83 EC 08 56 8B F1")]
pub fn process_boat_engine(this: GPtr<u8>, params: GPtr<u8>) {
    if !dummy_engine_bank_ready_ex(this, false) {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if rd::<u8>(veh, 0x428) & 0x10 == 0 || rd::<u8>(this, 0xA8) != 0 {
        stop_engine_sound(this, 0xF8);
        stop_engine_sound(this, 0x100);
        return;
    }
    if g::<u32>(0xB6B990).read() != 0 && rd::<u8>(this, 0xA5) != 0 {
        get_accel_and_brake(this, params);
    }
    let m: GPtr<u8> = g(rd::<u32>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x14) as usize);
    let up = x(gf(FLT_1)).fsub(
        x(rd::<f32>(m, 0x24))
            .fadd_m(rd::<f32>(m, 0x20))
            .fmul_m(gf(FLT_0))
            .fadd_m(rd::<f32>(m, 0x28)),
    );
    let p = if up.fcom(x(gf(FLT_1))).test_parity(0x05) {
        gf(FLT_1)
    } else {
        up.to_f32()
    };
    let c = clamp01_x(x(rd::<f32>(veh, 0x5A4)).fmul_m(gf(0x858B3C)));
    let k = if rd::<u8>(veh, 0x5AC) & 2 != 0 {
        gf(0x862D50)
    } else {
        gf(0xB6BA74)
    };
    let q = x(k).fmul(c).fadd_m(gf(0x858CC8));
    let mut p = x(gf(0x8CBED4)).fmul_m(p).fadd(q).to_f32();
    let cv = c.fmul_m(gf(0x858B3C)).fsub_m(gf(0x858B3C));
    let mut a = cv.to_f32();
    if !x(rd::<f32>(this, 0x234)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x234, p);
    }
    if !x(rd::<f32>(this, 0x230)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x230, a);
    }
    let cur = rd::<f32>(this, 0x234);
    if x(p).fcom(x(cur)).test_any(0x41) {
        let n = x(cur).fsub_m(gf(0x858B38));
        if x(p).fcom(n).test_any(0x41) {
            p = n.to_f32();
        }
    } else {
        let n = x(cur).fadd_m(gf(0x858B38));
        if x(p).fcom(n).test_parity(0x05) {
            p = n.to_f32();
        }
    }
    let curv = rd::<f32>(this, 0x230);
    if cv.fcom(x(curv)).test_any(0x41) {
        let n = x(curv).fsub_m(gf(0x858B3C)).to_f32();
        if cv.fcom(x(n)).test_any(0x41) {
            a = n;
        }
    } else {
        let n = x(curv).fadd_m(gf(0x858B3C)).to_f32();
        if cv.fcom(x(n)).test_parity(0x05) {
            a = n;
        }
    }
    let slot = rd::<u16>(this, 0xE0) as u32;
    if !x(gf(0xC8132C)).fcom(x(gf(0x858B8C))).test_any(0x01) {
        update_boat_sound_call(this, 2, slot, 0, ONE, 0xC2C8_0000);
        let v = x(a).fadd_m(gf(0x858B44)).to_f32();
        update_boat_sound_call(this, 3, 0x13, 0x1C, p.to_bits(), v.to_bits());
    } else {
        update_boat_sound_call(this, 2, slot, 0, p.to_bits(), a.to_bits());
        update_boat_sound_call(this, 3, 0x13, 0x1C, ONE, 0xC2C8_0000);
    }
    wr::<f32>(this, 0x230, a);
    wr::<f32>(this, 0x234, p);
}

/// `t = min(|v|, 1)` as the trains / hovercraft compute it: the f32 of `v` (or of `-v` below 0), 1.0
/// above 1 (and 0 for the unreachable `0 > |v|` case).
fn min_abs_one(v: X87) -> f32 {
    let zero = || x(gf(FLT_0));
    let abs = || {
        if v.fcom(zero()).test_parity(0x05) {
            v
        } else {
            v.fchs()
        }
    };
    if !x(gf(FLT_1)).fcom(abs()).test_parity(0x05) || x(gf(FLT_0)).fcom(abs()).test_any(0x41) {
        if x(gf(FLT_1)).fcom(abs()).test_parity(0x05) {
            if v.fcom(zero()).test_parity(0x05) {
                v.to_f32()
            } else {
                v.fchs().to_f32()
            }
        } else {
            gf(FLT_1)
        }
    } else {
        0.0
    }
}

/// `CAEVehicleAudioEntity::ProcessDummyHovercraft(cVehicleParams&)` (0x500F50): pad 0; the dummy bank
/// must be ready ([`dummy_engine_bank_ready`]); engine off (vehicle +0x428 bit 0x10) or wrecked: all
/// twelve engine sounds cancelled. Else `T = min(|+0x988 * [0x862D68]|, 1)` (f32, kept in the argument
/// slot); pedals: status 0 (player) with `speed < [0x858CDC]`: accel = low word of (accelerate - brake),
/// brake = 0; otherwise accelerate / brake; other statuses: both = the low word of T's bits (the
/// original reads its overwritten argument slot). `F = -0.05` with brake > 0, else 0.1 with accel > 0,
/// else T. `r = clamp01(+0x9C4 * [0x862D18])`; volume `f32([0xB6BA90] * r + [0x8CBFAC])`, speed
/// `f32((([0xB6BA94] * r + [0x8CBFBC] * T) + [0x8CBFB4]) + F)` smoothed by +0x22C as the RC aircraft;
/// `UpdateGenericVehicleSound(1, +0xE0, +0xDC, 0, speed, volume, 3.5)`.
#[hook(addr = 0x500F50, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyHovercraft", expect = "51 53 56 6A 00")]
pub fn process_dummy_hovercraft(this: GPtr<u8>, params: GPtr<u8>) {
    let pad = pad_get_pad(0);
    if !dummy_engine_bank_ready(this) {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    if rd::<u8>(veh, 0x428) & 0x10 == 0 || rd::<u8>(this, 0xA8) != 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    let t = min_abs_one(x(rd::<f32>(veh, 0x988)).fmul_m(gf(0x862D68)));
    let (accel, brake): (i16, i16) = if rd::<u8>(veh, 0x36) & 0xF8 == 0 {
        if !x(rd::<f32>(params, 0x1C)).fcom(x(gf(0x858CDC))).test_parity(0x05) {
            let a = pad_get_accelerate(pad) as u16;
            let b = pad_get_brake(pad) as u16;
            (a.wrapping_sub(b) as i16, 0)
        } else {
            let b = pad_get_brake(pad) as u16 as i16;
            let a = pad_get_accelerate(pad) as u16 as i16;
            (a, b)
        }
    } else {
        let w = t.to_bits() as u16 as i16;
        (w, w)
    };
    let mut f = if accel > 0 { x(gf(0x858B1C)) } else { x(t) };
    if brake > 0 {
        f = x(gf(0x85901C));
    }
    let r = clamp01_x(x(rd::<f32>(veh, 0x9C4)).fmul_m(gf(0x862D18)));
    let vol = x(gf(0xB6BA90)).fmul(r).fadd_m(gf(0x8CBFAC)).to_f32();
    let s = x(gf(0xB6BA94))
        .fmul(r)
        .fadd(x(gf(0x8CBFBC)).fmul_m(t))
        .fadd_m(gf(0x8CBFB4))
        .fadd(f)
        .to_f32();
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, s);
    }
    let cur = rd::<f32>(this, 0x22C);
    let s = if x(s).fcom(x(cur)).test_any(0x01) {
        let n = x(cur).fsub_m(gf(0x862D04));
        if x(s).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            s
        }
    } else {
        let n = x(cur).fadd_m(gf(0x862D04));
        if x(s).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            s
        }
    };
    wr::<f32>(this, 0x22C, s);
    let slot = rd::<u16>(this, 0xE0) as u32;
    let bank = rd::<u16>(this, 0xDC) as u32;
    update_generic_vehicle_sound_call(this, 1, slot, bank, 0, s.to_bits(), vol.to_bits(), 0x4060_0000);
}

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// `CAEVehicleAudioEntity::ProcessGenericJet(bool engineOn, cVehicleParams&, float throttle, float
/// accel, float brake, float volumeOffset, float speedFactor)` (0x4FF900): with bank 0x8A in slot 0x13;
/// jets 519 / 520 / 577 / 592 only (tuning: volume bases A / B / E and the afterburner volume C, from
/// `.data` / `.bss` by model), and `engineOn` (AL of the argument), else all twelve engine sounds are
/// cancelled. `n` = normalised (vehicle - camera) position; `D = f32(((n.y*f.y + n.z*f.z) + n.x*f.x + 1)
/// * 0.5)` with the vehicle's forward vector; pitch `L34 = f32(((f.y + f.x) * 0 + f.z) * -1)`, roll `L24
/// = (r.y + r.x) * 0 + r.z` (times -1 when negative); pedal term 0.1 accelerating, - 0.05 braking;
/// sound 2 (sfx 10): `f32(AudioLog10(f32((1 - [0x8CBD98] * D) * throttle)) * 20 + A)`; sound 4 (sfx 11):
/// `f32(AudioLog10(f32((D * 0.5 + 0.5) * throttle)) * 20 + B)`; +0x230 = C accelerating, else -1.0 per
/// frame; sound 5 (sfx 0x1A, speed 1): `f32(f32(AudioLog10(throttle) * 20 + +0x230) + volumeOffset)`;
/// sound 7 (sfx 0xE, speed 1): E. Sounds 2 / 4 at speed `f32((L24 * 0.1 + L34 * 0.15 + pedal + 1) *
/// speedFactor)` smoothed by +0x22C (as the RC aircraft). The stack `CMatrix` the original assigns
/// the camera matrix to is never read (no observable effect).
#[hook(addr = 0x4FF900, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessGenericJet", expect = "6A FF 68 C8 C4 83 00")]
#[allow(clippy::too_many_arguments)]
pub fn process_generic_jet(
    this: GPtr<u8>,
    on: u32,
    params: GPtr<u8>,
    throttle: u32,
    accel: u32,
    brake: u32,
    vol_off: u32,
    speed_factor: u32,
) {
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let (a, b, c, e) = match rd::<i16>(veh(), 0x22) {
        519 => (0x8CBE68, 0x8CBE6C, 0xB6B9F4, 0x8CBE78),
        520 => (0x8CBE7C, 0x8CBE80, 0xB6B9F8, 0x8CBE8C),
        577 => (0x8CBE90, 0x8CBE94, 0x8CBE98, 0x8CBEA4),
        592 => (0x8CBEA8, 0x8CBEAC, 0xB6B9FC, 0x8CBEB8),
        _ => {
            for i in 0..12u32 {
                cancel_vehicle_engine_sound(this, i);
            }
            return;
        }
    };
    let (a, b, c, e) = (gf(a), gf(b), g::<u32>(c).read(), g::<u32>(e).read());
    if on as u8 == 0 {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    let cm = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cm != 0 {
        g(cm as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let p = veh_pos(this);
    let d = |i: usize| {
        x(f32::from_bits(p[i]))
            .fsub_m(rd::<f32>(cam, i as isize * 4))
            .to_f32()
    };
    let (dz, dy, dx) = (d(2), d(1), d(0));
    let mut n = [dx, dy, dz];
    vector_normalise(GPtr::from_raw(n.as_mut_ptr().cast::<u8>()));
    let m: GPtr<u8> = g(rd::<u32>(veh(), 0x14) as usize);
    let dot = x(n[1])
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(n[2]).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(n[0]).fmul_m(rd::<f32>(m, 0x10)));
    let l34 = x(rd::<f32>(m, 0x14))
        .fadd_m(rd::<f32>(m, 0x10))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 0x18))
        .fmul(gd(0x862D10))
        .to_f32();
    let mut l24 = x(rd::<f32>(m, 4))
        .fadd_m(rd::<f32>(m, 0))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 8))
        .to_f32();
    if !x(l24).fcom(x(gf(FLT_0))).test_parity(0x05) {
        l24 = x(l24).fmul(gd(0x862D10)).to_f32();
    }
    let dd = dot.fadd_m(gf(FLT_1)).fmul_m(gf(0x858B8C)).to_f32();
    let pos = |v: u32| !x(f32::from_bits(v)).fcom(x(gf(FLT_0))).test_any(0x41);
    let mut pedal = 0.0f32;
    if pos(accel) {
        pedal = f32::from_bits(0x3DCC_CCCD);
    }
    if pos(brake) {
        pedal = x(pedal).fsub_m(gf(0x858C28)).to_f32();
    }
    let thr = f32::from_bits(throttle);
    let v1 = x(gf(0x8CBD98)).fmul_m(dd).fsubr_m(gf(FLT_1)).fmul_m(thr).to_f32();
    let vol1 = crate::utility::audio_log10(v1)
        .fmul_m(gf(FLT_20))
        .fadd_m(a)
        .to_f32();
    let v2 = x(dd)
        .fmul_m(gf(0x858B8C))
        .fadd_m(gf(0x858B8C))
        .fmul_m(thr)
        .to_f32();
    let vol2 = crate::utility::audio_log10(v2)
        .fmul_m(gf(FLT_20))
        .fadd_m(b)
        .to_f32();
    if pos(accel) {
        wr::<u32>(this, 0x230, c);
    } else {
        wr::<f32>(this, 0x230, x(rd::<f32>(this, 0x230)).fsub(gd(0x85A310)).to_f32());
    }
    let vol3 = crate::utility::audio_log10(thr)
        .fmul_m(gf(FLT_20))
        .fadd_m(rd::<f32>(this, 0x230))
        .to_f32();
    let s = x(l24)
        .fmul_m(gf(0x858B1C))
        .fadd(x(l34).fmul_m(gf(0x858FCC)))
        .fadd_m(pedal)
        .fadd_m(gf(FLT_1))
        .fmul_m(f32::from_bits(speed_factor))
        .to_f32();
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, s);
    }
    let cur = rd::<f32>(this, 0x22C);
    let s = if x(s).fcom(x(cur)).test_any(0x01) {
        let n = x(cur).fsub_m(gf(0x862D04));
        if x(s).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            s
        }
    } else {
        let n = x(cur).fadd_m(gf(0x862D04));
        if x(s).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            s
        }
    };
    wr::<f32>(this, 0x22C, s);
    play_aircraft_sound_call(this, 2, 0x13, 10, vol1.to_bits(), s.to_bits());
    play_aircraft_sound_call(this, 4, 0x13, 11, vol2.to_bits(), s.to_bits());
    let v5 = x(vol3).fadd_m(f32::from_bits(vol_off)).to_f32();
    play_aircraft_sound_call(this, 5, 0x13, 0x1A, v5.to_bits(), ONE);
    play_aircraft_sound_call(this, 7, 0x13, 0xE, e, ONE);
}

/// `CAEVehicleAudioEntity::ProcessPlayerJet(cVehicleParams&)` (0x501650): with bank 0x8A in slot 0x13.
/// Forward-flying (vehicle byte +0x424 >= 0): pedals cleared, `f = clamp01(max(accel, brake) / 255) *
/// 0.3 + 0.7`; else `GetAccelAndBrake` and `f = +0x9C4 * [0x862D18]`. Throttle `f > 1 -> 1`, `f < 0.2
/// -> 0` (direct), else `(f - 0.2) * 1.25`, then at least 0.2 (double), f32. +0x7C set: +0xB4 =
/// `[0x8CBDC4]`. Landing gear (+0x4C0) below `[0x859F80]` / `[0x862D28]` / `[0x862D24]` / `[0x862D20]`
/// picks (speed factor, volume offset) pairs from 0x8CBDD8.. and runs the afterburner timer: once now >
/// +0xB8, +0xBC toggles and +0xB8 = now + `GetRandomNumberInRange` over the range `ResolveProbability`
/// picks; otherwise speed factor 1.0 and offset 0. +0xB4 = max(+0xB4 - 1, 0); offset 0 while +0xBC is
/// clear. `ProcessGenericJet((+0x428 >> 4) & 1, params, throttle, accel / 255, brake / 255, offset,
/// factor)`.
#[hook(addr = 0x501650, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessPlayerJet", expect = "83 EC 08 56 6A 13")]
pub fn process_player_jet(this: GPtr<u8>, params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let fwd = rd::<i8>(veh(), 0x424) >= 0;
    if !fwd {
        get_accel_and_brake(this, params);
    } else {
        wr::<u16>(params, 0x20, 0);
        wr::<u16>(params, 0x22, 0);
    }
    let a = rd::<i16>(params, 0x20);
    let b = rd::<i16>(params, 0x22);
    let m = if a > b { a } else { b };
    let f = clamp01_x(X87::from_i32(m as i32).fmul_m(gf(0x859A3C)));
    let f = if fwd {
        f.fmul_m(gf(0x858C24)).fadd_m(gf(0x858CB0))
    } else {
        x(rd::<f32>(veh(), 0x9C4)).fmul_m(gf(0x862D18))
    };
    let throttle = aircraft_throttle_x(f).to_f32();
    if rd::<u16>(this, 0x7C) != 0 {
        wr::<u16>(this, 0xB4, g::<u16>(0x8CBDC4).read());
    }
    let gear = || x(rd::<f32>(veh(), 0x4C0));
    let pick = if !gear().fcom(x(gf(0x859F80))).test_parity(0x05) {
        Some((0x8CBDD8, 0x8CBDDC))
    } else if !gear().fcom(x(gf(0x862D28))).test_parity(0x05) {
        Some((0x8CBDE0, 0x8CBDE4))
    } else if !gear().fcom(x(gf(0x862D24))).test_parity(0x05) {
        Some((0x8CBDE8, 0x8CBDEC))
    } else if !gear().fcom(x(gf(0x862D20))).test_parity(0x05) {
        Some((0x8CBDF0, 0x8CBDF4))
    } else {
        None
    };
    let (factor, mut offset) = match pick {
        Some((fa, of)) => (g::<u32>(fa).read(), g::<u32>(of).read()),
        None => (ONE, 0),
    };
    if pick.is_some() {
        let now = g::<u32>(TIME_MS).read();
        if now > rd::<u32>(this, 0xB8) {
            let (prob, yes, no) = if rd::<u8>(this, 0xBC) != 0 {
                wr::<u8>(this, 0xBC, 0);
                (0x8CBDC8, (0x8CBDA4, 0x8CBDA8), (0x8CBDAC, 0x8CBDB0))
            } else {
                wr::<u8>(this, 0xBC, 1);
                (0x8CBDCC, (0x8CBDB4, 0x8CBDB8), (0x8CBDBC, 0x8CBDC0))
            };
            let r = resolve_probability(g::<u32>(prob).read());
            let base = g::<u32>(TIME_MS).read();
            let (lo, hi) = if r as u8 != 0 { yes } else { no };
            let n = ae_random_int(g::<i32>(lo).read(), g::<i32>(hi).read());
            wr::<u32>(this, 0xB8, (n as u32).wrapping_add(base));
        }
        let c = (rd::<i16>(this, 0xB4) as i32).wrapping_sub(1);
        wr::<u16>(this, 0xB4, if c < 0 { 0 } else { c as u16 });
        if rd::<u8>(this, 0xBC) == 0 {
            offset = 0;
        }
    }
    let on = (rd::<u8>(veh(), 0x428) >> 4) & 1;
    let k = gf(0x859A3C);
    let brake = X87::from_i32(rd::<i16>(params, 0x22) as i32).fmul_m(k).to_f32();
    let accel = X87::from_i32(rd::<i16>(params, 0x20) as i32).fmul_m(k).to_f32();
    process_generic_jet_call(
        this,
        on as u32,
        params,
        throttle.to_bits(),
        accel.to_bits(),
        brake.to_bits(),
        offset,
        factor,
    );
}

orig! {
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80)
    fn resolve_probability(p: u32) -> u8 = cdecl 0x4D9C80 => unported;
}

/// [`aircraft_throttle`] on an x87 value: `f > 1 -> 1`, `f < 0.2 -> 0` (direct), else `(f - 0.2) *
/// 1.25`; then at least 0.2 (double).
fn aircraft_throttle_x(f: X87) -> X87 {
    let v = if f.fcom(x(gf(FLT_1))).test_any(0x41) {
        if f.fcom(x(gf(0x858CC4))).test_parity(0x05) {
            Some(f)
        } else {
            None
        }
    } else {
        Some(x(gf(FLT_1)))
    };
    let v = match v {
        Some(v) => v.fsub_m(gf(0x858CC4)).fmul_m(gf(0x8595F0)),
        None => x(gf(FLT_0)),
    };
    if v.fcom(gd(0x862D08)).test_any(0x41) {
        gd(0x862D08)
    } else {
        v
    }
}

/// `CAEVehicleAudioEntity::ProcessAIProp(cVehicleParams&)` (0x4FDFD0): nothing without an engine bank
/// (+0xDC = -1); the dummy bank must be ready ([`dummy_engine_bank_ready`]); bank 0x8A in slot 0x13.
/// Engine off (vehicle +0x428 bit 0x10) and not flying forward (+0x424 < 0): all twelve engine sounds
/// cancelled. Else `f` as `ProcessDummyJet` (max pedal / 255 clamped, `* 0.3 + 0.7` forward, else
/// `+0x9C4 * [0x862D18]`), clamped to 0..1 (f32); `p = GetPiecewiseLinear(f, 3, 0x8CBE00)`; pitch / roll
/// as the RC plane; `s = f32(roll * 0.1 + pitch * 0.15 + 1)` smoothed by +0x22C; sound 2 (+0xE0, sfx 1):
/// `f32(AudioLog10(p) * 20 + 8)` at s; sound 1 (+0xE0, sfx 0, speed 1) by camera distance `d`: at least
/// 128 (or NaN) the same volume, at least 48 `f32(AudioLog10(p) * 20 + f32(AudioLog10(f32((d - 48) *
/// 0.0125)) * 20) + 8)`, else -100. The stack `CMatrix` the original destroys is never set up.
#[hook(addr = 0x4FDFD0, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessAIProp", expect = "6A FF 68 08 C4 83 00")]
pub fn process_ai_prop(this: GPtr<u8>, params: GPtr<u8>) {
    if rd::<u16>(this, 0xDC) == 0xFFFF || !dummy_engine_bank_ready(this) {
        return;
    }
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let fwd = rd::<i8>(veh, 0x424) >= 0;
    if rd::<u8>(veh, 0x428) & 0x10 == 0 && !fwd {
        for i in 0..12u32 {
            cancel_vehicle_engine_sound(this, i);
        }
        return;
    }
    let k255 = gf(0x859AAC);
    let a = crate::common::ftol32(x(rd::<f32>(veh, 0x49C)).fmul_m(k255)) as i16;
    let b = crate::common::ftol32(x(rd::<f32>(veh, 0x4A0)).fmul_m(k255)) as i16;
    let m = if a <= b { b } else { a };
    let f = clamp01_x(X87::from_i32(m as i32).fmul_m(gf(0x859A3C)));
    let f = if fwd {
        f.fmul_m(gf(0x858C24)).fadd_m(gf(0x858CB0))
    } else {
        x(rd::<f32>(veh, 0x9C4)).fmul_m(gf(0x862D18))
    };
    let f = clamp01_f32(f.to_f32());
    let p = get_piecewise_linear(f.to_bits(), 3, 0x8CBE00);
    let mtx: GPtr<u8> = g(rd::<u32>(g::<u8>(rd::<u32>(params, 0x10) as usize), 0x14) as usize);
    let pitch = x(rd::<f32>(mtx, 0x14))
        .fadd_m(rd::<f32>(mtx, 0x10))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(mtx, 0x18))
        .fmul(gd(0x862D10))
        .to_f32();
    let mut roll = x(rd::<f32>(mtx, 4))
        .fadd_m(rd::<f32>(mtx, 0))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(mtx, 8))
        .to_f32();
    if !x(roll).fcom(x(gf(FLT_0))).test_parity(0x05) {
        roll = x(roll).fmul(gd(0x862D10)).to_f32();
    }
    let lp = || crate::utility::audio_log10(p).fmul_m(gf(FLT_20));
    let vol1 = lp().fadd_m(gf(0x859000)).to_f32();
    let s = x(roll)
        .fmul_m(gf(0x858B1C))
        .fadd(x(pitch).fmul_m(gf(0x858FCC)))
        .fadd_m(gf(FLT_1))
        .to_f32();
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, s);
    }
    let cur = rd::<f32>(this, 0x22C);
    let s = if x(s).fcom(x(cur)).test_any(0x01) {
        let n = x(cur).fsub_m(gf(0x862D04));
        if x(s).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            s
        }
    } else {
        let n = x(cur).fadd_m(gf(0x862D04));
        if x(s).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            s
        }
    };
    wr::<f32>(this, 0x22C, s);
    let cm = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cm != 0 {
        g(cm as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let vp = veh_pos(this);
    let dd = |i: usize| x(f32::from_bits(vp[i])).fsub_m(rd::<f32>(cam, i as isize * 4));
    let (dx, dy, dz) = (dd(0), dd(1), dd(2));
    let dist = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt().to_f32();
    let vol2 = if x(dist).fcom(x(gf(0x858BF4))).test_parity(0x05) {
        lp().fadd_m(gf(0x859000)).to_f32()
    } else if x(dist).fcom(x(gf(0x862D38))).test_parity(0x05) {
        let q = x(dist).fsub_m(gf(0x862D38)).fmul_m(gf(0x85A5C4)).to_f32();
        let lq = crate::utility::audio_log10(q).fmul_m(gf(FLT_20)).to_f32();
        lp().fadd_m(lq).fadd_m(gf(0x859000)).to_f32()
    } else {
        f32::from_bits(0xC2C8_0000)
    };
    let slot = || rd::<u16>(this, 0xE0) as u32;
    play_aircraft_sound_call(this, 2, slot(), 1, vol1.to_bits(), s.to_bits());
    play_aircraft_sound_call(this, 1, slot(), 0, vol2.to_bits(), ONE);
}

orig! {
    /// `CAEAudioUtility::GetRandomNumberInRange(float, float)` (0x4D9C50), ST0
    fn ae_random_float_st0(min: u32, max: u32) -> f64 = cdecl 0x4D9C50 => unported;
}

/// The camera-to-vehicle distance volume of the AI aircraft: `d >= 128` (or NaN) -> `f32(l + 8)`,
/// `d >= 48` -> `f32(l + f32(AudioLog10(f32((d - 48) * 0.0125)) * 20) + 8)`, else -100; `l =
/// AudioLog10(p) * 20`.
fn ai_distance_volume(this: GPtr<u8>, p: f32) -> f32 {
    let cm = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cm != 0 {
        g(cm as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let vp = veh_pos(this);
    let dd = |i: usize| x(f32::from_bits(vp[i])).fsub_m(rd::<f32>(cam, i as isize * 4));
    let (dx, dy, dz) = (dd(0), dd(1), dd(2));
    let dist = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt().to_f32();
    let lp = || crate::utility::audio_log10(p).fmul_m(gf(FLT_20));
    if x(dist).fcom(x(gf(0x858BF4))).test_parity(0x05) {
        lp().fadd_m(gf(0x859000)).to_f32()
    } else if x(dist).fcom(x(gf(0x862D38))).test_parity(0x05) {
        let q = x(dist).fsub_m(gf(0x862D38)).fmul_m(gf(0x85A5C4)).to_f32();
        let lq = crate::utility::audio_log10(q).fmul_m(gf(FLT_20)).to_f32();
        lp().fadd_m(lq).fadd_m(gf(0x859000)).to_f32()
    } else {
        f32::from_bits(0xC2C8_0000)
    }
}

/// The +0x22C speed smoothing (by at most `[0x862D04]`, set while negative); returns the new speed.
fn smooth_22c(this: GPtr<u8>, s: f32) -> f32 {
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, s);
    }
    let cur = rd::<f32>(this, 0x22C);
    let s = if x(s).fcom(x(cur)).test_any(0x01) {
        let n = x(cur).fsub_m(gf(0x862D04));
        if x(s).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            s
        }
    } else {
        let n = x(cur).fadd_m(gf(0x862D04));
        if x(s).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            s
        }
    };
    wr::<f32>(this, 0x22C, s);
    s
}

/// `CAEVehicleAudioEntity::ProcessAIHeli(cVehicleParams&)` (0x4FEE20): the police heli bank (+0xDE
/// 0xC: bank 0xD in slot 0x12) and bank 0x8A in slot 0x13 must be loaded. `t = clamp01(f32(+0x84C *
/// [0x862D1C]))`; `p1 = GetPiecewiseLinear(t, 5, 0x8CBE40)`, `p2 = ...(t, 5, 0x8CBE18)`; `n` =
/// normalised (vehicle - camera); `S = f32((n . forward + 1) * 0.5)`, `W = f32((1 - ((u.y + u.x) * 0 +
/// u.z)) * 0.5)`; volumes `a = f32(AudioLog10(f32((1 - S) * p1)) * 20 + 8)`, `b = f32(AudioLog10(f32((S
/// * 0.5 + 0.5) * p1)) * 20 + 8)`, `c = f32(AudioLog10(p1) * 20 + 2)`; speed `f32((W * 0.2 + 1) * p2)`,
/// times `GetRandomNumberInRange(-0.1, 0.1) + 0.75` with the landing gear (+0x4C0) below `[0x862D28]`,
/// smoothed by +0x22C; distance volume ([`ai_distance_volume`] of p1). Underwater: all engine sounds
/// but 1 cancelled, sound 1 (0x13, sfx 3) at `f32(AudioLog10(p1) * 20 + 8)`. Police heli: sound 2
/// (0x12, sfx 1) at `that + 6` and speed, sound 1 (0x12, sfx 0) at distance volume + 6. Else sounds 2
/// (sfx 5, a), 4 (sfx 0x12, b) at the speed, 3 (sfx 0x17, c) and 1 (sfx 3, distance volume) at 1.0.
#[hook(addr = 0x4FEE20, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessAIHeli", expect = "6A FF 68 68 C4 83 00")]
pub fn process_ai_heli(this: GPtr<u8>, params: GPtr<u8>) {
    if rd::<u16>(this, 0xDE) == 0xC && is_sound_bank_loaded(g(HW), 0xD, 0x12) == 0 {
        return;
    }
    if is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0 {
        return;
    }
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let t0 = clamp01_f32(x(rd::<f32>(veh(), 0x84C)).fmul_m(gf(0x862D1C)).to_f32());
    let p1 = get_piecewise_linear(t0.to_bits(), 5, 0x8CBE40);
    let p2 = get_piecewise_linear(t0.to_bits(), 5, 0x8CBE18);
    let cm = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cm != 0 {
        g(cm as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let vp = veh_pos(this);
    let d = |i: usize| {
        x(f32::from_bits(vp[i]))
            .fsub_m(rd::<f32>(cam, i as isize * 4))
            .to_f32()
    };
    let (dz, dy, dx) = (d(2), d(1), d(0));
    let mut n = [dx, dy, dz];
    vector_normalise(GPtr::from_raw(n.as_mut_ptr().cast::<u8>()));
    let m: GPtr<u8> = g(rd::<u32>(veh(), 0x14) as usize);
    let dot = x(n[1])
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(n[2]).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(n[0]).fmul_m(rd::<f32>(m, 0x10)));
    let w = x(rd::<f32>(m, 0x24))
        .fadd_m(rd::<f32>(m, 0x20))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 0x28))
        .fsubr_m(gf(FLT_1))
        .fmul_m(gf(0x858B8C))
        .to_f32();
    let s = dot.fadd_m(gf(FLT_1)).fmul_m(gf(0x858B8C)).to_f32();
    let u = x(gf(FLT_1)).fsub_m(s).fmul_m(p1).to_f32();
    let vol_a = crate::utility::audio_log10(u)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x859000))
        .to_f32();
    let u2 = x(s).fmul_m(gf(0x858B8C)).fadd_m(gf(0x858B8C)).fmul_m(p1).to_f32();
    let vol_b = crate::utility::audio_log10(u2)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x859000))
        .to_f32();
    let vol_c = crate::utility::audio_log10(p1)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x858CA0))
        .to_f32();
    let mut sp = x(w).fmul_m(gf(0x858CC4)).fadd_m(gf(FLT_1)).fmul_m(p2).to_f32();
    if !x(rd::<f32>(veh(), 0x4C0)).fcom(x(gf(0x862D28))).test_parity(0x05) {
        let fc = gf(0x8CBDFC);
        let lo = x(fc).fmul_m(gf(0x858C1C)).to_f32();
        let r = X87::from_f64_exact(ae_random_float_st0(lo.to_bits(), fc.to_bits()));
        sp = r.fadd_m(gf(0x8CBDF8)).fmul_m(sp).to_f32();
    }
    let sp = smooth_22c(this, sp);
    let dist_vol = ai_distance_volume(this, p1);
    let l1 = || {
        crate::utility::audio_log10(p1)
            .fmul_m(gf(FLT_20))
            .fadd_m(gf(0x859000))
    };
    if !x(gf(0xC8132C)).fcom(x(gf(0x858B8C))).test_any(0x01) {
        for i in 0..12u32 {
            if i != 1 {
                cancel_vehicle_engine_sound(this, i);
            }
        }
        let v = l1().to_f32();
        play_aircraft_sound_call(this, 1, 0x13, 3, v.to_bits(), ONE);
        return;
    }
    if rd::<u16>(this, 0xDE) == 0xC {
        let v = x(gf(0x8CBDA0)).fadd(l1()).to_f32();
        play_aircraft_sound_call(this, 2, 0x12, 1, v.to_bits(), sp.to_bits());
        let v = x(gf(0x8CBDA0)).fadd_m(dist_vol).to_f32();
        play_aircraft_sound_call(this, 1, 0x12, 0, v.to_bits(), ONE);
        return;
    }
    play_aircraft_sound_call(this, 2, 0x13, 5, vol_a.to_bits(), sp.to_bits());
    play_aircraft_sound_call(this, 4, 0x13, 0x12, vol_b.to_bits(), sp.to_bits());
    play_aircraft_sound_call(this, 3, 0x13, 0x17, vol_c.to_bits(), ONE);
    play_aircraft_sound_call(this, 1, 0x13, 3, dist_vol.to_bits(), ONE);
}

/// `CAEVehicleAudioEntity::ProcessDummyHeli(cVehicleParams&)` (0x4FE940): engine bank +0xDE in slot 0x28
/// and bank 0x8A in slot 0x13 must be loaded. `t = clamp01(f32(+0x84C * [0x862D1C]))`; `a =
/// GetPiecewiseLinear(t, 5, 0x8CBE40)`, `b = ...(t, 5, 0x8CBE18)`; `S`, `W` as `ProcessAIHeli`; volumes
/// `va = f32(AudioLog10(f32((1 - 0.67 * S) * a)) * 20 + 8)`, `vb = f32(AudioLog10(f32((S * 0.5 + 0.5)
/// * a)) * 20 + 8)`, `vc = f32(AudioLog10(a) * 20 + 2)`; speed `f32((W * 0.2 + 1) * b)` (times the
/// landing-gear random factor), smoothed by +0x22C by at most 0.03 (double) while `t < 0.63`, else
/// `[0x862D04]`; sounds 2 (0x28, sfx 0, va) and 4 (0x28, sfx 1, vb) at the speed, 3 (0x28, sfx 3, vc)
/// and 1 (0x13, sfx 3, the distance volume of a) at 1.0.
#[hook(addr = 0x4FE940, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyHeli", expect = "6A FF 68 48 C4 83 00")]
pub fn process_dummy_heli(this: GPtr<u8>, params: GPtr<u8>) {
    if is_sound_bank_loaded(g(HW), rd::<u16>(this, 0xDE) as u32, 0x28) == 0
        || is_sound_bank_loaded(g(HW), 0x8A, 0x13) == 0
    {
        return;
    }
    let veh = || -> GPtr<u8> { g(rd::<u32>(params, 0x10) as usize) };
    let mut t = clamp01_f32(x(rd::<f32>(veh(), 0x84C)).fmul_m(gf(0x862D1C)).to_f32());
    let a = get_piecewise_linear(t.to_bits(), 5, 0x8CBE40);
    let b = get_piecewise_linear(t.to_bits(), 5, 0x8CBE18);
    let cm = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cm != 0 {
        g(cm as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let vp = veh_pos(this);
    let d = |i: usize| {
        x(f32::from_bits(vp[i]))
            .fsub_m(rd::<f32>(cam, i as isize * 4))
            .to_f32()
    };
    let (dz, dy, dx) = (d(2), d(1), d(0));
    let mut n = [dx, dy, dz];
    vector_normalise(GPtr::from_raw(n.as_mut_ptr().cast::<u8>()));
    let m: GPtr<u8> = g(rd::<u32>(veh(), 0x14) as usize);
    let dot = x(n[1])
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(n[2]).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(n[0]).fmul_m(rd::<f32>(m, 0x10)));
    let w = x(rd::<f32>(m, 0x24))
        .fadd_m(rd::<f32>(m, 0x20))
        .fmul_m(gf(FLT_0))
        .fadd_m(rd::<f32>(m, 0x28))
        .fsubr_m(gf(FLT_1))
        .fmul_m(gf(0x858B8C))
        .to_f32();
    let s = dot.fadd_m(gf(FLT_1)).fmul_m(gf(0x858B8C)).to_f32();
    if !x(t).fcom(x(gf(FLT_1))).test_any(0x41) {
        t = gf(FLT_1);
    }
    let v = x(gf(0x8CBD98)).fmul_m(s).fsubr_m(gf(FLT_1)).fmul_m(a).to_f32();
    let va = crate::utility::audio_log10(v)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x859000))
        .to_f32();
    let v2 = x(s).fmul_m(gf(0x858B8C)).fadd_m(gf(0x858B8C)).fmul_m(a).to_f32();
    let vb = crate::utility::audio_log10(v2)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x859000))
        .to_f32();
    let vc = crate::utility::audio_log10(a)
        .fmul_m(gf(FLT_20))
        .fadd_m(gf(0x858CA0))
        .to_f32();
    let mut sp = x(w).fmul_m(gf(0x858CC4)).fadd_m(gf(FLT_1)).fmul_m(b).to_f32();
    if !x(rd::<f32>(veh(), 0x4C0)).fcom(x(gf(0x862D28))).test_parity(0x05) {
        let fc = gf(0x8CBDFC);
        let lo = x(fc).fmul_m(gf(0x858C1C)).to_f32();
        let r = X87::from_f64_exact(ae_random_float_st0(lo.to_bits(), fc.to_bits()));
        sp = r.fadd_m(gf(0x8CBDF8)).fmul_m(sp).to_f32();
    }
    if !x(rd::<f32>(this, 0x22C)).fcom(x(gf(FLT_0))).test_parity(0x05) {
        wr::<f32>(this, 0x22C, sp);
    }
    let cur = rd::<f32>(this, 0x22C);
    let slow = !x(t).fcom(gd(0x862D48)).test_parity(0x05);
    let sp = if !x(sp).fcom(x(cur)).test_any(0x01) {
        let n = if slow {
            x(cur).fadd(gd(0x862D40))
        } else {
            x(cur).fadd_m(gf(0x862D04))
        };
        if x(sp).fcom(n).test_parity(0x05) {
            n.to_f32()
        } else {
            sp
        }
    } else {
        let n = if slow {
            x(cur).fsub(gd(0x862D40))
        } else {
            x(cur).fsub_m(gf(0x862D04))
        };
        if x(sp).fcom(n).test_any(0x41) {
            n.to_f32()
        } else {
            sp
        }
    };
    wr::<f32>(this, 0x22C, sp);
    let dv = ai_distance_volume(this, a);
    play_aircraft_sound_call(this, 2, 0x28, 0, va.to_bits(), sp.to_bits());
    play_aircraft_sound_call(this, 4, 0x28, 1, vb.to_bits(), sp.to_bits());
    play_aircraft_sound_call(this, 3, 0x28, 3, vc.to_bits(), ONE);
    play_aircraft_sound_call(this, 1, 0x13, 3, dv.to_bits(), ONE);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PLAYER_ABOUT_TO_EXIT_VEHICLE_AS_DRIVER,
    HOOK_PROCESS_RAIN_ON_VEHICLE,
    HOOK_UPDATE_BOAT_SOUND,
    HOOK_UPDATE_GENERIC_VEHICLE_SOUND,
    HOOK_PROCESS_VEHICLE_SIREN_ALARM_HORN,
    HOOK_PROCESS_AIRCRAFT,
    HOOK_PLAY_SKID_SOUND,
    HOOK_PLAY_TRAIN_BRAKE_SOUND,
    HOOK_PLAY_ROAD_NOISE_SOUND,
    HOOK_PLAY_FLAT_TYRE_SOUND,
    HOOK_PLAY_REVERSE_SOUND,
    HOOK_PROCESS_VEHICLE_FLAT_TYRE,
    HOOK_PROCESS_VEHICLE_ROAD_NOISE,
    HOOK_PROCESS_VEHICLE_SKIDDING,
    HOOK_UPDATE_PARAMETERS,
    HOOK_ADD_AUDIO_EVENT_PHYSICAL,
    HOOK_PROCESS_VEHICLE,
    HOOK_START_VEHICLE_ENGINE_SOUND,
    HOOK_GET_FREQ_FOR_PLAYER_ENGINE_SOUND,
    HOOK_PLAY_HORN_OR_SIREN,
    HOOK_PROCESS_MOVING_PARTS,
    HOOK_REQUEST_NEW_PLAYER_CAR_ENGINE_SOUND,
    HOOK_PROCESS_DUMMY_STATE_TRANSITION,
    HOOK_PLAY_AIRCRAFT_SOUND,
    HOOK_PLAY_BICYCLE_SOUND,
    HOOK_PROCESS_PLAYER_COMBINE,
    HOOK_PROCESS_DUMMY_SEA_PLANE,
    HOOK_PROCESS_DUMMY_RC_CAR,
    HOOK_PROCESS_DUMMY_JET,
    HOOK_PROCESS_DUMMY_GOLF_CART,
    HOOK_PROCESS_SPECIAL_VEHICLE,
    HOOK_PROCESS_TRAIN_TRACK_SOUND,
    HOOK_PROCESS_PLAYER_TRAIN_BRAKES,
    HOOK_PROCESS_DUMMY_TRAIN_ENGINE,
    HOOK_PROCESS_DUMMY_BICYCLE,
    HOOK_PROCESS_PLAYER_BICYCLE,
    HOOK_PROCESS_DUMMY_RC_PLANE,
    HOOK_PROCESS_DUMMY_RC_HELI,
    HOOK_PROCESS_BOAT_ENGINE,
    HOOK_PROCESS_DUMMY_HOVERCRAFT,
    HOOK_PROCESS_GENERIC_JET,
    HOOK_PROCESS_PLAYER_JET,
    HOOK_PROCESS_AI_PROP,
    HOOK_PROCESS_AI_HELI,
    HOOK_PROCESS_DUMMY_HELI,
];
