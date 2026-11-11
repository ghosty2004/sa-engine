//! `CAEPedAudioEntity`, `CAEPedSpeechAudioEntity`, `CAEPedlessSpeechAudioEntity` and
//! `CAEPedWeaponAudioEntity` (PORT-AUDIO).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids. The
//! global-context predicates follow the exe's jump tables (gta-reversed uses a context-info lookup).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};

use crate::common::{ae_random_int, audio_hardware, g, rd, sound_manager, wr};
use crate::entities::{
    cancel_sounds_owned_by_audio_entity, load_sound_bank, stop_sound_field, weapon_report_chainsaw_event,
    weapon_report_stealth_kill, weapon_weapon_fire, weapon_weapon_reload,
};

sa_core::globals! {
    list GLOBALS;
    /// `CAEPedSpeechAudioEntity::s_nCJWellDressed`, `s_nCJFat`, `s_nCJGangBanging`, `s_nCJBasicMood`
    pub static CJ_WELL_DRESSED @ 0xB613D0: u16 = 0;
    pub static CJ_FAT @ 0xB613D4: u16 = 0;
    pub static CJ_GANG_BANGING @ 0xB613D8: u16 = 0;
    pub static CJ_BASIC_MOOD @ 0xB613DC: u16 = 0;
    /// `CAEPedSpeechAudioEntity::s_bAPlayerSpeaking`
    pub static A_PLAYER_SPEAKING @ 0xB613E5: u8 = 0;
    /// `CAEPedSpeechAudioEntity::s_bAllSpeechDisabled`
    pub static ALL_SPEECH_DISABLED @ 0xB613E6: u8 = 0;
    /// `CAEPedSpeechAudioEntity::s_Conversation[8]` (int16)
    pub static CONVERSATION @ 0xB613EC: [u16; 8] = [0; 8];
    /// `CAEPedSpeechAudioEntity::s_bPlayerConversationHappening`
    pub static PLAYER_CONVERSATION_HAPPENING @ 0xB613FC: u8 = 0;
    /// `CAEPedSpeechAudioEntity::s_bPedConversationHappening`
    pub static PED_CONVERSATION_HAPPENING @ 0xB613FD: u8 = 0;
    /// `CAEPedSpeechAudioEntity::s_pPlayerConversationPed`
    pub static PLAYER_CONVERSATION_PED @ 0xB61400: u32 = 0;
    /// `CAEPedSpeechAudioEntity::s_pConversationPedSlot2` / `s_pConversationPedSlot1` (int16)
    pub static CONVERSATION_PED_SLOT2 @ 0xB61404: u16 = 0;
    pub static CONVERSATION_PED_SLOT1 @ 0xB61408: u16 = 0;
    /// `CAEPedSpeechAudioEntity::s_pConversationPed2` / `s_pConversationPed1`
    pub static CONVERSATION_PED2 @ 0xB6140C: u32 = 0;
    pub static CONVERSATION_PED1 @ 0xB61410: u32 = 0;
    /// `CAEPedSpeechAudioEntity::s_NextSpeechSlot`
    pub static NEXT_SPEECH_SLOT @ 0xB61414: u16 = 0;
    /// `CAEPedSpeechAudioEntity::s_PhraseMemory[150][2]` (int16)
    pub static PHRASE_MEMORY @ 0xB61418: [u16; 300] = [0; 300];
    /// `CAEPedSpeechAudioEntity::s_nCJMoodOverrideTime`
    pub static CJ_MOOD_OVERRIDE_TIME @ 0xB613E0: u32 = 0;
    /// `CAEPedSpeechAudioEntity::s_bForceAudible` (catalogue name; set by `Service`)
    pub static SPEECH_FORCE_AUDIBLE @ 0xB613E4: u8 = 0;
    /// `CAEPedSpeechAudioEntity::gGlobalSpeechContextNextPlayTime[360]`
    pub static GLOBAL_SPEECH_CONTEXT_NEXT_PLAY_TIME @ 0xB61670: [u32; 0x168] = [0; 0x168];
}

/// `CAEPedSpeechAudioEntity` specific-context table: 16-byte rows `{ int16 global, int16 specific[6], .. }`,
/// ended by a row whose `global` is -1.
const SPECIFIC_CONTEXT_TABLE: usize = 0x8C6A68;
/// `const char* [6]`: the audio ped type names (`GetAudioPedType`).
const AUDIO_PED_TYPE_NAMES: usize = 0x8C8108;
/// `CAEPedSpeechAudioEntity::s_PedSpeechSlots[6]` (0x1C each: +0 status u16, +4 owner, +0xC sfx u16,
/// +0xE bank u16, +0x10 start time, +0x18 flag); constructed by statics.rs, accessed by address here.
const PED_SPEECH_SLOTS: usize = 0xB61C38;

orig! {
    /// `CAEPedAudioEntity::StopJetPackSound()` (0x4E1120)
    pub fn ped_stop_jet_pack_sound_call(this: GPtr<u8>) = thiscall 0x4E1120 => ped_stop_jet_pack_sound;
    /// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960)
    pub fn twin_loop_stop_sound_and_forget(this: GPtr<u8>) = thiscall 0x4F2960 => crate::entities::twin_loop_stop_sound_and_forget;
    /// `CAEWeaponAudioEntity::Terminate()` (0x503480)
    pub fn weapon_terminate(this: GPtr<u8>) = thiscall 0x503480 => crate::entities::weapon_terminate;
    /// `CAEPedSpeechAudioEntity::Reset()` (0x4E37B0; not hookable, see `speech_reset`)
    pub fn speech_reset_call() = cdecl 0x4E37B0 => unported;
    /// `CAEPedSpeechAudioEntity::StopCurrentSpeech()` (0x4E3FB0)
    /// `CAEPedSpeechAudioEntity::CAEPedSpeechAudioEntity()` (0x4E4F10)
    pub fn speech_constructor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4E4F10 => unported;
    /// `CAEAudioHardware::IsSoundLoaded(uint16 bank, uint16 sfx, int16 slot)` (0x4D8EF0)
    pub fn hw_is_sound_loaded(hw: GPtr<u8>, bank: u32, sfx: u32, slot: u32) -> u8 = thiscall 0x4D8EF0 => unported;
    /// Virtual slot 0xC of the speech slot's owner (`CAEPedSpeechAudioEntity::PlayLoadedSound`)
    pub fn vcall_speech_play_loaded_sound(this: GPtr<u8>) = thiscall vslot 0x0C => unported;
    pub fn speech_stop_current_speech(this: GPtr<u8>) = thiscall 0x4E3FB0 => unported;
}

// ---- CAEPedAudioEntity ----------------------------------------------------------------------------

/// `CAEPedAudioEntity::StaticInitialise()` (0x5B98A0): bank 0 into slot 0x29, bank 0x80 into slot 0x20.
#[hook(addr = 0x5B98A0, conv = cdecl, name = "CAEPedAudioEntity::StaticInitialise", expect = "6A 29 6A 00 B9 B8 F8 B5 00")]
pub fn ped_static_initialise() {
    load_sound_bank(audio_hardware(), 0, 0x29);
    load_sound_bank(audio_hardware(), 0x80, 0x20);
}

/// `CAEPedAudioEntity::Initialise(CEntity*)` (0x4E0E80): ped at +0x94, state cleared, the three
/// "last volume" floats = -100.0, initialised flag +0x7C = 1.
#[hook(addr = 0x4E0E80, conv = thiscall, name = "CAEPedAudioEntity::Initialise", expect = "8B 44 24 04 BA 00 00 C8 C2")]
pub fn ped_initialise(this: GPtr<u8>, ped: u32) {
    const MINUS_100: u32 = 0xC2C8_0000;
    wr::<u32>(this, 0x94, ped);
    wr::<u16>(this, 0x7E, 0);
    wr::<u32>(this, 0x80, 0);
    wr::<u8>(this, 0x98, 0);
    wr::<u32>(this, 0x9C, 0);
    wr::<u32>(this, 0x84, MINUS_100);
    wr::<u32>(this, 0xA0, 0);
    wr::<u32>(this, 0x88, MINUS_100);
    wr::<u32>(this, 0x150, 0);
    wr::<u32>(this, 0x154, MINUS_100);
    wr::<u32>(this, 0x158, MINUS_100);
    wr::<u8>(this, 0x7C, 1);
}

/// `CAEPedAudioEntity::StopJetPackSound()` (0x4E1120): the three jet-pack sounds (+0x9C, +0xA0, +0xA4).
#[hook(addr = 0x4E1120, conv = thiscall, name = "CAEPedAudioEntity::StopJetPackSound", expect = "56 8B F1 8B 8E 9C 00 00 00")]
pub fn ped_stop_jet_pack_sound(this: GPtr<u8>) {
    stop_sound_field(this, 0x9C);
    stop_sound_field(this, 0xA0);
    stop_sound_field(this, 0xA4);
}

/// `CAEPedAudioEntity::TurnOffJetPack()` (0x4E2A70): stops the sounds, clears the jet-pack flag +0x98.
#[hook(addr = 0x4E2A70, conv = thiscall, name = "CAEPedAudioEntity::TurnOffJetPack", expect = "56 8B F1 E8 A8 E6 FF FF")]
pub fn ped_turn_off_jet_pack(this: GPtr<u8>) {
    ped_stop_jet_pack_sound_call(this);
    wr::<u8>(this, 0x98, 0);
}

/// `CAEPedAudioEntity::Terminate()` (0x4E1360): clears the flag and ped, stops the jet pack, cancels its
/// sounds, and stops the twin-loop entity at +0xA8 when its bank slot word (+0x130) is set.
#[hook(addr = 0x4E1360, conv = thiscall, name = "CAEPedAudioEntity::Terminate", expect = "56 8B F1 C6 46 7C 00")]
pub fn ped_terminate(this: GPtr<u8>) {
    wr::<u8>(this, 0x7C, 0);
    wr::<u32>(this, 0x94, 0);
    ped_stop_jet_pack_sound_call(this);
    cancel_sounds_owned_by_audio_entity(sound_manager(), this, 1);
    if rd::<u16>(this, 0x130) != 0 {
        twin_loop_stop_sound_and_forget(this.byte_add(0xA8));
    }
}

// ---- CAEPedWeaponAudioEntity ----------------------------------------------------------------------

/// `CAEPedWeaponAudioEntity::Terminate()` (0x4E6AC0): clears +0xA0 (byte) and +0xA4 (ped), then
/// `CAEWeaponAudioEntity::Terminate`.
#[hook(addr = 0x4E6AC0, conv = thiscall, name = "CAEPedWeaponAudioEntity::Terminate", expect = "33 C0 88 81 A0 00 00 00")]
pub fn ped_weapon_terminate(this: GPtr<u8>) {
    wr::<u8>(this, 0xA0, 0);
    wr::<u32>(this, 0xA4, 0);
    weapon_terminate(this);
}

/// `CAEPedWeaponAudioEntity::AddAudioEvent(eAudioEvents)` (0x4E69F0): when initialised (+0xA0) with a
/// ped (+0xA4), dispatches on the ped's active weapon (`ped + 0x5A0 + 0x1C * (int8)ped[0x718]`, type at
/// +0): 0x91 / 0x96 / 0x97 fire, 0x92 / 0x93 reload, 0x99..=0x9B chainsaw, 0x9C stealth kill.
#[hook(addr = 0x4E69F0, conv = thiscall, name = "CAEPedWeaponAudioEntity::AddAudioEvent", expect = "8A 81 A0 00 00 00")]
pub fn ped_weapon_add_audio_event(this: GPtr<u8>, event: u32) {
    if rd::<u8>(this, 0xA0) == 0 {
        return;
    }
    let ped = rd::<u32>(this, 0xA4);
    if ped == 0 {
        return;
    }
    let p: GPtr<u8> = g(ped as usize);
    let slot = rd::<i8>(p, 0x718) as i32;
    let weapon = (slot.wrapping_mul(0x1C) as u32)
        .wrapping_add(ped)
        .wrapping_add(0x5A0);
    if weapon == 0 {
        return;
    }
    let ty = || g::<u32>(weapon as usize).read();
    match event.wrapping_sub(0x91) {
        0 | 5 | 6 => weapon_weapon_fire(this, ty(), ped, event),
        1 | 2 => weapon_weapon_reload(this, ty(), ped, event),
        8..=10 => weapon_report_chainsaw_event(this, ped, event),
        11 => weapon_report_stealth_kill(this, ty(), ped, event),
        _ => {}
    }
}

// ---- CAEPedSpeechAudioEntity ----------------------------------------------------------------------

/// `CAEPedSpeechAudioEntity::SetNextPlayTime(int16 globalCtx)` (0x4E4A20): for a listed context
/// (< 360), next time = now + (int16)(`GetRandomNumberInRange(1, 1000)` + the row's repeat time at +0xC);
/// pain contexts (0x154..=0x166) go to the entity's own table (+0xB4), others to the global one.
#[hook(addr = 0x4E4A20, conv = thiscall, name = "CAEPedSpeechAudioEntity::SetNextPlayTime", expect = "53 57 8B 7C 24 0C")]
pub fn speech_set_next_play_time(this: GPtr<u8>, global_ctx: u32) {
    let gc = global_ctx as u16 as i16;
    if !(0..0x168).contains(&gc) {
        return;
    }
    let mut i: i16 = 0;
    loop {
        let key = g::<u16>(SPECIFIC_CONTEXT_TABLE + ((i as isize as usize) << 4)).read();
        if key == 0xFFFF || key as i16 == gc {
            break;
        }
        i = i.wrapping_add(1);
    }
    let row = SPECIFIC_CONTEXT_TABLE + ((i as isize as usize) << 4);
    if g::<u16>(row).read() == 0xFFFF {
        return;
    }
    let r = ae_random_int(1, 1000);
    let delay = (r as u16).wrapping_add(g::<u16>(row + 0xC).read()) as i16 as i32;
    let t = delay.wrapping_add(TIME_MS.get() as i32) as u32;
    if gc > 0x153 && gc < 0x167 {
        wr::<u32>(this, 0xB4 + (gc as isize - 0x154) * 4, t);
    } else {
        GLOBAL_SPEECH_CONTEXT_NEXT_PLAY_TIME
            .ptr()
            .cast::<u32>()
            .byte_add(gc as isize * 4)
            .write(t);
    }
}

/// `CAEPedSpeechAudioEntity::IsGlobalContextPain(int16)` (0x4E44F0, static): 0x153 < ctx < 0x167.
/// Not hooked: the caller at 0x4E6683 keeps EDX across the call (sa-hook callers test).
pub fn speech_is_global_context_pain(ctx: u32) -> u8 {
    let c = ctx as u16 as i16;
    (c > 0x153 && c < 0x167) as u8
}

/// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForInterupting(int16)` (0x4E4600, static): the
/// exe's jump table is true for 13, 15, 125, 126 and 127.
/// Not hooked: callers 0x4E5F29 / 0x4E5F76 / 0x4E668D keep ECX/EDX across the call.
pub fn speech_is_global_context_important_for_interupting(ctx: u32) -> u8 {
    matches!(ctx as u16 as i16, 13 | 15 | 125..=127) as u8
}

/// `CAEPedSpeechAudioEntity::IsGlobalContextImportantForStreaming(int16)` (0x4E4510): always for audio
/// ped types 2 and 4 (+0x92); otherwise the exe's jump table (13, 15, 48..=64, 125..=127, 161..=171).
#[hook(addr = 0x4E4510, conv = thiscall, name = "CAEPedSpeechAudioEntity::IsGlobalContextImportantForStreaming", expect = "66 8B 81 92 00 00 00")]
pub fn speech_is_global_context_important_for_streaming(this: GPtr<u8>, ctx: u32) -> u8 {
    let ty = rd::<u16>(this, 0x92);
    if ty == 4 || ty == 2 {
        return 1;
    }
    matches!(ctx as u16 as i16, 13 | 15 | 48..=64 | 125..=127 | 161..=171) as u8
}

/// `CAEPedSpeechAudioEntity::GetSpecificSpeechContext(int16 globalCtx, int16 audioPedType)` (0x4E4470,
/// static): the row of the specific-context table for `globalCtx` (< 360), column `audioPedType` (< 6);
/// -1 when out of range or not listed.
#[hook(addr = 0x4E4470, conv = stdcall, name = "CAEPedSpeechAudioEntity::GetSpecificSpeechContext", expect = "66 8B 54 24 04")]
pub fn speech_get_specific_speech_context(global_ctx: u32, ped_type: u32) -> u16 {
    let gc = global_ctx as u16 as i16;
    let pt = ped_type as u16 as i16;
    if !(0..0x168).contains(&gc) || !(0..6).contains(&pt) {
        return 0xFFFF;
    }
    let mut i: i16 = 0;
    loop {
        let key = g::<u16>(SPECIFIC_CONTEXT_TABLE + ((i as isize as usize) << 4)).read();
        if key == 0xFFFF || key as i16 == gc {
            break;
        }
        i = i.wrapping_add(1);
    }
    let row = SPECIFIC_CONTEXT_TABLE + ((i as isize as usize) << 4);
    if g::<u16>(row).read() == 0xFFFF {
        return 0xFFFF;
    }
    g::<u16>(row + 2 + pt as usize * 2).read()
}

/// `CAEPedSpeechAudioEntity::GetAudioPedType(const char*)` (0x4E3C60, static): the index of the name in
/// the six-entry name table (exact `strcmp` match), else -1.
#[hook(addr = 0x4E3C60, conv = cdecl, name = "CAEPedSpeechAudioEntity::GetAudioPedType", expect = "53 55 8B 6C 24 0C")]
pub fn speech_get_audio_ped_type(name: GPtr<u8>) -> u16 {
    for i in 0..6u16 {
        let s = g::<u32>(AUDIO_PED_TYPE_NAMES + i as usize * 4).read();
        let s: GPtr<u8> = g(s as usize);
        let mut k = 0isize;
        loop {
            let a = name.byte_add(k).read();
            if a != s.byte_add(k).read() {
                break;
            }
            if a == 0 {
                return i;
            }
            k += 1;
        }
    }
    0xFFFF
}

/// `CAEPedSpeechAudioEntity::Reset()` (0x4E37B0, static): every global context's next play time =
/// now + `GetRandomNumberInRange(3000, 7000)` (the exe's audio RNG, not `CGeneral`); CJ mood override 0.
/// Not hookable: the protection reads the dword at 0x4E37B4 (inside the patched entry) at 0x156FF88.
pub fn speech_reset() {
    let times = GLOBAL_SPEECH_CONTEXT_NEXT_PLAY_TIME.ptr().cast::<u32>();
    for i in 0..0x168isize {
        let now = TIME_MS.get();
        let r = ae_random_int(3000, 7000);
        times.byte_add(i * 4).write(r.wrapping_add(now as i32) as u32);
    }
    CJ_MOOD_OVERRIDE_TIME.set(0);
}

/// `CAEPedSpeechAudioEntity::Service()` (0x4E3710, static): per speech slot, a loading sound (1) whose
/// sound is loaded becomes 2; a loaded one (2) whose start time has come plays through its owner's
/// virtual slot 0xC, or is freed (0) without owner; `s_bForceAudible` = any slot in state 3 or 5 with its
/// flag (+0x18) set (freed slots are skipped).
#[hook(addr = 0x4E3710, conv = cdecl, name = "CAEPedSpeechAudioEntity::Service", expect = "53 56 57 32 DB")]
pub fn speech_service() {
    let mut any = false;
    for i in 0..6u32 {
        let sl: GPtr<u8> = g(PED_SPEECH_SLOTS + i as usize * 0x1C);
        let mut st = rd::<u16>(sl, 0);
        if st == 1 {
            let loaded = hw_is_sound_loaded(
                audio_hardware(),
                rd::<u16>(sl, 0xE) as u32,
                rd::<u16>(sl, 0xC) as u32,
                i + 0x14,
            );
            if loaded != 0 {
                wr::<u16>(sl, 0, 2);
                st = 2;
            }
        }
        if st == 2 && rd::<u32>(sl, 0x10) <= TIME_MS.get() {
            let owner = rd::<u32>(sl, 4);
            if owner == 0 {
                wr::<u16>(sl, 0, 0);
                continue;
            }
            vcall_speech_play_loaded_sound(g(owner as usize));
        }
        let st = rd::<u16>(sl, 0);
        if (st == 5 || st == 3) && rd::<u8>(sl, 0x18) != 0 {
            any = true;
        }
    }
    SPEECH_FORCE_AUDIBLE.set(any as u8);
}

/// `CAEPedSpeechAudioEntity::StaticInitialise()` (0x5B98C0): clears the six speech slots, the phrase
/// memory (-1), calls `Reset` and then redraws every global context's next play time itself (now +
/// `GetRandomNumberInRange(3000, 7000)`, a second 360 draws), then resets the conversation, CJ mood and
/// speaking statics.
#[hook(addr = 0x5B98C0, conv = cdecl, name = "CAEPedSpeechAudioEntity::StaticInitialise", expect = "51 53 56 33 DB")]
pub fn speech_static_initialise() {
    for i in 0..6usize {
        let sl: GPtr<u8> = g(PED_SPEECH_SLOTS + i * 0x1C);
        wr::<u16>(sl, 0, 0);
        wr::<u32>(sl, 4, 0);
        wr::<u32>(sl, 8, 0);
        wr::<u16>(sl, 0xC, 0xFFFF);
        wr::<u16>(sl, 0xE, 0xFFFF);
        wr::<u32>(sl, 0x10, 0);
        wr::<u16>(sl, 0x14, 0xFFFF);
        wr::<u16>(sl, 0x16, 0xFFFF);
        wr::<u8>(sl, 0x19, 0);
        wr::<u8>(sl, 0x1A, 0);
        wr::<u8>(sl, 0x18, 0);
    }
    PHRASE_MEMORY.set([0xFFFF; 300]);
    NEXT_SPEECH_SLOT.set(0);
    speech_reset_call();
    let times = GLOBAL_SPEECH_CONTEXT_NEXT_PLAY_TIME.ptr().cast::<u32>();
    for i in 0..0x168isize {
        let now = TIME_MS.get();
        let r = ae_random_int(3000, 7000);
        times.byte_add(i * 4).write(r.wrapping_add(now as i32) as u32);
    }
    CONVERSATION_PED1.set(0);
    CONVERSATION_PED2.set(0);
    CONVERSATION_PED_SLOT1.set(0xFFFF);
    CONVERSATION_PED_SLOT2.set(0xFFFF);
    PLAYER_CONVERSATION_PED.set(0);
    PED_CONVERSATION_HAPPENING.set(0);
    PLAYER_CONVERSATION_HAPPENING.set(0);
    CONVERSATION.set([0xFFFF; 8]);
    CJ_BASIC_MOOD.set(0xFFFF);
    CJ_GANG_BANGING.set(0xFFFF);
    CJ_FAT.set(0xFFFF);
    CJ_WELL_DRESSED.set(0xFFFF);
    NEXT_SPEECH_SLOT.set(0);
    ALL_SPEECH_DISABLED.set(0);
    A_PLAYER_SPEAKING.set(0);
    SPEECH_FORCE_AUDIBLE.set(0);
    CJ_MOOD_OVERRIDE_TIME.set(0);
}

/// `CAEPedSpeechAudioEntity::Terminate()` (0x4E5670): stops the current speech and resets the state
/// (entity, flags, sound / phrase ids = -1, last volume = -100.0).
#[hook(addr = 0x4E5670, conv = thiscall, name = "CAEPedSpeechAudioEntity::Terminate", expect = "56 8B F1 E8 38 E9 FF FF")]
pub fn speech_terminate(this: GPtr<u8>) {
    speech_stop_current_speech(this);
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 0x90, 0);
    wr::<u16>(this, 0x92, 0xFFFF);
    wr::<u16>(this, 0x94, 0xFFFF);
    wr::<u8>(this, 0x98, 0);
    wr::<u8>(this, 0x99, 0);
    wr::<u8>(this, 0x9A, 0);
    wr::<u32>(this, 0xA0, 0);
    wr::<u16>(this, 0xA4, 0xFFFF);
    wr::<u16>(this, 0xA6, 0xFFFF);
    wr::<u16>(this, 0xA8, 0xFFFF);
    wr::<u32>(this, 0xAC, 0xC2C8_0000);
}

// ---- CAEPedlessSpeechAudioEntity ------------------------------------------------------------------

/// `CAEPedlessSpeechAudioEntity::Constructor()` (0x4E6070): the base constructor, then vtable 0x85F330;
/// returns `this`. (The base constructor 0x4E4F10 is not hooked: this caller keeps `this` in EDX
/// across it.)
#[hook(addr = 0x4E6070, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::Constructor", expect = "8B D1 E8 99 EE FF FF")]
pub fn pedless_constructor(this: GPtr<u8>) -> GPtr<u8> {
    speech_constructor(this);
    wr::<u32>(this, 0, 0x85F330);
    this
}

/// `CAEPedlessSpeechAudioEntity::Initialise()` (0x4E4E10): entity and state cleared, 19 dwords from
/// +0xB4 zeroed, last volume -100.0, +0xB0 = -1, audio ped type (+0x92) = 4, +0x90 = 1.
#[hook(addr = 0x4E4E10, conv = thiscall, name = "CAEPedlessSpeechAudioEntity::Initialise", expect = "8B D1 53 33 DB")]
pub fn pedless_initialise(this: GPtr<u8>) {
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 0x114, 0);
    wr::<u32>(this, 0xA0, 0);
    wr::<u32>(this, 0xAC, 0xC2C8_0000);
    wr::<u16>(this, 0xB0, 0xFFFF);
    for i in 0..0x13isize {
        wr::<u32>(this, 0xB4 + i * 4, 0);
    }
    wr::<u8>(this, 0x98, 0);
    wr::<u8>(this, 0x99, 0);
    wr::<u8>(this, 0x9A, 0);
    wr::<u8>(this, 0x9B, 0);
    wr::<u8>(this, 0x9C, 0);
    wr::<u16>(this, 0x96, 0);
    wr::<u16>(this, 0x92, 4);
    wr::<u8>(this, 0x90, 1);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PED_STATIC_INITIALISE,
    HOOK_PED_INITIALISE,
    HOOK_PED_STOP_JET_PACK_SOUND,
    HOOK_PED_TURN_OFF_JET_PACK,
    HOOK_PED_TERMINATE,
    HOOK_PED_WEAPON_TERMINATE,
    HOOK_PED_WEAPON_ADD_AUDIO_EVENT,
    HOOK_SPEECH_SET_NEXT_PLAY_TIME,
    HOOK_SPEECH_SERVICE,
    HOOK_SPEECH_STATIC_INITIALISE,
    HOOK_SPEECH_IS_GLOBAL_CONTEXT_IMPORTANT_FOR_STREAMING,
    HOOK_SPEECH_GET_SPECIFIC_SPEECH_CONTEXT,
    HOOK_SPEECH_GET_AUDIO_PED_TYPE,
    HOOK_SPEECH_TERMINATE,
    HOOK_PEDLESS_CONSTRUCTOR,
    HOOK_PEDLESS_INITIALISE,
];
