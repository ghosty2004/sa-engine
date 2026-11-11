//! PORT-AUDIO-3 hand ports, lead: fyl2x users outside the vehicle and weather entities, hardware / sound-manager / track managers / loaders, frontend and script entities (see `docs/arch/audio-port.md`, PORT-AUDIO-3).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids (the exe wins).

use sa_core::abi::{Ax, HookReg};
#[allow(unused_imports)]
use sa_core::{hook, orig};

#[allow(unused_imports)]
use crate::common::{GPtr, X87, g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
}

// ---- AX-only results (sa_core::abi::Ax: the oracles compare AX; every caller reads AX only) ----------
// Callers checked with objdump: GetNumAvailableChannels 0x5B96AE (`cmp ax` / `movzx eax, ax`); GetVoice
// 0x5B75AB / 0x5B75BD (`mov [esi+0x3E], ax` / `mov [esi+0x40], ax`); CanWePlayGlobalSpeechContext 0x4E623A
// (`test ax, ax`; the dword spill at [esp+0x30] is read back as a word) and 0x4E67C0 (`mov ebp, eax`, read
// as BP, EBP restored); GetVehicleTypeForAudio 0x6497AA / 0x6497FD / 0x64982E / 0x6A8227 (`test/cmp ax`,
// `movsx eax, ax`); the speech / pedless virtuals GetAllocatedVoice (slot 0x10) and GetPedType (slot 0x18)
// are called virtually only from RequestPedConversation 0x4E5162 / 0x4E516C (`mov bp, ax`, `cmp bp, ax`);
// GetVirtualChannelForPhysicalChannel has no reference in the exe.

/// `CAEAudioHardware::GetNumAvailableChannels()` (0x4D8810), see [`crate::port2_a::hw_get_num_available_channels`].
#[hook(addr = 0x4D8810, conv = thiscall, name = "CAEAudioHardware::GetNumAvailableChannels", expect = "66 8B 81 8C 00 00 00 C3")]
pub fn hw_get_num_available_channels_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_a::hw_get_num_available_channels(this))
}

/// `CAESoundManager::GetVirtualChannelForPhysicalChannel(int16)` (0x4EF630), see
/// [`crate::port2_a::sm_get_virtual_channel_for_physical_channel`].
#[hook(addr = 0x4EF630, conv = thiscall, name = "CAESoundManager::GetVirtualChannelForPhysicalChannel", expect = "66 8B 44 24 04 66 2B 41")]
pub fn sm_get_virtual_channel_for_physical_channel_ax(this: GPtr<u8>, physical: u32) -> Ax {
    Ax(crate::port2_a::sm_get_virtual_channel_for_physical_channel(
        this, physical,
    ))
}

/// `CAEPedlessSpeechAudioEntity::GetAllocatedVoice()` (0x4E6090): `or ax, 0xFFFF`.
#[hook(addr = 0x4E6090, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::GetAllocatedVoice", expect = "66 0D FF FF C3")]
pub fn pedless_get_allocated_voice_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_b::pedless_get_allocated_voice(this))
}

/// `CAEPedlessSpeechAudioEntity::GetPedType()` (0x4E60B0): `or ax, 0xFFFF`.
#[hook(addr = 0x4E60B0, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::GetPedType", expect = "66 0D FF FF C3")]
pub fn pedless_get_ped_type_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_b::pedless_get_ped_type(this))
}

/// `CAEPedSpeechAudioEntity::GetAllocatedVoice()` (0x4E4120).
#[hook(addr = 0x4E4120, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetAllocatedVoice", expect = "66 8B 81 94 00 00 00 C3")]
pub fn speech_get_allocated_voice_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_b::speech_get_allocated_voice(this))
}

/// `CAEPedSpeechAudioEntity::GetPedType()` (0x4E4130).
#[hook(addr = 0x4E4130, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetPedType", expect = "8A 81 90 00 00 00 84 C0")]
pub fn speech_get_ped_type_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_b::speech_get_ped_type(this))
}

/// `CAEPedSpeechAudioEntity::GetVoice(const char*, int16)` (0x4E3CD0, cdecl).
#[hook(addr = 0x4E3CD0, conv = cdecl, name = "CAEPedSpeechAudioEntity::GetVoice", expect = "0F BF 44 24 08 83 F8 04")]
pub fn speech_get_voice_ax(name: GPtr<u8>, ty: u32) -> Ax {
    Ax(crate::port2_b::speech_get_voice(name, ty))
}

/// `CAEPedSpeechAudioEntity::CanWePlayGlobalSpeechContext(int16)` (0x4E5F10).
#[hook(addr = 0x4E5F10, conv = thiscall, name = "CAEPedSpeechAudioEntity::CanWePlayGlobalSpeechContext", expect = "A0 FD 13 B6 00 55 8B 6C")]
pub fn speech_can_we_play_global_speech_context_ax(this: GPtr<u8>, ctx: u32) -> Ax {
    Ax(crate::port2_b::speech_can_we_play_global_speech_context(
        this, ctx,
    ))
}

/// `CAEVehicleAudioEntity::GetVehicleTypeForAudio()` (0x4F4F00).
#[hook(addr = 0x4F4F00, conv = thiscall, name = "CAEVehicleAudioEntity::GetVehicleTypeForAudio", expect = "8A 81 A4 00 00 00 84 C0")]
pub fn vehicle_get_vehicle_type_for_audio_ax(this: GPtr<u8>) -> Ax {
    Ax(crate::port2_b::vehicle_get_vehicle_type_for_audio(this))
}

// ---- CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume (fyl2x at 0x4D6ED0) ---------------------------

orig! {
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, AL)
    fn cull_player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `CCullZones::CamNoRain()` (0x72DDB0, AL)
    fn cull_cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10, AL)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CStats::FindMostFavoriteRadioStation()` (0x558FA0, AL)
    fn stats_find_most_favorite_radio_station() -> u8 = cdecl 0x558FA0 => unported;
    /// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, const CVector* pos)` (0x4D80B0)
    fn env_position_relative_to_camera(out: GPtr<u8>, pos: GPtr<u8>) = cdecl 0x4D80B0 => unported;
    /// `CAEAudioEnvironment::GetDistanceAttenuation(float)` (0x4D7F20), ST0
    fn env_distance_attenuation_st0(dist: f32) -> f64 = cdecl 0x4D7F20 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0
    fn vector_magnitude_st0(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CAEAmbienceTrackManager::StopAmbienceTrack()` (0x4D6CC0)
    fn amb_stop_ambience_track(this: GPtr<u8>) = thiscall 0x4D6CC0 => crate::radio::stop_ambience_track;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0)
    fn engine_report_frontend_audio_event(e: GPtr<u8>, event: u32, volume: u32, speed: u32) = thiscall 0x506EA0 => unported;
    /// `CAudioEngine::StopRadio(tVehicleAudioSettings*, bool)` (0x506F70)
    fn engine_stop_radio(e: GPtr<u8>, settings: u32, during_pause: u32) = thiscall 0x506F70 => crate::engine::stop_radio;
    /// `CAudioEngine::IsRadioOn()` (0x506FD0, AL)
    fn engine_is_radio_on(e: GPtr<u8>) -> u8 = thiscall 0x506FD0 => unported;
    /// `CAudioEngine::GetCurrentRadioStationID()` (0x507040): AL is the station; the whole EAX is what a
    /// caller pushing it passes on
    fn engine_get_current_radio_station_id(e: GPtr<u8>) -> u32 = thiscall 0x507040 => unported;
    /// `CAudioEngine::IsVehicleRadioActive()` (0x507050, AL)
    fn engine_is_vehicle_radio_active(e: GPtr<u8>) -> u8 = thiscall 0x507050 => unported;
    /// `CAudioEngine::StartRadio(int8 station, int8 bass)` (0x507DC0, `ret 8`)
    fn engine_start_radio(e: GPtr<u8>, station: u32, bass: u32) = thiscall 0x507DC0 => unported;
}

/// `CWeather::Sandstorm`
const WEATHER_SANDSTORM: usize = 0xC812F4;
/// `CWeather::UnderWaterness`
const WEATHER_UNDER_WATERNESS: usize = 0xC8132C;
/// `CGame::currArea`
const GAME_CURR_AREA: usize = 0xB72914;

/// `TheCamera`'s position z as `CPlaceable::GetPosition()` inlines it: `m_matrix ? m_matrix->pos : m_placement.pos`.
fn camera_pos_z() -> f32 {
    let m = g::<u32>(0xB6F03C).read();
    let p = if m != 0 { m.wrapping_add(0x30) } else { 0xB6F02C };
    g::<f32>(p.wrapping_add(8) as usize).read()
}

/// The ambience distance volume of zones 5 / 10 / 13 (0x4D6FA7..): `GetDistanceAttenuation(f32(|pos - camera| *
/// 0.2f)) - 6.0f`.
fn amb_zone_distance_volume(pos: [u32; 3]) -> f32 {
    let mut pos = pos;
    let mut rel = [0u32; 3];
    let relp: GPtr<u8> = GPtr::from_raw(rel.as_mut_ptr()).cast();
    env_position_relative_to_camera(relp, GPtr::from_raw(pos.as_mut_ptr()).cast());
    let m = X87::from_f64_exact(vector_magnitude_st0(relp))
        .fmul_m(0.2)
        .to_f32();
    X87::from_f64_exact(env_distance_attenuation_st0(m))
        .fsub_m(6.0)
        .to_f32()
}

/// Steps the f32 field `off` toward `target` by `delta` (0x4D73DB / 0x4D742A): up when `target > cur` (stores
/// `cur + delta` only when strictly below `target`, else `target`), down when `target < cur` (`cur - delta` only
/// when strictly above), no store when equal or unordered.
fn amb_step_toward(this: GPtr<u8>, off: isize, target: f32, delta: f32) {
    use sa_math::x87::Cmp;
    let cur = rd::<f32>(this, off);
    let v = match x(target).fcom(x(cur)) {
        Cmp::Greater => {
            let v = x(cur).fadd_m(delta);
            if v.fcom(x(target)) == Cmp::Less {
                v
            } else {
                x(target)
            }
        }
        Cmp::Less => {
            let v = x(cur).fsub_m(delta);
            if v.fcom(x(target)) == Cmp::Greater {
                v
            } else {
                x(target)
            }
        }
        _ => return,
    };
    wr::<f32>(this, off, v.to_f32());
}

/// The fade-out step (0x4D74DB / 0x4D73B7): volume (+0x18) above -40 dB: `-= 0.4f`, else `StopAmbienceTrack`.
fn amb_fade_out(this: GPtr<u8>) {
    let cur = rd::<f32>(this, 0x18);
    if x(cur).fcom(x(-40.0)) == sa_math::x87::Cmp::Greater {
        wr::<f32>(this, 0x18, x(cur).fsub_m(0.4).to_f32());
    } else {
        amb_stop_ambience_track(this);
    }
}

/// `CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume()` (0x4D6E60), from the bytes (gta-reversed's unhooked
/// C++ differs in nine places, `docs/re/specs/AUDIO/CAEAmbienceTrackManager.md`):
/// 1. `+0 = +2 = 0`, `+3 = -1` (the previous `+3` kept); a mission track (`+0xC >= 0`) is the track (`+0 = +2 = 1`);
/// 2. otherwise `Sandstorm > 0`: outdoors (`currArea == 0`, not `PlayerNoRain`, not `CamNoRain`) track 167 at
///    `lg2 * log2(Sandstorm) * 20 - 6` dB (`fldlg2; fyl2x`: the 64-bit FYL2X result multiplied directly);
/// 3. without a track: the first active audio sphere (or box) zone picks a track / volume / radio station
///    through the two-level jump table 0x4D7680 / 0x4D75DC. Box zones 5 / 10 / 13 read the sphere position
///    local, which the box path never writes: the port reads the original's stack residue there (`peek`);
/// 4. radio: start / stop / retune the radio for a zone station; with the radio on, nothing else happens;
/// 5. a track: start (-34 dB, the mission track -2 dB), ramp the volume up by 1 dB (to the target), or fade out
///    by 0.4 dB and stop; no track: the underwater / water-distance track 174 with its volume and frequency
///    ramps, the LA riots track 166, or the fade-out / stop with the radio restarted.
#[hook(addr = 0x4D6E60, conv = thiscall, name = "CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume", expect = "83 EC 24 53 55 56 8B F1", peek = 0x18)]
pub fn amb_update_ambience_track_and_volume(this: GPtr<u8>) {
    use sa_math::x87::Cmp;
    let prev_station = rd::<i8>(this, 3);
    let mut is_sandstorm = false;
    let mut volume: f32 = -6.0;
    let mut track: i32 = -1;
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 2, 0);
    wr::<u8>(this, 3, 0xFF);
    let mission = rd::<i32>(this, 0xC);
    'pick: {
        if mission >= 0 {
            track = mission;
            wr::<u8>(this, 2, 1);
            wr::<u8>(this, 0, 1);
            break 'pick;
        }
        let sand = g::<f32>(WEATHER_SANDSTORM).read();
        if x(sand).fcom(X87::zero()) == Cmp::Greater {
            if g::<i32>(GAME_CURR_AREA).read() == 0 && cull_player_no_rain() == 0 && cull_cam_no_rain() == 0 {
                track = 0xA7;
                volume = X87::lg2().fyl2x(x(sand)).fmul_m(20.0).fsub_m(6.0).to_f32();
            }
            is_sandstorm = true;
            if track != -1 {
                break 'pick;
            }
        }
        // the first active sphere / box zone (0x4D6EFC)
        let (zone, pos): (i16, [u32; 3]) = if g::<i32>(0xB6DCC0).read() > 0 {
            let e = g::<i32>(0xB6DC94).read().wrapping_mul(0x1C) as u32;
            let p = e.wrapping_add(0xB6EBB4) as usize;
            (
                g::<i16>(e.wrapping_add(0xB6EBB0) as usize).read(),
                [g::<u32>(p).read(), g::<u32>(p + 4).read(), g::<u32>(p + 8).read()],
            )
        } else if g::<i32>(0xB6DCBC).read() > 0 {
            let e = g::<i32>(0xB6DC6C).read().wrapping_mul(0x18) as u32;
            // the sphere-position local [esp+0x1C] = entry ESP - 0x18, never written on this path
            let r = |off: i32| sa_core::abi::peek_stack(0x4D6E60, off).unwrap_or(0);
            (
                g::<i16>(e.wrapping_add(0xB6DCD8) as usize).read(),
                [r(-0x18), r(-0x14), r(-0x10)],
            )
        } else {
            break 'pick;
        };
        if zone < 0 || (i32::from(zone) - 4) as u32 > 0x3F {
            break 'pick;
        }
        track = match zone {
            4 => {
                let z = camera_pos_z();
                if x(z).fcom(x(1372.0)) == Cmp::Greater {
                    volume = -6.0;
                } else {
                    volume = x(1372.0).fsub_m(z).fmul_m(9.0).fsubr_m(-6.0).to_f32();
                }
                0x8F
            }
            5 => {
                volume = amb_zone_distance_volume(pos);
                0x8C
            }
            10 => {
                volume = amb_zone_distance_volume(pos);
                0x8B
            }
            13 => {
                volume = amb_zone_distance_volume(pos);
                wr::<u8>(this, 0, 1);
                0x9D
            }
            8 => 0xA5,
            12 => 0xA8,
            15 => 0xA4,
            17 => 0x92,
            19 => 0x8A,
            20 => 0x88,
            21 => 0x87,
            23 => 0x94,
            24 => 0x9F,
            25 => 0x9E,
            26 => 0x9A,
            28 | 29 => 0x93,
            34 => 0xA2,
            36 => 0x9B,
            37 => {
                wr::<u8>(this, 2, 1);
                0x90
            }
            39 => 0xA3,
            41 => {
                wr::<u8>(this, 0, 1);
                0xA9
            }
            44 => 0x98,
            48 => 0x89,
            50 => 0xAD,
            51 => 0x9C,
            64 => 0x97,
            66 => 0xAA,
            67 => 0xAB,
            30 => {
                wr::<u8>(this, 3, stats_find_most_favorite_radio_station());
                -1
            }
            52..=62 => {
                wr::<u8>(this, 3, (zone - 51) as u8);
                -1
            }
            _ => -1,
        };
    }
    // radio (0x4D718C)
    let eng = g::<u8>(crate::common::AUDIO_ENGINE);
    let st = rd::<i8>(this, 3);
    if st >= 0 {
        if engine_is_radio_on(eng) == 0 {
            engine_start_radio(eng, u32::from(st as u8), 2);
        } else if engine_get_current_radio_station_id(eng) as u8 != st as u8 {
            engine_stop_radio(eng, 0, 0);
        }
        return;
    }
    if prev_station >= 0 {
        if engine_is_radio_on(eng) != 0 {
            engine_stop_radio(eng, 0, 0);
        }
        return;
    }
    if engine_is_radio_on(eng) != 0 {
        if rd::<u8>(this, 0) != 0 {
            engine_stop_radio(eng, 0, 0);
            engine_report_frontend_audio_event(eng, 0x23, 0, 0x3F80_0000);
        }
        wr::<u8>(this, 1, rd::<u8>(this, 0));
        return;
    }
    if track >= 0 {
        // a track (0x4D7250)
        let status = rd::<i32>(this, 8);
        if track == rd::<i32>(this, 0xC) {
            if status == 8 {
                wr::<i32>(this, 0x20, track);
                wr::<i32>(this, 8, 0);
                wr::<f32>(this, 0x18, -2.0);
                wr::<f32>(this, 0x1C, 1.0);
            } else if rd::<i32>(this, 0x20) != track {
                amb_stop_ambience_track(this);
            }
            return;
        }
        if status == 8 {
            wr::<i32>(this, 0x20, track);
            wr::<i32>(this, 8, 0);
            wr::<f32>(this, 0x18, -34.0);
            wr::<f32>(this, 0x1C, 1.0);
            return;
        }
        if rd::<i32>(this, 0x20) == track {
            let v = x(rd::<f32>(this, 0x18)).fadd_m(1.0);
            let v = if v.fcom(x(volume)) == Cmp::Less {
                v
            } else {
                x(volume)
            };
            wr::<f32>(this, 0x18, v.to_f32());
        } else {
            amb_fade_out(this);
        }
        return;
    }
    // no track: water / riots (0x4D72E0)
    let freq_target: f32;
    let water = if x(g::<f32>(WEATHER_UNDER_WATERNESS).read()).fcom(x(0.5)).c0() {
        let t = x(camera_pos_z()).fsub_m(g::<f32>(0xB6F110).read());
        let d = g::<f32>(0xB6F10C).read();
        let r = x(d)
            .fmul_m(d)
            .fadd(t.fmul(t))
            .fsqrt()
            .fmul_m(f32::from_bits(0x3F12_4925))
            .to_f32();
        volume = X87::from_f64_exact(env_distance_attenuation_st0(r))
            .fsub_m(12.0)
            .to_f32();
        freq_target = 1.0;
        x(volume).fcom(x(-34.0)) == Cmp::Greater
    } else {
        volume = 14.0;
        freq_target = 0.0625;
        true
    };
    let status = rd::<i32>(this, 8);
    if water {
        // 0x4D7370
        if status == 8 {
            wr::<i32>(this, 0x20, 0xAE);
            wr::<i32>(this, 8, 0);
            wr::<f32>(this, 0x18, -34.0);
            let shallow = x(g::<f32>(WEATHER_UNDER_WATERNESS).read()).fcom(x(0.5)).c0();
            wr::<f32>(this, 0x1C, if shallow { 1.0 } else { 0.0625 });
        } else if rd::<i32>(this, 0x20) != 0xAE {
            amb_fade_out(this);
        } else {
            amb_step_toward(this, 0x18, volume, 1.0);
        }
        amb_step_toward(this, 0x1C, freq_target, 0.25);
        return;
    }
    // 0x4D7494
    if la_riots_active_here() != 0 && g::<i32>(GAME_CURR_AREA).read() == 0 {
        if rd::<i32>(this, 8) == 8 {
            wr::<i32>(this, 8, 0);
            wr::<i32>(this, 0x20, 0xA6);
            wr::<f32>(this, 0x18, -34.0);
            wr::<f32>(this, 0x1C, 1.0);
        } else if rd::<i32>(this, 0x20) == 0xA6 {
            let v = x(rd::<f32>(this, 0x18)).fadd_m(1.0);
            let v = if v.fcom(x(-12.0)) == Cmp::Less {
                v
            } else {
                x(-12.0)
            };
            wr::<f32>(this, 0x18, v.to_f32());
        } else {
            amb_fade_out(this);
        }
        return;
    }
    if rd::<i32>(this, 8) == 8 {
        return;
    }
    if rd::<i32>(this, 0x20) == 0xAE {
        if x(volume).fcom(x(-40.0)) == Cmp::Less {
            amb_stop_ambience_track(this);
        }
        wr::<f32>(this, 0x18, -100.0);
        wr::<f32>(this, 0x1C, 1.0);
        return;
    }
    if is_sandstorm {
        let cur = rd::<f32>(this, 0x18);
        if x(cur).fcom(x(-40.0)) == Cmp::Greater {
            wr::<f32>(this, 0x1C, 1.0);
            wr::<f32>(this, 0x18, x(cur).fsub_m(0.4).to_f32());
            return;
        }
    }
    amb_stop_ambience_track(this);
    if rd::<u8>(this, 1) != 0 || engine_is_vehicle_radio_active(eng) != 0 {
        let station = engine_get_current_radio_station_id(eng);
        engine_start_radio(eng, station, 0);
    }
    wr::<f32>(this, 0x1C, 1.0);
}

// ---- CAEAudioHardware::RescaleChannelVolumes (AudioLog10 + _CIpow) --------------------------------------

orig! {
    /// `CAEAudioChannel` virtual slot 0xC (`GetPlayTime()`: the play position in ms, -1 when not playing)
    fn channel_get_play_time(ch: GPtr<u8>) -> i32 = thiscall vslot 0xC => unported;
    /// `CAEAudioChannel` virtual slot 0x10 (`GetLength()`: AX, sign-extended by the caller)
    fn channel_get_length(ch: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CAEAudioChannel` virtual slot 0x20 (`SetFrequencyScalingFactor(float)`)
    fn channel_set_frequency_scaling_factor(ch: GPtr<u8>, f: u32) = thiscall vslot 0x20 => unported;
    /// `CAEAudioChannel::SetVolume(float)` (0x4D7C60)
    fn channel_set_volume(ch: GPtr<u8>, v: u32) = thiscall 0x4D7C60 => unported;
}

/// CRT `_CIpow` (0x822130): `st1 ^ st0` (computed by the CRT at 53 bits; the double result is exact in an f64).
fn ci_pow(base: f64, exp: X87) -> X87 {
    // phase A: the CRT intrinsic; phase B: libm's pow (crate::port4_hw::ci_pow)
    crate::port4_hw::ci_pow(base, exp)
}

/// `CAEAudioHardware::RescaleChannelVolumes()` (0x4D8990), from the bytes. Per channel slot `i < +0x8E`
/// (flags word `+6 + 2i`, dB volume `+0x110 + 4i`, linear volume `+0x210 + 4i`, frequency factor `+0x310 + 4i`,
/// channel `+0xE64 + 4i`):
/// 1. the loudest playing volume overall (`maxA`, its flag bit 7 -> `flagA`) and among flag-1 slots (`maxB`,
///    `flagB`), both from 0.0; a slot without a playing channel gets -100 dB;
/// 2. each maximum below last frame's (+0x42C / +0x428) decays from it by 0.5 dB (previous flag set: the flag
///    becomes 1) or 1.2 dB when that is louder;
/// 3. volumes relative to the maximum (flag 4: `min(vol, 0)`; flag 1: `- maxB`; else `- maxA`), linear
///    `10 ^ (vol * 0.05)` (CRT `_CIpow`); flag-2 linear volumes summed (`sum2`, from the unrounded power), the
///    others summed (`sum1`; flag 0x40 with a playing channel: scaled by the remaining play time
///    `(len - pos) / len`, clamped to 0..linear);
/// 4. the maxima and flags stored; `scale = (6.4 - sum2 * 0.8) * 16383 / sum1` (0 when `sum1 == 0`, at most
///    16383);
/// 5. linear volumes: flag 2 `* 16383`, else `* scale`; `* (+0x418 * +0x410)` (flag 0x10) or
///    `* (+0x41C * +0x414)`; `* +0x420` (flag 0x20) or `* +0x424`; each channel's volume set to
///    `AudioLog10(lin * 6.1038882e-5) * 20` dB (-100 dB for 0) and its frequency factor (+0x310) passed on.
#[hook(addr = 0x4D8990, conv = thiscall, name = "CAEAudioHardware::RescaleChannelVolumes", expect = "83 EC 24 53 55 56 57 8B")]
pub fn hw_rescale_channel_volumes(this: GPtr<u8>) {
    use sa_math::x87::Cmp;
    let n = || i32::from(rd::<u16>(this, 0x8E));
    let ch_at = |i: i32| rd::<GPtr<u8>>(this, 0xE64 + 4 * i as isize);
    let vol_off = |i: i32| 0x110 + 4 * i as isize;
    let lin_off = |i: i32| 0x210 + 4 * i as isize;
    let flags = |i: i32| rd::<u16>(this, 6 + 2 * i as isize);
    let (mut max_a, mut max_b) = (0.0f32, 0.0f32);
    let (mut flag_a, mut flag_b) = (0u8, 0u8);
    let (mut sum1, mut sum2) = (0.0f32, 0.0f32);
    let mut i = 0;
    while i < n() {
        let ch = ch_at(i);
        if !ch.is_null() && channel_get_play_time(ch) != -1 {
            let v = rd::<f32>(this, vol_off(i));
            if x(v).fcom(x(max_a)) == Cmp::Greater {
                max_a = v;
                flag_a = ((flags(i) as u8) >> 7) & 1;
            }
            let f = flags(i);
            let v = rd::<f32>(this, vol_off(i));
            if f & 1 != 0 && x(v).fcom(x(max_b)) == Cmp::Greater {
                max_b = v;
                flag_b = ((f >> 7) & 1) as u8;
            }
        } else {
            wr::<f32>(this, vol_off(i), -100.0);
        }
        i += 1;
    }
    // decay of the maxima (0x4D8A53)
    let decay = |max: &mut f32, flag: &mut u8, prev_off: isize, prev_flag_off: isize| {
        let prev = rd::<f32>(this, prev_off);
        if x(*max).fcom(x(prev)) != Cmp::Less {
            return;
        }
        let soft = rd::<u8>(this, prev_flag_off) != 0;
        let v = x(prev).fsub_m(if soft { 0.5 } else { f32::from_bits(0x3F99_999A) });
        if v.fcom(x(*max)) == Cmp::Greater {
            *max = v.to_f32();
            if soft {
                *flag = 1;
            }
        }
    };
    decay(&mut max_a, &mut flag_a, 0x42C, 3);
    decay(&mut max_b, &mut flag_b, 0x428, 2);
    // relative and linear volumes (0x4D8AF9)
    let mut i = 0;
    while i < n() {
        let f = flags(i);
        let v = rd::<f32>(this, vol_off(i));
        let rel = if f & 4 != 0 {
            if x(v).fcom(X87::zero()) == Cmp::Less {
                x(v)
            } else {
                X87::zero()
            }
        } else if f & 1 != 0 {
            x(v).fsub_m(max_b)
        } else {
            x(v).fsub_m(max_a)
        };
        let rel = rel.to_f32();
        wr::<f32>(this, vol_off(i), rel);
        let p = ci_pow(10.0, x(rel).fmul_m(0.05));
        let lin = p.to_f32();
        wr::<f32>(this, lin_off(i), lin);
        if f & 2 != 0 {
            sum2 = p.fadd_m(sum2).to_f32();
        } else {
            let ch = ch_at(i);
            let add = if f & 0x40 != 0 && !ch.is_null() && channel_get_play_time(ch) >= 0 {
                let len = i32::from(channel_get_length(ch) as u16 as i16);
                let left = len.wrapping_sub(channel_get_play_time(ch));
                let r = X87::from_i32(left).fmul_m(lin).fdiv(X87::from_i32(len));
                if r.fcom(X87::zero()) == Cmp::Less {
                    X87::zero()
                } else if r.fcom(x(lin)) == Cmp::Greater {
                    x(lin)
                } else {
                    r
                }
                .fadd_m(sum1)
            } else {
                x(sum1).fadd_m(lin)
            };
            sum1 = add.to_f32();
        }
        i += 1;
    }
    wr::<f32>(this, 0x42C, max_a);
    wr::<f32>(this, 0x428, max_b);
    wr::<u8>(this, 2, flag_b);
    wr::<u8>(this, 3, flag_a);
    const FULL: f32 = 16383.0;
    let scale = if x(sum1).fcom(X87::zero()) == Cmp::Equal {
        0.0
    } else {
        // `fst` stores the f32; the compare uses ST0
        let s = x(sum2)
            .fmul_m(0.8)
            .fsubr_m(f32::from_bits(0x40CC_CCCD))
            .fmul_m(FULL)
            .fdiv_m(sum1);
        if s.fcom(x(FULL)) == Cmp::Greater {
            FULL
        } else {
            s.to_f32()
        }
    };
    // per-channel volume (0x4D8CAC)
    let mut i = 0;
    while i < n() {
        let f = flags(i);
        let lo = lin_off(i);
        let lin = rd::<f32>(this, lo);
        let v = if f & 2 != 0 {
            x(lin).fmul_m(FULL)
        } else {
            x(scale).fmul_m(lin)
        };
        wr::<f32>(this, lo, v.to_f32());
        let m = if f & 0x10 != 0 {
            x(rd::<f32>(this, 0x418)).fmul_m(rd::<f32>(this, 0x410))
        } else {
            x(rd::<f32>(this, 0x41C)).fmul_m(rd::<f32>(this, 0x414))
        };
        wr::<f32>(this, lo, m.fmul_m(rd::<f32>(this, lo)).to_f32());
        let m = x(rd::<f32>(this, if f & 0x20 != 0 { 0x420 } else { 0x424 }));
        wr::<f32>(this, lo, m.fmul_m(rd::<f32>(this, lo)).to_f32());
        let ch = ch_at(i);
        if !ch.is_null() {
            let lin = rd::<f32>(this, lo);
            if x(lin).fcom(X87::zero()) == Cmp::Equal {
                channel_set_volume(ch, 0xC2C8_0000);
            } else {
                let arg = x(lin).fmul_m(f32::from_bits(0x3880_0200)).to_f32();
                let db = crate::utility::audio_log10(arg).fmul_m(20.0).to_f32();
                channel_set_volume(ch, db.to_bits());
            }
            channel_set_frequency_scaling_factor(ch, rd::<u32>(this, lo + 0x100));
        }
        i += 1;
    }
}

// ---- CAERadioTrackManager::UpdateRadioVolumes (fyl2x at 0x4EA1CB / 0x4EA206) ----------------------------

orig! {
    /// `CTimer::GetIsSlowMotionActive()` (0x561AD0, AL)
    fn timer_get_is_slow_motion_active() -> u8 = cdecl 0x561AD0 => unported;
    /// `CAEAudioHardware::GetEffectsMasterScalingFactor()` (0x4D9570), ST0 (`fld [ecx+0x414]`)
    fn hw_get_effects_master_scaling_factor_st0(hw: GPtr<u8>) -> f64 = thiscall 0x4D9570 => unported;
    /// `CAEAudioHardware::GetMusicMasterScalingFactor()` (0x4D9580), ST0 (`fld [ecx+0x410]`)
    fn hw_get_music_master_scaling_factor_st0(hw: GPtr<u8>) -> f64 = thiscall 0x4D9580 => unported;
    /// `CAudioEngine::IsMissionAudioSampleFinished(uint8 slot)` (0x5072C0, AL)
    fn engine_is_mission_audio_sample_finished(e: GPtr<u8>, slot: u32) -> u8 = thiscall 0x5072C0 => unported;
    /// `CAudioEngine::GetMissionAudioEvent(uint8 slot)` (0x5072D0): the caller compares the whole EAX with 0xFFFF
    fn engine_get_mission_audio_event(e: GPtr<u8>, slot: u32) -> u32 = thiscall 0x5072D0 => unported;
    /// `CAudioEngine::GetMissionAudioPosition(uint8 slot)` (0x5072E0)
    fn engine_get_mission_audio_position(e: GPtr<u8>, slot: u32) -> GPtr<u8> = thiscall 0x5072E0 => unported;
    /// `CAudioEngine::IsAmbienceRadioActive()` (0x507280, AL)
    fn engine_is_ambience_radio_active(e: GPtr<u8>) -> u8 = thiscall 0x507280 => unported;
    /// `CAEAudioHardware::SetChannelVolume(int16 channel, uint16 slot, float volume, uint8 unused)` (0x4D8870)
    fn hw_set_channel_volume(hw: GPtr<u8>, channel: u32, slot: u32, volume: u32, unused: u32) = thiscall 0x4D8870 => unported;
}

/// The radio duck volume `min(lg2 * log2(effects / music) * 20 - 9, 0)` (0x4EA1AB..): `effects` is stored as an
/// f32 first; when the value is negative both factors are read again and it is recomputed (the bytes call the
/// getters twice), the 64-bit FYL2X result feeding `* 20` directly.
fn radio_duck_volume() -> X87 {
    use sa_math::x87::Cmp;
    let hw = g::<u8>(crate::common::AE_AUDIO_HARDWARE);
    let calc = || {
        let eff = X87::from_f64_exact(hw_get_effects_master_scaling_factor_st0(hw)).to_f32();
        let q = X87::from_f64_exact(hw_get_music_master_scaling_factor_st0(hw)).fdivr_m(eff);
        X87::lg2().fyl2x(q).fmul_m(20.0).fsub_m(9.0)
    };
    if calc().fcom(X87::zero()) == Cmp::Less {
        calc()
    } else {
        X87::zero()
    }
}

/// `CAERadioTrackManager::UpdateRadioVolumes()` (0x4EA010), from the bytes: volume -4 dB; unless paused (user
/// or code pause) with `+3` (enabled in pause mode): slow motion -100 dB, widescreen -16 dB, else with both
/// master factors above 0: when `CAEPedSpeechAudioEntity::s_bForceAudible` (0xB613E4) is set, or a mission
/// audio slot (0, 1) is playing an event (`!= 0xFFFF`) without a position or within 15 m of the camera, the
/// duck volume is recomputed (`+0x80 = duck`, `+0x84 = -0.02 * duck`, volume = duck - 4); otherwise a negative
/// `+0x80` fades back (`+0x80 = min(+0x80 + +0x84, 0)`, volume = that - 4). Then the ambience radio -20 dB; with
/// the bass enhancer (+4) bass setting (+0xEA) 1 -2 dB, 2 +1.5 dB; `SetChannelVolume(+0x64, 0, volume, 0)`.
/// gta-reversed differs: it skips the duck computation when force-audible is set (the bytes go straight to it).
#[hook(addr = 0x4EA010, conv = thiscall, name = "CAERadioTrackManager::UpdateRadioVolumes", expect = "83 EC 10 A0 49 CB B7 00")]
pub fn radio_update_radio_volumes(this: GPtr<u8>) {
    use sa_math::x87::Cmp;
    let hw = g::<u8>(crate::common::AE_AUDIO_HARDWARE);
    let eng = g::<u8>(crate::common::AUDIO_ENGINE);
    let mut volume: f32 = -4.0;
    let paused = g::<u8>(0xB7CB49).read() != 0 || g::<u8>(0xB7CB48).read() != 0;
    if !paused || rd::<u8>(this, 3) == 0 {
        if timer_get_is_slow_motion_active() != 0 {
            volume = -100.0;
        } else if g::<u8>(0xB6F065).read() != 0 {
            volume = -16.0;
        } else if X87::from_f64_exact(hw_get_effects_master_scaling_factor_st0(hw)).fcom(X87::zero())
            == Cmp::Greater
            && X87::from_f64_exact(hw_get_music_master_scaling_factor_st0(hw)).fcom(X87::zero())
                == Cmp::Greater
        {
            let mut duck = g::<u8>(0xB613E4).read() != 0;
            if !duck {
                for slot in 0..2u32 {
                    if engine_is_mission_audio_sample_finished(eng, slot) != 0
                        || engine_get_mission_audio_event(eng, slot) == 0xFFFF
                    {
                        continue;
                    }
                    let pos = engine_get_mission_audio_position(eng, slot);
                    if pos.is_null() {
                        duck = true;
                        break;
                    }
                    let mut rel = [0u32; 3];
                    let relp: GPtr<u8> = GPtr::from_raw(rel.as_mut_ptr()).cast();
                    env_position_relative_to_camera(relp, pos);
                    let c = X87::from_f64_exact(vector_magnitude_st0(relp)).fcom(x(15.0));
                    if c == Cmp::Less || c == Cmp::Equal {
                        duck = true;
                        break;
                    }
                }
            }
            if duck {
                let d = radio_duck_volume();
                wr::<f32>(this, 0x80, d.to_f32());
                wr::<f32>(this, 0x84, x(-0.02).fmul(d).to_f32());
                volume = d.fsub_m(4.0).to_f32();
            } else if x(rd::<f32>(this, 0x80)).fcom(X87::zero()) == Cmp::Less {
                let v = x(rd::<f32>(this, 0x84)).fadd_m(rd::<f32>(this, 0x80));
                let v = if v.fcom(X87::zero()) == Cmp::Less {
                    v
                } else {
                    X87::zero()
                };
                wr::<f32>(this, 0x80, v.to_f32());
                volume = v.fsub_m(4.0).to_f32();
            }
        }
        if engine_is_ambience_radio_active(eng) != 0 {
            volume = x(volume).fsub_m(20.0).to_f32();
        }
    }
    if rd::<u8>(this, 4) != 0 {
        match rd::<i8>(this, 0xEA) {
            1 => volume = x(volume).fsub_m(2.0).to_f32(),
            2 => volume = x(volume).fadd_m(1.5).to_f32(),
            _ => {}
        }
    }
    hw_set_channel_volume(hw, u32::from(rd::<u16>(this, 0x64)), 0, volume.to_bits(), 0);
}

// ---- CAEAudioEnvironment::GetPositionRelativeToCamera(CVector*, const CVector*) -------------------------

orig! {
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BBC0)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

/// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, const CVector* pos)` (0x4D80B0, cdecl), from
/// the bytes: `out` = `pos` relative to a listener point in the camera frame (`TheCamera`'s camera matrix
/// 0xB6F99C copied into a local `CMatrix`): `out.x = -(right . d)`, `out.y = up . d`, `out.z = at . d`,
/// `d = pos - listener`. Listener: camera modes 7, 8, 16 (the active cam's mode word): `cam - up * [0x8AC26C]`;
/// otherwise `cam + up * s`, `s` = the player ped's distance from the camera minus [0x8AC268], clamped to
/// `0..[0x8AC264]` (no player ped: `[0x8AC268] - [0x8AC268]`). Null `pos`: nothing (the local matrix is still
/// destroyed). The bytes reuse the `pos` argument slot for `d.z` (f32); the port leaves the same value there
/// (`footprint`), the caller's slot being compared by L2. Dot products in the bytes' order (x87, 24-bit).
#[hook(addr = 0x4D80B0, conv = cdecl, name = "CAEAudioEnvironment::GetPositionRelativeToCamera-vec", expect = "6A FF 68 F8 BD 83 00", footprint = 0x1000)]
pub fn env_get_position_relative_to_camera(out: GPtr<u8>, pos: GPtr<u8>) {
    use sa_core::abi::{Fp, stack_footprint};
    use sa_math::x87::Cmp;
    let mut m = [0u32; 0x12];
    let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
    if !pos.is_null() {
        let mode = {
            let active = u32::from(g::<u8>(0xB6F081).read());
            i32::from(g::<i16>((0xB6F1A8 + active * 0x238) as usize).read())
        };
        let cam_pos = || {
            let mat = g::<u32>(0xB6F03C).read();
            g::<u8>(if mat != 0 {
                mat.wrapping_add(0x30)
            } else {
                0xB6F02C
            } as usize)
        };
        let (cx, cy, cz);
        if (7..=8).contains(&mode) || mode == 16 {
            matrix_copy_ctor(mp, g(0xB6F99C));
            let k = g::<f32>(0x8AC26C).read();
            let a = x(rd::<f32>(mp, 0x18)).fmul_m(k);
            let ty = x(rd::<f32>(mp, 0x14)).fmul_m(k).to_f32();
            let tx = x(rd::<f32>(mp, 0x10)).fmul_m(k).to_f32();
            let c = cam_pos();
            cz = a.fsubr_m(rd::<f32>(c, 8));
            cy = x(rd::<f32>(c, 4)).fsub_m(ty);
            cx = x(rd::<f32>(c, 0)).fsub_m(tx).to_f32();
        } else {
            let ped = find_player_ped(-1);
            let d0 = g::<f32>(0x8AC268).read();
            let dist = if ped.is_null() {
                x(d0)
            } else {
                let c = cam_pos();
                let pm = rd::<u32>(ped, 0x14);
                let p = if pm != 0 {
                    g::<u8>(pm.wrapping_add(0x30) as usize)
                } else {
                    ped.byte_add(4)
                };
                let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(c, 8));
                let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(c, 4));
                let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(c, 0));
                dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt()
            };
            let max = g::<f32>(0x8AC264).read();
            let v = dist.fsub_m(d0);
            let below = v.fcom(x(max)) == Cmp::Less;
            let c = if below { v } else { x(max) };
            let s = if X87::zero().fcom(c) == Cmp::Greater {
                0.0
            } else if below {
                v.to_f32()
            } else {
                max
            };
            matrix_copy_ctor(mp, g(0xB6F99C));
            let a = x(rd::<f32>(mp, 0x18)).fmul_m(s);
            let ty = x(rd::<f32>(mp, 0x14)).fmul_m(s).to_f32();
            let tx = x(rd::<f32>(mp, 0x10)).fmul_m(s).to_f32();
            let c = cam_pos();
            cz = a.fadd_m(rd::<f32>(c, 8));
            cy = x(ty).fadd_m(rd::<f32>(c, 4));
            cx = x(tx).fadd_m(rd::<f32>(c, 0)).to_f32();
        }
        let dz = x(rd::<f32>(pos, 8)).fsub(cz).to_f32();
        let dy = x(rd::<f32>(pos, 4)).fsub(cy);
        let dx = x(rd::<f32>(pos, 0)).fsub_m(cx);
        let rx = rd::<f32>(mp, 0);
        let ry = rd::<f32>(mp, 4);
        let rz = rd::<f32>(mp, 8);
        let ox = x(rz).fmul_m(dz).fadd(x(ry).fmul(dy)).fadd(dx.fmul_m(rx)).fchs();
        wr::<f32>(out, 0, ox.to_f32());
        let oy = x(rd::<f32>(mp, 0x10))
            .fmul(dx)
            .fadd(x(rd::<f32>(mp, 0x14)).fmul(dy))
            .fadd(x(rd::<f32>(mp, 0x18)).fmul_m(dz));
        wr::<f32>(out, 4, oy.to_f32());
        let oz = x(rd::<f32>(mp, 0x20))
            .fmul(dx)
            .fadd(x(rd::<f32>(mp, 0x24)).fmul(dy))
            .fadd(x(rd::<f32>(mp, 0x28)).fmul_m(dz));
        wr::<f32>(out, 8, oz.to_f32());
        stack_footprint(&[(8, Fp::Val(dz.to_bits()))]);
    }
    matrix_dtor(mp);
}

// ---- CAERadioTrackManager::QueueUpTracksForStation ----------------------------------------------------

orig! {
    /// `CAERadioTrackManager::ChooseMusicTrackIndex(eRadioID)` (0x4E94C0)
    fn radio_choose_music_track_index(this: GPtr<u8>, station: u32) -> u32 = thiscall 0x4E94C0 => unported;
    /// `CAERadioTrackManager::ChooseAdvertIndex(eRadioID)` (0x4E9570)
    fn radio_choose_advert_index(this: GPtr<u8>, station: u32) -> u32 = thiscall 0x4E9570 => crate::port2_a::radio_choose_advert_index;
    /// `CAERadioTrackManager::ChooseDJBanterIndex(eRadioID)` (0x4EA2D0)
    fn radio_choose_dj_banter_index(this: GPtr<u8>, station: u32) -> u32 = thiscall 0x4EA2D0 => unported;
    /// `CAERadioTrackManager::ChooseIdentIndex(eRadioID)` (0x4EA270): AL
    fn radio_choose_ident_index(this: GPtr<u8>, station: u32) -> u8 = thiscall 0x4EA270 => unported;
    /// `CAEUserRadioTrackManager::SelectUserTrackIndex()` (0x4F3250)
    fn user_select_user_track_index(this: GPtr<u8>) -> u32 = thiscall 0x4F3250 => crate::port2_a::user_select_user_track_index;
}

/// `CAERadioTrackManager::QueueUpTracksForStation(eRadioID station, int8* count, int8 type, tRadioSettings&)`
/// (0x4EA670, entry `jmp` to the detour 0x4020D0 that loads the type; `ret 0x10`), from the bytes. Settings:
/// track ids `+4 * n`, track types `+0x2C + n`, ident indices `+0x33 + n`; `n = *count` (signed byte), bumped
/// after each queued track. Types (signed byte; anything outside 0..5 returns 1 without queueing):
/// 0 music (`ChooseMusicTrackIndex`; -1 fails), 1 advert, 2 DJ banter (-1 fails), 3 ident + intro + outro
/// (`ChooseIdentIndex`, then `GetRandomNumberInRange` over the per-station tables 0x8C9610 / 0x8CA1B0 and the
/// fixed track 0x8C9040), 4 intro + outro (user station 12: `SelectUserTrackIndex`, type 7, its low byte as
/// the index), 5 outro. Station 12 fails for types 0, 2, 3, 5. AL = 1 queued, 0 failed.
/// Not hooked: the entry is a 5-byte `jmp` into the in-text detour 0x4020D0, which sa-hook treats as a
/// `.HOODLUM` stub (its target is not in `.HOODLUM`: table test); reached through `orig!`.
pub fn radio_queue_up_tracks_for_station(
    this: GPtr<u8>,
    station: u32,
    count: GPtr<u8>,
    ty: u32,
    s: GPtr<u8>,
) -> u8 {
    let st = i32::from(station as u8 as i8);
    let n = || isize::from(rd::<i8>(count, 0));
    let bump = || wr::<i8>(count, 0, rd::<i8>(count, 0).wrapping_add(1));
    let set = |track: u32, kind: u8| {
        wr::<u32>(s, 4 * n(), track);
        wr::<u8>(s, 0x2C + n(), kind);
    };
    let idx = || i32::from(rd::<i8>(s, 0x33 + n()));
    let copy_idx = || wr::<u8>(s, 0x33 + n(), rd::<u8>(s, 0x32 + n()));
    let rand_from = |table: usize| {
        let e = ((idx() + st * 0x1F) as u32).wrapping_mul(8) as usize;
        crate::common::ae_random_int(g::<i32>(table + e).read(), g::<i32>(table + e + 4).read()) as u32
    };
    let fixed =
        || g::<u32>((0x8C9040u32.wrapping_add(((idx() + st * 0x1F) as u32).wrapping_mul(4))) as usize).read();
    match ty as u8 as i8 {
        0 => {
            if station as u8 == 12 {
                return 0;
            }
            let t = radio_choose_music_track_index(this, station);
            wr::<u32>(s, 4 * n(), t);
            if t == u32::MAX {
                return 0;
            }
            wr::<u8>(s, 0x2C + n(), 0);
            bump();
        }
        1 => {
            set(radio_choose_advert_index(this, station), 1);
            bump();
        }
        2 => {
            if station as u8 == 12 {
                return 0;
            }
            let t = radio_choose_dj_banter_index(this, station);
            wr::<u32>(s, 4 * n(), t);
            if t == u32::MAX {
                return 0;
            }
            wr::<u8>(s, 0x2C + n(), 2);
            bump();
        }
        3 => {
            if station as u8 == 12 {
                return 0;
            }
            wr::<u8>(s, 0x33 + n(), radio_choose_ident_index(this, station));
            set(rand_from(0x8C9610), 3);
            bump();
            copy_idx();
            set(fixed(), 4);
            bump();
            copy_idx();
            set(rand_from(0x8CA1B0), 5);
            bump();
        }
        4 => {
            if station as u8 == 12 {
                let t = user_select_user_track_index(g(0xB6B970));
                set(t, 7);
                wr::<u8>(s, 0x33 + n(), rd::<u8>(s, 4 * n()));
                bump();
                return 1;
            }
            wr::<u8>(s, 0x33 + n(), radio_choose_ident_index(this, station));
            set(fixed(), 4);
            bump();
            copy_idx();
            set(rand_from(0x8CA1B0), 5);
            bump();
        }
        5 => {
            if station as u8 == 12 {
                return 0;
            }
            wr::<u8>(s, 0x33 + n(), radio_choose_ident_index(this, station));
            set(rand_from(0x8CA1B0), 5);
            bump();
        }
        _ => {}
    }
    1
}

// ---- CAERadioTrackManager::CheckForTrackConcatenation -------------------------------------------------

orig! {
    /// `CAEUserRadioTrackManager::SetUserTrackIndex(int32)` (0x4F3340)
    fn user_set_user_track_index(this: GPtr<u8>, i: u32) = thiscall 0x4F3340 => unported;
    /// `CAEAudioHardware::GetActiveTrackID()` (0x4D8F80)
    fn hw_get_active_track_id(hw: GPtr<u8>) -> u32 = thiscall 0x4D8F80 => unported;
    /// `CAERadioTrackManager::QueueUpTracksForStation` (0x4EA670), AL
    fn radio_queue_up(this: GPtr<u8>, station: u32, count: GPtr<u8>, ty: u32, s: GPtr<u8>) -> u8 = thiscall 0x4EA670 => radio_queue_up_tracks_for_station;
}

/// `CAERadioTrackManager::CheckForTrackConcatenation()` (0x4EA930), from the bytes. Settings at `+0xC4`
/// (tracks `+0xC4 + 4n`, types `+0xF0 + n`, indices `+0xF7 + n`), station `+0xE9`, bass `+0xE8`:
/// 1. user station (12) with a changed user play mode (`+0x36C` vs `GetUserTrackPlayMode`) into or out of mode 2
///    and a current track (`+0xE0 != -4`): `SetUserTrackIndex(track 0)`, a new user track as the next one
///    (type 7), the queue count 2, `PlayTrack`; the mode is remembered;
/// 2. when the hardware's active track is the queued next one (>= 0): the queue shifts by one (the old track 0
///    kept at `+0xDC` / `+0xF6` / `+0xFD`), slot 4 cleared (-1, type 6, index -1); with no next track the station's
///    tracks are queued: user station: an advert (radio mode 0, probability 0.17) and two user tracks; after
///    an ident / intro / outro (types 3..5): station 0 DJ banter, else (station byte `+8 + station` < 2 and two
///    0.5 draws) music, then an ident set; or (one 0.5 draw) music, then DJ banter or (failing) an advert;
///    otherwise an ident set (the station pushed with the upper EAX bytes 0xFFFFFF, as the bytes do); then
///    `PlayTrack(track 0, track 1, 0, bass, type 0 == 7, type 1 == 7)`.
#[hook(addr = 0x4EA930, conv = thiscall, name = "CAERadioTrackManager::CheckForTrackConcatenation", expect = "51 53 56 8B F1 80 BE E9 00 00 00 0C")]
pub fn radio_check_for_track_concatenation(this: GPtr<u8>) {
    let hw = g::<u8>(crate::common::AE_AUDIO_HARDWARE);
    let user = g::<u8>(0xB6B970);
    let mut count = [1i8; 1];
    let cp: GPtr<u8> = GPtr::from_raw(count.as_mut_ptr()).cast();
    let mut bl: i8 = 1;
    let play = |this: GPtr<u8>| {
        crate::radio::hw_play_track(
            hw,
            rd::<u32>(this, 0xC4),
            rd::<i32>(this, 0xC8),
            0,
            u32::from(rd::<u8>(this, 0xE8)),
            u32::from(rd::<u8>(this, 0xF0) == 7),
            u32::from(rd::<u8>(this, 0xF1) == 7),
        )
    };
    if rd::<u8>(this, 0xE9) == 12 {
        let prev = rd::<u8>(this, 0x36C);
        if prev != crate::radio::get_user_track_play_mode_call(user) {
            if (prev == 2 || crate::radio::get_user_track_play_mode_call(user) == 2)
                && rd::<i32>(this, 0xE0) != -4
            {
                user_set_user_track_index(user, rd::<u32>(this, 0xC4));
                wr::<u32>(this, 0xC8, user_select_user_track_index(user));
                wr::<u8>(this, 0xF1, 7);
                wr::<u8>(this, 0xF8, rd::<u8>(this, 0xC8));
                bl = 2;
                wr::<i8>(cp, 0, 2);
                play(this);
            }
            wr::<u8>(this, 0x36C, crate::radio::get_user_track_play_mode_call(user));
        }
    }
    let next = rd::<i32>(this, 0xC8);
    if hw_get_active_track_id(hw) as i32 != next || next < 0 {
        return;
    }
    wr::<u32>(this, 0xDC, rd::<u32>(this, 0xC4));
    wr::<u8>(this, 0xF6, rd::<u8>(this, 0xF0));
    wr::<u8>(this, 0xFD, rd::<u8>(this, 0xF7));
    for k in 0..4isize {
        wr::<u32>(this, 0xC4 + 4 * k, rd::<u32>(this, 0xC8 + 4 * k));
        wr::<u8>(this, 0xF0 + k, rd::<u8>(this, 0xF1 + k));
        wr::<u8>(this, 0xF7 + k, rd::<u8>(this, 0xF8 + k));
    }
    wr::<i32>(this, 0xD4, -1);
    wr::<u8>(this, 0xF4, 6);
    wr::<u8>(this, 0xFB, 0xFF);
    let s = this.byte_add(0xC4);
    if rd::<i32>(this, 0xC8) == -1 {
        let st = rd::<u8>(this, 0xE9);
        let zx = u32::from(st);
        if st == 12 {
            if g::<u8>(0xBA67F8).read() == 0
                && crate::common::ae_resolve_probability(f32::from_bits(0x3E2E_147B)) != 0
            {
                wr::<u32>(s, 4 * isize::from(bl), radio_choose_advert_index(this, 12));
                wr::<u8>(s, 0x2C + isize::from(bl), 1);
                bl += 1;
            }
            for last in [false, true] {
                let n = isize::from(bl);
                wr::<u32>(s, 4 * n, user_select_user_track_index(user));
                wr::<u8>(s, 0x2C + n, 7);
                if !last {
                    bl += 1;
                }
                wr::<u8>(s, 0x33 + n, rd::<u8>(s, 4 * n));
            }
        } else {
            let t0 = rd::<u8>(this, 0xF0);
            if !(3..=5).contains(&t0) {
                radio_queue_up(this, 0xFFFF_FF00 | zx, cp, 3, s);
            } else if st == 0 {
                radio_queue_up(this, 0, cp, 2, s);
            } else {
                let mut tail = true;
                if rd::<i8>(this, 8 + isize::from(st as i8)) < 2
                    && crate::common::ae_resolve_probability(0.5) != 0
                {
                    if crate::common::ae_resolve_probability(0.5) != 0 {
                        radio_queue_up(this, zx, cp, 0, s);
                    }
                    radio_queue_up(this, zx, cp, 3, s);
                    tail = false;
                }
                if tail {
                    if crate::common::ae_resolve_probability(0.5) != 0 {
                        radio_queue_up(this, zx, cp, 0, s);
                    }
                    if radio_queue_up(this, zx, cp, 2, s) == 0 {
                        radio_queue_up(this, zx, cp, 1, s);
                    }
                }
            }
        }
    }
    play(this);
}

// ---- CAERadioTrackManager::Save / Load ----------------------------------------------------------------

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data_to_work_buffer(p: u32, size: u32) -> u8 = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data_from_work_buffer(p: u32, size: u32) -> u8 = cdecl 0x5D1300 => unported;
}

/// The radio save block in the bytes' order (shared by `Save` and `Load`): per station (14, 0x14 bytes each from
/// 0xB62B40) its 20 history bytes, then the next 8 dwords from 0xB62980, 40 from 0xB620C0 and 15 from 0xB61D78
/// (all three running on across the stations); then the bytes 0xB62C71 down to 0xB62C58, 0xB62C73, 0xB62C72.
fn radio_save_block(io: fn(u32, u32) -> u8) {
    let (mut a, mut b, mut c) = (0xB6_2980u32, 0xB6_20C0u32, 0xB6_1D78u32);
    let mut st = 0xB6_2B40u32;
    while st < 0xB6_2C58 {
        for i in 0..0x14 {
            io(st + i, 1);
        }
        for _ in 0..8 {
            io(a, 4);
            a += 4;
        }
        for _ in 0..0x28 {
            io(b, 4);
            b += 4;
        }
        for _ in 0..0xF {
            io(c, 4);
            c += 4;
        }
        st += 0x14;
    }
    for p in (0xB6_2C58u32..=0xB6_2C71).rev() {
        io(p, 1);
    }
    io(0xB6_2C73, 1);
    io(0xB6_2C72, 1);
}

/// `CAERadioTrackManager::Save()` (0x5D3EE0, static): [`radio_save_block`] through `SaveDataToWorkBuffer`; AL = 1.
#[hook(addr = 0x5D3EE0, conv = cdecl, name = "CAERadioTrackManager::Save", expect = "51 53 55 56 57 C7 44 24")]
pub fn radio_save() -> u8 {
    radio_save_block(save_data_to_work_buffer);
    1
}

/// `CAERadioTrackManager::Load()` (0x5D40E0, static): [`radio_save_block`] through `LoadDataFromWorkBuffer`; AL = 1.
#[hook(addr = 0x5D40E0, conv = cdecl, name = "CAERadioTrackManager::Load", expect = "51 53 55 56 57 C7 44 24")]
pub fn radio_load() -> u8 {
    radio_save_block(load_data_from_work_buffer);
    1
}

// ---- CAEPedSpeechAudioEntity::Constructor ------------------------------------------------------------

/// `CAEPedSpeechAudioEntity::CAEPedSpeechAudioEntity()` (0x4E4F10): vtable 0x85F310; `+0x10`, `+4`, `+0xA0` = 0;
/// `+0x90`, `+0x98`, `+0x99`, `+0x9A` = 0, `+0x92` = 0; `+0xA4`, `+0xA6`, `+0xA8`, `+0xB0` = -1; `+0xAC` = -100.0;
/// EAX = this. EDX preserved: the pedless constructor 0x4E6070 keeps `this` in EDX across the call (0x4E6077),
/// which the original never touches.
#[hook(addr = 0x4E4F10, conv = thiscall, name = "CAEPedSpeechAudioEntity::Constructor", expect = "8B C1 33 C9 89 48 10", preserve = "edx")]
pub fn speech_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    wr::<u8>(this, 0x90, 0);
    wr::<u16>(this, 0x92, 0);
    wr::<u8>(this, 0x98, 0);
    wr::<u8>(this, 0x99, 0);
    wr::<u8>(this, 0x9A, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 0xA0, 0);
    wr::<u32>(this, 0, 0x85F310);
    wr::<u16>(this, 0xA4, 0xFFFF);
    wr::<u16>(this, 0xA6, 0xFFFF);
    wr::<u16>(this, 0xA8, 0xFFFF);
    wr::<u32>(this, 0xAC, 0xC2C8_0000);
    wr::<u16>(this, 0xB0, 0xFFFF);
    this
}

// ---- CAEAudioUtility::GetBankAndSoundFromScriptSlotAudioEvent -------------------------------------------

/// `floor` of a finite double below 2^53 in magnitude, as the CRT's `floor` (0x8219F0) returns it.
fn floor_f64(v: f64) -> f64 {
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}

/// `CAEAudioUtility::GetBankAndSoundFromScriptSlotAudioEvent(const int32* event, int32* bank, int32* sound, int32
/// slot)` (0x4D9CC0, cdecl), from the bytes: events below 1800 fail (AL = 0); 1800..1999: bank = the table entry
/// 0x8ABC70 + 4 * event (indexed by the event itself), sound -1; 0xFFFF: bank 0x123, sound `(slot % 2) * 2` for
/// slots 0..3 else 0; otherwise `e = event - 2000` (stored into the `event` argument slot, as the bytes do):
/// bank = `_ftol(floor(f32(e) * 0.005f)) + 0x93`, sound = `e % 200`. AL = 1.
#[hook(addr = 0x4D9CC0, conv = cdecl, name = "CAEAudioUtility::GetBankAndSoundFromScriptSlotAudioEvent", expect = "56 8B 74 24 08 8B 06 3D", footprint = 0x400)]
pub fn utility_get_bank_and_sound_from_script_slot_audio_event(
    event: GPtr<u8>,
    bank: GPtr<u8>,
    sound: GPtr<u8>,
    slot: i32,
) -> u8 {
    let e = rd::<i32>(event, 0);
    if e < 0x708 {
        return 0;
    }
    if e < 0x7D0 {
        wr::<u32>(
            bank,
            0,
            g::<u32>((0x8A_BC70u32.wrapping_add((e as u32).wrapping_mul(4))) as usize).read(),
        );
        wr::<i32>(sound, 0, -1);
        return 1;
    }
    if e == 0xFFFF {
        wr::<u32>(bank, 0, 0x123);
        wr::<i32>(sound, 0, if (0..4).contains(&slot) { (slot % 2) * 2 } else { 0 });
        return 1;
    }
    let d = e.wrapping_sub(0x7D0);
    sa_core::abi::stack_footprint(&[(4, sa_core::abi::Fp::Val(d as u32))]);
    let q = floor_f64(X87::from_i32(d).fmul_m(f32::from_bits(0x3BA3_D70A)).to_f64());
    wr::<i32>(
        bank,
        0,
        (X87::from_f64_exact(q).ftol() as u32 as i32).wrapping_add(0x93),
    );
    wr::<i32>(sound, 0, rd::<i32>(event, 0).wrapping_sub(0x7D0) % 200);
    1
}

// ---- destructors and forwarders --------------------------------------------------------------------

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// CRT `operator delete` (0x8214BD)
    fn crt_delete(p: u32) = cdecl 0x8214BD => unported;
    /// `CFileMgr::CloseFile` (0x72F430, `jmp` to the CRT `fclose`)
    fn file_close(f: u32) -> i32 = cdecl 0x72F430 => unported;
    /// `eh vector destructor iterator(void*, size, count, dtor)` (0x821E02, `ret 0x10`)
    fn eh_vector_destructor_iterator(p: u32, size: u32, count: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CAESoundManager::~CAESoundManager()` (0x4EFD60)
    fn sound_manager_dtor(sm: GPtr<u8>) = thiscall 0x4EFD60 => unported;
    /// `CAEStreamThread::~CAEStreamThread()` (0x4F11E0)
    fn stream_thread_dtor(t: GPtr<u8>) = thiscall 0x4F11E0 => unported;
}

/// `CAESound::~CAESound()` (0x4EF660): the physical entity reference (+8) cleaned up and cleared. Void (the
/// callers ignore EAX).
#[hook(addr = 0x4EF660, conv = thiscall, name = "CAESound", expect = "56 8D 71 08 8B 0E 85 C9")]
pub fn sound_destructor(this: GPtr<u8>) {
    let e = rd::<GPtr<u8>>(this, 8);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(8));
        wr::<u32>(this, 8, 0);
    }
}

/// `__dtor_CAEPedSpeechAudioEntity__s_PedSpeechSlots` (0x856330): the six speech slots (0xB61C38, 0x1C each)
/// destroyed with their destructor 0x4E3340.
#[hook(addr = 0x856330, conv = cdecl, name = "__dtor_CAEPedSpeechAudioEntity__s_PedSpeechSlots", expect = "68 40 33 4E 00 6A 06 6A")]
pub fn dtor_ped_speech_slots() {
    eh_vector_destructor_iterator(0xB6_1C38, 0x1C, 6, 0x4E_3340);
}

/// `__dtor_AESoundManager` (0x856360): `~CAESoundManager` on `AESoundManager`.
#[hook(addr = 0x856360, conv = cdecl, name = "__dtor_AESoundManager", expect = "B9 B0 2C B6 00")]
pub fn dtor_ae_sound_manager() {
    sound_manager_dtor(g(crate::common::AE_SOUND_MANAGER));
}

/// `CAEAudioHardware::~CAEAudioHardware()` (0x4D83A0): `~CAEStreamThread` on its stream thread (+0xE14).
#[hook(addr = 0x4D83A0, conv = thiscall, name = "CAEAudioHardware::Destructor", expect = "81 C1 14 0E 00 00")]
pub fn hw_destructor(this: GPtr<u8>) {
    stream_thread_dtor(this.byte_add(0xE14));
}

/// `CAEBankLoader::~CAEBankLoader()` (0x4DFB20): when initialised (+0x14): delete the bank slots (+0), close the
/// SFX pak file (+0x1C), delete the bank lookups (+4) and the pak lookups (+0x20).
#[hook(addr = 0x4DFB20, conv = thiscall, name = "CAEBankLoader::Deconstructor", expect = "56 8B F1 8A 46 14 84 C0")]
pub fn bank_loader_destructor(this: GPtr<u8>) {
    if rd::<u8>(this, 0x14) != 0 {
        crt_delete(rd::<u32>(this, 0));
        file_close(rd::<u32>(this, 0x1C));
        crt_delete(rd::<u32>(this, 4));
        crt_delete(rd::<u32>(this, 0x20));
    }
}

/// `CAEMP3TrackLoader::~CAEMP3TrackLoader()` (0x4E0940): when initialised (+0) the two files (+0xC, +0x10)
/// closed; the track lookup (+0x18) deleted when set.
#[hook(addr = 0x4E0940, conv = thiscall, name = "CAEMP3TrackLoader::Deconstructor", expect = "56 8B F1 80 3E 00 74 15")]
pub fn mp3_track_loader_destructor(this: GPtr<u8>) {
    if rd::<u8>(this, 0) != 0 {
        file_close(rd::<u32>(this, 0xC));
        file_close(rd::<u32>(this, 0x10));
    }
    let p = rd::<u32>(this, 0x18);
    if p != 0 {
        crt_delete(p);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SOUND_DESTRUCTOR,
    HOOK_DTOR_PED_SPEECH_SLOTS,
    HOOK_DTOR_AE_SOUND_MANAGER,
    HOOK_HW_DESTRUCTOR,
    HOOK_BANK_LOADER_DESTRUCTOR,
    HOOK_MP3_TRACK_LOADER_DESTRUCTOR,
    HOOK_UTILITY_GET_BANK_AND_SOUND_FROM_SCRIPT_SLOT_AUDIO_EVENT,
    HOOK_SPEECH_CONSTRUCTOR,
    HOOK_RADIO_SAVE,
    HOOK_RADIO_LOAD,
    HOOK_RADIO_CHECK_FOR_TRACK_CONCATENATION,
    HOOK_ENV_GET_POSITION_RELATIVE_TO_CAMERA,
    HOOK_RADIO_UPDATE_RADIO_VOLUMES,
    HOOK_HW_RESCALE_CHANNEL_VOLUMES,
    HOOK_AMB_UPDATE_AMBIENCE_TRACK_AND_VOLUME,
    HOOK_HW_GET_NUM_AVAILABLE_CHANNELS_AX,
    HOOK_SM_GET_VIRTUAL_CHANNEL_FOR_PHYSICAL_CHANNEL_AX,
    HOOK_PEDLESS_GET_ALLOCATED_VOICE_AX,
    HOOK_PEDLESS_GET_PED_TYPE_AX,
    HOOK_SPEECH_GET_ALLOCATED_VOICE_AX,
    HOOK_SPEECH_GET_PED_TYPE_AX,
    HOOK_SPEECH_GET_VOICE_AX,
    HOOK_SPEECH_CAN_WE_PLAY_GLOBAL_SPEECH_CONTEXT_AX,
    HOOK_VEHICLE_GET_VEHICLE_TYPE_FOR_AUDIO_AX,
];

/// An uninitialised stack local of the original (decision PORT-AUDIO-3: the residue is reproduced, never
/// zeroed): `N` dwords from `entry ESP + off` as they were when the `#[hook(addr = hook, peek = W)]` port was
/// entered (`sa_core::abi::peek_stack`); 0 for dwords outside the window or off the hook (a direct Rust call).
pub fn stack_local<const N: usize>(hook: u32, off: i32) -> [u32; N] {
    let mut b = [0u32; N];
    for (i, w) in b.iter_mut().enumerate() {
        *w = sa_core::abi::peek_stack(hook, off + 4 * i as i32).unwrap_or(0);
    }
    b
}
