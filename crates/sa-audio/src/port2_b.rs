//! PORT-AUDIO-2 hand ports, helper B (see `docs/arch/audio-port.md`, PORT-AUDIO-2).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids (the exe wins).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CAESound::StopSound()` (0x4EF1C0)
    fn sound_stop_sound(sound: GPtr<u8>) = thiscall 0x4EF1C0 => crate::sound::stop_sound;
    /// `CAEPedSpeechAudioEntity::StopCurrentSpeech()` (0x4E3FB0)
    fn speech_stop_current_speech(this: GPtr<u8>) = thiscall 0x4E3FB0 => unported;
    /// `CVehicle` virtual slot 0x64 (`GetComponentWorldPosition(int32 component, CVector& out)`)
    fn vcall_vehicle_component_world_position(veh: GPtr<u8>, component: i32, out: GPtr<u8>) = thiscall vslot 0x64 => unported;
}

/// Stops (not forgets) the `CAESound*` at `this + off` when non-null.
fn stop_sound_at(this: GPtr<u8>, off: isize) {
    let s = rd::<u32>(this, off);
    if s != 0 {
        sound_stop_sound(g(s as usize));
    }
}

// ---- CAEPedSpeechAudioEntity / CAEPedlessSpeechAudioEntity small members -------------------------

/// `CAEPedlessSpeechAudioEntity::GetAllocatedVoice()` (0x4E6090): AX = -1 (upper EAX untouched).
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX, needs an AX return kind in sa-core::abi.
pub fn pedless_get_allocated_voice(_this: GPtr<u8>) -> u16 {
    0xFFFF
}

/// `CAEPedlessSpeechAudioEntity::GetPedType()` (0x4E60B0): AX = -1 (upper EAX untouched).
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX, needs an AX return kind in sa-core::abi.
pub fn pedless_get_ped_type(_this: GPtr<u8>) -> u16 {
    0xFFFF
}

/// `CAEPedSpeechAudioEntity::GetAllocatedVoice()` (0x4E4120): AX = the voice word +0x94.
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX (port-l2-a2-menus-hud mismatch), needs an AX return kind in sa-core::abi.
pub fn speech_get_allocated_voice(this: GPtr<u8>) -> u16 {
    rd::<u16>(this, 0x94)
}

/// `CAEPedSpeechAudioEntity::EnablePedSpeech()` (0x4E3F70): initialised (+0x90) -> disabled flag +0x99 = 0.
#[hook(addr = 0x4E3F70, conv = thiscall, name = "CAEPedSpeechAudioEntity::EnablePedSpeech", expect = "8A 81 90 00 00 00")]
pub fn speech_enable_ped_speech(this: GPtr<u8>) {
    if rd::<u8>(this, 0x90) != 0 {
        wr::<u8>(this, 0x99, 0);
    }
}

/// `CAEPedSpeechAudioEntity::EnablePedSpeechForScriptSpeech()` (0x4E3F90): initialised -> +0x9A = 0.
#[hook(addr = 0x4E3F90, conv = thiscall, name = "CAEPedSpeechAudioEntity::EnablePedSpeechForScriptSpeech", expect = "8A 81 90 00 00 00")]
pub fn speech_enable_ped_speech_for_script_speech(this: GPtr<u8>) {
    if rd::<u8>(this, 0x90) != 0 {
        wr::<u8>(this, 0x9A, 0);
    }
}

/// `CAEPedSpeechAudioEntity::GetPedTalking()` (0x4E3F50): initialised ? +0x98 : 0 (AL).
#[hook(addr = 0x4E3F50, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetPedTalking", expect = "8A 81 90 00 00 00")]
pub fn speech_get_ped_talking(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x90) == 0 {
        0
    } else {
        rd::<u8>(this, 0x98)
    }
}

/// `CAEPedSpeechAudioEntity::IsPedFemaleForAudio()` (0x4E4150): initialised ? +0x96 : 0 (AL).
#[hook(addr = 0x4E4150, conv = thiscall, name = "CAEPedSpeechAudioEntity::IsPedFemaleForAudio", expect = "8A 81 90 00 00 00")]
pub fn speech_is_ped_female_for_audio(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x90) == 0 {
        0
    } else {
        rd::<u8>(this, 0x96)
    }
}

/// `CAEPedSpeechAudioEntity::GetPedType()` (0x4E4130): initialised ? the word +0x92 : -1 (AX).
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX, needs an AX return kind in sa-core::abi.
pub fn speech_get_ped_type(this: GPtr<u8>) -> u16 {
    if rd::<u8>(this, 0x90) == 0 {
        0xFFFF
    } else {
        rd::<u16>(this, 0x92)
    }
}

/// `CAEPedSpeechAudioEntity::DisablePedSpeech(int16 stopCurrent)` (0x4E56D0): initialised -> +0x99 = 1
/// and, when the word argument is non-zero, `StopCurrentSpeech`.
#[hook(addr = 0x4E56D0, conv = thiscall, name = "CAEPedSpeechAudioEntity::DisablePedSpeech", expect = "8A 81 90 00 00 00")]
pub fn speech_disable_ped_speech(this: GPtr<u8>, stop: u32) {
    if rd::<u8>(this, 0x90) == 0 {
        return;
    }
    wr::<u8>(this, 0x99, 1);
    if stop as u16 != 0 {
        speech_stop_current_speech(this);
    }
}

/// `CAEPedSpeechAudioEntity::DisablePedSpeechForScriptSpeech(int16 stopCurrent)` (0x4E5700): as
/// `DisablePedSpeech` with the flag +0x9A.
#[hook(addr = 0x4E5700, conv = thiscall, name = "CAEPedSpeechAudioEntity::DisablePedSpeechForScriptSpeech", expect = "8A 81 90 00 00 00")]
pub fn speech_disable_ped_speech_for_script_speech(this: GPtr<u8>, stop: u32) {
    if rd::<u8>(this, 0x90) == 0 {
        return;
    }
    wr::<u8>(this, 0x9A, 1);
    if stop as u16 != 0 {
        speech_stop_current_speech(this);
    }
}

// ---- CAETwinLoopSoundEntity / CAEWaterCannonAudioEntity --------------------------------------------

/// `CAETwinLoopSoundEntity::StopSound()` (0x4F2930): `StopSound` on both sounds (+0xA0, +0xA4).
#[hook(addr = 0x4F2930, conv = thiscall, name = "CAETwinLoopSoundEntity::StopSound", expect = "56 8B F1 8B 8E A0 00 00 00")]
pub fn twin_loop_stop_sound(this: GPtr<u8>) {
    stop_sound_at(this, 0xA0);
    stop_sound_at(this, 0xA4);
}

/// `CAEWaterCannonAudioEntity::Terminate()` (0x5030A0): `StopSound` on the four sounds +0x90..+0x9C.
#[hook(addr = 0x5030A0, conv = thiscall, name = "CAEWaterCannonAudioEntity::Terminate", expect = "56 57 8D B1 90 00 00 00")]
pub fn water_cannon_terminate(this: GPtr<u8>) {
    for i in 0..4isize {
        stop_sound_at(this, 0x90 + i * 4);
    }
}

// ---- CAEVehicleAudioEntity ----------------------------------------------------------------------

/// `CAEVehicleAudioEntity::GetAircraftNearPosition()` (0x4F96A0, returns `CVector` through the hidden
/// pointer, `ret 4`): audio vehicle type +0x80 == 4 -> the vehicle's virtual 0x64 with the component
/// word at 0x8CBD4C (sign-extended) into a local; otherwise the entity's position (matrix +0x30 or
/// placement +4). EAX = `out`. The local starts uninitialised in the original (zeroed here).
#[hook(addr = 0x4F96A0, conv = thiscall, name = "CAEVehicleAudioEntity::GetAircraftNearPosition", expect = "8A 81 80 00 00 00")]
pub fn vehicle_get_aircraft_near_position(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let ent: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let v: [u32; 3] = if rd::<u8>(this, 0x80) == 4 {
        let mut v = [0u32; 3];
        let comp = g::<i16>(0x8CBD4C).read() as i32;
        vcall_vehicle_component_world_position(ent, comp, GPtr::from_raw(v.as_mut_ptr()).cast());
        v
    } else {
        let m = rd::<u32>(ent, 0x14);
        let p: GPtr<u8> = if m != 0 {
            g(m as usize + 0x30)
        } else {
            ent.byte_add(4)
        };
        [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
    };
    wr::<u32>(out, 0, v[0]);
    wr::<u32>(out, 4, v[1]);
    wr::<u32>(out, 8, v[2]);
    out
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SPEECH_ENABLE_PED_SPEECH,
    HOOK_SPEECH_ENABLE_PED_SPEECH_FOR_SCRIPT_SPEECH,
    HOOK_SPEECH_GET_PED_TALKING,
    HOOK_SPEECH_IS_PED_FEMALE_FOR_AUDIO,
    HOOK_SPEECH_DISABLE_PED_SPEECH,
    HOOK_SPEECH_DISABLE_PED_SPEECH_FOR_SCRIPT_SPEECH,
    HOOK_TWIN_LOOP_STOP_SOUND,
    HOOK_WATER_CANNON_TERMINATE,
    HOOK_VEHICLE_GET_AIRCRAFT_NEAR_POSITION,
    HOOK_VEHICLE_IS_ACC_INHIBITED_BACKWARDS,
    HOOK_VEHICLE_GET_VEHICLE_AUDIO_SETTINGS,
    HOOK_VEHICLE_DOES_BANK_SLOT_CONTAIN_THIS_BANK,
    HOOK_SPEECH_GET_NUM_SLOTS_PLAYING_CONTEXT,
    HOOK_WATER_CANNON_UPDATE_PARAMETERS,
    HOOK_SPEECH_IS_GLOBAL_CONTEXT_IMPORTANT_FOR_WIDESCREEN,
    HOOK_WEATHER_STATIC_RESET,
    HOOK_GLOBAL_WEAPON_UPDATE_PARAMETERS,
    HOOK_COLLISION_RESET,
    HOOK_SPEECH_GET_VOICE_FOR_MOOD,
    HOOK_SPEECH_GET_VOICE_AND_TYPE_FROM_MODEL,
    HOOK_VEHICLE_DEMAND_BANK_SLOT,
    HOOK_SPEECH_GET_REPEAT_TIME,
    HOOK_SPEECH_CAN_WE_PLAY_SCRIPTED_SPEECH,
    HOOK_SPEECH_SET_CJ_MOOD,
    HOOK_SPEECH_GET_VOICE_AND_TYPE_FOR_SPECIAL_PED,
    HOOK_SPEECH_GET_SOUND_AND_BANK_IDS_FOR_SCRIPTED_SPEECH,
    HOOK_SPEECH_RESERVE_PED_CONVERSATION_SPEECH_SLOTS,
    HOOK_PEDLESS_STOP_CURRENT_SPEECH,
    HOOK_SPEECH_RELEASE_PLAYER_CONVERSATION,
    HOOK_SPEECH_REQUEST_PLAYER_CONVERSATION,
    HOOK_SPEECH_CAN_PED_SAY_GLOBAL_CONTEXT,
    HOOK_VEHICLE_DESTRUCTOR,
    HOOK_FIRE_UPDATE_PARAMETERS,
    HOOK_PED_HANDLE_SWIM_WAKE,
    HOOK_PED_HANDLE_SWIM_SPLASH,
    HOOK_PED_HANDLE_LANDING_EVENT,
    HOOK_PED_HANDLE_SKATE_EVENT,
    HOOK_FIRE_PLAY_WATER_SOUNDS,
    HOOK_COLLISION_GET_COLLISION_SOUND_STATUS,
    HOOK_VEHICLE_TERMINATE_PORT,
    HOOK_VEHICLE_JUST_WRECKED_VEHICLE,
    HOOK_COLLISION_REPORT_OBJECT_DESTRUCTION,
    HOOK_PED_HANDLE_PED_SWING,
    HOOK_PED_UPDATE_PARAMETERS,
    HOOK_GLOBAL_WEAPON_PROJECTILE_FIRE,
    HOOK_WATER_CANNON_SERVICE,
    HOOK_SPEECH_GET_SPEECH_CONTEXT_VOLUME_OFFSET,
    HOOK_SPEECH_GET_SOUND_AND_BANK_IDS,
    HOOK_SPEECH_ADD_SAY_EVENT,
    HOOK_SPEECH_UPDATE_PARAMETERS,
    HOOK_VEHICLE_PROCESS_NITRO,
    HOOK_PED_HANDLE_FOOTSTEP_EVENT,
    HOOK_VEHICLE_PROCESS_ENGINE_DAMAGE,
    HOOK_VEHICLE_PROCESS_DUMMY_VEHICLE_ENGINE,
    HOOK_VEHICLE_UPDATE_TRAIN_SOUND,
    HOOK_WEAPON_UPDATE_PARAMETERS,
    HOOK_DOOR_PLAY_DOOR_SOUND,
    HOOK_FIRE_PLAY_FIRE_SOUNDS,
    HOOK_VEHICLE_INITIALISE,
    HOOK_PEDLESS_UPDATE_PARAMETERS,
    HOOK_SPEECH_ADD_SCRIPT_SAY_EVENT,
    HOOK_SPEECH_PLAY_LOADED_SOUND,
    HOOK_VEHICLE_JUST_GOT_IN_VEHICLE_AS_DRIVER,
    HOOK_COLLISION_REPORT_GLASS_COLLISION_EVENT,
    HOOK_COLLISION_REPORT_WATER_SPLASH_AT_POSITION,
    HOOK_WEAPON_PLAY_FLAME_THROWER_IDLE_GAS_LOOP,
    HOOK_WEAPON_PLAY_CHAINSAW_STOP_SOUND_PORT,
    HOOK_WEAPON_PLAY_MINI_GUN_STOP_SOUND_PORT,
    HOOK_WEAPON_PLAY_WEAPON_LOOP_SOUND,
    HOOK_WEAPON_PLAY_CAMERA_SOUND,
    HOOK_WEAPON_WEAPON_RELOAD,
    HOOK_WEAPON_PLAY_FLAME_THROWER_SOUNDS,
    HOOK_WEAPON_PLAY_MINI_GUN_FIRE_SOUNDS,
    HOOK_WEAPON_PLAY_GOGGLE_SOUND,
];

// ---- batch 2 -------------------------------------------------------------------------------------

/// `CAEVehicleAudioEntity::IsAccInhibitedBackwards(tVehicleParams&)` (0x4F4FC0, thiscall, `this`
/// unused; ECX is overwritten with the argument): AL = 0 when `!(params+0x40 > [0x858C80])` (unordered
/// included) and the byte +0x46 is set, else 1.
#[hook(addr = 0x4F4FC0, conv = thiscall, name = "CAEVehicleAudioEntity::IsAccInhibitedBackwards", expect = "8B 4C 24 04 D9 41 40")]
pub fn vehicle_is_acc_inhibited_backwards(_this: GPtr<u8>, params: GPtr<u8>) -> u8 {
    let v = rd::<f32>(params, 0x40);
    if v > g::<f32>(0x858C80).read() {
        return 1;
    }
    if rd::<u8>(params, 0x46) != 0 { 0 } else { 1 }
}

/// `CAEVehicleAudioEntity::GetVehicleAudioSettings(int16 vehId)` (0x4F5C10, static, returns the 36-byte
/// `tVehicleAudioSettings` through the hidden pointer, `ret 8`): copies row `vehId - 400` of the table
/// at 0x860AF0. EAX = `out`.
#[hook(addr = 0x4F5C10, conv = stdcall, name = "CAEVehicleAudioEntity::GetVehicleAudioSettings", expect = "0F BF 44 24 08")]
pub fn vehicle_get_vehicle_audio_settings(out: GPtr<u8>, veh_id: u32) -> GPtr<u8> {
    let row = (veh_id as u16 as i16 as i32).wrapping_sub(400).wrapping_mul(36);
    let src: GPtr<u8> = g((0x860AF0i32).wrapping_add(row) as u32 as usize);
    for i in 0..9isize {
        wr::<u32>(out, i * 4, rd::<u32>(src, i * 4));
    }
    out
}

/// `CAEVehicleAudioEntity::DoesBankSlotContainThisBank(int16 slot, int16 bank)` (0x4F4E30, cdecl):
/// `slot - 7` in 0..10 (16-bit signed) and `s_DummyEngineSlots[slot - 7].bankSlot == bank` (AL).
#[hook(addr = 0x4F4E30, conv = cdecl, name = "CAEVehicleAudioEntity::DoesBankSlotContainThisBank", expect = "8B 44 24 04 83 C0 F9")]
pub fn vehicle_does_bank_slot_contain_this_bank(slot: u32, bank: u32) -> u8 {
    let i = slot.wrapping_add(0xFFFF_FFF9) as u16 as i16;
    if !(0..10).contains(&i) {
        return 0;
    }
    (g::<u16>(0xB6B9A0 + i as usize * 4).read() == bank as u16) as u8
}

/// `CAEPedSpeechAudioEntity::GetNumSlotsPlayingContext(int16 gCtx)` (0x4E49B0): counts the five speech
/// slots (0x1C bytes from 0xB61C38) with a non-zero status word whose context word (+0x14) equals the
/// argument word. Full EAX.
#[hook(addr = 0x4E49B0, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetNumSlotsPlayingContext", expect = "56 66 8B 74 24 08")]
pub fn speech_get_num_slots_playing_context(_this: GPtr<u8>, ctx: u32) -> u32 {
    let mut n = 0u32;
    for i in 0..5usize {
        let slot: GPtr<u8> = g(0xB61C38 + i * 0x1C);
        if rd::<u16>(slot, 0) != 0 && rd::<u16>(slot, 0x14) == ctx as u16 {
            n += 1;
        }
    }
    n
}

/// `CAEWaterCannonAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x502EF0): play position
/// word -1 clears each of the four sound pointers (+0x90..+0x9C) equal to `sound`.
#[hook(addr = 0x502EF0, conv = thiscall, name = "CAEWaterCannonAudioEntity::UpdateParameters", expect = "66 83 7C 24 08 FF")]
pub fn water_cannon_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    if pos as u16 != 0xFFFF {
        return;
    }
    for i in 0..4isize {
        if rd::<u32>(this, 0x90 + i * 4) == sound {
            wr::<u32>(this, 0x90 + i * 4, 0);
        }
    }
}

/// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForWidescreen(int16 gCtx)` (0x4E46B0): AL = 1 for
/// ped types (+0x92) 4 and 2, or contexts 0xD, 0xF, 0x74; else 0. Callers 0x4E612C / 0x4E65A0 do not
/// read ECX/EDX afterwards.
#[hook(addr = 0x4E46B0, conv = thiscall, name = "CAEPedSpeechAudioEntity::IsGlobalContextImportantForWidescreen", expect = "66 8B 81 92 00 00 00")]
pub fn speech_is_global_context_important_for_widescreen(this: GPtr<u8>, ctx: u32) -> u8 {
    let ty = rd::<u16>(this, 0x92);
    if ty == 4 || ty == 2 {
        return 1;
    }
    let c = ctx as u16 as i16 as i32;
    (c == 0xD || c == 0xF || c == 0x74) as u8
}

// ---- batch 3 -------------------------------------------------------------------------------------

orig! {
    /// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960)
    fn twin_loop_stop_sound_and_forget(this: GPtr<u8>) = thiscall 0x4F2960 => crate::entities::twin_loop_stop_sound_and_forget;
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    fn sound_stop_and_forget(sound: GPtr<u8>) = thiscall 0x4EF850 => unported;
}

/// `GetVehicleTypeForAudio` jump-table index bytes (`.text`, 0x27 entries, read in place): 0 / 1 / 2.
const VEHICLE_TYPE_FOR_AUDIO_INDEX: usize = 0x4F4F48;

/// `CAEVehicleAudioEntity::GetVehicleTypeForAudio()` (0x4F4F00): AX = 2 unless initialised (+0xA4) with
/// an entity (+4); then the byte at 0x4F4F48 indexed by the signed audio type byte +0x9C (> 0x26 -> 2).
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX (port-l2-a2-menus-hud mismatch), needs an AX return kind in sa-core::abi.
pub fn vehicle_get_vehicle_type_for_audio(this: GPtr<u8>) -> u16 {
    if rd::<u8>(this, 0xA4) == 0 || rd::<u32>(this, 4) == 0 {
        return 2;
    }
    let ty = rd::<u8>(this, 0x9C) as i8 as i32 as u32;
    if ty > 0x26 {
        return 2;
    }
    g::<u8>(VEHICLE_TYPE_FOR_AUDIO_INDEX + ty as usize).read() as u16
}

/// `CAEWeatherAudioEntity::StaticReset()` (0x5052B0; entry `nop nop nop jmp` into a `.text` chunk at
/// 0x4064BF): `m_sfRainVolume` (0x8CC30C) = -100.0f, then the two twin-loop statics at 0xB6BB18 and
/// 0xB6BBC0 `StopSoundAndForget` when their initialised word (+0x88) is non-zero.
#[hook(addr = 0x5052B0, conv = cdecl, name = "CAEWeatherAudioEntity::StaticReset", expect = "90 90 90 E9 07 12 F0 FF")]
pub fn weather_static_reset() {
    let first = g::<u16>(0xB6BBA0).read();
    g::<u32>(0x8CC30C).write(0xC2C8_0000);
    if first != 0 {
        twin_loop_stop_sound_and_forget(g(0xB6BB18));
    }
    if g::<u16>(0xB6BC48).read() != 0 {
        twin_loop_stop_sound_and_forget(g(0xB6BBC0));
    }
}

/// `CAEGlobalWeaponAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DEF90): play position
/// -1 clears the global-weapon sound pointer equal to `sound`: 0xB612FC first (alone), else the first
/// match in 0xB612F0[3] (the bytes address these fields absolutely, not through `this`).
#[hook(addr = 0x4DEF90, conv = thiscall, name = "CAEGlobalWeaponAudioEntity::UpdateParameters", expect = "66 83 7C 24 08 FF")]
pub fn global_weapon_update_parameters(_this: GPtr<u8>, sound: u32, pos: u32) {
    if pos as u16 != 0xFFFF {
        return;
    }
    if g::<u32>(0xB612FC).read() == sound {
        g::<u32>(0xB612FC).write(0);
        return;
    }
    for i in 0..3usize {
        let p = g::<u32>(0xB612F0 + i * 4);
        if p.read() == sound {
            p.write(0);
            return;
        }
    }
}

/// `CAEPedSpeechAudioEntity::ReservePlayerConversationSpeechSlot()` (0x4E3870, static): the first of
/// the five speech slots with status 0 gets status 4 and its +0x1A byte = 1, `s_pConversationPedSlot1`
/// (0xB61408) = its index; AL = 1. None free: AL = 0.
/// Not hooked: loop branches back into the patched entry (0x4E3887 -> 0x4E3872).
pub fn speech_reserve_player_conversation_speech_slot() -> u8 {
    for i in 0..5usize {
        let slot: GPtr<u8> = g(0xB61C38 + i * 0x1C);
        if rd::<u16>(slot, 0) == 0 {
            g::<u16>(0xB61408).write(i as u16);
            wr::<u8>(slot, 0x1A, 1);
            wr::<u16>(slot, 0, 4);
            return 1;
        }
    }
    0
}

/// `CAECollisionAudioEntity::Reset()` (0x4DA320): every entry (300 x 0x14 from +0x208) in state 2 stops
/// and forgets its sound (+8), is cleared (surfaces 0xC3) and the active count (+0x204) drops.
#[hook(addr = 0x4DA320, conv = thiscall, name = "CAECollisionAudioEntity::Reset", expect = "53 55 56 57 8B F9")]
pub fn collision_reset(this: GPtr<u8>) {
    for i in 0..300isize {
        let e = this.byte_add(0x208 + i * 0x14);
        if rd::<u8>(e, 0x10) != 2 {
            continue;
        }
        let snd = rd::<u32>(e, 8);
        if snd != 0 {
            sound_stop_and_forget(g(snd as usize));
        }
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 4, 0);
        wr::<u8>(e, 0x11, 0xC3);
        wr::<u8>(e, 0x12, 0xC3);
        wr::<u32>(e, 8, 0);
        wr::<u8>(e, 0x10, 0);
        wr::<u32>(this, 0x204, rd::<u32>(this, 0x204).wrapping_sub(1));
    }
}

/// `CAEPedSpeechAudioEntity::GetVoiceForMood(int16 mood)` (0x4E4760): `CAEAudioUtility::GetRandomNumberInRange(0, 1)`
/// (drawn before the mood is read) plus 2 * the mood's slot from the bytes' jump table: moods 0..9 ->
/// +0, 2, 4, 6, 8, 10, 12, 14, 16, 18; others +10. Full EAX. (gta-reversed picks the voice the other way.)
#[hook(addr = 0x4E4760, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetVoiceForMood", expect = "6A 01 6A 00 E8 A7 54 FF FF")]
pub fn speech_get_voice_for_mood(_this: GPtr<u8>, mood: u32) -> u32 {
    let r = crate::common::ae_random_int(0, 1) as u32;
    let m = mood as u16 as i16 as i32 as u32;
    let add = if m > 9 { 10 } else { m * 2 };
    r.wrapping_add(add)
}

// ---- batch 4 -------------------------------------------------------------------------------------

orig! {
    /// `CAEVehicleAudioEntity::RequestBankSlot(int16 bankId)` (0x4F4D10, static)
    fn vehicle_request_bank_slot(bank: u32) -> u32 = cdecl 0x4F4D10 => crate::vehicle::request_bank_slot;
    /// `CPedModelInfo::IncrementVoice()` (0x4C7300)
    fn ped_model_info_increment_voice(mi: GPtr<u8>) = thiscall 0x4C7300 => unported;
    /// `CAEPedSpeechAudioEntity::GetVoiceAndTypeForSpecialPed(uint32 modelNameHash)` (0x4E4170)
    fn speech_get_voice_and_type_for_special_ped_call(this: GPtr<u8>, hash: u32) -> u8 = thiscall 0x4E4170 => speech_get_voice_and_type_for_special_ped;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CAEPedSpeechAudioEntity` specific-context table (16-byte rows, `global` word at +0, repeat time at +0xC)
const SPECIFIC_CONTEXT_TABLE: usize = 0x8C6A68;

/// `CAEPedSpeechAudioEntity::GetVoiceAndTypeFromModel(eModelID)` (0x4E58C0): ped type word = model info
/// +0x3C (stored at +0x92 before the range check); outside 0..6 -> AL 0; type 5 tail-calls
/// `GetVoiceAndTypeForSpecialPed(model info +4 = name hash)`; otherwise voice word = model info +0x42
/// (stored at +0x94); negative -> 0, else `CPedModelInfo::IncrementVoice` and AL 1.
#[hook(addr = 0x4E58C0, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetVoiceAndTypeFromModel", expect = "8B 44 24 04 8B 04 85 C8 B0 A9 00")]
pub fn speech_get_voice_and_type_from_model(this: GPtr<u8>, model: u32) -> u8 {
    let mi: GPtr<u8> =
        g(g::<u32>((MODEL_INFO_PTRS as u32).wrapping_add(model.wrapping_mul(4)) as usize).read() as usize);
    let ty = rd::<u16>(mi, 0x3C);
    wr::<u16>(this, 0x92, ty);
    let ty = ty as i16;
    if !(0..6).contains(&ty) {
        return 0;
    }
    if ty == 5 {
        return speech_get_voice_and_type_for_special_ped_call(this, rd::<u32>(mi, 4));
    }
    let voice = rd::<u16>(mi, 0x42);
    wr::<u16>(this, 0x94, voice);
    if (voice as i16) < 0 {
        return 0;
    }
    ped_model_info_increment_voice(mi);
    1
}

/// `CAEVehicleAudioEntity::DemandBankSlot(int16 bankId)` (0x4F4E60, static): `RequestBankSlot`; when it
/// returns AX -1, the first dummy engine slot 1..9 whose reference count is below slot 0's (else slot
/// 0) gets count 0 and `RequestBankSlot` is called again. EAX = the last `RequestBankSlot` EAX.
#[hook(addr = 0x4F4E60, conv = cdecl, name = "CAEVehicleAudioEntity::DemandBankSlot", expect = "56 8B 74 24 08")]
pub fn vehicle_demand_bank_slot(bank: u32) -> u32 {
    let r = vehicle_request_bank_slot(bank);
    if r as u16 != 0xFFFF {
        return r;
    }
    let count = |i: usize| g::<i16>(0xB6B9A2 + i * 4);
    let c0 = count(0).read();
    let mut pick = 0usize;
    for i in 1..10usize {
        if count(i).read() < c0 {
            pick = i;
            break;
        }
    }
    count(pick).write(0);
    vehicle_request_bank_slot(bank)
}

/// `CAEPedSpeechAudioEntity::GetRepeatTime(int16 gCtx)` (0x4E47E0): AX = the repeat-time word (+0xC) of
/// the specific-context row whose global context equals `gCtx` (0..0x168), else 0.
#[hook(addr = 0x4E47E0, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetRepeatTime", expect = "66 8B 54 24 04")]
pub fn speech_get_repeat_time(_this: GPtr<u8>, ctx: u32) -> u16 {
    let c = ctx as u16 as i16;
    if !(0..0x168).contains(&c) {
        return 0;
    }
    let row = |i: i16| SPECIFIC_CONTEXT_TABLE + ((i as i32) << 4) as usize;
    let mut i: i16 = 0;
    loop {
        let gl = g::<u16>(row(i)).read();
        if gl == 0xFFFF || gl == c as u16 {
            break;
        }
        i = i.wrapping_add(1);
    }
    if g::<u16>(row(i)).read() == 0xFFFF {
        return 0;
    }
    g::<u16>(row(i) + 0xC).read()
}

/// `CAEPedSpeechAudioEntity::CanWePlayScriptedSpeech()` (0x4E4950, static): from `s_NextSpeechSlot`
/// (0xB61414) round robin (`%` 5, signed), the first speech slot with status 0: AX = its index and
/// `s_NextSpeechSlot` = (index + 1) % 5. None: AX = -1.
#[hook(addr = 0x4E4950, conv = cdecl, name = "CAEPedSpeechAudioEntity::CanWePlayScriptedSpeech", expect = "56 57 0F BF 3D 14 14 B6 00")]
pub fn speech_can_we_play_scripted_speech() -> u16 {
    let next = g::<i16>(0xB61414).read() as i32;
    for c in 0..5i32 {
        let s = (c + next) % 5;
        let s16 = s as i16;
        if g::<u16>((0xB61C38i32 + s16 as i32 * 0x1C) as u32 as usize).read() == 0 {
            g::<u16>(0xB61414).write(((s16 as i32 + 1) % 5) as u16);
            return s16 as u16;
        }
    }
    0xFFFF
}

/// `CAEPedSpeechAudioEntity::SetCJMood(int16 mood, uint32 overrideTime, int16 gangBanging, int16 fat,
/// int16 wellDressed)` (0x4E3ED0, static): override end = now + time; basic mood = `mood` for 1, 5, 7, 9
/// (the bytes' jump table), else -1; then the gang-banging, fat and well-dressed words.
#[hook(addr = 0x4E3ED0, conv = cdecl, name = "CAEPedSpeechAudioEntity::SetCJMood", expect = "8B 44 24 08 8B 0D 84 CB B7 00")]
pub fn speech_set_cj_mood(mood: u32, time: u32, gang: u32, fat: u32, dressed: u32) {
    g::<u32>(0xB613E0).write(sa_core::timer::TIME_MS.get().wrapping_add(time));
    let m = mood as u16;
    let basic = match m as i16 {
        1 | 5 | 7 | 9 => m,
        _ => 0xFFFF,
    };
    g::<u16>(0xB613DC).write(basic);
    g::<u16>(0xB613D8).write(gang as u16);
    g::<u16>(0xB613D4).write(fat as u16);
    g::<u16>(0xB613D0).write(dressed as u16);
}

// ---- batch 5 -------------------------------------------------------------------------------------

orig! {
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn keygen_get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `CAEAudioUtility::GetBankAndSoundFromScriptSlotAudioEvent(int32* event, int32* bank, int32* sound, int32 slot)` (0x4D9CC0)
    fn get_bank_and_sound_from_script_slot_audio_event(event: GPtr<u8>, bank: GPtr<u8>, sound: GPtr<u8>, slot: i32) -> u8 = cdecl 0x4D9CC0 => unported;
}

/// Special-ped names: 45 rows of 20 chars (0x8C65C0).
const SPECIAL_PED_NAMES: usize = 0x8C65C0;
/// Special-ped voices: 45 rows `{ int16 voice, int16 type, uint8 female, pad }` (0x8C6948).
const SPECIAL_PED_VOICES: usize = 0x8C6948;

/// `CAEPedSpeechAudioEntity::GetVoiceAndTypeForSpecialPed(uint32 modelNameHash)` (0x4E4170): the first of
/// the 45 special-ped names whose `GetUppercaseKey` equals the hash (each name hashed in turn until the
/// match); with voice and type both != -1: +0x92 = type, +0x94 = voice, +0x96 = the female byte as a
/// word, AL 1; otherwise AL 0.
#[hook(addr = 0x4E4170, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetVoiceAndTypeForSpecialPed", expect = "53 56 57 8B 7C 24 10")]
pub fn speech_get_voice_and_type_for_special_ped(this: GPtr<u8>, hash: u32) -> u8 {
    let mut i = 0usize;
    while i < 0x2D {
        if keygen_get_uppercase_key(g(SPECIAL_PED_NAMES + i * 20)) == hash {
            break;
        }
        i += 1;
    }
    if i == 0x2D {
        return 0;
    }
    let row: GPtr<u8> = g(SPECIAL_PED_VOICES + i * 6);
    let voice = rd::<u16>(row, 0);
    let ty = rd::<u16>(row, 2);
    let female = rd::<u8>(row, 4);
    if voice == 0xFFFF || ty == 0xFFFF {
        return 0;
    }
    wr::<u16>(this, 0x92, ty);
    wr::<u16>(this, 0x94, voice);
    wr::<u16>(this, 0x96, female as u16);
    1
}

/// `CAEPedSpeechAudioEntity::GetSoundAndBankIDsForScriptedSpeech(eAudioEvents)` (0x4E4400): +0xA6 and
/// +0xA4 = -1; `GetBankAndSoundFromScriptSlotAudioEvent(&event, &bank, &sound, -1)` with both locals -1;
/// both >= 0 -> +0xA6 = bank word, +0xA4 = sound word, AL 1; else AL 0. The original passes the address
/// of its own argument slot (the callee may rewrite it); here a local copy.
#[hook(addr = 0x4E4400, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetSoundAndBankIDsForScriptedSpeech", expect = "83 EC 08 83 C8 FF")]
pub fn speech_get_sound_and_bank_ids_for_scripted_speech(this: GPtr<u8>, event: u32) -> u8 {
    wr::<u16>(this, 0xA6, 0xFFFF);
    wr::<u16>(this, 0xA4, 0xFFFF);
    let mut ev = [event];
    let mut bank = [u32::MAX];
    let mut sound = [u32::MAX];
    get_bank_and_sound_from_script_slot_audio_event(
        GPtr::from_raw(ev.as_mut_ptr()).cast(),
        GPtr::from_raw(bank.as_mut_ptr()).cast(),
        GPtr::from_raw(sound.as_mut_ptr()).cast(),
        -1,
    );
    if (bank[0] as i32) < 0 || (sound[0] as i32) < 0 {
        return 0;
    }
    wr::<u16>(this, 0xA6, bank[0] as u16);
    wr::<u16>(this, 0xA4, sound[0] as u16);
    1
}

/// `CAEPedSpeechAudioEntity::ReservePedConversationSpeechSlots()` (0x4E37F0, static): the first two
/// speech slots with status 0 get their +0x19 byte = 1 and status 4 (first, second);
/// `s_pConversationPedSlot1` (0xB61408) = the first index, `s_pConversationPedSlot2` (0xB61404) = the
/// second; AL 1. Fewer than two free: AL 0.
#[hook(addr = 0x4E37F0, conv = cdecl, name = "CAEPedSpeechAudioEntity::ReservePedConversationSpeechSlots", expect = "57 83 CF FF 33 C9")]
pub fn speech_reserve_ped_conversation_speech_slots() -> u8 {
    let slot = |i: usize| -> GPtr<u8> { g(0xB61C38 + i * 0x1C) };
    let mut first: Option<usize> = None;
    for c in 0..5usize {
        if rd::<u16>(slot(c), 0) != 0 {
            continue;
        }
        let Some(f) = first else {
            first = Some(c);
            continue;
        };
        wr::<u8>(slot(f), 0x19, 1);
        wr::<u8>(slot(c), 0x19, 1);
        wr::<u16>(slot(f), 0, 4);
        wr::<u16>(slot(c), 0, 4);
        g::<u16>(0xB61408).write(f as u16);
        g::<u16>(0xB61404).write(c as u16);
        return 1;
    }
    0
}

// ---- batch 6 -------------------------------------------------------------------------------------

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> u32 = cdecl 0x56E210 => unported;
    /// `CPed::GetPedTalking()` thunk (0x5EFF50: `add ecx, 0x294` + jmp 0x4E3F50)
    fn ped_get_ped_talking(ped: GPtr<u8>) -> u8 = thiscall 0x5EFF50 => unported;
    /// `CAEPedSpeechAudioEntity::ReservePlayerConversationSpeechSlot()` (0x4E3870)
    fn speech_reserve_player_conversation_speech_slot_call() -> u8 = cdecl 0x4E3870 => speech_reserve_player_conversation_speech_slot;
}

/// Clears speech slot `slot` (0x1C bytes at 0xB61C38) in the bytes' store order.
fn clear_speech_slot(slot: GPtr<u8>) {
    wr::<u16>(slot, 0, 0);
    wr::<u32>(slot, 4, 0);
    wr::<u32>(slot, 8, 0);
    wr::<u16>(slot, 0xC, 0xFFFF);
    wr::<u16>(slot, 0xE, 0xFFFF);
    wr::<u32>(slot, 0x10, 0);
    wr::<u16>(slot, 0x14, 0xFFFF);
    wr::<u16>(slot, 0x16, 0xFFFF);
    wr::<u8>(slot, 0x19, 0);
    wr::<u8>(slot, 0x1A, 0);
    wr::<u8>(slot, 0x18, 0);
}

fn speech_slot(i: i16) -> GPtr<u8> {
    g((0xB61C38i32 + i as i32 * 0x1C) as u32 as usize)
}

/// `CAEPedlessSpeechAudioEntity::StopCurrentSpeech()` (0x4E4E80): initialised (+0x90) and talking
/// (+0x98): if its speech slot (+0xA8) has status 5, the sound +0xA0 is stopped and forgotten and +0xA0
/// = 0; the slot is cleared, +0x98 = 0, +0xA8 = -1.
#[hook(addr = 0x4E4E80, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::StopCurrentSpeech", expect = "53 56 8B F1 8A 86 90 00 00 00")]
pub fn pedless_stop_current_speech(this: GPtr<u8>) {
    if rd::<u8>(this, 0x90) == 0 || rd::<u8>(this, 0x98) == 0 {
        return;
    }
    if rd::<u16>(speech_slot(rd::<i16>(this, 0xA8)), 0) == 5 {
        let s = rd::<u32>(this, 0xA0);
        if s != 0 {
            sound_stop_and_forget(g(s as usize));
        }
        wr::<u32>(this, 0xA0, 0);
    }
    clear_speech_slot(speech_slot(rd::<i16>(this, 0xA8)));
    wr::<u8>(this, 0x98, 0);
    wr::<u16>(this, 0xA8, 0xFFFF);
}

/// `CAEPedSpeechAudioEntity::ReleasePlayerConversation()` (0x4E3960, static): with a player conversation
/// (0xB613FC) and a slot (0xB61408) >= 0: the slot is cleared when its status is 0 or 4; then slot word =
/// -1, the slot's +0x1A byte = 0, the conversation ped (0xB61400) = 0. The flag is cleared in both cases.
#[hook(addr = 0x4E3960, conv = cdecl, name = "CAEPedSpeechAudioEntity::ReleasePlayerConversation", expect = "A0 FC 13 B6 00")]
pub fn speech_release_player_conversation() {
    if g::<u8>(0xB613FC).read() == 0 {
        return;
    }
    let idx = g::<i16>(0xB61408).read();
    if idx >= 0 {
        let slot = speech_slot(idx);
        let st = rd::<i16>(slot, 0);
        if st == 0 || st == 4 {
            clear_speech_slot(slot);
        }
        g::<u16>(0xB61408).write(0xFFFF);
        wr::<u8>(slot, 0x1A, 0);
        g::<u32>(0xB61400).write(0);
    }
    g::<u8>(0xB613FC).write(0);
}

/// `CAEPedSpeechAudioEntity::RequestPlayerConversation(CPed*)` (0x4E38C0, static): AL 0 when all speech
/// is disabled, the ped's or the player's (`FindPlayerPed(-1)`, called three times) bytes +0x32E / +0x32D
/// are set, a conversation runs, the ped is talking or no player slot can be reserved; else the
/// conversation ped (0xB61400) = ped, the flag 0xB613FC = 1, AL 1.
#[hook(addr = 0x4E38C0, conv = cdecl, name = "CAEPedSpeechAudioEntity::RequestPlayerConversation", expect = "A0 E6 13 B6 00")]
pub fn speech_request_player_conversation(ped: u32) -> u8 {
    if g::<u8>(0xB613E6).read() != 0 {
        return 0;
    }
    let p: GPtr<u8> = g(ped as usize);
    if rd::<u8>(p, 0x32E) != 0 || rd::<u8>(p, 0x32D) != 0 {
        return 0;
    }
    if find_player_ped(-1) == 0 {
        return 0;
    }
    if rd::<u8>(g(find_player_ped(-1) as usize), 0x32D) != 0 {
        return 0;
    }
    if rd::<u8>(g(find_player_ped(-1) as usize), 0x32E) != 0 {
        return 0;
    }
    if g::<u8>(0xB613FD).read() != 0 || g::<u8>(0xB613FC).read() != 0 {
        return 0;
    }
    if ped_get_ped_talking(p) != 0 {
        return 0;
    }
    if speech_reserve_player_conversation_speech_slot_call() == 0 {
        return 0;
    }
    g::<u32>(0xB61400).write(ped);
    g::<u8>(0xB613FC).write(1);
    1
}

// ---- batch 7 -------------------------------------------------------------------------------------

orig! {
    /// `CAEPedSpeechAudioEntity::GetSpecificSpeechContext(int16 globalCtx, int16 audioPedType)` (0x4E4470; ECX ignored)
    fn speech_get_specific_speech_context(ctx: u32, ped_type: u32) -> u16 = stdcall 0x4E4470 => crate::ped::speech_get_specific_speech_context;
}

/// `CAEPedSpeechAudioEntity::CanPedSayGlobalContext(int16 gCtx)` (0x4E5730; entry `push esi; mov esi,
/// ecx; nop; jmp` into a `.text` chunk at 0x405FD4): AL 0 unless initialised (+0x90); the specific
/// context `GetSpecificSpeechContext(gCtx dword, ped type word +0x92 zero-extended)` must be >= 0 and,
/// per ped type (sign-extended, the bytes' jump table), the voice's (+0x94) entry must exist: type 0
/// byte table 0x8AF8A8 (`(spec * 0xD1 + voice) * 2`, != 0xFF), type 1 words 0x8BA4D0 (`spec * 0x2E`),
/// type 2 words 0x8BBEF8 (`spec * 20`), type 3 words 0x8BE620 (`spec * 0x34`) (words != 0xFFFF, rows
/// of 4 bytes); other types 0.
#[hook(addr = 0x4E5730, conv = thiscall, name = "CAEPedSpeechAudioEntity::CanPedSayGlobalContext", expect = "56 8B F1 90 E9 9B 08 F2 FF")]
pub fn speech_can_ped_say_global_context(this: GPtr<u8>, ctx: u32) -> u8 {
    if rd::<u8>(this, 0x90) == 0 {
        return 0;
    }
    let ty = rd::<u16>(this, 0x92);
    let spec = speech_get_specific_speech_context(ctx, ty as u32) as i16;
    if spec < 0 {
        return 0;
    }
    let voice = rd::<i16>(this, 0x94) as i32;
    let s = spec as i32;
    let word_at =
        |base: i32, idx: i32| g::<u16>(base.wrapping_add(idx.wrapping_mul(4)) as u32 as usize).read();
    let ok = match ty as i16 {
        0 => {
            g::<u8>((0x8AF8A8i32).wrapping_add((s * 0xD1 + voice).wrapping_mul(2)) as u32 as usize).read()
                != 0xFF
        }
        1 => word_at(0x8BA4D0, voice + s * 0x2E) != 0xFFFF,
        2 => word_at(0x8BBEF8, voice + s * 20) != 0xFFFF,
        3 => word_at(0x8BE620, voice + s * 0x34) != 0xFFFF,
        _ => false,
    };
    ok as u8
}

/// `CAEPedSpeechAudioEntity::ReleasePedConversation()` (0x4E52A0, static): with a ped conversation
/// (0xB613FD) and both slots (0xB61408, 0xB61404) >= 0: each slot whose status is 1..3 or 5 has its
/// owner (+4) `StopCurrentSpeech`; slot 1 is cleared, then slot 2 (in the bytes' order, with both slot
/// words = -1 in between) and the two conversation peds (0xB61410, 0xB6140C) = 0. The flag is cleared
/// in both cases.
/// Not hooked: L2 mismatch on air-water and police-gang (1 case each, port-l2-a2-*): a sound's physical pointer in AESoundManager stays set in the Rust run after StopCurrentSpeech; cause not found.
pub fn speech_release_ped_conversation() {
    if g::<u8>(0xB613FD).read() == 0 {
        return;
    }
    let speaking = |st: i16| (1..=3).contains(&st) || st == 5;
    if g::<i16>(0xB61408).read() >= 0 && g::<i16>(0xB61404).read() >= 0 {
        let s1 = speech_slot(g::<i16>(0xB61408).read());
        if speaking(rd::<i16>(s1, 0)) {
            speech_stop_current_speech(g(rd::<u32>(s1, 4) as usize));
        }
        clear_speech_slot(speech_slot(g::<i16>(0xB61408).read()));
        let s2 = speech_slot(g::<i16>(0xB61404).read());
        if speaking(rd::<i16>(s2, 0)) {
            speech_stop_current_speech(g(rd::<u32>(s2, 4) as usize));
        }
        let s2 = speech_slot(g::<i16>(0xB61404).read());
        wr::<u16>(s2, 0xC, 0xFFFF);
        wr::<u16>(s2, 0xE, 0xFFFF);
        wr::<u16>(s2, 0x14, 0xFFFF);
        wr::<u16>(s2, 0x16, 0xFFFF);
        g::<u16>(0xB61408).write(0xFFFF);
        g::<u16>(0xB61404).write(0xFFFF);
        wr::<u16>(s2, 0, 0);
        wr::<u32>(s2, 4, 0);
        wr::<u32>(s2, 8, 0);
        wr::<u32>(s2, 0x10, 0);
        wr::<u8>(s2, 0x19, 0);
        wr::<u8>(s2, 0x1A, 0);
        wr::<u8>(s2, 0x18, 0);
        g::<u32>(0xB61410).write(0);
        g::<u32>(0xB6140C).write(0);
    }
    g::<u8>(0xB613FD).write(0);
}

// ---- batch 8 -------------------------------------------------------------------------------------

orig! {
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    fn vehicle_terminate(this: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity()` (0x4F2B80)
    fn twin_loop_destructor(this: GPtr<u8>) = thiscall 0x4F2B80 => crate::entities::twin_loop_destructor;
    /// `CAESound::~CAESound()` (0x4EF660)
    fn sound_destructor(sound: GPtr<u8>) -> u32 = thiscall 0x4EF660 => unported;
}

/// `CAEVehicleAudioEntity::~CAEVehicleAudioEntity()` (0x6D0A10): vtable 0x862CEC; `Terminate` when
/// initialised (+0xA4); the twin-loop member (+0x184) destroyed; vtable 0x859988 (`CAEAudioEntity`) and
/// its `CAESound` (+8) destroyed. Void (the SEH frame in the original's stack is not reproduced; no
/// exception is raised on these paths; callers 0x6E2D26 and the unwind funclets ignore EAX).
#[hook(addr = 0x6D0A10, conv = thiscall, name = "CAEVehicleAudioEntity::Destructor", expect = "6A FF 68 C6 86 84 00")]
pub fn vehicle_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x862CEC);
    if rd::<u8>(this, 0xA4) != 0 {
        vehicle_terminate(this);
    }
    twin_loop_destructor(this.byte_add(0x184));
    wr::<u32>(this, 0, 0x859988);
    sound_destructor(this.byte_add(8));
}

// ---- pass 2, batch 9 -----------------------------------------------------------------------------

orig! {
    /// `FxSystem_c::GetCompositeMatrix(RwMatrix* out)` (0x4AA8C0)
    fn fx_get_composite_matrix(fx: GPtr<u8>, out: GPtr<u8>) -> u32 = thiscall 0x4AA8C0 => unported;
    /// `FxSystem_c::GetPlayStatus()` (0x4AA900)
    fn fx_get_play_status(fx: GPtr<u8>) -> u32 = thiscall 0x4AA900 => unported;
    /// `CAESound::SetPosition(CVector)` (0x4EF880, the vector by value)
    fn sound_set_position(sound: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4EF880 => unported;
}

/// `CAEFireAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DCF60). Play position > 0:
/// with an fx system (+0x84) the sound follows its composite matrix position (a 64-byte local, pos at
/// +0x30); then by the sound's state dword +0xC: 1 = fade in (+0x14 += [0x858CA0] up to +0x10; at or
/// past the target, or unordered, state = 2), 2 = fade out (+0x14 -= [0x858F34] while > [0x859CE4],
/// else `StopSoundAndForget`), 4 = fx playing -> +0x14 = -100, 3 = fx playing -> +0x14 -= 1.0 while >
/// -100. Play position -1: the fire sound pointer (+0x7C, else +0x80) equal to the sound is cleared.
/// The x87 work is single precision (24-bit PC): every intermediate is an f32.
#[hook(addr = 0x4DCF60, conv = thiscall, name = "CAEFireAudioEntity::UpdateParameters", expect = "66 8B 44 24 08")]
pub fn fire_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    let pos = pos as u16 as i16;
    if pos <= 0 {
        if pos == -1 {
            if sound == rd::<u32>(this, 0x7C) {
                wr::<u32>(this, 0x7C, 0);
            } else if sound == rd::<u32>(this, 0x80) {
                wr::<u32>(this, 0x80, 0);
            }
        }
        return;
    }
    let s: GPtr<u8> = g(sound as usize);
    let fx = rd::<u32>(this, 0x84);
    if fx != 0 {
        let mut m = [0u32; 16];
        fx_get_composite_matrix(g(fx as usize), GPtr::from_raw(m.as_mut_ptr()).cast());
        sound_set_position(s, m[12], m[13], m[14]);
    }
    match rd::<u32>(s, 0xC) {
        1 => {
            let v = rd::<f32>(s, 0x14);
            let max = rd::<f32>(s, 0x10);
            if v.partial_cmp(&max) != Some(core::cmp::Ordering::Less) {
                wr::<u32>(s, 0xC, 2);
                return;
            }
            let n = v + g::<f32>(0x858CA0).read();
            let n = if n < rd::<f32>(s, 0x10) {
                n
            } else {
                rd::<f32>(s, 0x10)
            };
            wr::<f32>(s, 0x14, n);
        }
        2 => {
            let v = rd::<f32>(s, 0x14);
            if v > g::<f32>(0x859CE4).read() {
                wr::<f32>(s, 0x14, v - g::<f32>(0x858F34).read());
            } else {
                sound_stop_and_forget(s);
            }
        }
        4 => {
            let fx = rd::<u32>(this, 0x84);
            if fx != 0 && fx_get_play_status(g(fx as usize)) == 1 {
                wr::<u32>(s, 0x14, 0xC2C8_0000);
            }
        }
        3 => {
            let fx = rd::<u32>(this, 0x84);
            if fx != 0 && fx_get_play_status(g(fx as usize)) == 1 {
                let v = rd::<f32>(s, 0x14);
                if v > g::<f32>(0x859014).read() {
                    wr::<f32>(s, 0x14, v - g::<f32>(0x858624).read());
                }
            }
        }
        _ => {}
    }
}

// ---- pass 2, batch 10 ----------------------------------------------------------------------------

orig! {
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntityAndPhysical(int16, CAEAudioEntity*, CPhysical*)` (0x4EF5D0)
    fn sm_are_sounds_of_event_playing_for_entity_and_physical(sm: GPtr<u8>, event: u32, entity: u32, physical: u32) -> u16 = thiscall 0x4EF5D0 => crate::sound::are_sounds_of_this_event_playing_for_this_entity_and_physical;
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAEAudioHardware::LoadSoundBank(uint16 bankId, int16 slot)` (0x4D88A0)
    fn hw_load_sound_bank(hw: GPtr<u8>, bank: u32, slot: u32) -> u32 = thiscall 0x4D88A0 => unported;
    /// `CAudioEngine::IsLoadingTuneActive()` (0x506D90)
    fn engine_is_loading_tune_active(engine: GPtr<u8>) -> u8 = thiscall 0x506D90 => unported;
    /// `CAESound::Initialise(...)` (0x4EFE50, 14 argument dwords)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, id: u32, entity: u32, px: u32, py: u32, pz: u32, volume: u32, max_distance: u32, speed: u32, time_scale: u32, ignored_cycles: u32, flags: u32, speed_variance: u32, play_pos: u32) = thiscall 0x4EFE50 => crate::sound::initialise;
    /// `CAESound::RegisterWithPhysicalEntity(CEntity*)` (0x4EF820)
    fn sound_register_with_physical_entity(sound: GPtr<u8>, entity: GPtr<u8>) = thiscall 0x4EF820 => crate::sound::register_with_physical_entity;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
}

const FLT_ONE_BITS: u32 = 0x3F80_0000;

/// `AEAudioEventVolumes[event]` (int8 table through the pointer at 0xBD00F8) as a float.
fn event_volume(event: u32) -> f32 {
    let t = g::<u32>(0xBD00F8).read();
    g::<i8>(t.wrapping_add(event) as usize).read() as f32
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

/// `CAEPedAudioEntity::HandleSwimWake(eAudioEvents)` (0x4E2790): unless a sound of this event is
/// already playing for this entity and ped (+0x94): bank 0x27 not in slot 2 -> loaded there (unless the
/// loading tune plays) and return without the time stamp; otherwise the temp sound (+8) is initialised
/// (slot 2, sfx 3, the ped's position, volume = event volume - [0x858BA4], 1.0 x3, zeros), +0x24 = 0.75,
/// flags word +0x5E = 0x84, event +0x14, registered with the ped and requested. Then +0x80 = now (also
/// when it was already playing).
#[hook(addr = 0x4E2790, conv = thiscall, name = "CAEPedAudioEntity::HandleSwimWake", expect = "53 8B 5C 24 08")]
pub fn ped_handle_swim_wake(this: GPtr<u8>, event: u32) {
    let ped = rd::<u32>(this, 0x94);
    if sm_are_sounds_of_event_playing_for_entity_and_physical(g(0xB62CB0), event, this.addr() as u32, ped)
        == 0
    {
        if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x27, 2) == 0 {
            if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
                hw_load_sound_bank(g(0xB5F8B8), 0x27, 2);
            }
            return;
        }
        let vol = event_volume(event) - g::<f32>(0x858BA4).read();
        let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
        let snd = this.byte_add(8);
        sound_initialise(
            snd,
            2,
            3,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            vol.to_bits(),
            FLT_ONE_BITS,
            FLT_ONE_BITS,
            FLT_ONE_BITS,
            0,
            0,
            0,
            0,
        );
        wr::<u32>(this, 0x24, 0x3F40_0000);
        wr::<u16>(this, 0x5E, 0x84);
        wr::<u32>(this, 0x14, event);
        sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
        sm_request_new_sound(g(0xB62CB0), snd);
    }
    wr::<u32>(this, 0x80, sa_core::timer::TIME_MS.get());
}

// ---- pass 2, batch 11 ----------------------------------------------------------------------------

orig! {
    /// `SurfaceInfos_c::IsAudioWater(uint32 surface)` (0x55EAD0, `g_surfaceInfos` 0xB79538)
    fn surface_is_audio_water(infos: GPtr<u8>, surface: u32) -> u32 = thiscall 0x55EAD0 => unported;
    /// `CAESound::SetFlags(uint16 flags, uint16 enabled)` (0x4EF2B0)
    fn sound_set_flags(sound: GPtr<u8>, flags: u32, enabled: u32) = thiscall 0x4EF2B0 => unported;
}

/// `CAEPedAudioEntity::HandleSwimSplash(eAudioEvents)` (0x4E26A0): bank 0x80 not in slot 0x20 -> loaded
/// there, return. Otherwise the splash counter word +0x7E steps (past 4 -> 0) and is the sfx: the temp
/// sound (+8) is initialised (slot 0x20, the ped's position, event volume, 1.0 x3, zeros), +0x28 =
/// 0x3D70D845, `SetFlags(0x80, 1)`, registered with the ped (+0x94), requested. The sfx dword pushed by
/// the original keeps the upper half of the event-volume table pointer unless the counter wrapped
/// (reproduced).
#[hook(addr = 0x4E26A0, conv = thiscall, name = "CAEPedAudioEntity::HandleSwimSplash", expect = "53 56 57 6A 20")]
pub fn ped_handle_swim_splash(this: GPtr<u8>, event: u32) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x80, 0x20) == 0 {
        hw_load_sound_bank(g(0xB5F8B8), 0x80, 0x20);
        return;
    }
    let table = g::<u32>(0xBD00F8).read();
    let vol = event_volume(event);
    let c = rd::<u16>(this, 0x7E).wrapping_add(1);
    let sfx = if (c as i16) > 4 {
        0
    } else {
        (table & 0xFFFF_0000) | c as u32
    };
    wr::<u16>(this, 0x7E, sfx as u16);
    let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        0x20,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0,
        0,
        0,
    );
    wr::<u32>(this, 0x28, 0x3D70_D845);
    sound_set_flags(snd, 0x80, 1);
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEPedAudioEntity::HandleLandingEvent(eAudioEvents)` (0x4E18E0): nothing when the ped's flags dword
/// +0x46C has 0x200. Water surface (ped byte +0xBA, `IsAudioWater`): bank 0x80 must be in slot 0x20
/// (else loaded, return); sfx = `GetRandomNumberInRange(0, 4)`, play position 0x32, volume = event
/// volume when > 0 else 0. Otherwise bank 0 must be in slot 0x29 (else loaded, return); sfx = 6 for
/// event 0x3A else 0, play position 0. The temp sound (+8) is initialised (the ped's position, 1.0 x3,
/// cycles 0, flags 0xA0, speed variance 0x3D70D845), registered with the ped and requested.
#[hook(addr = 0x4E18E0, conv = thiscall, name = "CAEPedAudioEntity::HandleLandingEvent", expect = "53 55 56 57 8B F9")]
pub fn ped_handle_landing_event(this: GPtr<u8>, event: u32) {
    let ped: GPtr<u8> = g(rd::<u32>(this, 0x94) as usize);
    if rd::<u32>(ped, 0x46C) & 0x200 != 0 {
        return;
    }
    let mut vol = event_volume(event);
    let (slot, sfx, play_pos) = if surface_is_audio_water(g(0xB79538), rd::<u8>(ped, 0xBA) as u32) as u8 != 0
    {
        if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x80, 0x20) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x80, 0x20);
            return;
        }
        let r = crate::common::ae_random_int(0, 4) as u32;
        if vol.partial_cmp(&g::<f32>(0x858B50).read()) != Some(core::cmp::Ordering::Greater) {
            vol = 0.0;
        }
        (0x20u32, r, 0x32u32)
    } else {
        if hw_is_sound_bank_loaded(g(0xB5F8B8), 0, 0x29) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0, 0x29);
            return;
        }
        (0x29u32, if event == 0x3A { 6 } else { 0 }, 0u32)
    };
    let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0xA0,
        0x3D70_D845,
        play_pos,
    );
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEPedAudioEntity::HandleSkateEvent(eAudioEvents, float volumeOffset, float speed)` (0x4E17E0):
/// nothing when the ped's flags dword +0x46C has 0x200. Volume = event volume + offset, then
/// `GetRandomNumberInRange(-3, 3)` + volume (drawn before the bank check). Bank 0 must be in slot 0x29
/// (else loaded, return). The temp sound (+8) is initialised (slot 0x29, sfx 7 for event 0x38 else 8,
/// the ped's position, volume, 1.0, `speed`, 1.0, cycles 0, flags 0x80, variance 0x3D70D845, 0),
/// registered with the ped and requested.
#[hook(addr = 0x4E17E0, conv = thiscall, name = "CAEPedAudioEntity::HandleSkateEvent", expect = "53 56 57 8B F9")]
pub fn ped_handle_skate_event(this: GPtr<u8>, event: u32, vol_offset: f32, speed: u32) {
    let ped: GPtr<u8> = g(rd::<u32>(this, 0x94) as usize);
    if rd::<u32>(ped, 0x46C) & 0x200 != 0 {
        return;
    }
    let vol = event_volume(event) + vol_offset;
    let r = crate::common::ae_random_int(-3, 3);
    let vol = r as f32 + vol;
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0, 0x29) == 0 {
        hw_load_sound_bank(g(0xB5F8B8), 0, 0x29);
        return;
    }
    let sfx = (event != 0x38) as u32 + 7;
    let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        0x29,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        speed,
        FLT_ONE_BITS,
        0,
        0x80,
        0x3D70_D845,
        0,
    );
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEFireAudioEntity::PlayWaterSounds(eAudioEvents, CVector& posn)` (0x4DD270): a `CAESound` local (only
/// its +8 physical pointer cleared, as the original; here the whole local starts zeroed, the original's
/// other unwritten bytes are stack garbage copied by `RequestNewSound`) is initialised twice and
/// requested: (slot 2, sfx 3, `posn`, event volume, 2.0, 0.75, 0.6, 0, flags 4, 0, 0) with event 3 and
/// +0x20 = 0x3D75C290 -> +0x7C; then (slot 0, sfx 0, volume + [0x858BA4], 2.0, 1.78, 0.6, 0, 4, 0, 0)
/// with event 4 -> +0x80; the local is destroyed. The SEH frame is not reproduced.
#[hook(addr = 0x4DD270, conv = thiscall, name = "CAEFireAudioEntity::PlayWaterSounds", expect = "6A FF 68 18 BF 83 00")]
pub fn fire_play_water_sounds(this: GPtr<u8>, event: u32, posn: u32) {
    let mut buf = [0u32; 0x74 / 4];
    let snd: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
    wr::<u32>(snd, 8, 0);
    let vol = event_volume(event);
    let p: GPtr<u8> = g(posn as usize);
    let me = this.addr() as u32;
    sound_initialise(
        snd,
        2,
        3,
        me,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol.to_bits(),
        0x4000_0000,
        0x3F40_0000,
        0x3F19_999A,
        0,
        4,
        0,
        0,
    );
    wr::<u32>(snd, 0xC, 3);
    wr::<u32>(snd, 0x20, 0x3D75_C290);
    let r = sm_request_new_sound(g(0xB62CB0), snd);
    let vol2 = vol + g::<f32>(0x858BA4).read();
    wr::<u32>(this, 0x7C, r);
    sound_initialise(
        snd,
        0,
        0,
        me,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol2.to_bits(),
        0x4000_0000,
        0x3FE3_D70A,
        0x3F19_999A,
        0,
        4,
        0,
        0,
    );
    wr::<u32>(snd, 0x20, 0x3D75_C290);
    wr::<u32>(snd, 0xC, 4);
    let r = sm_request_new_sound(g(0xB62CB0), snd);
    wr::<u32>(this, 0x80, r);
    sound_destructor(snd);
}

// ---- pass 3: collision ---------------------------------------------------------------------------

/// `CAECollisionAudioEntity::GetCollisionSoundStatus(CEntity* a, CEntity* b, eSurfaceType, eSurfaceType,
/// int32* outIndex)` (0x4DA830, `ret 0x14`): scans the 300 entries (0x14 bytes from +0x208) for the pair
/// in either order; a match in state 2 stops the scan with `*out` = its index and AL = 2; a match in
/// another state only sets AL to that state (the scan goes on). No state-2 match: `*out` = 300, AL = the
/// last matching entry's state (0 when none). The surfaces are unused.
#[hook(addr = 0x4DA830, conv = thiscall, name = "CAECollisionAudioEntity::GetCollisionSoundStatus", expect = "8B 54 24 04 53")]
pub fn collision_get_collision_sound_status(
    this: GPtr<u8>,
    a: u32,
    b: u32,
    _s1: u32,
    _s2: u32,
    out: GPtr<u8>,
) -> u8 {
    let mut al = 0u8;
    for i in 0..300u32 {
        let e = this.byte_add(0x208 + i as isize * 0x14);
        let (e0, e4) = (rd::<u32>(e, 0), rd::<u32>(e, 4));
        if (e0 == a && e4 == b) || (e0 == b && e4 == a) {
            al = rd::<u8>(e, 0x10);
            if al == 2 {
                wr::<u32>(out, 0, i);
                return al;
            }
        }
    }
    wr::<u32>(out, 0, 300);
    al
}

// ---- pass 3: vehicle -----------------------------------------------------------------------------

orig! {
    /// `CAEVehicleAudioEntity::PlaySkidSound(int16 sfx, float speed, float volume)` (0x4F8360)
    fn vehicle_play_skid_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4F8360 => unported;
    /// `CAEVehicleAudioEntity::PlayTrainBrakeSound(int16 sfx, float speed, float volume)` (0x4FA630)
    fn vehicle_play_train_brake_sound(this: GPtr<u8>, sfx: u32, speed: u32, volume: u32) = thiscall 0x4FA630 => unported;
    /// `CAudioEngine::StopRadio(tVehicleAudioSettings*, bool)` (0x506F70)
    fn engine_stop_radio(engine: GPtr<u8>, settings: u32, duck: u32) = thiscall 0x506F70 => crate::engine::stop_radio;
}

/// `SetFlags(4, 0)` + `StopSound` on the sound at `this + off` (when set), which is then cleared.
fn vehicle_stop_engine_sound(this: GPtr<u8>, off: isize) -> bool {
    let s = rd::<u32>(this, off);
    if s == 0 {
        return false;
    }
    sound_set_flags(g(s as usize), 4, 0);
    sound_stop_sound(g(rd::<u32>(this, off) as usize));
    wr::<u32>(this, off, 0);
    true
}

/// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0): when initialised (+0xA4): the 12 engine sounds
/// (+0xE8, stride 8) and the four loop sounds +0x158/+0x160/+0x168/+0x170 get `SetFlags(4, 0)` +
/// `StopSound` and are cleared (the loops' type words +0x156/+0x15C/+0x164/+0x16C = -1); skid and
/// train-brake sounds stopped (`Play*Sound(-1, 1.0, -100.0)`); +0x178/+0x17C/+0x180
/// `StopSoundAndForget`; a player-driven (+0xA5) vehicle with radio type (+0x9B) 0, 2 or 3 stops the
/// radio (`StopRadio(this+0x80, 0)`); its dummy bank slot (+0xE0) loses a reference when the slot
/// (minus 7, 0..10) still holds its bank (+0xDC) and is reset to -1; +0xDC/+0xDE = -1, entity +4 = 0,
/// +0xA9 = 0; then the player-driver (+0xA5: also `s_pPlayerDriver` 0xB6B990) or passenger (+0xA6)
/// flag clears `s_pPlayerAttachedForRadio` / `s_pVehicleAudioSettingsForRadio`; +0xA7 = +0xA4 = 0.
#[hook(addr = 0x4FB8C0, conv = thiscall, name = "CAEVehicleAudioEntity::Terminate", expect = "53 56 8B F1 8A 86 A4 00 00 00")]
pub fn vehicle_terminate_port(this: GPtr<u8>) {
    if rd::<u8>(this, 0xA4) == 0 {
        return;
    }
    for i in 0..12isize {
        vehicle_stop_engine_sound(this, 0xE8 + i * 8);
    }
    vehicle_play_skid_sound(this, 0xFFFF_FFFF, FLT_ONE_BITS, 0xC2C8_0000);
    vehicle_play_train_brake_sound(this, 0xFFFF_FFFF, FLT_ONE_BITS, 0xC2C8_0000);
    for (snd, ty) in [
        (0x158isize, 0x156isize),
        (0x160, 0x15C),
        (0x168, 0x164),
        (0x170, 0x16C),
    ] {
        if vehicle_stop_engine_sound(this, snd) {
            wr::<u16>(this, ty, 0xFFFF);
        }
    }
    for off in [0x178isize, 0x17C, 0x180] {
        let s = rd::<u32>(this, off);
        if s != 0 {
            sound_stop_and_forget(g(s as usize));
            wr::<u32>(this, off, 0);
        }
    }
    if rd::<u8>(this, 0xA5) != 0 {
        let rt = rd::<u8>(this, 0x9B) as i8;
        if rt == 0 || rt == 2 || rt == 3 {
            engine_stop_radio(g(0xB6BC90), this.addr() as u32 + 0x80, 0);
        }
    }
    let slot = rd::<u16>(this, 0xE0);
    if slot != 0xFFFF {
        let i = slot.wrapping_sub(7) as i16;
        if (0..10).contains(&i) && g::<u16>(0xB6B9A0 + i as usize * 4).read() == rd::<u16>(this, 0xDC) {
            let c = g::<i16>(0xB6B9A2 + i as usize * 4);
            let v = c.read();
            c.write(if v > 0 { v - 1 } else { 0 });
        }
        wr::<u16>(this, 0xE0, 0xFFFF);
    }
    let driver = rd::<u8>(this, 0xA5);
    wr::<u16>(this, 0xDC, 0xFFFF);
    wr::<u16>(this, 0xDE, 0xFFFF);
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 0xA9, 0);
    if driver != 0 {
        wr::<u8>(this, 0xA5, 0);
        g::<u32>(0xB6B990).write(0);
        g::<u32>(0xB6B98C).write(0);
        g::<u32>(0xB6B99C).write(0);
    } else if rd::<u8>(this, 0xA6) != 0 {
        wr::<u8>(this, 0xA6, 0);
        g::<u32>(0xB6B98C).write(0);
        g::<u32>(0xB6B99C).write(0);
    }
    wr::<u8>(this, 0xA7, 0);
    wr::<u8>(this, 0xA4, 0);
}

orig! {
    /// `CAEVehicleAudioEntity::CancelVehicleEngineSound(int16)` (0x4F55C0)
    fn vehicle_cancel_vehicle_engine_sound(this: GPtr<u8>, ty: u32) = thiscall 0x4F55C0 => crate::vehicle::cancel_vehicle_engine_sound;
}

/// `CAEVehicleAudioEntity::JustWreckedVehicle()` (0x4FD0B0): a player-driven (+0xA5) vehicle with radio
/// type (+0x9B) 0, 2 or 3 stops the radio, and `s_pPlayerAttachedForRadio` / `s_pVehicleAudioSettingsForRadio`
/// are cleared (any radio type). Audio types (+0x80, signed) 0..9 except 7 cancel engine sounds 0..11
/// (the bytes' jump table). Then `PlaySkidSound(-1, 1.0, -100.0)`; the loop sounds +0x158/+0x160/
/// +0x168/+0x170 (type words = -1) and +0x178/+0x17C/+0x180 get `SetFlags(4, 0)` + `StopSound` and are
/// cleared.
#[hook(addr = 0x4FD0B0, conv = thiscall, name = "CAEVehicleAudioEntity::JustWreckedVehicle", expect = "53 56 8B F1 8A 86 A5 00 00 00")]
pub fn vehicle_just_wrecked_vehicle(this: GPtr<u8>) {
    if rd::<u8>(this, 0xA5) != 0 {
        let rt = rd::<u8>(this, 0x9B) as i8;
        if rt == 0 || rt == 2 || rt == 3 {
            engine_stop_radio(g(0xB6BC90), this.addr() as u32 + 0x80, 0);
        }
        g::<u32>(0xB6B98C).write(0);
        g::<u32>(0xB6B99C).write(0);
    }
    let ty = rd::<u8>(this, 0x80) as i8;
    if (0..=9).contains(&ty) && ty != 7 {
        for i in 0..12u32 {
            vehicle_cancel_vehicle_engine_sound(this, i);
        }
    }
    vehicle_play_skid_sound(this, 0xFFFF_FFFF, FLT_ONE_BITS, 0xC2C8_0000);
    for (snd, ty) in [
        (0x158isize, 0x156isize),
        (0x160, 0x15C),
        (0x168, 0x164),
        (0x170, 0x16C),
    ] {
        if vehicle_stop_engine_sound(this, snd) {
            wr::<u16>(this, ty, 0xFFFF);
        }
    }
    for off in [0x178isize, 0x17C, 0x180] {
        vehicle_stop_engine_sound(this, off);
    }
}

// ---- pass 3: speech ------------------------------------------------------------------------------

/// Voice-name tables per audio ped type (20-byte names): [start, end).
const VOICE_NAME_TABLES: [(usize, usize); 5] = [
    (0x8AE6A8, 0x8AF6FC),
    (0x8BA0D8, 0x8BA470),
    (0x8BBD40, 0x8BBED0),
    (0x8BE1A8, 0x8BE5B8),
    (0x8C4120, 0x8C4288),
];

/// The inline `strcmp` of the bytes: equal (including the terminator).
fn cstr_eq(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(a, i);
        if c != rd::<u8>(b, i) {
            return false;
        }
        if c == 0 {
            return true;
        }
        i += 1;
    }
}

/// `CAEPedSpeechAudioEntity::GetVoice(const char* name, int16 type)` (0x4E3CD0, static cdecl): AX = the
/// index of `name` in the type's voice-name table (types 0..4, the bytes' jump table), else -1.
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX (port-l2-a2-menus-hud mismatch), needs an AX return kind in sa-core::abi.
pub fn speech_get_voice(name: GPtr<u8>, ty: u32) -> u16 {
    let t = ty as u16 as i16 as i32 as u32;
    if t > 4 {
        return 0xFFFF;
    }
    let (start, end) = VOICE_NAME_TABLES[t as usize];
    let mut p = start;
    let mut i = 0u16;
    while p < end {
        if cstr_eq(name, g(p)) {
            return i;
        }
        p += 20;
        i = i.wrapping_add(1);
    }
    0xFFFF
}

// ---- pass 3: collision (2) -----------------------------------------------------------------------

/// `ReportObjectDestruction` jump-table index bytes (`.text`, 0x85 entries for surfaces 0x2A..0xAE, read
/// in place): 0 = random glass sfx 0x29..0x2C, 1 = sfx 0xB, 2 = nothing.
const OBJECT_DESTRUCTION_INDEX: usize = 0x4DADAC;

/// `CAECollisionAudioEntity::ReportObjectDestruction(CEntity*)` (0x4DAB60): nothing for a null entity.
/// Model 0x158 uses surface 0x2D. The model in the word at 0x8CD6F4 plays two temp sounds (+8): (slot 2,
/// sfx 0x38, event-0x7D volume, 1.5, 0.75, 1.0, zeros), then (slot 2, sfx `GetRandomNumberInRange(0xF,
/// 0x12)`, event-0x80 volume, 1.5, 0.56, 1.0, zeros) with +0x24 = 0, +0x18 = (float)(uint32)(now + 600),
/// event +0x14 = 0x80, flags word +0x5E = 4. Other models take the surface of their collision data's
/// first sphere (+8 -> +0x10), box (+0xC -> +0x18) or triangle (+0x18 -> +6); no col data / none of them
/// -> nothing. Surfaces mapped by the bytes' jump table play (slot 2, sfx, event-0x74 volume, 2.0, 1.0,
/// 1.0, 0, 0, 0x3D70D845, 0) at the entity's position.
#[hook(addr = 0x4DAB60, conv = thiscall, name = "CAECollisionAudioEntity::ReportObjectDestruction", expect = "53 8B 5C 24 08")]
pub fn collision_report_object_destruction(this: GPtr<u8>, entity: u32) {
    if entity == 0 {
        return;
    }
    let ent: GPtr<u8> = g(entity as usize);
    let me = this.addr() as u32;
    let snd = this.byte_add(8);
    let model = rd::<u16>(ent, 0x22);
    let surface: u8 = if model == 0x158 {
        0x2D
    } else if model as i16 as i32 == g::<u16>(0x8CD6F4).read() as i32 {
        let p = entity_pos(ent);
        let vol = event_volume(0x7D);
        sound_initialise(
            snd,
            2,
            0x38,
            me,
            p[0],
            p[1],
            p[2],
            vol.to_bits(),
            0x3FC0_0000,
            0x3F40_0000,
            FLT_ONE_BITS,
            0,
            0,
            0,
            0,
        );
        sm_request_new_sound(g(0xB62CB0), snd);
        let p = entity_pos(ent);
        let r = crate::common::ae_random_int(0xF, 0x12) as u32;
        let vol = event_volume(0x80);
        sound_initialise(
            snd,
            2,
            r,
            me,
            p[0],
            p[1],
            p[2],
            vol.to_bits(),
            0x3FC0_0000,
            0x3F0F_5C29,
            FLT_ONE_BITS,
            0,
            0,
            0,
            0,
        );
        wr::<u32>(this, 0x24, 0);
        let t = sa_core::timer::TIME_MS.get().wrapping_add(600);
        wr::<f32>(this, 0x18, t as f32);
        wr::<u32>(this, 0x14, 0x80);
        wr::<u16>(this, 0x5E, 4);
        sm_request_new_sound(g(0xB62CB0), snd);
        return;
    } else {
        let mi = g::<u32>(
            (MODEL_INFO_PTRS as u32).wrapping_add((model as i16 as i32 as u32).wrapping_mul(4)) as usize,
        )
        .read();
        let col = rd::<u32>(g(mi as usize), 0x14);
        let data = rd::<u32>(g(col as usize), 0x2C);
        if data == 0 {
            return;
        }
        let d: GPtr<u8> = g(data as usize);
        if rd::<i16>(d, 0) > 0 {
            rd::<u8>(g(rd::<u32>(d, 8) as usize), 0x10)
        } else if rd::<i16>(d, 2) > 0 {
            rd::<u8>(g(rd::<u32>(d, 0xC) as usize), 0x18)
        } else if rd::<i16>(d, 4) > 0 {
            rd::<u8>(g(rd::<u32>(d, 0x18) as usize), 6)
        } else {
            return;
        }
    };
    let idx = (surface as u32).wrapping_sub(0x2A);
    if idx > 0x84 {
        return;
    }
    let sfx = match g::<u8>(OBJECT_DESTRUCTION_INDEX + idx as usize).read() {
        0 => {
            let r = crate::common::ae_random_int(0x29, 0x2C);
            if (r as i16) < 0 {
                return;
            }
            r as u32
        }
        1 => 0xB,
        _ => return,
    };
    let vol = event_volume(0x74);
    let p = entity_pos(ent);
    sound_initialise(
        snd,
        2,
        sfx,
        me,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        0x4000_0000,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0,
        0x3D70_D845,
        0,
    );
    sm_request_new_sound(g(0xB62CB0), snd);
}

// ---- pass 3: ped (2) -----------------------------------------------------------------------------

/// `HandlePedSwing` weapon-type jump-table index bytes (`.text`, types 0..0x2D, read in place).
const PED_SWING_WEAPON_INDEX: usize = 0x4E1C7C;

/// `CAEPedAudioEntity::HandlePedSwing(eAudioEvents, int32 animSfx, uint32 delayMs)` (0x4E1A40): bank
/// 0x8F must be in slot 5 (else loaded unless the loading tune plays; return). From the ped's current
/// weapon type (`ped + 0x5A0 + slot(+0x718) * 0x1C`, types 0..0x2D through the bytes' table): sfx / volume
/// offset / speed = 0x57 / -6 / 0.84, 0x2F / 0 / 1, 0x56 / 0 / 0.84, 0x58 / -3 / 1, 0x56 / 0 / 0.67,
/// 0x56 / -6 / 0.84, or none (sfx -1). `animSfx` 0x40..0x42 forces 0x57 / -6 / 0.84, 0x43..0x44 0x57 /
/// -2 / 0.84; otherwise no sfx -> nothing. The temp sound (+8): (slot 5, sfx, the ped's position, event
/// volume + offset, 1.0, speed, 1.0, 0, flags 0x80, 0.04, 0), registered with the ped; a non-zero delay
/// gives +0x28 = +0x24 = 0, +0x18 = (float)(uint32)(now + delay), event +0x14, `SetFlags(4, 1)`; then
/// requested.
#[hook(addr = 0x4E1A40, conv = thiscall, name = "CAEPedAudioEntity::HandlePedSwing", expect = "83 EC 08 53 55")]
pub fn ped_handle_ped_swing(this: GPtr<u8>, event: u32, anim_sfx: u32, delay: u32) {
    let ped = rd::<u32>(this, 0x94);
    let slot = rd::<u8>(g(ped as usize), 0x718) as i8 as i32;
    let weapon = (slot.wrapping_mul(0x1C) as u32)
        .wrapping_add(ped)
        .wrapping_add(0x5A0);
    if weapon == 0 {
        return;
    }
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    const SPEED_084: u32 = 0x3F57_0A3D;
    let ev_vol = event_volume(event);
    let wtype = g::<u32>(weapon as usize).read();
    let mut sfx: u32 = 0xFFFF_FFFF;
    let mut offset: f32 = 0.0;
    let mut speed: u32 = FLT_ONE_BITS;
    if wtype <= 0x2D {
        match g::<u8>(PED_SWING_WEAPON_INDEX + wtype as usize).read() {
            0 => (sfx, offset, speed) = (0x57, g::<f32>(0x85A5C0).read(), SPEED_084),
            1 => sfx = 0x2F,
            2 => (sfx, speed) = (0x56, SPEED_084),
            3 => (sfx, offset) = (0x58, g::<f32>(0x859024).read()),
            4 => (sfx, speed) = (0x56, 0x3F2B_851F),
            5 => (sfx, offset, speed) = (0x56, g::<f32>(0x85A5C0).read(), SPEED_084),
            _ => {}
        }
    }
    match anim_sfx.wrapping_sub(0x40) {
        0..=2 => (sfx, offset, speed) = (0x57, g::<f32>(0x85A5C0).read(), SPEED_084),
        3 | 4 => (sfx, offset, speed) = (0x57, g::<f32>(0x858B18).read(), SPEED_084),
        _ => {
            if (sfx as i16) < 0 {
                return;
            }
        }
    }
    let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
    let vol = ev_vol + offset;
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        speed,
        FLT_ONE_BITS,
        0,
        0x80,
        0x3D23_D70A,
        0,
    );
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
    if delay != 0 {
        wr::<u32>(this, 0x28, 0);
        wr::<u32>(this, 0x24, 0);
        let t = sa_core::timer::TIME_MS.get().wrapping_add(delay);
        wr::<f32>(this, 0x18, t as f32);
        wr::<u32>(this, 0x14, event);
        sound_set_flags(snd, 4, 1);
    }
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEPedAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4E1180): for the sounds +0xA0,
/// +0x9C, +0xA4: position -1 clears the pointer, otherwise the sound follows the ped (+0x94, when set).
/// +0x150: position -1 clears it, otherwise nothing. Other sounds with position >= 0, by event (+0xC,
/// the bytes' table): 0x3C..0x49 -> once `now >= ftol(sound+0x10)`, speed +0x1C = 1.0; 0x4C -> volume
/// (+0x14) follows the event volume: after `this+0x80 + 100` ms fading down by [0x858CC8] towards
/// event volume - [0x858BA4] (at or below it, or unordered: `StopSoundAndForget` and +0x80 = 0), before
/// that fading up by the same step to the event volume.
#[hook(addr = 0x4E1180, conv = thiscall, name = "CAEPedAudioEntity::UpdateParameters", expect = "56 8B F1 8B 86 A0 00 00 00")]
pub fn ped_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    let pos = pos as u16 as i16;
    let s: GPtr<u8> = g(sound as usize);
    for off in [0xA0isize, 0x9C, 0xA4] {
        if sound == rd::<u32>(this, off) {
            if pos == -1 {
                wr::<u32>(this, off, 0);
                return;
            }
            let ped = rd::<u32>(this, 0x94);
            if ped != 0 {
                let p = entity_pos(g(ped as usize));
                sound_set_position(s, p[0], p[1], p[2]);
            }
            return;
        }
    }
    if sound == rd::<u32>(this, 0x150) {
        if pos == -1 {
            wr::<u32>(this, 0x150, 0);
        }
        return;
    }
    if pos < 0 {
        return;
    }
    let ev = rd::<u32>(s, 0xC).wrapping_sub(0x3C);
    if ev <= 0xD {
        let t = crate::common::ftol32(crate::common::x(rd::<f32>(s, 0x10))) as u32;
        if sa_core::timer::TIME_MS.get() >= t {
            wr::<u32>(s, 0x1C, FLT_ONE_BITS);
        }
        return;
    }
    if ev != 0x10 {
        return;
    }
    let mut target = event_volume(0x4C);
    let now = sa_core::timer::TIME_MS.get();
    let step = g::<f32>(0x858CC8).read();
    if now > rd::<u32>(this, 0x80).wrapping_add(100) {
        target -= g::<f32>(0x858BA4).read();
        let v = rd::<f32>(s, 0x14);
        if v > target {
            let n = v - step;
            wr::<f32>(s, 0x14, if n > target { n } else { target });
        } else {
            sound_stop_and_forget(s);
            wr::<u32>(this, 0x80, 0);
        }
    } else {
        let v = rd::<f32>(s, 0x14);
        if v < target {
            let n = v + step;
            wr::<f32>(s, 0x14, if n < target { n } else { target });
        }
    }
}

// ---- pass 3: global weapon -----------------------------------------------------------------------

/// `CAEGlobalWeaponAudioEntity::ProjectileFire(eWeaponType, CPhysical*, int32 event)` (0x4DF060): only
/// types 0x13..0x14. The variation byte +0xA4 = (signed +1) % 3. Bank 0x8F must be in slot 5 (else
/// loaded unless the loading tune plays; return): temp sound (+8) (slot 5, sfx 0x44, the physical's
/// position, event volume - [0x859000], 3.0, speed = dword [0x8AE5AC + v*4], 1.0, 0, flags 0x80, 0.02,
/// 0), registered with the physical, requested. Bank 0x8A must be in slot 0x13 (else loaded; return):
/// (slot 0x13, sfx 0x1A, position re-read, event volume, 3.0, speed * [0x8595F0], 1.0, 0, 0x80, 0.02,
/// 0), registered, requested.
#[hook(addr = 0x4DF060, conv = thiscall, name = "CAEGlobalWeaponAudioEntity::ProjectileFire", expect = "8B 44 24 0C 53")]
pub fn global_weapon_projectile_fire(this: GPtr<u8>, ty: u32, physical: u32, event: u32) {
    let vol = event_volume(event);
    if !(0x13..=0x14).contains(&(ty as i32)) {
        return;
    }
    let v = ((rd::<u8>(this, 0xA4) as i8 as i32 + 1) % 3) as i8;
    wr::<u8>(this, 0xA4, v as u8);
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let phys: GPtr<u8> = g(physical as usize);
    let me = this.addr() as u32;
    let snd = this.byte_add(8);
    let speed_at = (0x8AE5ACi32 + v as i32 * 4) as u32 as usize;
    let p = entity_pos(phys);
    let vol1 = vol - g::<f32>(0x859000).read();
    let speed = g::<u32>(speed_at).read();
    sound_initialise(
        snd,
        5,
        0x44,
        me,
        p[0],
        p[1],
        p[2],
        vol1.to_bits(),
        0x4040_0000,
        speed,
        FLT_ONE_BITS,
        0,
        0x80,
        0x3CA3_D70A,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    sm_request_new_sound(g(0xB62CB0), snd);
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8A, 0x13) == 0 {
        hw_load_sound_bank(g(0xB5F8B8), 0x8A, 0x13);
        return;
    }
    let p = entity_pos(phys);
    let speed2 = g::<f32>(speed_at).read() * g::<f32>(0x8595F0).read();
    sound_initialise(
        snd,
        0x13,
        0x1A,
        me,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        0x4040_0000,
        speed2.to_bits(),
        FLT_ONE_BITS,
        0,
        0x80,
        0x3CA3_D70A,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    sm_request_new_sound(g(0xB62CB0), snd);
}

// ---- pass 3: water cannon ------------------------------------------------------------------------

orig! {
    /// `CAEWaterCannonAudioEntity::UpdateGenericWaterCannonSound(bool, int16 id, int16 slot, int16 sfx,
    /// float a, float b, CVector posn, float c)` (0x502F80; protection code inside, not ported)
    fn water_cannon_update_generic_sound(this: GPtr<u8>, on: u32, id: u32, slot: u32, sfx: u32, a: u32, b: u32, px: u32, py: u32, pz: u32, c: u32) = thiscall 0x502F80 => unported;
}

/// `CAEWaterCannonAudioEntity::Service()` (0x5030D0): when initialised (+0x7C) with bank 0x3B in slot 0
/// and bank 0x27 in slot 2: "on" = the cannon's (+4) active byte for its current index (word +4,
/// `cannon + idx + 0x30C`) and its non-zero first dword; `UpdateGenericWaterCannonSound` four times:
/// (on, 0, 0, 0, [0x8CC27C], [0x8CC278], cannon pos[idx] (+0xC + idx*12), [0x8CC280]), (on, 1, 2, 3,
/// [0x8CC288], [0x8CC284], same pos, [0x8CC28C]), (+0x7D, 2, 2, 3, [0x8CC294], [0x8CC290], +0x80 pos,
/// [0x8CC298]), (+0x7D, 3, 2, 3, [0x8CC294] * [0x858B8C], [0x8CC290] - [0x858B44], +0x80 pos,
/// [0x8CC298]). The "on" dword of the first two calls is the original's `push ecx` slot with only its low
/// byte written: `(this & ~0xFF) | on` (reproduced).
#[hook(addr = 0x5030D0, conv = thiscall, name = "CAEWaterCannonAudioEntity::Service", expect = "51 53 56 8B F1")]
pub fn water_cannon_service(this: GPtr<u8>) {
    let cannon: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u8>(this, 0x7C) == 0
        || hw_is_sound_bank_loaded(g(0xB5F8B8), 0x3B, 0) == 0
        || hw_is_sound_bank_loaded(g(0xB5F8B8), 0x27, 2) == 0
    {
        return;
    }
    let idx = rd::<i16>(cannon, 4) as isize;
    let on = (rd::<u8>(cannon, idx + 0x30C) != 0 && rd::<u32>(cannon, 0) != 0) as u32;
    let on = (this.addr() as u32 & !0xFF) | on;
    let gd = |a: usize| g::<u32>(a).read();
    let p = cannon.byte_add(0xC + idx * 12);
    let pv = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    water_cannon_update_generic_sound(
        this,
        on,
        0,
        0,
        0,
        gd(0x8CC27C),
        gd(0x8CC278),
        pv[0],
        pv[1],
        pv[2],
        gd(0x8CC280),
    );
    let idx = rd::<i16>(cannon, 4) as isize;
    let p = cannon.byte_add((idx + 1) * 12);
    let pv = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    water_cannon_update_generic_sound(
        this,
        on,
        1,
        2,
        3,
        gd(0x8CC288),
        gd(0x8CC284),
        pv[0],
        pv[1],
        pv[2],
        gd(0x8CC28C),
    );
    let pv = [
        rd::<u32>(this, 0x80),
        rd::<u32>(this, 0x84),
        rd::<u32>(this, 0x88),
    ];
    water_cannon_update_generic_sound(
        this,
        rd::<u8>(this, 0x7D) as u32,
        2,
        2,
        3,
        gd(0x8CC294),
        gd(0x8CC290),
        pv[0],
        pv[1],
        pv[2],
        gd(0x8CC298),
    );
    let pv = [
        rd::<u32>(this, 0x80),
        rd::<u32>(this, 0x84),
        rd::<u32>(this, 0x88),
    ];
    let b = g::<f32>(0x8CC290).read() - g::<f32>(0x858B44).read();
    let a = g::<f32>(0x8CC294).read() * g::<f32>(0x858B8C).read();
    water_cannon_update_generic_sound(
        this,
        rd::<u8>(this, 0x7D) as u32,
        3,
        2,
        3,
        a.to_bits(),
        b.to_bits(),
        pv[0],
        pv[1],
        pv[2],
        gd(0x8CC298),
    );
}

// ---- pass 3: speech (2) --------------------------------------------------------------------------

orig! {
    /// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForInterupting(int16)` (0x4E4600, `ret 4`,
    /// ECX/EDX preserved; unhooked)
    fn speech_is_important_for_interupting(ctx: u32) -> u8 = stdcall 0x4E4600 => unported;
}

/// `CAEPedSpeechAudioEntity::CanWePlayGlobalSpeechContext(int16 gCtx)` (0x4E5F10): AX = a speech slot or
/// -1. In a ped conversation (0xB613FD), for a context that is not "important for interrupting" and not
/// in 0x154..0x166, conversation ped 1 (0xB61410) uses slot word 0xB61408, ped 2 (0xB6140C) slot word
/// 0xB61404; else in a player conversation (0xB613FC) with the same context tests, the player
/// conversation ped (0xB61400) uses 0xB61408 (peds compared as `ped + 0x294 == this`). A chosen slot
/// (!= -1) is returned when its status is 4, else -1. Otherwise the round-robin free slot from
/// `s_NextSpeechSlot` (none: -1; `s_NextSpeechSlot` = (slot + 1) % 5), refused (-1) while `now` is below
/// the context's next play time (contexts 0x154..0x166: the dword `this + 0xB4 + (ctx - 0x154) * 4`,
/// other 0..0x167: `gGlobalSpeechContextNextPlayTime[ctx]`). Return u16: callers use AX only (the
/// original's upper EAX is path-dependent).
/// Not hooked: AX-only result whose upper EAX is the caller's leftover on some paths; L2 compares EAX (port-l2-a2-menus-hud mismatch), needs an AX return kind in sa-core::abi.
pub fn speech_can_we_play_global_speech_context(this: GPtr<u8>, ctx: u32) -> u16 {
    let c = ctx as u16 as i16;
    let me = this.addr() as u32;
    let slot1 = g::<u16>(0xB61408).read();
    let not_scripted = !(c > 0x153 && c < 0x167);
    let mut chosen: u16 = 0xFFFF;
    let mut test = false;
    if g::<u8>(0xB613FD).read() != 0 && speech_is_important_for_interupting(ctx) == 0 && not_scripted {
        if me == g::<u32>(0xB61410).read().wrapping_add(0x294) {
            chosen = slot1;
            test = chosen != 0xFFFF;
        } else if me == g::<u32>(0xB6140C).read().wrapping_add(0x294) {
            chosen = g::<u16>(0xB61404).read();
            test = chosen != 0xFFFF;
        }
    }
    if !test {
        if g::<u8>(0xB613FC).read() == 0 || speech_is_important_for_interupting(ctx) != 0 || !not_scripted {
            return speech_free_slot_for_context(this, c);
        }
        if me == g::<u32>(0xB61400).read().wrapping_add(0x294) {
            chosen = slot1;
        }
        if chosen == 0xFFFF {
            return speech_free_slot_for_context(this, c);
        }
    }
    if rd::<u16>(speech_slot(chosen as i16), 0) == 4 {
        chosen
    } else {
        0xFFFF
    }
}

/// The round-robin tail of `CanWePlayGlobalSpeechContext` (0x4E5FC4..).
fn speech_free_slot_for_context(this: GPtr<u8>, c: i16) -> u16 {
    let next = g::<i16>(0xB61414).read() as i32;
    let mut found: Option<i16> = None;
    for k in 0..5i32 {
        let s = ((k + next) % 5) as i16;
        if rd::<u16>(speech_slot(s), 0) == 0 {
            found = Some(s);
            break;
        }
    }
    let Some(s) = found else {
        return 0xFFFF;
    };
    g::<u16>(0xB61414).write(((s as i32 + 1) % 5) as u16);
    if (0..0x168).contains(&c) {
        let t = if c > 0x153 && c < 0x167 {
            rd::<u32>(this, 0xB4 + (c as isize - 0x154) * 4)
        } else {
            g::<u32>(0xB61670 + c as usize * 4).read()
        };
        if sa_core::timer::TIME_MS.get() < t {
            return 0xFFFF;
        }
    }
    s as u16
}

orig! {
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn stats_get_stat_value(stat: u32) -> f32 = cdecl 0x558E40 => unported;
}

/// `(stat 0x15 - stat 0x17) * [0x858CDC]` as the bytes compute it (two `GetStatValue` calls).
fn speech_stat_mood() -> f32 {
    let a = stats_get_stat_value(0x15);
    let b = stats_get_stat_value(0x17);
    (a - b) * g::<f32>(0x858CDC).read()
}

/// The stat-scaled offset of `GetSpeechContextVolumeOffset`: the mood value clamped to [-1, 1] (with
/// the bytes' re-evaluations: up to three pairs of `GetStatValue` calls; unordered values pass), then
/// `((r + 1.0) * 0.5) * (top - base) + base`.
fn speech_stat_volume_offset(top: usize, base: usize) -> f32 {
    let one = g::<f32>(0x858624).read();
    let minus_one = g::<f32>(0x858C1C).read();
    let r = if speech_stat_mood() > one {
        if speech_stat_mood() > one {
            one
        } else {
            speech_stat_mood()
        }
    } else if minus_one > speech_stat_mood() {
        minus_one
    } else if speech_stat_mood() > one {
        one
    } else {
        speech_stat_mood()
    };
    let base_v = g::<f32>(base).read();
    ((r + 1.0) * 0.5) * (g::<f32>(top).read() - base_v) + base_v
}

/// `CAEPedSpeechAudioEntity::GetSpeechContextVolumeOffset(int16 gCtx)` (0x4E4AE0, ST0): context 0x151
/// -> [0x8C8128], 0x152 -> [0x8C8130], 0x162 -> stat offset (0xB61C14, 0x8C8120), 0x163 -> stat offset
/// (0xB61C18, 0x8C8124), 0x165 -> [0x8C812C], others 0.0 (the bytes' jump table). All arithmetic is
/// single precision (24-bit PC; the double constants 1.0 and 0.5 are exact).
#[hook(addr = 0x4E4AE0, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetSpeechContextVolumeOffset", expect = "0F BF 44 24 04")]
pub fn speech_get_speech_context_volume_offset(_this: GPtr<u8>, ctx: u32) -> crate::common::X87 {
    let v = match ctx as u16 as i16 {
        0x151 => g::<f32>(0x8C8128).read(),
        0x152 => g::<f32>(0x8C8130).read(),
        0x162 => speech_stat_volume_offset(0xB61C14, 0x8C8120),
        0x163 => speech_stat_volume_offset(0xB61C18, 0x8C8124),
        0x165 => g::<f32>(0x8C812C).read(),
        _ => g::<f32>(0x858B50).read(),
    };
    crate::common::x(v)
}

orig! {
    /// `CAEPedSpeechAudioEntity::IsGlobalContextPain(int16)` (0x4E44F0, `ret 4`, unhooked)
    fn speech_is_global_context_pain(ctx: u32) -> u8 = stdcall 0x4E44F0 => unported;
    /// `CAEPedSpeechAudioEntity::GetCurrentCJMood()` (0x4E53B0, static)
    fn speech_get_current_cj_mood() -> u32 = cdecl 0x4E53B0 => unported;
    /// `CAEPedSpeechAudioEntity::GetNextMoodToUse(int16)` (0x4E4700)
    fn speech_get_next_mood_to_use(this: GPtr<u8>, mood: u32) -> u32 = thiscall 0x4E4700 => unported;
    /// `CAEPedSpeechAudioEntity::GetVoiceForMood(int16)` (0x4E4760)
    fn speech_get_voice_for_mood_call(this: GPtr<u8>, mood: u32) -> u32 = thiscall 0x4E4760 => speech_get_voice_for_mood;
}

/// `CAEPedSpeechAudioEntity::GetSoundAndBankIDs(int16 gCtx, int16* outSpecific)` (0x4E5920): -1 (AX)
/// unless `gCtx` is in 0..0x168 and the ped type word (+0x92) in 0..6 and the specific context
/// (`GetSpecificSpeechContext`) >= 0, which is stored to `*outSpecific`. Pain contexts use the 0x8C64AC
/// bank / 0x8C64B8 range tables (column 0 for type 2, else 1 + (+0x96 == 0)); otherwise per type (the
/// bytes' jump table) the voice (+0x94) row: 0 bytes 0x8AF8A8 (spec <= 0x67, end byte 0xFF = none),
/// 1 words 0x8BA4D0 (<= 0x22), 2 words 0x8BBEF8 (<= 0x6F; CJ: the mood's voice, walking
/// `GetNextMoodToUse` until a row exists or the mood is 5), 3 words 0x8BE620 (<= 0x70), 4 words
/// 0x8C42B0 (<= 0x78), with the bank word (+0xA6) from the type's bank table. A range [first, last]
/// gives up to 20 candidates `first + (r + i) % (last - first + 1)` (r = `GetRandomNumberInRange(0,
/// last - first)`), stored to +0xA4 in turn; the first candidate not in the phrase memory (150 entries
/// at 0xB61418: sound, bank) is kept; when all are remembered, the oldest one (largest memory index) is
/// used. The memory shifts down one entry and entry 0 = (+0xA4, +0xA6). EAX = +0xA4 (zero-extended) -
/// first. The original also writes its own argument slots (limit and counter), not reproduced; a range
/// of zero candidates would read an uninitialised local in the original.
#[hook(addr = 0x4E5920, conv = thiscall, name = "CAEPedSpeechAudioEntity::GetSoundAndBankIDs", expect = "83 EC 5C 53 8B 5C 24 64")]
pub fn speech_get_sound_and_bank_ids(this: GPtr<u8>, ctx: u32, out: GPtr<u8>) -> u32 {
    const NONE: u32 = 0xFFFF;
    let c = ctx as u16 as i16;
    if !(0..0x168).contains(&c) {
        return NONE;
    }
    let ty = rd::<u16>(this, 0x92);
    if !(0..6).contains(&(ty as i16)) {
        return NONE;
    }
    let spec = speech_get_specific_speech_context(ctx, ty as u32) as i16;
    if spec < 0 {
        return NONE;
    }
    wr::<u16>(out, 0, spec as u16);
    let s = spec as i32;
    let voice = || rd::<i16>(this, 0x94) as i32;
    let w = |a: i32| g::<u16>(a as u32 as usize).read();
    let (first, last): (u16, u16);
    if speech_is_global_context_pain(ctx) != 0 {
        let sel: i32 = if rd::<u16>(this, 0x92) == 2 {
            0
        } else {
            (rd::<u16>(this, 0x96) == 0) as i32 + 1
        };
        wr::<u16>(this, 0xA6, w(0x8C64AC + sel * 2));
        let idx = (s * 3 + sel) * 4;
        first = w(0x8C64B8 + idx);
        last = w(0x8C64BA + idx);
        if first == 0xFFFF {
            return NONE;
        }
    } else {
        match rd::<u16>(this, 0x92) as i16 {
            0 => {
                if spec > 0x67 {
                    return NONE;
                }
                let v = voice();
                wr::<u16>(this, 0xA6, w(0x8AF700 + v * 2));
                let idx = (s * 0xD1 + v) * 2;
                first = g::<u8>((0x8AF8A8 + idx) as u32 as usize).read() as u16;
                last = g::<u8>((0x8AF8A9 + idx) as u32 as usize).read() as u16;
                if first == 0xFF {
                    return NONE;
                }
            }
            1 => {
                if spec > 0x22 {
                    return NONE;
                }
                let v = voice();
                wr::<u16>(this, 0xA6, w(0x8BA470 + v * 2));
                let idx = (s * 0x2E + v) * 4;
                first = w(0x8BA4D0 + idx);
                last = w(0x8BA4D2 + idx);
                if first == 0xFFFF {
                    return NONE;
                }
            }
            2 => {
                if spec > 0x6F {
                    return NONE;
                }
                let mut mood = speech_get_current_cj_mood();
                let mut v = speech_get_voice_for_mood_call(this, mood) as u16 as i16 as i32;
                if mood as u16 != 5 {
                    loop {
                        if w(0x8BBEF8 + (v + s * 20) * 4) != 0xFFFF {
                            break;
                        }
                        mood = speech_get_next_mood_to_use(this, mood);
                        v = speech_get_voice_for_mood_call(this, mood) as u16 as i16 as i32;
                        if mood as u16 == 5 {
                            break;
                        }
                    }
                }
                wr::<u16>(this, 0xA6, w(0x8BBED0 + v * 2));
                let idx = (v + s * 20) * 4;
                first = w(0x8BBEF8 + idx);
                last = w(0x8BBEFA + idx);
                if first == 0xFFFF {
                    return NONE;
                }
            }
            3 => {
                if spec > 0x70 {
                    return NONE;
                }
                let v = voice();
                wr::<u16>(this, 0xA6, w(0x8BE5B8 + v * 2));
                let idx = (s * 0x34 + v) * 4;
                first = w(0x8BE620 + idx);
                last = w(0x8BE622 + idx);
                if first == 0xFFFF {
                    return NONE;
                }
            }
            4 => {
                if spec > 0x78 {
                    return NONE;
                }
                let v = voice();
                wr::<u16>(this, 0xA6, w(0x8C4288 + v * 2));
                let idx = (v + s * 18) * 4;
                first = w(0x8C42B0 + idx);
                last = w(0x8C42B2 + idx);
                if first == 0xFFFF {
                    return NONE;
                }
            }
            _ => return NONE,
        }
    }
    let first_i = first as i16 as i32;
    let n = (last as i16 as i32).wrapping_sub(first_i);
    let r = crate::common::ae_random_int(0, n);
    let count = n.wrapping_add(1);
    let limit: i32 = if count < 0x14 { count } else { 0x14 };
    let mut cand = [(0u16, 0u16); 20];
    let mut i: i16 = 0;
    let mut dx: i16 = 0;
    let memory = |k: i16| -> GPtr<u8> { g(0xB61418 + k as usize * 4) };
    if (limit as i16) > 0 {
        loop {
            if dx == 0x96 {
                break;
            }
            let sum = (r as i16 as i32).wrapping_add(i as i32);
            let snd = (sum.wrapping_rem(count)).wrapping_add(first_i) as u16;
            wr::<u16>(this, 0xA4, snd);
            cand[i as usize].0 = snd;
            dx = 0;
            while dx < 0x96 {
                let m = memory(dx);
                if rd::<u16>(m, 0) == snd && rd::<u16>(m, 2) == rd::<u16>(this, 0xA6) {
                    cand[i as usize].1 = dx as u16;
                    break;
                }
                dx += 1;
            }
            i += 1;
            if i >= limit as i16 {
                break;
            }
        }
    }
    if i == limit as i16 && dx != 0x96 {
        let mut best = cand[0].0;
        let mut best_idx: i16 = 0;
        let n = if (limit as i16) > 0 { limit as u16 } else { 0 };
        for k in 0..n {
            let (snd, idx) = cand[k as usize];
            if (idx as i16) > best_idx {
                best = snd;
                best_idx = idx as i16;
            }
        }
        wr::<u16>(this, 0xA4, best);
    }
    for k in (1..=0x95i16).rev() {
        let src = memory(k - 1);
        let dst = memory(k);
        wr::<u16>(dst, 0, rd::<u16>(src, 0));
        wr::<u16>(dst, 2, rd::<u16>(src, 2));
    }
    wr::<u16>(memory(0), 0, rd::<u16>(this, 0xA4));
    wr::<u16>(memory(0), 2, rd::<u16>(this, 0xA6));
    (rd::<u16>(this, 0xA4) as u32).wrapping_sub(first_i as u32)
}

orig! {
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80; the argument word passed through)
    fn ae_resolve_probability_bits(p: u32) -> u8 = cdecl 0x4D9C80 => unported;
    /// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForStreaming(int16)` (0x4E4510)
    fn speech_is_important_for_streaming(this: GPtr<u8>, ctx: u32) -> u8 = thiscall 0x4E4510 => unported;
    /// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForWidescreen(int16)` (0x4E46B0)
    fn speech_is_important_for_widescreen_call(this: GPtr<u8>, ctx: u32) -> u8 = thiscall 0x4E46B0 => speech_is_global_context_important_for_widescreen;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CStreaming::IsVeryBusy()` (0x4076A0)
    fn streaming_is_very_busy() -> u8 = cdecl 0x4076A0 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u32 = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr_st0(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CAEPedSpeechAudioEntity::GetNextPlayTime(int16)` (0x4E49E0)
    fn speech_get_next_play_time(this: GPtr<u8>, ctx: u32) -> u32 = thiscall 0x4E49E0 => unported;
    /// `CAEPedSpeechAudioEntity::SetNextPlayTime(int16)` (0x4E4A20)
    fn speech_set_next_play_time(this: GPtr<u8>, ctx: u32) = thiscall 0x4E4A20 => unported;
    /// `CAEPedSpeechAudioEntity::CanWePlayGlobalSpeechContext(int16)` (0x4E5F10)
    fn speech_can_we_play_global_speech_context_call(this: GPtr<u8>, ctx: u32) -> u16 = thiscall 0x4E5F10 => speech_can_we_play_global_speech_context;
    /// `CAEPedSpeechAudioEntity::GetSoundAndBankIDs(int16, int16*)` (0x4E5920)
    fn speech_get_sound_and_bank_ids_call(this: GPtr<u8>, ctx: u32, out: GPtr<u8>) -> u32 = thiscall 0x4E5920 => speech_get_sound_and_bank_ids;
    /// `CAEPedSpeechAudioEntity::GetSpeechContextVolumeOffset(int16)` (0x4E4AE0, ST0)
    fn speech_get_speech_context_volume_offset_call_st0(this: GPtr<u8>, ctx: u32) -> f64 = thiscall 0x4E4AE0 => speech_get_speech_context_volume_offset_target_f64;
    /// `CAEPedSpeechAudioEntity::LoadAndPlaySpeech(uint32 delay)` (0x4E4840)
    fn speech_load_and_play_speech(this: GPtr<u8>, delay: u32) = thiscall 0x4E4840 => unported;
    /// `CAEAudioUtility::FindVehicleOfPlayer()` (0x4D9E10)
    fn ae_find_vehicle_of_player() -> u32 = cdecl 0x4D9E10 => unported;
}

/// `CAEPedSpeechAudioEntity::AddSayEvent(int32 event, int16 gCtx, uint32 delay, float probability,
/// bool overrideSilence, bool isForceAudible, bool isFrontEnd)` (0x4E6550, `ret 0x1C`): AX = the sound
/// (from `GetSoundAndBankIDs`) or -1. Refused when: not initialised (+0x90), all speech disabled, script
/// speech disabled (+0x9A); widescreen and the context not important for it; `0x96A8B0 == 2`; context
/// 0x2D unless frame counter & 0x3F == 0 and no conversation; context 0x165 while [0xC8132C] <
/// [0x858B8C]; `ResolveProbability` fails; event != 0x34; speech disabled (+0x99) without
/// `overrideSilence`. A pain / interrupting context stops a talking ped's current speech unless that one
/// is itself pain / interrupting (except 0x166); others are refused while talking. Then: dead and not
/// pain; streaming-unimportant contexts while streaming is very busy (without the two flags), or
/// important ones / flags while 0x8E4CB8 > 15 with a radio player attached; CJ (type 2) when another
/// player is speaking and CJ is not, or the ped is not `FindPlayerPed(0)`; squared camera distance >
/// [0x859600]; CJ before `GetNextPlayTime` (slot 5), others without a free slot
/// (`CanWePlayGlobalSpeechContext`, 0..4); `GetSoundAndBankIDs` < 0. On success: `SetNextPlayTime`,
/// +0xAC = event volume 0x34, +0x9B = isFrontEnd, +0x9C = isForceAudible, +0x9B = 1 for a driver of the
/// player's vehicle (flag 0x100 at +0x46C) unless context 0x151, CJ adds [0x8C80E8], +0xA8 = slot, +0x98
/// = 1 (CJ: `s_bAPlayerSpeaking` = 1), +0xB0 = context, +0xAC += `GetSpeechContextVolumeOffset`,
/// `LoadAndPlaySpeech(delay + [0xB61C10])`. The original writes its own argument slots (delay, the
/// `GetSoundAndBankIDs` out word, the volume temp): a local here. Return u16 (AX; upper EAX is the
/// callee's).
#[hook(addr = 0x4E6550, conv = thiscall, name = "CAEPedSpeechAudioEntity::AddSayEvent", expect = "A1 10 1C B6 00")]
#[allow(clippy::too_many_arguments)]
pub fn speech_add_say_event(
    this: GPtr<u8>,
    event: u32,
    ctx: u32,
    delay: u32,
    prob: u32,
    override_silence: u32,
    force_audible: u32,
    front_end: u32,
) -> u16 {
    const NONE: u16 = 0xFFFF;
    let delay = delay.wrapping_add(g::<u32>(0xB61C10).read());
    if rd::<u8>(this, 0x90) == 0 || g::<u8>(0xB613E6).read() != 0 || rd::<u8>(this, 0x9A) != 0 {
        return NONE;
    }
    let c = ctx as u16 as i16;
    if g::<u8>(0xB6F065).read() != 0 && speech_is_important_for_widescreen_call(this, ctx) == 0 {
        return NONE;
    }
    if g::<u8>(0x96A8B0).read() == 2 {
        return NONE;
    }
    if c == 0x2D {
        if g::<u8>(0xB7CB4C).read() & 0x3F != 0
            || g::<u8>(0xB613FD).read() != 0
            || g::<u8>(0xB613FC).read() != 0
        {
            return NONE;
        }
    } else if c == 0x165 && g::<f32>(0xC8132C).read() < g::<f32>(0x858B8C).read() {
        return NONE;
    }
    if ae_resolve_probability_bits(prob) == 0 || event != 0x34 {
        return NONE;
    }
    if rd::<u8>(this, 0x99) != 0 && override_silence as u8 == 0 {
        return NONE;
    }
    let pain = speech_is_global_context_pain(ctx);
    if pain != 0 || speech_is_important_for_interupting(ctx) != 0 {
        if rd::<u8>(this, 0x98) != 0 {
            let cur = rd::<i16>(speech_slot(rd::<i16>(this, 0xA8)), 0x14) as i32 as u32;
            if (speech_is_global_context_pain(cur) != 0 || speech_is_important_for_interupting(cur) != 0)
                && cur as u16 != 0x166
            {
                return NONE;
            }
            speech_stop_current_speech(this);
        }
    } else if rd::<u8>(this, 0x98) != 0 {
        return NONE;
    }
    if ped_is_alive(g(rd::<u32>(this, 4) as usize)) == 0 && pain == 0 {
        return NONE;
    }
    if speech_is_important_for_streaming(this, ctx) != 0 || force_audible as u8 != 0 || front_end as u8 != 0 {
        if g::<i32>(0x8E4CB8).read() > 0xF && g::<u32>(0xB6B98C).read() != 0 {
            return NONE;
        }
    } else if streaming_is_very_busy() != 0 {
        return NONE;
    }
    if rd::<u16>(this, 0x92) == 2 {
        if g::<u8>(0xB613E5).read() != 0
            && rd::<u8>(this, 0x98) == 0
            && rd::<u32>(this, 4) == find_player_ped(0)
        {
            return NONE;
        }
        if rd::<u32>(this, 4) != find_player_ped(0) {
            return NONE;
        }
    }
    let cam_m = g::<u32>(0xB6F03C).read();
    let cam: GPtr<u8> = if cam_m != 0 {
        g(cam_m as usize + 0x30)
    } else {
        g(0xB6F02C)
    };
    let ped: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    let m = rd::<u32>(ped, 0x14);
    let pos: GPtr<u8> = if m != 0 {
        g(m as usize + 0x30)
    } else {
        ped.byte_add(4)
    };
    let mut d = [0u32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_sub(dp, pos, cam);
    let mut d2 = d;
    let mag = vector_magnitude_sqr(GPtr::from_raw(d2.as_mut_ptr()).cast());
    if mag.fcom(crate::common::x(g::<f32>(0x859600).read())) == sa_math::x87::Cmp::Greater {
        return NONE;
    }
    let slot: u16 = if rd::<u16>(this, 0x92) == 2 {
        if sa_core::timer::TIME_MS.get() < speech_get_next_play_time(this, ctx) {
            return NONE;
        }
        5
    } else {
        let s = speech_can_we_play_global_speech_context_call(this, ctx);
        if !(0..5).contains(&(s as i16)) {
            return NONE;
        }
        s
    };
    let mut spec = [override_silence];
    let r = speech_get_sound_and_bank_ids_call(this, ctx, GPtr::from_raw(spec.as_mut_ptr()).cast());
    if (r as i16) < 0 {
        return NONE;
    }
    speech_set_next_play_time(this, ctx);
    let vol = event_volume(0x34);
    wr::<u8>(this, 0x9B, front_end as u8);
    wr::<u8>(this, 0x9C, force_audible as u8);
    wr::<f32>(this, 0xAC, vol);
    let ped: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u32>(ped, 0x58C) != 0 && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = ae_find_vehicle_of_player();
        if rd::<u32>(g(rd::<u32>(this, 4) as usize), 0x58C) == v && c != 0x151 {
            wr::<u8>(this, 0x9B, 1);
        }
    }
    let ty = rd::<u16>(this, 0x92);
    if ty == 2 {
        wr::<f32>(this, 0xAC, g::<f32>(0x8C80E8).read() + rd::<f32>(this, 0xAC));
    }
    wr::<u16>(this, 0xA8, slot);
    wr::<u8>(this, 0x98, 1);
    if ty == 2 {
        g::<u8>(0xB613E5).write(1);
    }
    wr::<u16>(this, 0xB0, ctx as u16);
    let off = speech_get_speech_context_volume_offset_call(this, ctx);
    wr::<f32>(this, 0xAC, off.fadd_m(rd::<f32>(this, 0xAC)).to_f32());
    speech_load_and_play_speech(this, delay);
    r as u16
}

orig! {
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    fn task_manager_get_task_secondary(tm: GPtr<u8>, slot: u32) -> u32 = thiscall 0x681810 => unported;
    /// `CTaskComplexFacial::StopAll()` (0x691250)
    fn task_facial_stop_all(task: GPtr<u8>) = thiscall 0x691250 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_destructor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CAEPedSpeechAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4E3520). Position -1 (the
/// speech ended): +0x98 = 0 (CJ: `s_bAPlayerSpeaking` = 0), its speech slot (+0xA8) cleared, +0xA0 = 0,
/// +0xA4 / +0xA6 = -1; in a ped conversation the matching conversation ped (1 with slot 0xB61408, else
/// 2 with 0xB61404), or in a player conversation the player ped with 0xB61408, puts that slot back to
/// status 4; +0xA8 = -1, +0x9C = 0; a ped with intelligence (+0x47C) and a secondary facial task (3)
/// gets `StopAll` (the task looked up twice). Otherwise: the slot's status = 5; a ped with flag 0x8000
/// (+0x46C) and byte +0x754 == 2 stops the sound; else, not front-end (+0x9B == 0), the sound follows
/// the ped. The original keeps an unconstructed `CMatrix` local (only +0x40/+0x44 zeroed) and destroys
/// it at the end; reproduced with a zeroed local. The SEH frame is not reproduced.
#[hook(addr = 0x4E3520, conv = thiscall, name = "CAEPedSpeechAudioEntity::UpdateParameters", expect = "6A FF 68 F8 BF 83 00")]
pub fn speech_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    let mut mat = [0u32; 0x48 / 4];
    if pos as u16 == 0xFFFF {
        wr::<u8>(this, 0x98, 0);
        if rd::<u16>(this, 0x92) == 2 {
            g::<u8>(0xB613E5).write(0);
        }
        clear_speech_slot(speech_slot(rd::<i16>(this, 0xA8)));
        wr::<u32>(this, 0xA0, 0);
        wr::<u16>(this, 0xA4, 0xFFFF);
        wr::<u16>(this, 0xA6, 0xFFFF);
        let me = this.addr() as u32;
        let a8 = rd::<u16>(this, 0xA8);
        let back = if g::<u8>(0xB613FD).read() != 0 {
            (me == g::<u32>(0xB61410).read().wrapping_add(0x294) && a8 == g::<u16>(0xB61408).read())
                || (me == g::<u32>(0xB6140C).read().wrapping_add(0x294) && a8 == g::<u16>(0xB61404).read())
        } else {
            g::<u8>(0xB613FC).read() != 0
                && me == g::<u32>(0xB61400).read().wrapping_add(0x294)
                && g::<u16>(0xB61408).read() == a8
        };
        if back {
            wr::<u16>(speech_slot(a8 as i16), 0, 4);
        }
        let ped: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
        wr::<u16>(this, 0xA8, 0xFFFF);
        wr::<u8>(this, 0x9C, 0);
        let intel = rd::<u32>(ped, 0x47C);
        if intel != 0 && task_manager_get_task_secondary(g(intel as usize + 4), 3) != 0 {
            let intel = rd::<u32>(g(rd::<u32>(this, 4) as usize), 0x47C);
            let t = task_manager_get_task_secondary(g(intel as usize + 4), 3);
            task_facial_stop_all(g(t as usize));
        }
    } else {
        wr::<u16>(speech_slot(rd::<i16>(this, 0xA8)), 0, 5);
        let ped = rd::<u32>(this, 4);
        let p: GPtr<u8> = g(ped as usize);
        if rd::<u32>(p, 0x46C) & 0x8000 != 0 && rd::<u8>(p, 0x754) == 2 {
            sound_stop_sound(g(sound as usize));
        } else if rd::<u8>(this, 0x9B) == 0 && ped != 0 {
            let v = entity_pos(p);
            sound_set_position(g(sound as usize), v[0], v[1], v[2]);
        }
    }
    matrix_destructor(GPtr::from_raw(mat.as_mut_ptr()).cast());
}

orig! {
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn vehicle_add_audio_event(this: GPtr<u8>, event: u32, volume: u32) = thiscall 0x4F6420 => unported;
    /// `CAEVehicleAudioEntity::UpdateGenericVehicleSound(int16 id, int16 slot, int16 bank, int16 sfx,
    /// float speed, float volume, float distance)` (0x4FAD40; protection code inside, not ported)
    fn vehicle_update_generic_vehicle_sound(this: GPtr<u8>, id: u32, slot: u32, bank: u32, sfx: u32, speed: u32, volume: u32, distance: u32) = thiscall 0x4FAD40 => unported;
}

/// `CAEVehicleAudioEntity::ProcessNitro(cVehicleParams&)` (0x4FB070). Active when the vehicle (+0x10)
/// has flag 0x80000 at +0x38C, its nitro value +0x8A4 < 0, bit 0x10 of byte +0x428 and not bit 0x40 of
/// byte +0x42B: the first frame (+0x244 == 0) adds audio event 0x66 (volume 0); the level v =
/// (int16 params+0x20) * [0x859A3C]; a negative stored level (+0x248) takes v; the stored level moves
/// towards v by [0x8CC040] up / [0x8CC044] down (clamped at v; unordered moves down) and is stored;
/// `UpdateGenericVehicleSound(7, 5, 0x8F, 0xA, [0xB6BAB4]*v + [0x8CC028], [0xB6BAB0]*v + [0x8CC020],
/// 3.0)` and `(8, 0x13, 0x8A, 0x1A, [0xB6BABC]*v + [0x8CC038], [0xB6BAB8]*v + [0x8CC030], 3.0)`;
/// +0x244 = 1. Otherwise the two sounds +0x120 / +0x128 get `SetFlags(4, 0)` + `StopSound` and are
/// cleared (when the flag +0x244 was set), +0x244 = 0, +0x248 = -1.0. Every x87 value is single
/// precision (24-bit PC). The original keeps two of the floats in its own argument and saved-ECX slots.
#[hook(addr = 0x4FB070, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessNitro", expect = "51 56 57 8B 7C 24 10")]
pub fn vehicle_process_nitro(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let zero = g::<f32>(0x858B50).read();
    let active = rd::<u32>(veh, 0x38C) & 0x80000 != 0
        && rd::<f32>(veh, 0x8A4) < zero
        && rd::<u8>(veh, 0x428) & 0x10 != 0
        && rd::<u8>(veh, 0x42B) & 0x40 == 0;
    if !active {
        if rd::<u8>(this, 0x244) != 0 {
            vehicle_stop_engine_sound(this, 0x120);
            vehicle_stop_engine_sound(this, 0x128);
        }
        wr::<u8>(this, 0x244, 0);
        wr::<u32>(this, 0x248, 0xBF80_0000);
        return;
    }
    if rd::<u8>(this, 0x244) == 0 {
        vehicle_add_audio_event(this, 0x66, 0);
    }
    let mut v = (rd::<i16>(params, 0x20) as i32 as f32) * g::<f32>(0x859A3C).read();
    if rd::<f32>(this, 0x248) < zero {
        wr::<f32>(this, 0x248, v);
    }
    let cur = rd::<f32>(this, 0x248);
    if v.partial_cmp(&cur)
        .is_some_and(|o| o != core::cmp::Ordering::Less)
    {
        let t = g::<f32>(0x8CC040).read() + cur;
        if v.partial_cmp(&t) != Some(core::cmp::Ordering::Less) {
            v = t;
        }
    } else {
        let t = cur - g::<f32>(0x8CC044).read();
        if v.partial_cmp(&t) != Some(core::cmp::Ordering::Greater) {
            v = t;
        }
    }
    wr::<f32>(this, 0x248, v);
    let f = |k: usize, c: usize| g::<f32>(k).read() * v + g::<f32>(c).read();
    let a = f(0xB6BABC, 0x8CC038);
    let b = f(0xB6BAB8, 0x8CC030);
    let c = f(0xB6BAB0, 0x8CC020);
    let d = f(0xB6BAB4, 0x8CC028);
    vehicle_update_generic_vehicle_sound(this, 7, 5, 0x8F, 0xA, d.to_bits(), c.to_bits(), 0x4040_0000);
    vehicle_update_generic_vehicle_sound(this, 8, 0x13, 0x8A, 0x1A, a.to_bits(), b.to_bits(), 0x4040_0000);
    wr::<u8>(this, 0x244, 1);
}

// ---- pass 3: collision (3) -----------------------------------------------------------------------

orig! {
    /// `SurfaceInfos_c::GetFrictionEffect(uint32 surface)` (0x55E650)
    fn surface_get_friction_effect(infos: GPtr<u8>, surface: u32) -> u32 = thiscall 0x55E650 => unported;
    /// `CEntity::GetIsTypePhysical()` (0x4DA030)
    fn entity_get_is_type_physical(e: GPtr<u8>) -> u8 = thiscall 0x4DA030 => unported;
    /// `CAECollisionAudioEntity::PlayOneShotCollisionSound(CEntity*, CEntity*, uint8, uint8, float, CVector&)` (0x4DB150)
    fn collision_play_one_shot(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32) = thiscall 0x4DB150 => unported;
    /// `CAECollisionAudioEntity::PlayLoopingCollisionSound(CEntity*, CEntity*, uint8, uint8, float, CVector&, bool)` (0x4DB450)
    fn collision_play_looping(this: GPtr<u8>, e1: u32, e2: u32, s1: u32, s2: u32, impulse: u32, posn: u32, flag: u32) = thiscall 0x4DB450 => unported;
    /// `CAECollisionAudioEntity::UpdateLoopingCollisionSound(CAESound*, CEntity*, CEntity*, uint8, uint8, float, CVector&, bool)` (0x4DA540)
    fn collision_update_looping(this: GPtr<u8>, sound: u32, e1: u32, e2: u32, s1: u32, s2: u32, a: u32, b: u32, flag: u32) = thiscall 0x4DA540 => unported;
    /// `CAECollisionAudioEntity::GetCollisionSoundStatus(...)` (0x4DA830)
    fn collision_get_collision_sound_status_call(this: GPtr<u8>, a: u32, b: u32, s1: u32, s2: u32, out: GPtr<u8>) -> u8 = thiscall 0x4DA830 => collision_get_collision_sound_status;
}

/// The collision surface override of `ReportCollision` for entity `e` (the other one `other`); `cur` is
/// the surface being replaced, `other_surface` the other side's (read for the friction test).
fn collision_surface_for(e: GPtr<u8>, other: u32, normal: u32, cur: u8, other_surface: u8) -> u8 {
    if rd::<u8>(e, 0x36) & 7 == 2 {
        let mut scraping = false;
        if other != 0 && rd::<u8>(g(other as usize), 0x36) & 7 == 1 && normal != 0 {
            let m: GPtr<u8> = g(rd::<u32>(e, 0x14) as usize);
            let n: GPtr<u8> = g(normal as usize);
            let d = rd::<f32>(m, 0x28) * rd::<f32>(n, 8)
                + rd::<f32>(m, 0x24) * rd::<f32>(n, 4)
                + rd::<f32>(m, 0x20) * rd::<f32>(n, 0);
            scraping = d > g::<f32>(0x858CC8).read();
        }
        if scraping {
            if rd::<u32>(e, 0x594) == 0xA {
                return 0xBC;
            }
            if surface_get_friction_effect(g(0xB79538), other_surface as u32) != 1 {
                return 0x43;
            }
        }
        return if rd::<u32>(e, 0x594) == 0xA { 0xBC } else { 0x3F };
    }
    let model = rd::<u16>(e, 0x22);
    match model {
        0x158 => return 0x2D,
        0x16B => return 0xBE,
        0x156 | 0x16C | 0x157 => return 0xBF,
        _ => {}
    }
    let m = model as i16 as i32;
    if m == g::<u16>(0x8CD774).read() as i32 {
        0xC1
    } else if m == g::<u16>(0x8CD77C).read() as i32 {
        0xC2
    } else if m == g::<u16>(0x8CD6F4).read() as i32 {
        0xAF
    } else if m == g::<u16>(0x8CD780).read() as i32 {
        0x2B
    } else if entity_get_is_type_physical(e) != 0 && rd::<u8>(e, 0x40) & 0x80 != 0 {
        0xC0
    } else {
        cur
    }
}

/// `CAECollisionAudioEntity::ReportCollision(CEntity* e1, CEntity* e2, uint8 s1, uint8 s2, CVector&
/// posn, CVector* normal, float impulse, float speed, bool oneShot, bool unknown)` (0x4DBA10, `ret
/// 0x28`): nothing unless bank 0x27 is in slot 2, fewer than 300 collisions are active (+0x204) and
/// `speed` is not <= 0. Each entity in turn (e1 then e2) may override its own surface: vehicles 0x3F
/// (bikes, +0x594 == 10: 0xBC), or 0x43 when scraping a building (matrix up . normal > [0x858CC8]) and
/// the other side's surface friction effect is not 1; models 0x158 / 0x16B / 0x156,0x16C,0x157 and the
/// model words at 0x8CD774 / 0x8CD77C / 0x8CD6F4 / 0x8CD780 fixed surfaces; physicals with bit 0x80 of
/// +0x40 0xC0. The surface arguments are passed on as their argument dwords with the low byte replaced.
/// `oneShot` -> `PlayOneShotCollisionSound(e1, e2, s1, s2, impulse, posn)`. Otherwise
/// `GetCollisionSoundStatus`: playing (2) -> the entry's end time = now + 100 and, with a sound,
/// `UpdateLoopingCollisionSound(sound, entry e1/e2/s1/s2, impulse, speed, unknown)`; with `unknown`
/// any other status -> `PlayLoopingCollisionSound(.., impulse, posn, unknown)`; without it status 1
/// -> `PlayLoopingCollisionSound(.., 0)`, others -> `PlayOneShotCollisionSound`. The status index is
/// written into the original's `oneShot` argument slot (a local here).
/// Not hooked: L2 mismatch in every recording (port-l2-a2-*: 4..222 cases): the CAESound it builds on the stack carries the original's uninitialised bytes into AESoundManager, the port's are zero; kept as an original-only port until the stack-sound question is decided.
#[allow(clippy::too_many_arguments)]
pub fn collision_report_collision(
    this: GPtr<u8>,
    e1: u32,
    e2: u32,
    s1: u32,
    s2: u32,
    posn: u32,
    normal: u32,
    impulse: u32,
    speed: u32,
    one_shot: u32,
    unknown: u32,
) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x27, 2) == 0 || rd::<u32>(this, 0x204) == 300 {
        return;
    }
    let sp = f32::from_bits(speed);
    let zero = g::<f32>(0x858B50).read();
    if sp <= zero {
        return;
    }
    let mut a = s1 as u8;
    let mut b = s2 as u8;
    a = collision_surface_for(g(e1 as usize), e2, normal, a, b);
    b = collision_surface_for(g(e2 as usize), e1, normal, b, a);
    let s1 = (s1 & !0xFF) | a as u32;
    let s2 = (s2 & !0xFF) | b as u32;
    if one_shot as u8 != 0 {
        collision_play_one_shot(this, e1, e2, s1, s2, impulse, posn);
        return;
    }
    let mut out = [one_shot];
    let status = collision_get_collision_sound_status_call(
        this,
        e1,
        e2,
        s1,
        s2,
        GPtr::from_raw(out.as_mut_ptr()).cast(),
    );
    let flag = if unknown as u8 != 0 { unknown } else { 0 };
    if status == 2 {
        let e = this.byte_add(0x208 + out[0] as i32 as isize * 0x14);
        wr::<u32>(e, 0xC, sa_core::timer::TIME_MS.get().wrapping_add(100));
        let snd = rd::<u32>(e, 8);
        if snd != 0 {
            collision_update_looping(
                this,
                snd,
                rd::<u32>(e, 0),
                rd::<u32>(e, 4),
                rd::<u8>(e, 0x11) as u32,
                rd::<u8>(e, 0x12) as u32,
                impulse,
                speed,
                flag,
            );
        }
        return;
    }
    if unknown as u8 != 0 || status == 1 {
        collision_play_looping(this, e1, e2, s1, s2, impulse, posn, flag);
    } else {
        collision_play_one_shot(this, e1, e2, s1, s2, impulse, posn);
    }
}

// ---- pass 3: ped footsteps -----------------------------------------------------------------------

orig! {
    /// `SurfaceInfos_c::IsAudio*(uint32 surface)` (thiscall on `g_surfaceInfos` 0xB79538, EAX)
    fn surface_is_audio_concrete(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EA30 => unported;
    fn surface_is_audio_grass(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EA50 => unported;
    fn surface_is_audio_long_grass(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EB10 => unported;
    fn surface_is_audio_sand(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EA70 => unported;
    fn surface_is_audio_gravel(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EA90 => unported;
    fn surface_is_audio_wood(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EAB0 => unported;
    fn surface_is_audio_tile(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EB30 => unported;
    fn surface_is_audio_metal(infos: GPtr<u8>, s: u32) -> u32 = thiscall 0x55EAF0 => unported;
    /// `CAESoundManager::CancelSoundsInBankSlot(int16 slot, bool)` (0x4EFC60)
    fn sm_cancel_sounds_in_bank_slot(sm: GPtr<u8>, slot: u32, b: u32) = thiscall 0x4EFC60 => unported;
    /// `CAESoundManager::AreSoundsPlayingInBankSlot(int16)` (0x4EF520, AX)
    fn sm_are_sounds_playing_in_bank_slot(sm: GPtr<u8>, slot: u32) -> u32 = thiscall 0x4EF520 => unported;
}

/// Footstep bank `bank` in slot 0x1E: true when loaded; otherwise sounds playing in the slot (1 or 2)
/// are cancelled, the bank is loaded and false is returned.
fn footstep_bank_ready(bank: u32) -> bool {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), bank, 0x1E) != 0 {
        return true;
    }
    let n = sm_are_sounds_playing_in_bank_slot(g(0xB62CB0), 0x1E) as u16 as i16;
    if n == 1 || n == 2 {
        sm_cancel_sounds_in_bank_slot(g(0xB62CB0), 0x1E, 0);
    }
    hw_load_sound_bank(g(0xB5F8B8), bank, 0x1E);
    false
}

/// `CAEPedAudioEntity::HandleFootstepEvent(eAudioEvents, float volumeOffset, float speed, uint8 surface)`
/// (0x4E13A0, `ret 0x10`): nothing when the ped's flags dword +0x46C has 0x200. Volume = event volume +
/// offset, then `GetRandomNumberInRange(-3, 3)` + volume. For the player (`FindPlayerPed(-1)`): water
/// (the argument surface or the ped's +0xBA) -> bank 0x80 in slot 0x20 (sfx random 0..4, play position
/// 0x32; not loaded: loaded, then the generic step); otherwise by the ped's surface: concrete -> generic;
/// grass / long grass bank 1, sand 4 (sfx 0..3), gravel 2, wood 6, tile 5, metal 3 in slot 0x1E (sfx
/// 0..4; bank not loaded: slot sounds cancelled, bank loaded, generic step); other surfaces generic.
/// Generic: bank 0 in slot 0x29 (else loaded, return), sfx random 1..5. Other peds: bank 0 in 0x29
/// required (else loaded, return), volume - [0x858B3C], sfx 1..5. The temp sound (+8): (slot, sfx, the
/// ped's position, volume, 1.0, `speed`, 1.0, 0, flags 0xA0, 0x3D70D845, play position), registered with
/// the ped, requested. The original keeps the volumes in its argument slots.
#[hook(addr = 0x4E13A0, conv = thiscall, name = "CAEPedAudioEntity::HandleFootstepEvent", expect = "53 8B D9 8B 83 94 00 00 00")]
pub fn ped_handle_footstep_event(this: GPtr<u8>, event: u32, vol_offset: f32, speed: u32, surface: u32) {
    let ped0: GPtr<u8> = g(rd::<u32>(this, 0x94) as usize);
    if rd::<u32>(ped0, 0x46C) & 0x200 != 0 {
        return;
    }
    let vol = event_volume(event) + vol_offset;
    let r = crate::common::ae_random_int(-3, 3);
    let mut vol = r as f32 + vol;
    let si = || -> GPtr<u8> { g(0xB79538) };
    let ped_surface = || rd::<u8>(g(rd::<u32>(this, 0x94) as usize), 0xBA) as u32;
    let mut play_pos = 0u32;
    // (slot, sfx) or None for the generic step
    let mut chosen: Option<(u32, u32)> = None;
    if find_player_ped(-1) == rd::<u32>(this, 0x94) {
        if surface_is_audio_water(si(), surface as u8 as u32) as u8 != 0
            || surface_is_audio_water(si(), ped_surface()) as u8 != 0
        {
            if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x80, 0x20) != 0 {
                let s = crate::common::ae_random_int(0, 4) as u32;
                play_pos = 0x32;
                chosen = Some((0x20, s));
            } else {
                hw_load_sound_bank(g(0xB5F8B8), 0x80, 0x20);
            }
        } else if surface_is_audio_concrete(si(), ped_surface()) as u8 != 0 {
        } else {
            let bank: Option<(u32, i32)> = if surface_is_audio_grass(si(), ped_surface()) as u8 != 0
                || surface_is_audio_long_grass(si(), ped_surface()) as u8 != 0
            {
                Some((1, 4))
            } else if surface_is_audio_sand(si(), ped_surface()) as u8 != 0 {
                Some((4, 3))
            } else if surface_is_audio_gravel(si(), ped_surface()) as u8 != 0 {
                Some((2, 4))
            } else if surface_is_audio_wood(si(), ped_surface()) as u8 != 0 {
                Some((6, 4))
            } else if surface_is_audio_tile(si(), ped_surface()) as u8 != 0 {
                Some((5, 4))
            } else if surface_is_audio_metal(si(), ped_surface()) as u8 != 0 {
                Some((3, 4))
            } else {
                None
            };
            if let Some((bank, max)) = bank
                && footstep_bank_ready(bank)
            {
                let s = crate::common::ae_random_int(0, max) as u32;
                chosen = Some((0x1E, s));
            }
        }
        if chosen.is_none() {
            if hw_is_sound_bank_loaded(g(0xB5F8B8), 0, 0x29) == 0 {
                hw_load_sound_bank(g(0xB5F8B8), 0, 0x29);
                return;
            }
            chosen = Some((0x29, crate::common::ae_random_int(1, 5) as u32));
        }
    } else {
        if hw_is_sound_bank_loaded(g(0xB5F8B8), 0, 0x29) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0, 0x29);
            return;
        }
        vol -= g::<f32>(0x858B3C).read();
        chosen = Some((0x29, crate::common::ae_random_int(1, 5) as u32));
    }
    let Some((slot, sfx)) = chosen else {
        return;
    };
    let p = entity_pos(g(rd::<u32>(this, 0x94) as usize));
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        speed,
        FLT_ONE_BITS,
        0,
        0xA0,
        0x3D70_D845,
        play_pos,
    );
    sound_register_with_physical_entity(snd, g(rd::<u32>(this, 0x94) as usize));
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEVehicleAudioEntity::ProcessEngineDamage(cVehicleParams&)` (0x4FAE20). From the vehicle's
/// (params +0x10) health (+0x4C0) against [0x859F80] / [0x862D28] (smoke and fire), [0x862D24] /
/// [0x862D20] (smoke only); the entity's bit 0x40 of +0x42B forces fire only; audio type +0x80 != 0
/// clears fire. Ratio = min(|speed (+0x1C) / transmission (+0x14) max (+0x58)|, 1.0) (no transmission:
/// 0; unordered passes). Engine off flag +0xA8 set: no fire, the smoke sound +0x130 stops. Smoke (when
/// +0xA8 == 0): `UpdateGenericVehicleSound(9, 5, 0x8F, 0x1C, [0xB6BAA4]*r + [0x8CC000], [0xB6BAA0]*r +
/// [0x8CBFF8], [0x8CC008])`, else +0x130 stops. Fire with bit 0x10 of the vehicle's +0x428:
/// `(0xA, 0x13, 0x8A, 2, speed, volume, [0x8CC01C])` with volume [0xB6BAA8]*r + [0x8CC00C] and speed
/// [0xB6BAAC]*r + [0x8CC014] (bit 0x40: volume - [0x8CBC44], speed * [0x8CBC48]); else +0x138 stops.
/// Single-precision x87 throughout.
#[hook(addr = 0x4FAE20, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessEngineDamage", expect = "51 53 55 8B 6C 24 10")]
pub fn vehicle_process_engine_damage(this: GPtr<u8>, params: GPtr<u8>) {
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let hp = rd::<f32>(veh, 0x4C0);
    let below = |c: usize| hp < g::<f32>(c).read();
    let (mut smoke, mut fire) = if below(0x859F80) || below(0x862D28) {
        (true, true)
    } else if below(0x862D24) || below(0x862D20) {
        (true, false)
    } else {
        (false, false)
    };
    if rd::<u8>(g(rd::<u32>(this, 4) as usize), 0x42B) & 0x40 != 0 {
        smoke = false;
        fire = true;
    }
    if rd::<u8>(this, 0x80) != 0 {
        fire = false;
    }
    let one = g::<f32>(0x858624).read();
    let tr = rd::<u32>(params, 0x14);
    let ratio: f32 = if tr == 0 {
        0.0
    } else {
        let r = (rd::<f32>(params, 0x1C) / rd::<f32>(g(tr as usize), 0x58)).abs();
        if one < r { one } else { r }
    };
    if rd::<u8>(this, 0xA8) != 0 {
        fire = false;
        vehicle_stop_engine_sound(this, 0x130);
    } else if smoke {
        let a = g::<f32>(0xB6BAA0).read() * ratio + g::<f32>(0x8CBFF8).read();
        let b = g::<f32>(0xB6BAA4).read() * ratio + g::<f32>(0x8CC000).read();
        vehicle_update_generic_vehicle_sound(
            this,
            9,
            5,
            0x8F,
            0x1C,
            b.to_bits(),
            a.to_bits(),
            g::<u32>(0x8CC008).read(),
        );
    } else {
        vehicle_stop_engine_sound(this, 0x130);
    }
    if fire && rd::<u8>(veh, 0x428) & 0x10 != 0 {
        let flag = rd::<u8>(g(rd::<u32>(this, 4) as usize), 0x42B) & 0x40 != 0;
        let mut vol = g::<f32>(0xB6BAA8).read() * ratio + g::<f32>(0x8CC00C).read();
        let mut spd = g::<f32>(0xB6BAAC).read() * ratio + g::<f32>(0x8CC014).read();
        if flag {
            vol -= g::<f32>(0x8CBC44).read();
            spd *= g::<f32>(0x8CBC48).read();
        }
        vehicle_update_generic_vehicle_sound(
            this,
            0xA,
            0x13,
            0x8A,
            2,
            spd.to_bits(),
            vol.to_bits(),
            g::<u32>(0x8CC01C).read(),
        );
        return;
    }
    vehicle_stop_engine_sound(this, 0x138);
}

orig! {
    /// `CAEVehicleAudioEntity::ProcessDummyStateTransition(int16 newState, float ratio, cVehicleParams&)` (0x4FCA10)
    fn vehicle_process_dummy_state_transition(this: GPtr<u8>, state: u32, ratio: u32, params: GPtr<u8>) = thiscall 0x4FCA10 => unported;
}

/// `min(|speed / transmission max velocity|, 1.0)` of `cVehicleParams` (no transmission: 0), as the
/// bytes compute it (single precision; unordered passes).
fn vehicle_speed_ratio(params: GPtr<u8>) -> f32 {
    let tr = rd::<u32>(params, 0x14);
    if tr == 0 {
        return 0.0;
    }
    let one = g::<f32>(0x858624).read();
    let r = (rd::<f32>(params, 0x1C) / rd::<f32>(g(tr as usize), 0x58)).abs();
    if one < r { one } else { r }
}

/// `ProcessDummyVehicleEngine` jump-table index bytes (`.text`, dummy states 0..10, read in place).
const DUMMY_ENGINE_STATE_INDEX: usize = 0x501640;

/// `CAEVehicleAudioEntity::ProcessDummyVehicleEngine(cVehicleParams&)` (0x501480): no bank slot (+0xE0 ==
/// -1) requests one for the engine bank (+0xDC) and stops if none. A bank not yet loaded in the slot:
/// still pending (the dummy slot holds the bank) -> nothing; else a new slot is requested and the dummy
/// state +0xA9 = 0. Loaded: ratio = the speed ratio when the vehicle's bit 0x10 of +0x428 is set (0 for
/// entity type bits 0x20 of +0x36, else 0 when the bit is clear); dummy state (+0xA9) 0, 1, 10: ratio <
/// [0x8CBBF0] -> `ProcessDummyStateTransition(1, ratio, params)` else 2; 2: against [0x8CBBF4]; others
/// and the bit-clear case: transition 0.
#[hook(addr = 0x501480, conv = thiscall, name = "CAEVehicleAudioEntity::ProcessDummyVehicleEngine", expect = "56 8B F1 66 83 BE E0 00 00 00 FF")]
pub fn vehicle_process_dummy_vehicle_engine(this: GPtr<u8>, params: GPtr<u8>) {
    if rd::<u16>(this, 0xE0) == 0xFFFF {
        let r = vehicle_request_bank_slot(rd::<u16>(this, 0xDC) as u32);
        wr::<u16>(this, 0xE0, r as u16);
        if r as u16 == 0xFFFF {
            return;
        }
    }
    if hw_is_sound_bank_loaded(
        g(0xB5F8B8),
        rd::<u16>(this, 0xDC) as u32,
        rd::<u16>(this, 0xE0) as u32,
    ) == 0
    {
        let i = rd::<u16>(this, 0xE0).wrapping_sub(7) as i16;
        if (0..10).contains(&i) && g::<u16>(0xB6B9A0 + i as usize * 4).read() == rd::<u16>(this, 0xDC) {
            return;
        }
        let r = vehicle_request_bank_slot(rd::<u16>(this, 0xDC) as u32);
        wr::<u16>(this, 0xE0, r as u16);
        wr::<u8>(this, 0xA9, 0);
        return;
    }
    let veh: GPtr<u8> = g(rd::<u32>(params, 0x10) as usize);
    let mut state = 0u32;
    let mut ratio = 0.0f32;
    if rd::<u8>(veh, 0x428) & 0x10 != 0 {
        ratio = vehicle_speed_ratio(params);
        if rd::<u8>(g(rd::<u32>(this, 4) as usize), 0x36) & 0xF8 == 0x20 {
            ratio = 0.0;
        }
        let ds = rd::<u8>(this, 0xA9) as i8 as i32 as u32;
        if ds <= 10 {
            let limit = match g::<u8>(DUMMY_ENGINE_STATE_INDEX + ds as usize).read() {
                0 => Some(0x8CBBF0),
                1 => Some(0x8CBBF4),
                _ => None,
            };
            if let Some(c) = limit {
                state = if ratio < g::<f32>(c).read() { 1 } else { 2 };
            }
        }
    }
    vehicle_process_dummy_state_transition(this, state, ratio.to_bits(), params);
}

/// `CAEVehicleAudioEntity::UpdateTrainSound(int16 type, int16 bankSlot, int16 sfx, float speed, float
/// volume)` (0x4FA1C0, `ret 0x14`): nothing without a bank slot (+0xE0). Volume += +0xD8. An existing
/// sound of the type (+0xE8 + type*8) gets volume (+0x14) and speed (+0x1C). Otherwise types 1 and 2
/// need the engine bank (+0xDC) loaded in the slot (the slot dword passed keeps the upper half of
/// `this`, as the original's ECX), type 3 does not; the temp sound (+8) is initialised (bankSlot, sfx,
/// the entity's position, volume, 1.0 x3, zeros), +0x20 = type 1: [0x8CBF18] / [0x8CBEFC], type 2:
/// [0x8CBF1C] / [0x8CBF00] (bank 0x85 / other), type 3: [0x8CBEF4]; +0x1C = volume, +0x24 = speed,
/// flags word +0x5E = 4; the requested sound is stored at the type's slot. Other types: nothing.
#[hook(addr = 0x4FA1C0, conv = thiscall, name = "CAEVehicleAudioEntity::UpdateTrainSound", expect = "53 56 8B F1 66 8B 8E E0 00 00 00")]
pub fn vehicle_update_train_sound(
    this: GPtr<u8>,
    ty: u32,
    bank_slot: u32,
    sfx: u32,
    speed: u32,
    volume: f32,
) {
    let slot = rd::<u16>(this, 0xE0);
    if slot == 0xFFFF {
        return;
    }
    let vol = volume + rd::<f32>(this, 0xD8);
    let t = ty as u16 as i16 as isize;
    let field = 0xE8 + t * 8;
    let s = rd::<u32>(this, field);
    if s != 0 {
        wr::<f32>(g(s as usize), 0x14, vol);
        wr::<u32>(g(rd::<u32>(this, field) as usize), 0x1C, speed);
        return;
    }
    let slot_dword = (this.addr() as u32 & 0xFFFF_0000) | slot as u32;
    let bank = rd::<u16>(this, 0xDC);
    let dist_addr = match t {
        1 | 2 => {
            if hw_is_sound_bank_loaded(g(0xB5F8B8), bank as u32, slot_dword) == 0 {
                return;
            }
            None
        }
        3 => Some(0x8CBEF4usize),
        _ => return,
    };
    let p = entity_pos(g(rd::<u32>(this, 4) as usize));
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        bank_slot,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0,
        0,
        0,
    );
    let d = dist_addr.unwrap_or_else(|| {
        let b85 = rd::<u16>(this, 0xDC) == 0x85;
        match (t, b85) {
            (1, true) => 0x8CBF18,
            (1, false) => 0x8CBEFC,
            (_, true) => 0x8CBF1C,
            (_, false) => 0x8CBF00,
        }
    });
    wr::<u32>(this, 0x20, g::<u32>(d).read());
    wr::<f32>(this, 0x1C, vol);
    wr::<u32>(this, 0x24, speed);
    wr::<u16>(this, 0x5E, 4);
    let r = sm_request_new_sound(g(0xB62CB0), snd);
    wr::<u32>(this, field, r);
}

// ---- pass 3: weapon ------------------------------------------------------------------------------

orig! {
    /// `CAEWeaponAudioEntity::PlayMiniGunStopSound(CPhysical*)` (0x504960)
    fn weapon_play_mini_gun_stop_sound(this: GPtr<u8>, physical: u32) = thiscall 0x504960 => unported;
    /// `CAEWeaponAudioEntity::PlayChainsawStopSound(CPhysical*)` (0x504AA0)
    fn weapon_play_chainsaw_stop_sound(this: GPtr<u8>, physical: u32) = thiscall 0x504AA0 => unported;
}

/// `CAEWeaponAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x504B70). Position -1: the
/// sound +0x9C is forgotten, else a sound of event 8 with state +0x7F == 2 sets it to 3. Position >= 0,
/// by the sound's event (+0xC; `now` read at entry; "expired" = field + 300 < now, unsigned): 1 ->
/// volume -1 while 0xB72914 is set; 2 -> expired +0x84: stop and forget, +0x84 = 0; else volume fades
/// up by [0x858CA0] to event 0x91's volume - [0x859030]; 3 -> expired +0x88: stop, +0x88 = 0; 4 ->
/// expired +0x8C: stop, +0x8C = 0; else speed (+0x1C) up by [0x858C58] to [0x862E1C]; 5 / 6 -> expired
/// +0x90: `PlayMiniGunStopSound(sound physical)`; then unless state +0x7F is 0 / 1 (5) or 1 (6): stop,
/// +0x7C / +0x7D = 0; 7 -> unless +0x7F == 1: volume -[0x858CE8] while > [0x859CE4], else stop; 9 ->
/// expired +0x94: +0x80 = 4; +0x80 != 0: stop; 10 -> +0x80 == 2: +0x94 + 400 < now: +0x94 = now, +0x80
/// = 1; else speed down by [0x858FCC] to [0x862E1C]; +0x80 == 1: speed up by [0x858B10] to 1.0, then
/// expired +0x94: `PlayChainsawStopSound`; other: stop; 12 -> +0x80 == 3 and position > 1000: +0x80 =
/// 0; 13 / 14 -> once `ftol(sound+0x10) + 820 / 2200 < now` (signed): speed 0.84 / 1.0; others
/// (incl. 8, 11) -> volume fades down by [0x858FA0] to 0. Single-precision x87.
#[hook(addr = 0x504B70, conv = thiscall, name = "CAEWeaponAudioEntity::UpdateParameters", expect = "53 55 8B 2D 84 CB B7 00")]
pub fn weapon_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    use core::cmp::Ordering::{Greater, Less};
    let now = sa_core::timer::TIME_MS.get();
    let p = pos as u16 as i16;
    let s: GPtr<u8> = g(sound as usize);
    if p < 0 {
        if p == -1 {
            if sound == rd::<u32>(this, 0x9C) {
                wr::<u32>(this, 0x9C, 0);
            } else if rd::<u32>(s, 0xC) == 8 && rd::<u8>(this, 0x7F) == 2 {
                wr::<u8>(this, 0x7F, 3);
            }
        }
        return;
    }
    let expired = |off: isize, add: u32| rd::<u32>(this, off).wrapping_add(add) < now;
    let gf = |a: usize| g::<f32>(a).read();
    // value at `off` moves up by `step` towards `lim` when below it
    let fade_up = |off: isize, step: f32, lim: f32| {
        let v = rd::<f32>(s, off);
        if v.partial_cmp(&lim) == Some(Less) {
            let n = v + step;
            wr::<f32>(s, off, if n.partial_cmp(&lim) == Some(Less) { n } else { lim });
        }
    };
    let fade_down = |off: isize, step: f32, lim: f32| {
        let v = rd::<f32>(s, off);
        if v.partial_cmp(&lim) == Some(Greater) {
            let n = v - step;
            wr::<f32>(
                s,
                off,
                if n.partial_cmp(&lim) == Some(Greater) {
                    n
                } else {
                    lim
                },
            );
        }
    };
    match rd::<u32>(s, 0xC) {
        1 => {
            if g::<u32>(0xB72914).read() != 0 {
                wr::<f32>(s, 0x14, rd::<f32>(s, 0x14) - gf(0x858624));
            }
        }
        2 => {
            if expired(0x84, 300) {
                sound_stop_and_forget(s);
                wr::<u32>(this, 0x84, 0);
            } else {
                let target = event_volume(0x91) - gf(0x859030);
                fade_up(0x14, gf(0x858CA0), target);
            }
        }
        3 => {
            if expired(0x88, 300) {
                sound_stop_and_forget(s);
                wr::<u32>(this, 0x88, 0);
            }
        }
        4 => {
            if expired(0x8C, 300) {
                sound_stop_and_forget(s);
                wr::<u32>(this, 0x8C, 0);
            } else {
                fade_up(0x1C, gf(0x858C58), gf(0x862E1C));
            }
        }
        5 => {
            if expired(0x90, 300) {
                weapon_play_mini_gun_stop_sound(this, rd::<u32>(s, 8));
            }
            let st = rd::<u8>(this, 0x7F);
            if st != 0 && st != 1 {
                sound_stop_and_forget(s);
                wr::<u8>(this, 0x7C, 0);
            }
        }
        6 => {
            if expired(0x90, 300) {
                weapon_play_mini_gun_stop_sound(this, rd::<u32>(s, 8));
            }
            if rd::<u8>(this, 0x7F) != 1 {
                sound_stop_and_forget(s);
                wr::<u8>(this, 0x7D, 0);
            }
        }
        7 => {
            if rd::<u8>(this, 0x7F) != 1 {
                let v = rd::<f32>(s, 0x14);
                if v > gf(0x859CE4) {
                    wr::<f32>(s, 0x14, v - gf(0x858CE8));
                } else {
                    sound_stop_and_forget(s);
                }
            }
        }
        9 => {
            if expired(0x94, 300) {
                wr::<u8>(this, 0x80, 4);
            }
            if rd::<u8>(this, 0x80) != 0 {
                sound_stop_and_forget(s);
            }
        }
        10 => match rd::<u8>(this, 0x80) {
            2 => {
                if expired(0x94, 400) {
                    wr::<u32>(this, 0x94, now);
                    wr::<u8>(this, 0x80, 1);
                } else {
                    fade_down(0x1C, gf(0x858FCC), gf(0x862E1C));
                }
            }
            1 => {
                fade_up(0x1C, gf(0x858B10), gf(0x858624));
                if expired(0x94, 300) {
                    weapon_play_chainsaw_stop_sound(this, rd::<u32>(s, 8));
                }
            }
            _ => sound_stop_and_forget(s),
        },
        12 => {
            if rd::<u8>(this, 0x80) == 3 && p > 1000 {
                wr::<u8>(this, 0x80, 0);
            }
        }
        13 | 14 => {
            let add: i32 = if rd::<u32>(s, 0xC) == 13 { 0x334 } else { 0x898 };
            let t = crate::common::ftol32(crate::common::x(rd::<f32>(s, 0x10))).wrapping_add(add);
            if t < now as i32 {
                wr::<u32>(s, 0x1C, if add == 0x334 { 0x3F57_0A3D } else { FLT_ONE_BITS });
            }
        }
        _ => fade_down(0x14, gf(0x858FA0), gf(0x858B50)),
    }
}

// ---- pass 3: door --------------------------------------------------------------------------------

/// `CAEDoorAudioEntity::PlayDoorSound(int16 sfx, eAudioEvents, CVector& posn, float volumeDelta, float
/// speed)` (0x4DC6D0, `ret 0x14`): bank 0x33 must be in slot 0x1F (else loaded). A `CAESound` local
/// (only +8 cleared in the original; zeroed here, see `PlayWaterSounds`) is initialised (slot 0x1F, sfx,
/// position, event volume + delta, 2.0, speed, 1.0, 0, flags 4, 0, 0); the position is `posn` unless it
/// is (-1000, -1000, -1000) (dword compare) or exactly (0, 0, 0), then (0, 1, 0) with `SetFlags(1, 1)`
/// (front end; otherwise `SetFlags(1, 0)`); event +0xC = the event; requested; the local destroyed. The
/// `SetFlags` value dword's upper half is the original's leftover EAX (only the word is read). SEH frame
/// not reproduced.
#[hook(addr = 0x4DC6D0, conv = thiscall, name = "CAEDoorAudioEntity::PlayDoorSound", expect = "6A FF 68 B8 BE 83 00")]
pub fn door_play_door_sound(this: GPtr<u8>, sfx: u32, event: u32, posn: u32, vol_delta: f32, speed: u32) {
    let mut buf = [0u32; 0x74 / 4];
    let snd: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
    wr::<u32>(snd, 8, 0);
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x33, 0x1F) == 0 {
        hw_load_sound_bank(g(0xB5F8B8), 0x33, 0x1F);
    } else {
        let vol = event_volume(event) + vol_delta;
        let p: GPtr<u8> = g(posn as usize);
        let raw = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
        let off_screen = raw.iter().all(|&v| v == 0xC47A_0000);
        let zero = g::<f32>(0x858B50).read();
        let at_origin = !off_screen && (0..3).all(|i| rd::<f32>(p, i * 4) == zero);
        let (pos, front_end) = if off_screen || at_origin {
            ([0u32, FLT_ONE_BITS, 0u32], 1u32)
        } else {
            (raw, 0u32)
        };
        sound_initialise(
            snd,
            0x1F,
            sfx,
            this.addr() as u32,
            pos[0],
            pos[1],
            pos[2],
            vol.to_bits(),
            0x4000_0000,
            speed,
            FLT_ONE_BITS,
            0,
            4,
            0,
            0,
        );
        sound_set_flags(snd, 1, front_end);
        wr::<u32>(snd, 0xC, event);
        sm_request_new_sound(g(0xB62CB0), snd);
    }
    sound_destructor(snd);
}

/// `CAEFireAudioEntity::PlayFireSounds(eAudioEvents, CVector& posn)` (0x4DD0D0): the variation byte
/// 0xB612EC = (signed +1) % 5 picks the speed dword at 0x8AE598; a `CAESound` local (only +8 cleared in
/// the original; zeroed here) is initialised (slot 4, sfx 0, `posn`, event volume, 2.0, speed, 1.0, 0,
/// flags 4, 0, 0) and requested -> +0x7C. With bank 0x8A in slot 0x13: (slot 0x13, sfx 0x1A, volume -
/// [0x85F100], 2.0, speed * [0x858CC8], 1.0, 0, 4, 0, 0), then local +0x10 = volume + [0x858B3C] and
/// event +0xC = 1, requested (result unused); else the bank is loaded. The local is destroyed. SEH frame
/// not reproduced.
#[hook(addr = 0x4DD0D0, conv = thiscall, name = "CAEFireAudioEntity::PlayFireSounds", expect = "6A FF 68 F8 BE 83 00")]
pub fn fire_play_fire_sounds(this: GPtr<u8>, event: u32, posn: u32) {
    let mut buf = [0u32; 0x74 / 4];
    let snd: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
    wr::<u32>(snd, 8, 0);
    let v = ((g::<u8>(0xB612EC).read() as i8 as i32 + 1) % 5) as i8;
    let me = this.addr() as u32;
    g::<u8>(0xB612EC).write(v as u8);
    let vol = event_volume(event);
    let speed_at = (0x8AE598i32 + v as i32 * 4) as u32 as usize;
    let p: GPtr<u8> = g(posn as usize);
    sound_initialise(
        snd,
        4,
        0,
        me,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol.to_bits(),
        0x4000_0000,
        g::<u32>(speed_at).read(),
        FLT_ONE_BITS,
        0,
        4,
        0,
        0,
    );
    let r = sm_request_new_sound(g(0xB62CB0), snd);
    wr::<u32>(this, 0x7C, r);
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8A, 0x13) != 0 {
        let speed2 = g::<f32>(speed_at).read() * g::<f32>(0x858CC8).read();
        let vol2 = vol - g::<f32>(0x85F100).read();
        sound_initialise(
            snd,
            0x13,
            0x1A,
            me,
            rd::<u32>(p, 0),
            rd::<u32>(p, 4),
            rd::<u32>(p, 8),
            vol2.to_bits(),
            0x4000_0000,
            speed2.to_bits(),
            FLT_ONE_BITS,
            0,
            4,
            0,
            0,
        );
        wr::<f32>(snd, 0x10, vol + g::<f32>(0x858B3C).read());
        wr::<u32>(snd, 0xC, 1);
        sm_request_new_sound(g(0xB62CB0), snd);
    } else {
        hw_load_sound_bank(g(0xB5F8B8), 0x8A, 0x13);
    }
    sound_destructor(snd);
}

// ---- pass 3: vehicle (2) -------------------------------------------------------------------------

orig! {
    /// `CVehicle::UsesSiren()` (0x6D8470, AL)
    fn vehicle_uses_siren(veh: GPtr<u8>) -> u8 = thiscall 0x6D8470 => unported;
}

/// `CAEVehicleAudioEntity::Initialise` model jump-table index bytes (models 0x1C0..0x247, read in place):
/// 0 -> +0xB0 = 1, 1 -> 0.
const VEHICLE_INIT_MODEL_INDEX: usize = 0x4F7998;

/// `CAEVehicleAudioEntity::Initialise(CEntity*)` (0x4F7670): resets the entity's state (in the bytes'
/// store order), the 12 engine sound slots (+0xE4 type word = index, +0xE8 sound = 0), copies the
/// model's 36-byte `tVehicleAudioSettings` (0x860AF0 + (model - 400) * 36) to +0x80, +0xC8 =
/// `UsesSiren`, radio type +0x9B == 2 -> +0x9A = 0xD, +0xD8 = event 0x4D volume, +0xB0 from the bytes'
/// model table. Then by audio type (+0x80, the bytes' table): engine bank word +0xDC = +0x84 and
/// +0xDE = +0x82, types 0, 1, 9 lower +0xD8 by [0x8CBD50]; types 0, 3, 8 (not bank 0x81), 1, 2, 9
/// request a dummy bank slot (+0xE0) unless already initialised (+0xA4) or the bank is -1; type 5 too
/// (no 0x81 test); types 4 / 6 only set the banks; then +0xA4 = 1. Type 7 and types outside 0..9 return
/// before +0xA4 is set.
#[hook(addr = 0x4F7670, conv = thiscall, name = "CAEVehicleAudioEntity::Initialise", expect = "51 8B 54 24 08")]
pub fn vehicle_initialise(this: GPtr<u8>, entity: u32) {
    let e = this;
    wr::<u32>(e, 0x144, 0);
    wr::<u32>(e, 4, entity);
    for off in [0xA5isize, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA] {
        wr::<u8>(e, off, 0);
    }
    wr::<u32>(e, 0xAC, 0);
    wr::<u16>(e, 0xE0, 0xFFFF);
    wr::<u16>(e, 0xB2, 0);
    wr::<u16>(e, 0x7C, 0);
    wr::<u16>(e, 0xB4, 0);
    wr::<u32>(e, 0xB8, 0);
    wr::<u8>(e, 0xBC, 0);
    wr::<u32>(e, 0xCC, 0);
    wr::<u32>(e, 0xD0, 0);
    wr::<u32>(e, 0xD4, 0);
    wr::<u8>(e, 0x244, 0);
    wr::<u8>(e, 0xBD, 0);
    wr::<u16>(e, 0x148, 0xFFFF);
    wr::<u16>(e, 0x14A, 0xFFFF);
    wr::<u16>(e, 0x154, 0);
    wr::<u16>(e, 0x14E, 0);
    wr::<u16>(e, 0x14C, 0);
    for i in 0..12isize {
        wr::<u16>(e, 0xE4 + i * 8, i as u16);
        wr::<u32>(e, 0xE8 + i * 8, 0);
    }
    wr::<u32>(e, 0xC4, 0xC2C8_0000);
    wr::<u32>(e, 0x230, 0xC2C8_0000);
    for off in [0x156isize, 0x15C, 0x164, 0x16C] {
        wr::<u16>(e, off, 0xFFFF);
    }
    wr::<u32>(e, 0x234, 0xBF80_0000);
    wr::<u32>(e, 0x22C, 0xBF80_0000);
    wr::<u32>(e, 0x248, 0xBF80_0000);
    for off in [0x158isize, 0x160, 0x168, 0x170, 0x178, 0x17C, 0x180, 0x238] {
        wr::<u32>(e, off, 0);
    }
    wr::<u32>(e, 0x23C, FLT_ONE_BITS);
    wr::<u32>(e, 0x240, 0);
    let ent: GPtr<u8> = g(entity as usize);
    let row = (rd::<i16>(ent, 0x22) as i32).wrapping_sub(400).wrapping_mul(36);
    let src: GPtr<u8> = g((0x860AF0i32).wrapping_add(row) as u32 as usize);
    for i in 0..9isize {
        wr::<u32>(e, 0x80 + i * 4, rd::<u32>(src, i * 4));
    }
    wr::<u8>(e, 0xC8, vehicle_uses_siren(ent));
    if rd::<u8>(e, 0x9B) as i8 == 2 {
        wr::<u8>(e, 0x9A, 0xD);
    }
    wr::<f32>(e, 0xD8, event_volume(0x4D));
    let m = (rd::<i16>(g(entity as usize), 0x22) as i32).wrapping_sub(0x1C0) as u32;
    let b0 = m <= 0x87 && g::<u8>(VEHICLE_INIT_MODEL_INDEX + m as usize).read() == 0;
    wr::<u8>(e, 0xB0, b0 as u8);
    let ty = rd::<u8>(e, 0x80) as i8;
    let lower_volume = || wr::<f32>(e, 0xD8, rd::<f32>(e, 0xD8) - g::<f32>(0x8CBD50).read());
    let request = || {
        let r = vehicle_request_bank_slot(rd::<u16>(e, 0xDC) as u32);
        wr::<u16>(e, 0xE0, r as u16);
    };
    match ty {
        0 | 3 | 8 => {
            if ty == 0 {
                lower_volume();
            }
            wr::<u16>(e, 0xDE, rd::<u16>(e, 0x82));
            let initialised = rd::<u8>(e, 0xA4) != 0;
            let bank = rd::<u16>(e, 0x84);
            wr::<u16>(e, 0xDC, bank);
            if !initialised && bank != 0xFFFF && bank != 0x81 {
                request();
            }
        }
        1 | 2 | 9 => {
            wr::<u16>(e, 0xDE, rd::<u16>(e, 0x82));
            if ty != 2 {
                lower_volume();
            }
            let initialised = rd::<u8>(e, 0xA4) != 0;
            let bank = rd::<u16>(e, 0x84);
            wr::<u16>(e, 0xDC, bank);
            if !initialised && bank != 0xFFFF {
                request();
            }
        }
        5 => {
            let bank = rd::<u16>(e, 0x84);
            wr::<u16>(e, 0xDC, bank);
            wr::<u16>(e, 0xDE, rd::<u16>(e, 0x82));
            if bank != 0xFFFF && rd::<u8>(e, 0xA4) == 0 {
                request();
            }
        }
        4 | 6 => {
            wr::<u16>(e, 0xDC, rd::<u16>(e, 0x84));
            wr::<u16>(e, 0xDE, rd::<u16>(e, 0x82));
        }
        _ => return,
    }
    wr::<u8>(e, 0xA4, 1);
}

/// `CAEPedlessSpeechAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4E4D10): the speech
/// slot (+0xA8) address is taken first. Position -1: +0x98 = 0, the slot cleared, +0xA0 = 0, +0xA4 /
/// +0xA6 / +0xA8 = -1, +0x9C = 0. Otherwise the slot's status = 5 and, not front-end (+0x9B == 0) for
/// an event 0x34 sound with an entity (+0x114), the sound follows that entity. An unconstructed
/// `CMatrix` local (+0x40 / +0x44 zeroed) is destroyed at the end (zeroed local here); SEH frame not
/// reproduced.
#[hook(addr = 0x4E4D10, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::UpdateParameters", expect = "6A FF 68 18 C0 83 00")]
pub fn pedless_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    let mut mat = [0u32; 0x48 / 4];
    let slot = speech_slot(rd::<i16>(this, 0xA8));
    if pos as u16 == 0xFFFF {
        wr::<u8>(this, 0x98, 0);
        clear_speech_slot(slot);
        wr::<u32>(this, 0xA0, 0);
        wr::<u16>(this, 0xA4, 0xFFFF);
        wr::<u16>(this, 0xA6, 0xFFFF);
        wr::<u16>(this, 0xA8, 0xFFFF);
        wr::<u8>(this, 0x9C, 0);
    } else {
        wr::<u16>(slot, 0, 5);
        let s: GPtr<u8> = g(sound as usize);
        if rd::<u8>(this, 0x9B) == 0 && rd::<u32>(s, 0xC) == 0x34 {
            let ent = rd::<u32>(this, 0x114);
            if ent != 0 {
                let p = entity_pos(g(ent as usize));
                sound_set_position(s, p[0], p[1], p[2]);
            }
        }
    }
    matrix_destructor(GPtr::from_raw(mat.as_mut_ptr()).cast());
}

orig! {
    /// `CAEPedSpeechAudioEntity::CanWePlayScriptedSpeech()` (0x4E4950)
    fn speech_can_we_play_scripted_speech_call() -> u16 = cdecl 0x4E4950 => speech_can_we_play_scripted_speech;
    /// `CAEPedSpeechAudioEntity::GetSoundAndBankIDsForScriptedSpeech(eAudioEvents)` (0x4E4400)
    fn speech_get_sound_and_bank_ids_for_scripted_speech_call(this: GPtr<u8>, event: u32) -> u8 = thiscall 0x4E4400 => speech_get_sound_and_bank_ids_for_scripted_speech;
}

/// `CAEPedSpeechAudioEntity::AddScriptSayEvent(int32 event, eAudioEvents scriptEvent, bool
/// overrideSilence, bool isForceAudible, bool isFrontEnd)` (0x4E4F70, `ret 0x14`): nothing unless
/// initialised (+0x90), speech not all disabled, no widescreen, event 0x35, not silenced (+0x99) without
/// `overrideSilence`, not talking (+0x98) and streaming not very busy. CJ (type 2) uses slot 5 unless
/// another player is speaking; others `CanWePlayScriptedSpeech` (0..4). Needs
/// `GetSoundAndBankIDsForScriptedSpeech(scriptEvent)`. Then +0x9B = isFrontEnd, +0x9C = isForceAudible,
/// +0xAC = event 0x35 volume, +0x9B = 1 for a driver (flag 0x100 at +0x46C) of the player's vehicle,
/// CJ adds [0x8C80E8], +0xA8 = slot, +0x98 = 1 (CJ: `s_bAPlayerSpeaking` = 1), +0xB0 = -1,
/// `LoadAndPlaySpeech(0)`.
#[hook(addr = 0x4E4F70, conv = thiscall, name = "CAEPedSpeechAudioEntity::AddScriptSayEvent", expect = "56 8B F1 8A 86 90 00 00 00")]
pub fn speech_add_script_say_event(
    this: GPtr<u8>,
    event: u32,
    script_event: u32,
    override_silence: u32,
    force_audible: u32,
    front_end: u32,
) {
    if rd::<u8>(this, 0x90) == 0
        || g::<u8>(0xB613E6).read() != 0
        || g::<u8>(0xB6F065).read() != 0
        || event != 0x35
    {
        return;
    }
    if rd::<u8>(this, 0x99) != 0 && override_silence as u8 == 0 {
        return;
    }
    if rd::<u8>(this, 0x98) != 0 || streaming_is_very_busy() != 0 {
        return;
    }
    let slot: u16 = if rd::<u16>(this, 0x92) == 2 {
        if g::<u8>(0xB613E5).read() != 0 && rd::<u8>(this, 0x98) == 0 {
            return;
        }
        5
    } else {
        let s = speech_can_we_play_scripted_speech_call();
        if !(0..5).contains(&(s as i16)) {
            return;
        }
        s
    };
    if speech_get_sound_and_bank_ids_for_scripted_speech_call(this, script_event) == 0 {
        return;
    }
    wr::<u8>(this, 0x9B, front_end as u8);
    wr::<u8>(this, 0x9C, force_audible as u8);
    wr::<f32>(this, 0xAC, event_volume(0x35));
    let ped: GPtr<u8> = g(rd::<u32>(this, 4) as usize);
    if rd::<u32>(ped, 0x58C) != 0 && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = ae_find_vehicle_of_player();
        if rd::<u32>(g(rd::<u32>(this, 4) as usize), 0x58C) == v {
            wr::<u8>(this, 0x9B, 1);
        }
    }
    let ty = rd::<u16>(this, 0x92);
    if ty == 2 {
        wr::<f32>(this, 0xAC, g::<f32>(0x8C80E8).read() + rd::<f32>(this, 0xAC));
    }
    wr::<u16>(this, 0xA8, slot);
    wr::<u8>(this, 0x98, 1);
    if ty == 2 {
        g::<u8>(0xB613E5).write(1);
    }
    wr::<u16>(this, 0xB0, 0xFFFF);
    speech_load_and_play_speech(this, 0);
}

orig! {
    /// `CTaskComplexFacial::SetRequest(int32 type, int32 duration, int32 type2, float)` (0x691230)
    fn task_facial_set_request(task: GPtr<u8>, ty: u32, duration: u32, ty2: u32, f: u32) = thiscall 0x691230 => unported;
}

/// `CAEPedSpeechAudioEntity::PlayLoadedSound()` (0x4E5CD0): nothing without an entity (+4). A `CAESound`
/// local (only +8 cleared in the original; zeroed here) is initialised with bank slot `+0xA8 + 0x14`
/// and sfx +0xA4: front end (+0x9B) at (0, 1, 0) with volume [0x8C80EC] and flags word +0x56 = 0x307;
/// otherwise at the entity's position with volume +0xAC, flags 6 and +0x18 = [0x8C80F0] (radio player
/// attached, 0xB6B98C) or [0x8C80F4]; force audible (+0x9C): local +0x14 = [0x8C80EC]. The requested
/// sound goes to +0xA0; when one was created: its speech slot's sound (+8) and status 5, and, unless
/// the context (+0xB0) is 0x151 / 0x152, the ped's facial task (`GetTaskSecondary(3)`, not null-checked)
/// gets `SetRequest(7, 2800, -1, 0)`; otherwise the slot is cleared, +0x98 = 0 (CJ: `s_bAPlayerSpeaking`
/// = 0), +0xA8 = -1. The local is destroyed; SEH frame not reproduced.
#[hook(addr = 0x4E5CD0, conv = thiscall, name = "CAEPedSpeechAudioEntity::PlayLoadedSound", expect = "6A FF 68 38 C0 83 00")]
pub fn speech_play_loaded_sound(this: GPtr<u8>) {
    let mut buf = [0u32; 0x74 / 4];
    let snd: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
    wr::<u32>(snd, 8, 0);
    let ent = rd::<u32>(this, 4);
    if ent != 0 {
        let pos = entity_pos(g(ent as usize));
        let slot = rd::<u16>(this, 0xA8).wrapping_add(0x14) as u32;
        let sfx = rd::<u16>(this, 0xA4) as u32;
        let me = this.addr() as u32;
        if rd::<u8>(this, 0x9B) != 0 {
            sound_initialise(
                snd,
                slot,
                sfx,
                me,
                0,
                FLT_ONE_BITS,
                0,
                g::<u32>(0x8C80EC).read(),
                FLT_ONE_BITS,
                FLT_ONE_BITS,
                FLT_ONE_BITS,
                0,
                0,
                0,
                0,
            );
            wr::<u16>(snd, 0x56, 0x307);
        } else {
            sound_initialise(
                snd,
                slot,
                sfx,
                me,
                pos[0],
                pos[1],
                pos[2],
                rd::<u32>(this, 0xAC),
                FLT_ONE_BITS,
                FLT_ONE_BITS,
                FLT_ONE_BITS,
                0,
                0,
                0,
                0,
            );
            wr::<u16>(snd, 0x56, 6);
            let d = if g::<u32>(0xB6B98C).read() != 0 {
                0x8C80F0
            } else {
                0x8C80F4
            };
            wr::<u32>(snd, 0x18, g::<u32>(d).read());
        }
        if rd::<u8>(this, 0x9C) != 0 {
            wr::<u32>(snd, 0x14, g::<u32>(0x8C80EC).read());
        }
        let r = sm_request_new_sound(g(0xB62CB0), snd);
        wr::<u32>(this, 0xA0, r);
        if r != 0 {
            wr::<u32>(speech_slot(rd::<i16>(this, 0xA8)), 8, r);
            wr::<u16>(speech_slot(rd::<i16>(this, 0xA8)), 0, 5);
            let c = rd::<u16>(this, 0xB0);
            if c != 0x152 && c != 0x151 {
                let intel = rd::<u32>(g(rd::<u32>(this, 4) as usize), 0x47C);
                let t = task_manager_get_task_secondary(g(intel as usize + 4), 3);
                task_facial_set_request(g(t as usize), 7, 0xAF0, 0xFFFF_FFFF, 0);
            }
        } else {
            clear_speech_slot(speech_slot(rd::<i16>(this, 0xA8)));
            wr::<u8>(this, 0x98, 0);
            if rd::<u16>(this, 0x92) == 2 {
                g::<u8>(0xB613E5).write(0);
            }
            wr::<u16>(this, 0xA8, 0xFFFF);
        }
    }
    sound_destructor(snd);
}

orig! {
    /// `CAEVehicleAudioEntity::DemandBankSlot(int16)` (0x4F4E60)
    fn vehicle_demand_bank_slot_call(bank: u32) -> u32 = cdecl 0x4F4E60 => vehicle_demand_bank_slot;
}

/// `CAEVehicleAudioEntity::JustGotInVehicleAsDriver()` (0x4F5700): +0xA7 = 0, `s_pPlayerDriver`
/// (0xB6B990) = the vehicle's driver (+0x460). By audio type (+0x80): 0 / 1: +0xAA = 0, +0xAC = 0,
/// +0x148 / +0x14A = -1, +0xD8 = event 0x4D volume, slot 0x28's sounds cancelled when any play; with
/// both engine banks (+0xDC, +0xDE) set: a dummy slot demanded if none (+0xE0), bank +0xDE loaded into
/// slot 0x28 if missing, then dummy state 1 with a slot cancels engine sounds except 2 and sets state 6,
/// else cancels all 12 and sets state 0. 4 / 5 / 6: all engine sounds cancelled, +0xDE loaded into 0x28
/// (cancelling the slot's sounds first) if missing, a dummy slot demanded if none, +0x22C = -1.0, +0x230
/// = -100.0. 2: volume, slot 0x28 cancelled, engine sounds cancelled, dummy slot if none, +0xDE loaded
/// if missing. 3: volume, dummy slot if none. 8: as 2 without cancelling engine sounds. 9: as 2 but the
/// dummy slot only when +0xDC is set. Other types: nothing more.
#[hook(addr = 0x4F5700, conv = thiscall, name = "CAEVehicleAudioEntity::JustGotInVehicleAsDriver", expect = "51 53 56 8B F1")]
pub fn vehicle_just_got_in_vehicle_as_driver(this: GPtr<u8>) {
    let e = this;
    wr::<u8>(e, 0xA7, 0);
    g::<u32>(0xB6B990).write(rd::<u32>(g(rd::<u32>(e, 4) as usize), 0x460));
    let store_volume = || wr::<f32>(e, 0xD8, event_volume(0x4D));
    let cancel_slot = || {
        if sm_are_sounds_playing_in_bank_slot(g(0xB62CB0), 0x28) as u16 != 0 {
            sm_cancel_sounds_in_bank_slot(g(0xB62CB0), 0x28, 0);
        }
    };
    let cancel_engine = |skip: Option<u32>| {
        for i in 0..12u32 {
            if Some(i) != skip {
                vehicle_cancel_vehicle_engine_sound(e, i);
            }
        }
    };
    let demand = || {
        let r = vehicle_demand_bank_slot_call(rd::<u16>(e, 0xDC) as u32);
        wr::<u16>(e, 0xE0, r as u16);
    };
    let de_loaded = || hw_is_sound_bank_loaded(g(0xB5F8B8), rd::<u16>(e, 0xDE) as u32, 0x28) != 0;
    let load_de = || {
        hw_load_sound_bank(g(0xB5F8B8), rd::<u16>(e, 0xDE) as u32, 0x28);
    };
    match rd::<u8>(e, 0x80) {
        0 | 1 => {
            wr::<u8>(e, 0xAA, 0);
            wr::<u32>(e, 0xAC, 0);
            wr::<u16>(e, 0x148, 0xFFFF);
            wr::<u16>(e, 0x14A, 0xFFFF);
            store_volume();
            cancel_slot();
            if rd::<u16>(e, 0xDC) == 0xFFFF || rd::<u16>(e, 0xDE) == 0xFFFF {
                return;
            }
            if rd::<u16>(e, 0xE0) == 0xFFFF {
                demand();
            }
            if !de_loaded() {
                load_de();
            }
            if rd::<u8>(e, 0xA9) == 1 && rd::<u16>(e, 0xE0) != 0xFFFF {
                cancel_engine(Some(2));
                wr::<u8>(e, 0xA9, 6);
            } else {
                cancel_engine(None);
                wr::<u8>(e, 0xA9, 0);
            }
        }
        4..=6 => {
            cancel_engine(None);
            if rd::<u16>(e, 0xDE) != 0xFFFF && !de_loaded() {
                cancel_slot();
                load_de();
            }
            if rd::<u16>(e, 0xDC) != 0xFFFF && rd::<u16>(e, 0xE0) == 0xFFFF {
                demand();
            }
            wr::<u32>(e, 0x22C, 0xBF80_0000);
            wr::<u32>(e, 0x230, 0xC2C8_0000);
        }
        2 => {
            store_volume();
            cancel_slot();
            cancel_engine(None);
            if rd::<u16>(e, 0xE0) == 0xFFFF {
                demand();
            }
            if !de_loaded() {
                load_de();
            }
        }
        3 => {
            store_volume();
            if rd::<u16>(e, 0xE0) == 0xFFFF {
                demand();
            }
        }
        8 => {
            store_volume();
            cancel_slot();
            if rd::<u16>(e, 0xE0) == 0xFFFF {
                demand();
            }
            if !de_loaded() {
                load_de();
            }
        }
        9 => {
            store_volume();
            cancel_slot();
            cancel_engine(None);
            if rd::<u16>(e, 0xE0) == 0xFFFF && rd::<u16>(e, 0xDC) != 0xFFFF {
                demand();
            }
            if !de_loaded() {
                load_de();
            }
        }
        _ => {}
    }
}

// ---- pass 3: collision (4) -----------------------------------------------------------------------

/// `CAECollisionAudioEntity::ReportGlassCollisionEvent(eAudioEvents, CVector& posn, uint32 delay)`
/// (0x4DA070, `ret 0xC`): events 0x7B..0x80 (the bytes' table): sfx 0x33, 0x44, 0x38 (speed 0.75), 0x38,
/// `GetRandomNumberInRange(0xF, 0x12)`, the same with speed 0.56; others nothing. The temp sound (+8):
/// (slot 2, sfx, `posn`, event volume, 1.5, speed, 1.0, zeros); a non-zero delay gives +0x24 = 0, +0x18
/// = (float)(uint32)(now + delay), event +0x14, flags word +0x5E = 4; requested.
#[hook(addr = 0x4DA070, conv = thiscall, name = "CAECollisionAudioEntity::ReportGlassCollisionEvent", expect = "51 53 8B 5C 24 0C")]
pub fn collision_report_glass_collision_event(this: GPtr<u8>, event: u32, posn: u32, delay: u32) {
    let mut speed = FLT_ONE_BITS;
    let sfx: u32 = match event.wrapping_sub(0x7B) {
        0 => 0x33,
        1 => 0x44,
        2 => {
            speed = 0x3F40_0000;
            0x38
        }
        3 => 0x38,
        4 => crate::common::ae_random_int(0xF, 0x12) as u32,
        5 => {
            let r = crate::common::ae_random_int(0xF, 0x12) as u32;
            speed = 0x3F0F_5C29;
            r
        }
        _ => return,
    };
    let vol = event_volume(event);
    let p: GPtr<u8> = g(posn as usize);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        2,
        sfx,
        this.addr() as u32,
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        vol.to_bits(),
        0x3FC0_0000,
        speed,
        FLT_ONE_BITS,
        0,
        0,
        0,
        0,
    );
    if delay != 0 {
        wr::<u32>(this, 0x24, 0);
        let t = sa_core::timer::TIME_MS.get().wrapping_add(delay);
        wr::<f32>(this, 0x18, t as f32);
        wr::<u32>(this, 0x14, event);
        wr::<u16>(this, 0x5E, 4);
    }
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAECollisionAudioEntity::ReportWaterSplash(CVector posn, float volumeOffset)` (0x4DA190, the vector
/// by value, `ret 0x10`): bank 0x27 must be in slot 2 (else loaded unless the loading tune plays). Volume
/// = event 0x76 volume + offset. The temp sound (+8): (slot 2, sfx 0x43, `posn`, volume, 2.5, 1.26, 1.0,
/// 0, flags 4, 0, 0), event +0x14 = 1, requested; then (sfx 0x42, speed 0.0) with event 2 and +0x18 =
/// (float)(uint32)(now + 166), requested.
#[hook(addr = 0x4DA190, conv = thiscall, name = "CAECollisionAudioEntity::ReportWaterSplash-at-position", expect = "51 53 55 56 57")]
pub fn collision_report_water_splash_at_position(this: GPtr<u8>, px: u32, py: u32, pz: u32, vol_offset: f32) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x27, 2) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x27, 2);
        }
        return;
    }
    let vol = (event_volume(0x76) + vol_offset).to_bits();
    let me = this.addr() as u32;
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        2,
        0x43,
        me,
        px,
        py,
        pz,
        vol,
        0x4020_0000,
        0x3FA1_47AE,
        FLT_ONE_BITS,
        0,
        4,
        0,
        0,
    );
    wr::<u32>(this, 0x14, 1);
    sm_request_new_sound(g(0xB62CB0), snd);
    sound_initialise(
        snd,
        2,
        0x42,
        me,
        px,
        py,
        pz,
        vol,
        0x4020_0000,
        0,
        FLT_ONE_BITS,
        0,
        4,
        0,
        0,
    );
    wr::<u32>(this, 0x14, 2);
    let t = sa_core::timer::TIME_MS.get().wrapping_add(0xA6);
    wr::<f32>(this, 0x18, t as f32);
    sm_request_new_sound(g(0xB62CB0), snd);
}

// ---- pass 3: weapon (2) --------------------------------------------------------------------------

/// `CAEWeaponAudioEntity::PlayFlameThrowerIdleGasLoop(CPhysical*)` (0x503870): without a loop sound
/// (+0x9C): the temp sound (+8) (slot 5, sfx 0xA, the physical's position, event 0x9E volume, 0.66,
/// 1.0, 1.0, 0, flags 0x84, 0, 0), registered with the physical, requested -> +0x9C.
#[hook(addr = 0x503870, conv = thiscall, name = "CAEWeaponAudioEntity::PlayFlameThrowerIdleGasLoop", expect = "53 56 57 8B F9")]
pub fn weapon_play_flame_thrower_idle_gas_loop(this: GPtr<u8>, physical: u32) {
    if rd::<u32>(this, 0x9C) != 0 {
        return;
    }
    let phys: GPtr<u8> = g(physical as usize);
    let p = entity_pos(phys);
    let vol = event_volume(0x9E);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        0xA,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        0x3F28_F5C3,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    let r = sm_request_new_sound(g(0xB62CB0), snd);
    wr::<u32>(this, 0x9C, r);
}

/// `CAEWeaponAudioEntity::PlayChainsawStopSound(CPhysical*)` (0x504AA0): no physical or bank 0x24 not in
/// slot 0x28 -> chainsaw state +0x80 = 4. Otherwise, unless the state is already 3, the temp sound (+8)
/// (slot 0x28, sfx 2, the physical's position, event 0x9A volume, 0.66, 1.0, 1.0, 0, flags 0x84, 0, 0)
/// is registered with the physical, event +0x14 = 0xC, requested; state = 3.
#[hook(addr = 0x504AA0, conv = thiscall, name = "CAEWeaponAudioEntity::PlayChainsawStopSound", expect = "55 8B 6C 24 08")]
pub fn weapon_play_chainsaw_stop_sound_port(this: GPtr<u8>, physical: u32) {
    if physical == 0 || hw_is_sound_bank_loaded(g(0xB5F8B8), 0x24, 0x28) == 0 {
        wr::<u8>(this, 0x80, 4);
        return;
    }
    if rd::<u8>(this, 0x80) != 3 {
        let vol = event_volume(0x9A);
        let phys: GPtr<u8> = g(physical as usize);
        let p = entity_pos(phys);
        let snd = this.byte_add(8);
        sound_initialise(
            snd,
            0x28,
            2,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            vol.to_bits(),
            0x3F28_F5C3,
            FLT_ONE_BITS,
            FLT_ONE_BITS,
            0,
            0x84,
            0,
            0,
        );
        sound_register_with_physical_entity(snd, phys);
        wr::<u32>(this, 0x14, 0xC);
        sm_request_new_sound(g(0xB62CB0), snd);
    }
    wr::<u8>(this, 0x80, 3);
}

/// `CAEWeaponAudioEntity::PlayMiniGunStopSound(CPhysical*)` (0x504960): no physical -> minigun state
/// +0x7F = 3. State 2: stays 2. Bank 0x8F must be in slot 5 (else loaded unless the loading tune plays;
/// state unchanged). The temp sound (+8): (slot 5, sfx 0x3F, the physical's position, event 0x9D
/// volume, max distance [0x85BA50] * (1.0, or [0x859BB4] for a vehicle (+0x36 & 7 == 2) of class 4 at
/// +0x594, which also uses speed 0.79 instead of 1.0), 1.0, 0, flags 0x84, 0, 0), registered, event
/// +0x14 = 8, requested; state = 2.
#[hook(addr = 0x504960, conv = thiscall, name = "CAEWeaponAudioEntity::PlayMiniGunStopSound", expect = "83 EC 08 55 8B 6C 24 10")]
pub fn weapon_play_mini_gun_stop_sound_port(this: GPtr<u8>, physical: u32) {
    if physical == 0 {
        wr::<u8>(this, 0x7F, 3);
        return;
    }
    if rd::<u8>(this, 0x7F) != 2 {
        if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
            if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
                hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
            }
            return;
        }
        let vol = event_volume(0x9D);
        let phys: GPtr<u8> = g(physical as usize);
        let (k, speed) = if rd::<u8>(phys, 0x36) & 7 == 2 && rd::<u32>(phys, 0x594) == 4 {
            (g::<f32>(0x859BB4).read(), 0x3F4B_2FECu32)
        } else {
            (1.0f32, FLT_ONE_BITS)
        };
        let p = entity_pos(phys);
        let max_dist = k * g::<f32>(0x85BA50).read();
        let snd = this.byte_add(8);
        sound_initialise(
            snd,
            5,
            0x3F,
            this.addr() as u32,
            p[0],
            p[1],
            p[2],
            vol.to_bits(),
            max_dist.to_bits(),
            speed,
            FLT_ONE_BITS,
            0,
            0x84,
            0,
            0,
        );
        sound_register_with_physical_entity(snd, phys);
        wr::<u32>(this, 0x14, 8);
        sm_request_new_sound(g(0xB62CB0), snd);
    }
    wr::<u8>(this, 0x7F, 2);
}

/// `CAEWeaponAudioEntity::PlayWeaponLoopSound(CPhysical*, int16 sfx, eAudioEvents, float volumeOffset,
/// float speed, int32 soundEvent)` (0x504610, `ret 0x18`): bank 0x8F must be in slot 5 (else loaded
/// unless the loading tune plays). The temp sound (+8): (slot 5, sfx, the physical's position, event
/// volume + offset, 1.0, speed, 1.0, 0, flags 0xA4, 0, 0), registered with the physical, event +0x14 =
/// `soundEvent`, requested.
#[hook(addr = 0x504610, conv = thiscall, name = "CAEWeaponAudioEntity::PlayWeaponLoopSound", expect = "53 56 57 6A 05")]
#[allow(clippy::too_many_arguments)]
pub fn weapon_play_weapon_loop_sound(
    this: GPtr<u8>,
    physical: u32,
    sfx: u32,
    event: u32,
    vol_offset: f32,
    speed: u32,
    sound_event: u32,
) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let vol = event_volume(event) + vol_offset;
    let phys: GPtr<u8> = g(physical as usize);
    let p = entity_pos(phys);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        FLT_ONE_BITS,
        speed,
        FLT_ONE_BITS,
        0,
        0xA4,
        0,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    wr::<u32>(this, 0x14, sound_event);
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEWeaponAudioEntity::PlayCameraSound(CPhysical*, eAudioEvents, float volumeOffset)` (0x5046F0,
/// `ret 0xC`): bank 0x8F must be in slot 5 (else loaded unless the loading tune plays). The temp sound
/// (+8): (slot 5, sfx 0x2D, the physical's position, event volume + offset, 0.66, 1.0, 1.0, 0, flags
/// 0x90, 0, 0), registered with the physical, requested.
#[hook(addr = 0x5046F0, conv = thiscall, name = "CAEWeaponAudioEntity::PlayCameraSound", expect = "53 56 57 6A 05")]
pub fn weapon_play_camera_sound(this: GPtr<u8>, physical: u32, event: u32, vol_offset: f32) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let vol = event_volume(event) + vol_offset;
    let phys: GPtr<u8> = g(physical as usize);
    let p = entity_pos(phys);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        0x2D,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        0x3F28_F5C3,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0x90,
        0,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEWeaponAudioEntity::WeaponReload(eWeaponType, CPhysical*, eAudioEvents)` (0x503690, `ret 0xC`):
/// nothing without a physical; bank 0x8F must be in slot 5 (else loaded unless the loading tune
/// plays). Weapon types 22..34 (the bytes' table; `r` = event != 0x92): 22, 23 -> 0x42 / 0x37 (event
/// 0x92); 24 -> 0x33 / 0x37; 25, 27 -> 0x47 + r; 26 -> 0x45 + r; 28, 29, 32 -> 0x54 + r; 30, 31 ->
/// 0x1F + r; 33 -> only event 0x92: 0x20 with volume offset -6.0; 34 -> 0x20 / 0x37; other types
/// nothing. The temp sound (+8): (slot 5, sfx, the physical's position, event volume + offset, 0.66,
/// 1.0, 1.0, 0, flags 0x80, 0, 0), registered with the physical, requested.
#[hook(addr = 0x503690, conv = thiscall, name = "CAEWeaponAudioEntity::WeaponReload", expect = "51 53 55 56 57")]
pub fn weapon_weapon_reload(this: GPtr<u8>, ty: u32, physical: u32, event: u32) {
    if physical == 0 {
        return;
    }
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let r = (event != 0x92) as u32;
    let mut offset = 0.0f32;
    let sfx: u32 = match ty.wrapping_sub(0x16) {
        0 | 1 => {
            if r != 0 {
                0x42
            } else {
                0x37
            }
        }
        2 => {
            if r != 0 {
                0x33
            } else {
                0x37
            }
        }
        3 | 5 => 0x47 + r,
        4 => 0x45 + r,
        6 | 7 | 10 => 0x54 + r,
        8 | 9 => 0x1F + r,
        11 => {
            if r != 0 {
                return;
            }
            offset = f32::from_bits(0xC0C0_0000);
            0x20
        }
        12 => {
            if r != 0 {
                0x20
            } else {
                0x37
            }
        }
        _ => return,
    };
    let vol = event_volume(event) + offset;
    let phys: GPtr<u8> = g(physical as usize);
    let p = entity_pos(phys);
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        sfx,
        this.addr() as u32,
        p[0],
        p[1],
        p[2],
        vol.to_bits(),
        0x3F28_F5C3,
        FLT_ONE_BITS,
        FLT_ONE_BITS,
        0,
        0x80,
        0,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `CAEWeaponAudioEntity::PlayFlameThrowerSounds(CPhysical*, int16 sfx1, int16 sfx2, eAudioEvents, float
/// volumeOffset, float speed)` (0x504470, `ret 0x18`): bank 0x8F must be in slot 5 (else loaded unless
/// the loading tune plays). Volume = event volume + offset. sfx1 != -1: temp sound (+8) (slot 5, sfx1,
/// the physical's position, volume - [0x858B44], 0.66, speed, 1.0, 0, flags 0x84, 0.02, 0), registered,
/// requested. Bank 0x8A must be in slot 0x13 (else loaded). sfx2 != -1: (slot 0x13, sfx2, volume -
/// [0x858BA4], 2.0, speed * [0x862E18], 1.0, 0, 0x84, 0, 0), registered, event +0x14 = 2, requested.
#[hook(addr = 0x504470, conv = thiscall, name = "CAEWeaponAudioEntity::PlayFlameThrowerSounds", expect = "53 55 56 57 6A 05")]
#[allow(clippy::too_many_arguments)]
pub fn weapon_play_flame_thrower_sounds(
    this: GPtr<u8>,
    physical: u32,
    sfx1: u32,
    sfx2: u32,
    event: u32,
    vol_offset: f32,
    speed: u32,
) {
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let vol = event_volume(event) + vol_offset;
    let phys: GPtr<u8> = g(physical as usize);
    let me = this.addr() as u32;
    let snd = this.byte_add(8);
    if sfx1 as u16 != 0xFFFF {
        let p = entity_pos(phys);
        let v1 = vol - g::<f32>(0x858B44).read();
        sound_initialise(
            snd,
            5,
            sfx1,
            me,
            p[0],
            p[1],
            p[2],
            v1.to_bits(),
            0x3F28_F5C3,
            speed,
            FLT_ONE_BITS,
            0,
            0x84,
            0x3CA3_D70A,
            0,
        );
        sound_register_with_physical_entity(snd, phys);
        sm_request_new_sound(g(0xB62CB0), snd);
    }
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8A, 0x13) == 0 {
        hw_load_sound_bank(g(0xB5F8B8), 0x8A, 0x13);
        return;
    }
    if sfx2 as u16 == 0xFFFF {
        return;
    }
    let p = entity_pos(phys);
    let s2 = f32::from_bits(speed) * g::<f32>(0x862E18).read();
    let v2 = vol - g::<f32>(0x858BA4).read();
    sound_initialise(
        snd,
        0x13,
        sfx2,
        me,
        p[0],
        p[1],
        p[2],
        v2.to_bits(),
        0x4000_0000,
        s2.to_bits(),
        FLT_ONE_BITS,
        0,
        0x84,
        0,
        0,
    );
    sound_register_with_physical_entity(snd, phys);
    wr::<u32>(this, 0x14, 2);
    sm_request_new_sound(g(0xB62CB0), snd);
}

orig! {
    /// `CAEWeaponAudioEntity::PlayGunSounds(CPhysical*, int16 x5, int32 event, float, float, float)` (0x503CE0, `ret 0x28`)
    fn weapon_play_gun_sounds(this: GPtr<u8>, physical: u32, a: u32, b: u32, c: u32, d: u32, e: u32, event: u32, f: u32, g1: u32, h: u32) = thiscall 0x503CE0 => unported;
}

/// `CAEWeaponAudioEntity::PlayMiniGunFireSounds(CPhysical*, eAudioEvents)` (0x5047C0): events 0x96 /
/// 0x91 and 0x95: unless the fire flag +0x7D is set, `PlayGunSounds(physical, 0xF, 0x10, 0xB, 0xC, 0xD,
/// 0x96 (0x98 for 0x95), 0, 1.0, 1.0)` and +0x7D = 1; then state +0x7F = 1. Event 0x97: without the spin
/// flag +0x7C, bank 0x8F must be in slot 5 (else: loading tune -> return; otherwise `LoadSoundBank` and
/// return, the original's tail jump), the temp sound (+8) (slot 5, sfx 0xE, the physical's position,
/// event 0x97 volume, 0.66, 1.0, 0.0, 0, flags 0x84, 0, 0) registered, event +0x14 = 5, requested, +0x7C
/// = 1; then state = 0. All but the early returns end with +0x90 = now (other events only that).
#[hook(addr = 0x5047C0, conv = thiscall, name = "CAEWeaponAudioEntity::PlayMiniGunFireSounds", expect = "8B 44 24 08 3D 96 00 00 00")]
pub fn weapon_play_mini_gun_fire_sounds(this: GPtr<u8>, physical: u32, event: u32) {
    match event {
        0x96 | 0x91 | 0x95 => {
            if rd::<u8>(this, 0x7D) == 0 {
                let ev = if event == 0x95 { 0x98 } else { 0x96 };
                weapon_play_gun_sounds(
                    this,
                    physical,
                    0xF,
                    0x10,
                    0xB,
                    0xC,
                    0xD,
                    ev,
                    0,
                    FLT_ONE_BITS,
                    FLT_ONE_BITS,
                );
                wr::<u8>(this, 0x7D, 1);
            }
            wr::<u8>(this, 0x7F, 1);
        }
        0x97 => {
            if rd::<u8>(this, 0x7C) == 0 {
                if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
                    if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
                        hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
                    }
                    return;
                }
                let vol = event_volume(0x97);
                let phys: GPtr<u8> = g(physical as usize);
                let p = entity_pos(phys);
                let snd = this.byte_add(8);
                sound_initialise(
                    snd,
                    5,
                    0xE,
                    this.addr() as u32,
                    p[0],
                    p[1],
                    p[2],
                    vol.to_bits(),
                    0x3F28_F5C3,
                    FLT_ONE_BITS,
                    0,
                    0,
                    0x84,
                    0,
                    0,
                );
                sound_register_with_physical_entity(snd, phys);
                wr::<u32>(this, 0x14, 5);
                sm_request_new_sound(g(0xB62CB0), snd);
                wr::<u8>(this, 0x7C, 1);
            }
            wr::<u8>(this, 0x7F, 0);
        }
        _ => {}
    }
    wr::<u32>(this, 0x90, sa_core::timer::TIME_MS.get());
}

/// `CAEWeaponAudioEntity::PlayGoggleSound(int16 sfx, eAudioEvents)` (0x503500, `ret 8`): bank 0x8F must
/// be in slot 5 (else: unless the loading tune plays, `LoadSoundBank` through the original's tail jump).
/// `ResolveProbability(0.5)` picks speeds (1.19, 1.0) or (1.0, 1.19). The temp sound (+8) is
/// initialised (slot 5, sfx, (-1, 0, 0), event volume - [0x85EED4], 1.0, first speed, 1.0, zeros), flags
/// word +0x5E = 0x1001, requested; then at (1, 0, 0) with the second speed, likewise.
#[hook(addr = 0x503500, conv = thiscall, name = "CAEWeaponAudioEntity::PlayGoggleSound", expect = "83 EC 20 8B 44 24 28")]
pub fn weapon_play_goggle_sound(this: GPtr<u8>, sfx: u32, event: u32) {
    let vol = event_volume(event);
    if hw_is_sound_bank_loaded(g(0xB5F8B8), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(g(0xB6BC90)) == 0 {
            hw_load_sound_bank(g(0xB5F8B8), 0x8F, 5);
        }
        return;
    }
    let (s1, s2) = if ae_resolve_probability_bits(0x3F00_0000) != 0 {
        (0x3F98_3809u32, FLT_ONE_BITS)
    } else {
        (FLT_ONE_BITS, 0x3F98_3809u32)
    };
    let vol2 = (vol - g::<f32>(0x85EED4).read()).to_bits();
    let me = this.addr() as u32;
    let snd = this.byte_add(8);
    sound_initialise(
        snd,
        5,
        sfx,
        me,
        0xBF80_0000,
        0,
        0,
        vol2,
        FLT_ONE_BITS,
        s1,
        FLT_ONE_BITS,
        0,
        0,
        0,
        0,
    );
    wr::<u16>(this, 0x5E, 0x1001);
    sm_request_new_sound(g(0xB62CB0), snd);
    sound_initialise(
        snd,
        5,
        sfx,
        me,
        FLT_ONE_BITS,
        0,
        0,
        vol2,
        FLT_ONE_BITS,
        s2,
        FLT_ONE_BITS,
        0,
        0,
        0,
        0,
    );
    wr::<u16>(this, 0x5E, 0x1001);
    sm_request_new_sound(g(0xB62CB0), snd);
}

/// `vector_magnitude_sqr_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
fn vector_magnitude_sqr(v: GPtr<u8>) -> crate::common::X87 {
    crate::common::X87::from_f64_exact(vector_magnitude_sqr_st0(v))
}

/// `speech_get_speech_context_volume_offset_call_st0` with its ST0 result as `X87`.
#[allow(dead_code)]
fn speech_get_speech_context_volume_offset_call(this: GPtr<u8>, ctx: u32) -> crate::common::X87 {
    crate::common::X87::from_f64_exact(speech_get_speech_context_volume_offset_call_st0(this, ctx))
}

/// Phase B target of `speech_get_speech_context_volume_offset_call_st0`: the port's ST0 result as f64.
#[allow(dead_code)]
fn speech_get_speech_context_volume_offset_target_f64(this: GPtr<u8>, ctx: u32) -> f64 {
    speech_get_speech_context_volume_offset(this, ctx).to_f64()
}
