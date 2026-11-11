//! Conversation readiness and the per-frame conversation update (batch `mvpm-ai-1`):
//! `CPed::PedIsReadyForConversation` and `CConversations::Update`. Both bodies live in `.HOODLUM` behind 5-byte entry
//! stubs; ported from the 1.0 US bytes (gta-reversed `Ped.cpp` / `Conversations.cpp` as a reading aid; the bytes win).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_world::world::common::find_player_ped;

use crate::common::*;

orig! {
    /// `CPedIntelligence::GetTaskFighting()` (0x600F30)
    fn intel_get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650, `ret 4`)
    fn event_group_get_event_of_type(g: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CAudioEngine::IsMissionAudioSampleFinished(uint8 slot)` (0x5072C0, `ret 4`), AL
    fn audio_is_mission_audio_sample_finished(e: GPtr<u8>, slot: u32) -> u8 = thiscall 0x5072C0 => unported;
    /// `CAudioEngine::GetMissionAudioLoadingStatus(uint8 slot)` (0x5072A0, `ret 4`), AL
    fn audio_get_mission_audio_loading_status(e: GPtr<u8>, slot: u32) -> u8 = thiscall 0x5072A0 => unported;
    /// `CAudioEngine::PlayLoadedMissionAudio(uint8 slot)` (0x5072B0, `ret 4`)
    fn audio_play_loaded_mission_audio(e: GPtr<u8>, slot: u32) = thiscall 0x5072B0 => unported;
    /// `CConversationForPed::Update()` (0x43C190)
    fn conversation_for_ped_update(c: GPtr<u8>) = thiscall 0x43C190 => unported;
}

/// `AudioEngine` (CAudioEngine, 0xB6BC90)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CConversations::m_AwkwardSayStatus` (0x9691C4: 0 inactive, 1 loading, 2 playing)
const AWKWARD_SAY_STATUS: usize = 0x9691C4;
/// `CConversations::m_Conversations` (0x9691D8, 14 x 0x1C, end 0x969360)
const CONVERSATIONS: usize = 0x9691D8;
const CONVERSATIONS_END: usize = 0x969360;

/// `CPed::PedIsReadyForConversation(bool checkPlayerWanted)` (stub 0x43ABA0 → 0x156CAE0, thiscall, `ret 4`), AL.
///
/// False when the ped is in a vehicle (flags +0x46C bit 8) or `GetTaskFighting()` is set. With the argument byte set:
/// false when `FindPlayerPed(-1)`'s wanted level is non-zero (player data +0x480, its first field the `CWanted*`,
/// level at +0x2C; a null player data reads `[0x2C]` as the original does). False for ped states (+0x534) 5, 6 and 7.
/// A ped whose created-by byte (+0x484) is 2 (mission) is ready from here on; else false when it has a task of type
/// 0x4B4 (`FindTaskByType`) or an event of type 0x13 in its event group (intelligence +0x68), true otherwise.
///
/// The `FindPlayerPed` call is return-address obfuscated in the original (`push -1; call $+5; ... ret`): the target
/// is `~[0x1560580] ^ [0x6840B7]` = 0x56E210 and the return lands at 0x156CB2F (decoded from the image).
#[hook(addr = 0x43aba0, conv = thiscall, name = "CPed::PedIsReadyForConversation", expect = "E9 3B 1F 13 01",
       unverified = "mvpm-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "AL result, upper EAX bytes from the last callee or the state; FindPlayerPed reached through a decoded return-address-obfuscated call (0x56E210)")]
pub fn ped_is_ready_for_conversation(this: GPtr<u8>, check_wanted: u32) -> u8 {
    if rd::<u32>(this, 0x46C) & 0x100 != 0 {
        return 0;
    }
    if !intel_get_task_fighting(rd::<GPtr<u8>>(this, 0x47C)).is_null() {
        return 0;
    }
    if check_wanted as u8 != 0 {
        let player = find_player_ped(-1);
        let data = rd::<GPtr<u8>>(player, 0x480);
        let wanted = if data.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(data, 0) };
        if rd::<u32>(wanted, 0x2C) != 0 {
            return 0;
        }
    }
    let state = rd::<u32>(this, 0x534);
    if state == 5 || state == 6 || state == 7 {
        return 0;
    }
    if rd::<u8>(this, 0x484) == 2 {
        return 1;
    }
    if !intel_find_task_by_type(rd::<GPtr<u8>>(this, 0x47C), 0x4B4).is_null() {
        return 0;
    }
    let intel = rd::<GPtr<u8>>(this, 0x47C);
    if !event_group_get_event_of_type(intel.byte_add(0x68), 0x13).is_null() {
        return 0;
    }
    1
}

/// `CConversations::Update()` (stub 0x43C590 → 0x156D7C0, cdecl).
///
/// Awkward-say status 1 (loading): when `GetMissionAudioLoadingStatus(0) == 1`, `PlayLoadedMissionAudio(0)` and the
/// status becomes 2; no conversation update. Status 2 (playing): nothing until `IsMissionAudioSampleFinished(0)`; then
/// the status is reset and every conversation updates. Any other status: `CConversationForPed::Update()` on each of the
/// 14 conversations.
///
/// The reset in the playing branch happens inside the no-CD VM call (`push 0x4BAB; push 0x156007C; call 0xCB73E0`
/// between `pushfd`/`popfd`): the port writes status 0 there, the value gta-reversed has (assumed no-CD VM effect).
#[hook(addr = 0x43c590, conv = cdecl, name = "CConversations::Update", expect = "E9 2B 12 13 01",
       unverified = "mvpm-ai-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "assumed no-CD VM value: the VM call 0x4BAB in the playing branch sets m_AwkwardSayStatus (0x9691C4) to 0")]
pub fn conversations_update() {
    let engine: GPtr<u8> = GPtr::from_addr(AUDIO_ENGINE);
    let status = g::<i32>(AWKWARD_SAY_STATUS);
    match status.read() {
        1 => {
            if audio_get_mission_audio_loading_status(engine, 0) == 1 {
                audio_play_loaded_mission_audio(engine, 0);
                status.write(2);
            }
            return;
        }
        2 => {
            if audio_is_mission_audio_sample_finished(engine, 0) == 0 {
                return;
            }
            status.write(0);
        }
        _ => {}
    }
    let mut c = CONVERSATIONS;
    while c < CONVERSATIONS_END {
        conversation_for_ped_update(GPtr::from_addr(c));
        c += 0x1C;
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_IS_READY_FOR_CONVERSATION, HOOK_CONVERSATIONS_UPDATE];
