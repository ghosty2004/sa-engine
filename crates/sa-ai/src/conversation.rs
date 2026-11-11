//! Ped conversations (PORT-AI-5, helper A): `CPedToPlayerConversations::Clear` / `Update` and `CConversationForPed`
//! `IsPlayerInPositionForConversation` / `Update`. Ported from the 1.0 US bytes (the `.HOODLUM` bodies included; the
//! exe wins over gta-reversed, which has none of these reversed, and the specs `docs/re/specs/AI/CConversationForPed.md`,
//! `docs/re/specs/AI/CPedToPlayerConversations.md`). Every call to another catalogued function goes through `orig!`
//! at the original address; tasks come from the original task pool.
//!
//! Obfuscated constants in the bodies (`push r; pushfd; mov r, [A]; xor/add/adc/sbb r, [B]; popfd; xchg [esp], r`)
//! were decoded from the image: they are fixed values (listed at each use).
//!
//! `CConversationForPed` (0x1C): +4 current node (i32), +8 ped, +0xC last change (time), +0x10 last time we were
//! close enough, +0x14 status (0 inactive, 1 player speaking, 2 ped speaking, 3 waiting for input), +0x18 enabled
//! (u8), +0x19 suppress subtitles (u8). `CConversations::m_Nodes` 0x969570 (`CConversationNode`, 0x18: +0 name
//! char[8], +8 i16 node yes, +0xA i16 node no, +0xC speech, +0x10 speech yes, +0x14 speech no).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// `CPedToPlayerConversations::m_State` (0 inactive, 1 ped has opened, 2 waiting for final word, 3 waiting to
    /// finish)
    pub static P2P_STATE @ 0x969A20: u32 = 0;
    /// `CPedToPlayerConversations::m_pPed` (registered reference)
    pub static P2P_PED @ 0x9691C0: GPtr<u8> = GPtr::NULL;
    /// `CPedToPlayerConversations::m_Topic`
    pub static P2P_TOPIC @ 0x9691BC: i32 = 0;
    /// `CPedToPlayerConversations::m_TimeOfLastPlayerConversation`
    pub static P2P_TIME_OF_LAST @ 0x9691B4: u32 = 0;
    /// `CPedToPlayerConversations::m_StartTime`
    pub static P2P_START_TIME @ 0x9691B8: u32 = 0;
    /// `CPedToPlayerConversations::m_bPositiveOpening`
    pub static P2P_POSITIVE_OPENING @ 0x9691B1: u8 = 0;
    /// Ped pool scan index of `CPedToPlayerConversations::Update` (no gta-reversed name)
    pub static P2P_SCAN_INDEX @ 0x969A3C: i32 = 0;
    /// The vehicle the player was last in (registered reference; no gta-reversed name)
    pub static P2P_LAST_PLAYER_VEHICLE @ 0x969A40: GPtr<u8> = GPtr::NULL;
}

/// `CTimer::m_snTimeInMilliseconds` (owned by CTimer).
const TIME_MS: usize = 0xB7CB84;
/// `CConversations::m_Nodes` (owned by CConversations).
const NODES: usize = 0x969570;
/// `gString` (owned by the file loader).
const G_STRING: usize = 0xB71670;
/// `TheText` (owned by CText).
const THE_TEXT: usize = 0xC1B340;
/// `g_ikChainMan` (owned by IKChainManager_c).
const IK_CHAIN_MAN: usize = 0xC15448;
/// `CPools::ms_pPedPool`
const PED_POOL: usize = 0xB74490;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CWeather::Wind`
const WIND: usize = 0xC812F0;
/// `CClock::ms_nGameClockHours`
const CLOCK_HOURS: usize = 0xB70153;
/// 1.0f as the dword the original pushes.
const ONE_F: u32 = 0x3F80_0000;

orig! {
    /// `CPed::Say(uint16 phrase, uint32 offset, float probability, bool overrideSilence, bool forceAudible,
    /// bool frontEnd)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, prob: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader` (0x5F6AA0)
    fn membership_count_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPed::PedIsReadyForConversation(bool)` (0x43ABA0, `ret 4`)
    fn ped_is_ready_for_conversation(p: GPtr<u8>, random: u32) -> u8 = thiscall 0x43ABA0 => unported;
    /// `CConversationForPed::IsPlayerInPositionForConversation(bool)` (0x43AC40, `ret 4`)
    fn conv_is_player_in_position(c: GPtr<u8>, random: u32) -> u8 = thiscall 0x43AC40 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::ConversationNoJustDown` (0x541200)
    fn pad_conversation_no_just_down(p: GPtr<u8>) -> u8 = thiscall 0x541200 => unported;
    /// `CPad::ConversationYesJustDown` (0x5411D0)
    fn pad_conversation_yes_just_down(p: GPtr<u8>) -> u8 = thiscall 0x5411D0 => unported;
    /// `CMessages::ClearSmallMessagesOnly` (0x69DD30)
    fn clear_small_messages_only() = cdecl 0x69DD30 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageJump(const char* text, uint32 time, uint16 flag, bool previousBrief)` (0x69F1E0)
    fn add_message_jump(text: GPtr<u8>, time: u32, flag: u32, brief: u32) = cdecl 0x69F1E0 => unported;
    /// `CMessages::AddMessageQ(const char* text, uint32 time, uint16 flag, bool previousBrief)` (0x69F0B0)
    fn add_message_q(text: GPtr<u8>, time: u32, flag: u32, brief: u32) = cdecl 0x69F0B0 => unported;
    /// `CConversations::AwkwardSay(int32 sample, CPed*)` (0x43A810)
    fn awkward_say(sample: i32, ped: GPtr<u8>) = cdecl 0x43A810 => unported;
    /// `CAEPedSpeechAudioEntity::ReleasePlayerConversation()` (0x4E3960)
    fn release_player_conversation() = cdecl 0x4E3960 => unported;
    /// `CAEPedSpeechAudioEntity::RequestPlayerConversation(CPed*)` (0x4E38C0)
    fn request_player_conversation(ped: GPtr<u8>) -> u8 = cdecl 0x4E38C0 => unported;
    /// `CAEPedSpeechAudioEntity::WillPedChatAboutTopic(int16)` (vtable +0x14 of ped +0x294)
    fn v_will_ped_chat_about_topic(s: GPtr<u8>, topic: i32) -> u8 = thiscall vslot 0x14 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPlayerPed::GetWantedLevel` (0x41BE60)
    fn player_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CConversations::IsConversationGoingOn()` (0x43AAC0)
    fn conversations_is_going_on() -> u8 = cdecl 0x43AAC0 => unported;
    /// `CPool<CPed>::GetAt(int32)` (0x4082A0, `ret 4`)
    fn ped_pool_get_at(pool: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4082A0 => unported;
    /// `CEntity::GetIsOnScreen` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0, `ret 4`)
    fn placeable_get_top_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn stats_get_stat_value(id: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPedClothesDesc::HasVisibleNewHairCut(int32)` (0x5A7970, `ret 4`)
    fn clothes_has_visible_new_hair_cut(c: GPtr<u8>, a: u32) -> u8 = thiscall 0x5A7970 => unported;
    /// `CPedClothesDesc::HasVisibleTattoo` (0x5A79D0)
    fn clothes_has_visible_tattoo(c: GPtr<u8>) -> u8 = thiscall 0x5A79D0 => unported;
    /// `CPed::DisablePedSpeech(bool)` (0x5EFF60, `ret 4`)
    fn ped_disable_ped_speech(p: GPtr<u8>, b: u32) = thiscall 0x5EFF60 => unported;
    /// `CPed::EnablePedSpeech` (0x5EFF70)
    fn ped_enable_ped_speech(p: GPtr<u8>) = thiscall 0x5EFF70 => unported;
    /// `CPed::GetPedTalking` (0x5EFF50)
    fn ped_get_ped_talking(p: GPtr<u8>) -> u8 = thiscall 0x5EFF50 => unported;
    /// `IKChainManager_c::LookAt(const char* purpose, CPed*, CEntity* target, int32 time, ePedBones, CVector* pos,
    /// bool useTorso, float speed, int32 blendTime, int32 priority, bool)` (0x618970, `ret 0x2C`)
    #[allow(clippy::too_many_arguments)]
    fn ik_look_at(m: GPtr<u8>, purpose: u32, ped: GPtr<u8>, target: GPtr<u8>, time: u32, bone: u32, pos: u32, torso: u32, speed: u32, blend: u32, prio: u32, a: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOutTime)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, t: u32) = thiscall 0x618280 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(s: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32 time, bool looped, bool useAnimIdleStance, float)`
    /// (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32, uint32, uint32, uint32, int8)` (0x620E30,
    /// `ret 0x18`)
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, target: GPtr<u8>, time: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool affectsDeadPeds)` (0x4B0A00,
    /// `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `IsFemalePedType(ePedType)` (0x43A6A0, a `.HOODLUM` stub: pedType == 22 || pedType == 5; no catalogue name)
    fn is_female_ped_type(t: u32) -> u8 = cdecl 0x43A6A0 => unported;
    /// `CPedToPlayerConversations::EndConversation()` (0x43AB10)
    fn p2p_end_conversation() = cdecl 0x43AB10 => unported;
}

fn now() -> u32 {
    g::<u32>(TIME_MS).read()
}

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CConversations::m_Nodes[i]`
fn node(i: i32) -> GPtr<u8> {
    g::<u8>(NODES).byte_add(i.wrapping_mul(0x18) as isize)
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `CPedToPlayerConversations::Clear` (0x43AAE0, a `.HOODLUM` stub; body 0x15635E0): a non-zero state releases the
/// player conversation and becomes 0; the ped and the time of the last conversation are cleared (the ped's
/// reference is not unregistered).
#[hook(addr = 0x43AAE0, conv = cdecl, name = "CPedToPlayerConversations::Clear", expect = "E9 FB 8A 12 01")]
pub fn p2p_clear() {
    if P2P_STATE.get() != 0 {
        release_player_conversation();
        P2P_STATE.set(0);
    }
    P2P_PED.set(GPtr::NULL);
    P2P_TIME_OF_LAST.set(0);
}

/// `CConversationForPed::IsPlayerInPositionForConversation(bool randomConversation)` (0x43AC40, `ret 4`, AL).
///
/// False when the player is more than 4.0 from the ped (`(dz*dz + dy*dy) + dx*dx`, fsqrt; NaN passes), when the
/// two forward axes have a positive dot product (`(z*z' + y*y') + x*x'`, no null test of either matrix), when the
/// player's 2D speed (`x*x + y*y`, fsqrt) exceeds 0.01, when the ped (+0x764) last talked to the player less than
/// 6000 ms ago (+0x768, unsigned), or, for a random conversation, when the player's group has a member besides the
/// leader. Else `player->PedIsReadyForConversation(arg) && ped->PedIsReadyForConversation(arg)` (the argument dword
/// is passed on unchanged).
#[hook(addr = 0x43AC40, conv = thiscall, name = "CConversationForPed::IsPlayerInPositionForConversation", expect = "83 EC 0C 56 57")]
pub fn conv_is_player_in_position_for_conversation(this: GPtr<u8>, random: u32) -> u8 {
    let p = pos(rd::<GPtr<u8>>(this, 8));
    let mut buf = [0u32; 3];
    let c = find_player_coors(local(&mut buf), -1);
    let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(c, 8)).fsub_m(rd::<f32>(p, 8));
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if !dist.fcom(cf(0x858B90)).test_any(0x41) {
        return 0;
    }
    let pm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x14);
    let fm = rd::<GPtr<u8>>(find_player_ped(-1), 0x14);
    let dot = x(rd::<f32>(fm, 0x18))
        .fmul_m(rd::<f32>(pm, 0x18))
        .fadd(x(rd::<f32>(fm, 0x14)).fmul_m(rd::<f32>(pm, 0x14)))
        .fadd(x(rd::<f32>(fm, 0x10)).fmul_m(rd::<f32>(pm, 0x10)));
    if !dot.fcom(cf(0x858B50)).test_any(0x41) {
        return 0;
    }
    let pl = find_player_ped(-1);
    let sx = x(rd::<f32>(pl, 0x44));
    let sy = x(rd::<f32>(pl, 0x48));
    let speed = sx.fmul(sx).fadd(sy.fmul(sy)).fsqrt();
    if !speed.fcom(cf(0x858C58)).test_any(0x41) {
        return 0;
    }
    let ped = rd::<GPtr<u8>>(this, 8);
    if rd::<GPtr<u8>>(ped, 0x764) == find_player_ped(-1) && now() < rd::<u32>(ped, 0x768).wrapping_add(6000) {
        return 0;
    }
    if random as u8 != 0 {
        let grp = rd::<i32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
        let membership = g::<u8>(0xC09928).byte_add(grp.wrapping_mul(0x2D4) as isize);
        if membership_count_excluding_leader(membership) >= 1 {
            return 0;
        }
    }
    if ped_is_ready_for_conversation(find_player_ped(-1), random) == 0 {
        return 0;
    }
    (ped_is_ready_for_conversation(rd::<GPtr<u8>>(this, 8), random) != 0) as u8
}

/// `strcpy(gString, name)` followed by the two-byte suffix store at `strlen(gString)` (the original's byte loops).
fn g_string_with_suffix(name: GPtr<u8>, suffix: u16) {
    let dst = g::<u8>(G_STRING);
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(name, i);
        wr::<u8>(dst, i, c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    let mut n = 0isize;
    while rd::<u8>(dst, n) != 0 {
        n += 1;
    }
    wr::<u16>(dst, n, suffix);
}

/// `CConversationForPed::Update` (0x43C190, a `.HOODLUM` stub; body 0x15668B0-0x1566D41).
///
/// Status 0 without a ped, when disabled or when the player is not in position (`IsPlayerInPositionForConversation
/// (false)`). START (subtitle `AddMessageJump(node name, 4000, 1, 1)` after `ClearSmallMessagesOnly`, the node's
/// speech by the ped: `Say((u16)speech, 0, 1.0f, 0, 0, 0)` when > 0, `AwkwardSay(-speech, ped)` when < 0; status 2)
/// runs when the player was not close for more than 10 s, or in status 1 after 4 s or on either pad button. Else,
/// more than 400 ms after the last change and in status 0, 2 or 3, a No / Yes press with a node behind it shows the
/// answer subtitle (node name + "N" / "Y", `AddMessageJump(.., 4000, 3, 1)`), the player says the answer speech,
/// the node advances, the new node's subtitle is queued (`AddMessageQ(.., 4000, 1, 1)`) and the status becomes 1.
/// Every path with a ped in position stores the time as the last close time and turns status 2 into 3 after 4 s.
///
/// Decoded constants: the `IsPlayerInPositionForConversation` argument, `GetPad` argument and the No path's last
/// `Say` argument are 0; the Yes path's `AddMessageJump` flag is 3; the START `Say` probability is 1.0f; the Yes
/// path's time is read from 0xB7CB84. The Yes button test is a disguised `call` (0x1566AD3) into
/// `CPad::ConversationYesJustDown` 0x5411D0. The Yes path's `strcpy` destination is the result of the protection
/// VM 0xCB73E0 (descriptor 0x155FF8C, not evaluated); it is taken to be `gString` 0xB71670 because the following
/// `strlen` and suffix store operate on 0xB71670, as in the No path (spec `CConversationForPed.md`).
#[hook(addr = 0x43C190, conv = thiscall, name = "CConversationForPed::Update", expect = "E9 1B A7 12 01")]
pub fn conv_for_ped_update(this: GPtr<u8>) {
    if rd::<GPtr<u8>>(this, 8).is_null() || rd::<u8>(this, 0x18) == 0 || conv_is_player_in_position(this, 0) == 0 {
        wr::<u32>(this, 0x14, 0);
        return;
    }
    let mut start = now() > rd::<u32>(this, 0x10).wrapping_add(10000);
    if !start && rd::<u32>(this, 0x14) == 1 {
        start = now() > rd::<u32>(this, 0xC).wrapping_add(4000)
            || pad_conversation_no_just_down(get_pad(0)) != 0
            || pad_conversation_yes_just_down(get_pad(0)) != 0;
    }
    if start {
        if rd::<u8>(this, 0x19) == 0 {
            clear_small_messages_only();
            let name = node(rd::<i32>(this, 4));
            add_message_jump(text_get(g(THE_TEXT), name), 4000, 1, 1);
        }
        let speech = rd::<i32>(node(rd::<i32>(this, 4)), 0xC);
        if speech > 0 {
            let id = rd::<u16>(node(rd::<i32>(this, 4)), 0xC) as u32;
            ped_say(rd::<GPtr<u8>>(this, 8), id, 0, ONE_F, 0, 0, 0);
        }
        let speech = rd::<i32>(node(rd::<i32>(this, 4)), 0xC);
        if speech < 0 {
            awkward_say(speech.wrapping_neg(), rd::<GPtr<u8>>(this, 8));
        }
        wr::<u32>(this, 0x14, 2);
        wr::<u32>(this, 0xC, now());
    } else if now() > rd::<u32>(this, 0xC).wrapping_add(400) && matches!(rd::<u32>(this, 0x14), 0 | 2 | 3) {
        conv_answer(this);
    }
    // COMMON
    wr::<u32>(this, 0x10, now());
    if rd::<u32>(this, 0x14) == 2 && now() > rd::<u32>(this, 0xC).wrapping_add(4000) {
        wr::<u32>(this, 0x14, 3);
    }
}

/// The No / Yes input step of `CConversationForPed::Update` (0x1566973-0x1566C65).
fn conv_answer(this: GPtr<u8>) {
    let no = pad_conversation_no_just_down(get_pad(0)) != 0 && rd::<i16>(node(rd::<i32>(this, 4)), 0xA) >= 0;
    let (suffix_addr, flag, speech_off, next_off) = if no {
        (0x8594E4usize, 3u32, 0x14isize, 0xAisize)
    } else {
        if pad_conversation_yes_just_down(get_pad(0)) == 0 {
            return;
        }
        if rd::<i16>(node(rd::<i32>(this, 4)), 8) < 0 {
            return;
        }
        (0x8594E0, 3, 0x10, 8)
    };
    if rd::<u8>(this, 0x19) == 0 {
        clear_small_messages_only();
        g_string_with_suffix(node(rd::<i32>(this, 4)), g::<u16>(suffix_addr).read());
        add_message_jump(text_get(g(THE_TEXT), g(G_STRING)), 4000, flag, 1);
    }
    let speech = rd::<i32>(node(rd::<i32>(this, 4)), speech_off);
    if speech > 0 {
        ped_say(find_player_ped(-1), speech as u16 as u32, 0, ONE_F, 0, 0, 0);
    }
    if rd::<i32>(node(rd::<i32>(this, 4)), speech_off) < 0 {
        let pl = find_player_ped(-1);
        awkward_say(rd::<i32>(node(rd::<i32>(this, 4)), speech_off).wrapping_neg(), pl);
    }
    let next = rd::<i16>(node(rd::<i32>(this, 4)), next_off) as i32;
    wr::<i32>(this, 4, next);
    wr::<u32>(this, 0xC, now());
    if rd::<u8>(this, 0x19) == 0 {
        add_message_q(text_get(g(THE_TEXT), node(next)), 4000, 1, 1);
    }
    wr::<u32>(this, 0x14, 1);
}

// ---- CPedToPlayerConversations::Update ----------------------------------------------------------

/// `m_State = 3; m_StartTime = now` (0x43BE4F / 0x43C074).
fn p2p_wait_to_finish() {
    let t = now();
    P2P_STATE.set(3);
    P2P_START_TIME.set(t);
}

/// The no-ped exit (0x43BA40): state 0, release the conversation, abort the player's look-at.
fn p2p_null_ped(player: GPtr<u8>) {
    P2P_STATE.set(0);
    release_player_conversation();
    if ik_is_looking(g(IK_CHAIN_MAN), player) != 0 {
        ik_abort_look_at(g(IK_CHAIN_MAN), player, 250);
    }
}

/// The aggressive response (0x43BC19-0x43BE0F, three copies): a sequence of `CTaskSimpleStandStill(2000, 0, 0, 8.0f)`
/// and `CTaskComplexKillPedOnFoot(player, -1, 0, 0, 0, 0)` posted on `m_pPed` as a `CEventScriptCommand(3, seq, 0)`
/// (a stack object in the original, 0x18 bytes: a zeroed local here). A failed allocation gives a null task (or a
/// null sequence, on which `AddTask` is still called).
fn p2p_aggro(player: GPtr<u8>) {
    let mem = task_new(0x40);
    let seq = if mem.is_null() { GPtr::NULL } else { sequence_ctor(mem) };
    let mem = task_new(0x20);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        stand_still_ctor(mem, 2000, 0, 0, 0x4100_0000)
    };
    sequence_add_task(seq, t);
    let mem = task_new(0x38);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        kill_ped_on_foot_ctor(mem, player, 0xFFFF_FFFF, 0, 0, 0, 0)
    };
    sequence_add_task(seq, t);
    let mut ev = [0u32; 6];
    event_script_command_ctor(local(&mut ev), 3, seq, 0);
    let group = rd::<GPtr<u8>>(P2P_PED.get(), 0x47C).byte_add(0x68);
    event_group_add(group, local(&mut ev), 0);
    event_script_command_dtor(local(&mut ev));
}

/// Result of a candidate's topic (state 0).
enum Opening {
    /// START (0x43B99B)
    Start,
    /// SKIP (0x43B94E)
    Skip,
}

/// OPEN(positive, phrase): `RequestPlayerConversation(ped)`, then `m_bPositiveOpening = positive` and
/// `ped->Say(phrase, 0, 1.0f, 1, 0, 0)`.
fn p2p_open(ped: GPtr<u8>, positive: u8, phrase: u32) -> Opening {
    if request_player_conversation(ped) == 0 {
        return Opening::Skip;
    }
    P2P_POSITIVE_OPENING.set(positive);
    ped_say(ped, phrase, 0, ONE_F, 1, 0, 0);
    Opening::Start
}

/// `_ftol(GetStatValue(stat) + (int32)extra - [sub])` (`fiadd` then `fsub dword`).
fn stat_score(stat: u32, extra: u32, sub: usize) -> i32 {
    ftol32(X87::from_f64_exact(stats_get_stat_value(stat)).fadd(xi(extra as i32)).fsub_m(g::<f32>(sub).read()))
}

/// The player's `CPedClothesDesc` (`m_pPlayerData` +0x480 -> +4), through a fresh `FindPlayerPed(-1)`.
fn player_clothes() -> GPtr<u8> {
    rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 4)
}

/// Vehicle class - 1 -> case of the topic-0 switch (the byte table at 0x43C150; cases 0x43B44D, 0x43B406, skip).
const VEHICLE_CLASS_CASE: [u8; 9] = [0, 1, 1, 0, 0, 0, 2, 2, 1];

/// The topic switch of state 0 (jump table 0x43C11C) for the candidate `ped`.
fn p2p_topic_opening(ped: GPtr<u8>) -> Opening {
    match P2P_TOPIC.get() as u32 {
        0 => {
            let v = P2P_LAST_PLAYER_VEHICLE.get();
            if v.is_null() {
                return Opening::Skip;
            }
            let pp = pos(ped);
            let vp = pos(v);
            let mut buf = [0u32; 3];
            let d = vector_sub(local(&mut buf), vp, pp);
            let dy = x(rd::<f32>(d, 4));
            let dx = x(rd::<f32>(d, 0));
            if dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().fcom(cf(0x858BA4)).test_parity(5) {
                return Opening::Skip;
            }
            let model = rd::<i16>(v, 0x22) as i32;
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
            let c = (rd::<i8>(mi, 0x4D) as i32).wrapping_sub(1) as u32;
            if c > 8 {
                return Opening::Skip;
            }
            match VEHICLE_CLASS_CASE[c as usize] {
                0 => p2p_open(ped, 0, 0x30),
                1 => {
                    if x(rd::<f32>(v, 0x4C0)).fcom(cf(0x8594DC)).test_any(0x41) {
                        Opening::Skip
                    } else {
                        p2p_open(ped, 1, 0x39)
                    }
                }
                _ => Opening::Skip,
            }
        }
        1 => {
            let s = stat_score(0x42, (rd::<u16>(ped, 0x20) & 0x1FF) as u32, 0x858FB4);
            if s > 500 {
                p2p_open(ped, 1, 0x3A)
            } else if s < 150 {
                p2p_open(ped, 0, 0x31)
            } else {
                Opening::Skip
            }
        }
        2 => {
            let s = stat_score(0x42, (rd::<u16>(ped, 0x20) & 0x1FF) as u32, 0x858FB4);
            if clothes_has_visible_new_hair_cut(player_clothes(), 0) == 0 {
                return Opening::Skip;
            }
            if s > 500 {
                return p2p_open(ped, 1, 0x3B);
            }
            if s >= 150 && X87::from_f64_exact(stats_get_stat_value(0x1F)).fcom(cf(0x858C80)).test_parity(5) {
                return Opening::Skip;
            }
            p2p_open(ped, 0, 0x32)
        }
        3 => {
            let a = stat_score(0x17, rd::<u8>(ped, 0x20) as u32, 0x858BF4);
            let b = stat_score(0x15, rd::<u8>(ped, 0x20) as u32, 0x858BF4);
            if a > 400 && b < 250 {
                p2p_open(ped, 1, 0x3C)
            } else if a.wrapping_sub(b) < 200 && b > 500 {
                p2p_open(ped, 0, 0x33)
            } else {
                Opening::Skip
            }
        }
        4 => {
            let s = stat_score(0x42, (rd::<u16>(ped, 0x20) & 0x3FF) as u32, 0x8594D8);
            if s > 700 {
                p2p_open(ped, 1, 0x3D)
            } else if s < 300 {
                p2p_open(ped, 0, 0x34)
            } else {
                Opening::Skip
            }
        }
        5 => {
            let s = stat_score(0x19, (rd::<u16>(ped, 0x20) & 0x1FF) as u32, 0x858FB4);
            if s > 700 {
                p2p_open(ped, 1, 0x3E)
            } else if s < 300 {
                p2p_open(ped, 0, 0x35)
            } else {
                Opening::Skip
            }
        }
        6 => {
            if clothes_has_visible_tattoo(player_clothes()) == 0 {
                return Opening::Skip;
            }
            let r = general_random_int_in_range(0, 1000);
            if r > 700 {
                p2p_open(ped, 1, 0x3F)
            } else if r < 300 {
                p2p_open(ped, 0, 0x36)
            } else {
                Opening::Skip
            }
        }
        7 => {
            if !cf(WIND).fcom(cf(0x858B1C)).test_parity(5) {
                let h = g::<u8>(CLOCK_HOURS).read();
                if h >= 20 || h <= 6 {
                    return Opening::Skip;
                }
                p2p_open(ped, 1, 0x40)
            } else if cf(WIND).fcom(cf(0x858B8C)).test_any(0x41) {
                Opening::Skip
            } else {
                p2p_open(ped, 0, 0x37)
            }
        }
        t @ (8 | 9) => {
            if request_player_conversation(ped) == 0 {
                return Opening::Skip;
            }
            ped_say(ped, if t == 8 { 0xFB } else { 0xFC }, 0, ONE_F, 1, 0, 0);
            P2P_POSITIVE_OPENING.set(1);
            Opening::Start
        }
        _ => Opening::Skip,
    }
}

/// The candidate tests of state 0 (0x43B202-0x43B326), in the original's order.
fn p2p_is_candidate(ped: GPtr<u8>, player: GPtr<u8>) -> bool {
    if ped.is_null() || rd::<u8>(ped, 0x484) != 1 {
        return false;
    }
    if ped_is_ready_for_conversation(ped, 1) == 0 || rd::<u32>(ped, 0x474) & 0x80_0000 != 0 {
        return false;
    }
    if entity_get_is_on_screen(ped) == 0 {
        return false;
    }
    let (mut sub, mut top) = ([0u32; 3], [0u32; 3]);
    let d = vector_sub(local(&mut sub), pos(player), pos(ped));
    let up = placeable_get_top_direction(ped, local(&mut top));
    if X87::from_f64_exact(vector_dot_product(up, d)).fcom(cf(0x858B50)).test_any(0x41) {
        return false;
    }
    let (mut sub, mut top) = ([0u32; 3], [0u32; 3]);
    let d = vector_sub(local(&mut sub), pos(ped), pos(player));
    let up = placeable_get_top_direction(player, local(&mut top));
    if X87::from_f64_exact(vector_dot_product(up, d)).fcom(cf(0x858B50)).test_any(0x41) {
        return false;
    }
    let pp = pos(ped);
    let (mut coors, mut sub) = ([0u32; 3], [0u32; 3]);
    let c = find_player_coors(local(&mut coors), -1);
    let d = vector_sub(local(&mut sub), pp, c);
    !X87::from_f64_exact(vector_magnitude(d)).fcom(cf(0x858F48)).test_parity(5)
}

/// State 0 (0x43B15E): look for a ped that opens a conversation with the player.
fn p2p_state_inactive(player: GPtr<u8>) {
    if now() <= P2P_TIME_OF_LAST.get().wrapping_add(30000) {
        return;
    }
    if !find_player_vehicle(-1, 0).is_null() {
        return;
    }
    if player_get_wanted_level(find_player_ped(-1)) != 0 {
        return;
    }
    if conversations_is_going_on() != 0 {
        return;
    }
    if ped_is_ready_for_conversation(player, 1) == 0 {
        return;
    }
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    for _ in 0..8 {
        let size = rd::<i32>(pool, 8);
        let mut idx = P2P_SCAN_INDEX.get().wrapping_add(1);
        P2P_SCAN_INDEX.set(idx);
        if idx >= size {
            idx = 0;
            P2P_SCAN_INDEX.set(0);
        }
        let ped = ped_pool_get_at(pool, idx);
        if !p2p_is_candidate(ped, player) {
            continue;
        }
        let pt = rd::<i32>(ped, 0x598);
        let topic = if pt == 7 || (9..=16).contains(&pt) {
            general_random_int_in_range(8, 10)
        } else {
            general_random_int_in_range(0, 7)
        };
        P2P_TOPIC.set(topic);
        if v_will_ped_chat_about_topic(ped.byte_add(0x294), topic) == 0 {
            continue;
        }
        match p2p_topic_opening(ped) {
            Opening::Start => {
                P2P_STATE.set(1);
                P2P_PED.set(ped);
                entity_register_reference(ped, P2P_PED.ptr().cast());
                let t = now();
                P2P_TIME_OF_LAST.set(t);
                P2P_START_TIME.set(t);
                ped_disable_ped_speech(ped, 0);
                ik_look_at(g(IK_CHAIN_MAN), 0x8594C4, ped, player, 100000, 5, 0, 0, 0x3E80_0000, 500, 8, 0);
                ik_look_at(g(IK_CHAIN_MAN), 0x8594AC, player, ped, 100000, 5, 0, 0, 0x3E80_0000, 500, 8, 0);
                return;
            }
            Opening::Skip => {
                if at_site(0x43B953, rand) & 0xFFF == 3 {
                    ped_say(ped, 0x58, 0, ONE_F, 0, 0, 0);
                    return;
                }
            }
        }
    }
}

/// The player's reply chosen by the ped's sex (`IsFemalePedType(m_pPed->m_nPedType)`): `female` / `male`.
fn p2p_say_by_sex(player: GPtr<u8>, female: u32, male: u32) {
    let f = is_female_ped_type(rd::<u32>(P2P_PED.get(), 0x598));
    ped_say(player, if f != 0 { female } else { male }, 0, ONE_F, 1, 0, 0);
}

/// The player's "yes" to the positive opening of topics 0..7 (jump table 0x43C15C).
const YES_PHRASES: [u32; 8] = [0x81, 0x82, 0x86, 0x89, 0x8A, 0x8B, 0x8C, 0xEA];

/// State 1 (0x43BA38): the ped has opened; wait 4 s for the player's yes / no.
fn p2p_state_opened(player: GPtr<u8>) {
    if P2P_PED.get().is_null() {
        p2p_null_ped(player);
        return;
    }
    if now() > P2P_START_TIME.get().wrapping_add(4000) {
        P2P_STATE.set(0);
        release_player_conversation();
        ped_enable_ped_speech(P2P_PED.get());
        if ik_is_looking(g(IK_CHAIN_MAN), player) != 0 {
            ik_abort_look_at(g(IK_CHAIN_MAN), player, 250);
        }
        if ik_is_looking(g(IK_CHAIN_MAN), P2P_PED.get()) != 0 {
            ik_abort_look_at(g(IK_CHAIN_MAN), P2P_PED.get(), 250);
            if P2P_POSITIVE_OPENING.get() != 0 {
                ped_say(P2P_PED.get(), 0x38, 0, ONE_F, 0, 0, 0);
            }
        }
        return;
    }
    if pad_conversation_yes_just_down(get_pad(0)) != 0 {
        if P2P_POSITIVE_OPENING.get() == 0 {
            p2p_say_by_sex(player, 0x83, 0x84);
            p2p_wait_to_finish();
            return;
        }
        match P2P_TOPIC.get() as u32 {
            t @ 0..=7 => {
                ped_say(player, YES_PHRASES[t as usize], 0, ONE_F, 1, 0, 0);
                p2p_wait_to_finish();
            }
            t @ (8 | 9) => {
                ped_say(player, if t == 8 { 0xEC } else { 0x6B }, 0, ONE_F, 1, 0, 0);
                if general_random_int_in_range(0, 100) < 75 {
                    p2p_aggro(player);
                }
                p2p_end_conversation();
            }
            _ => p2p_wait_to_finish(),
        }
        return;
    }
    if pad_conversation_no_just_down(get_pad(0)) == 0 {
        return;
    }
    if P2P_POSITIVE_OPENING.get() != 0 {
        match P2P_TOPIC.get() {
            8 => {
                ped_say(player, 0xEB, 0, ONE_F, 1, 0, 0);
                p2p_end_conversation();
            }
            9 => {
                ped_say(player, 0x6A, 0, ONE_F, 1, 0, 0);
                p2p_end_conversation();
            }
            _ => {
                p2p_say_by_sex(player, 0x83, 0x84);
                let t = now();
                P2P_STATE.set(2);
                P2P_START_TIME.set(t);
            }
        }
        return;
    }
    p2p_say_by_sex(player, 0x87, 0x88);
    if rd::<i32>(P2P_PED.get(), 0x598) != 8 && general_random_int_in_range(0, 100) < 40 {
        p2p_aggro(player);
    }
    p2p_wait_to_finish();
}

/// `CPedToPlayerConversations::Update` (0x43B0F0; the START path continues through the island 0x401A1D, which
/// pushes the decoded constant 0x9691C0 = `&m_pPed` and jumps back to 0x43B9A0).
///
/// The vehicle the player is in (if any) is stored in 0x969A40 and registered (the old reference is not cleared).
/// State 0: after 30 s since the last conversation, on foot, without wanted level, with no other conversation going
/// on and the player ready, up to 8 peds of the ped pool (scan index 0x969A3C) are tested: a random ped
/// (`m_nCreatedBy == 1`) that is ready, not flagged +0x474 bit 0x800000, on screen, with both top-direction dot
/// products positive, within 7.0, gets a topic (ped types 7, 9..16: 8..9; else 0..6, so topic 7 never comes up) it
/// will chat about; the topic decides an opening (`p2p_topic_opening`); an opening starts the conversation (state 1,
/// the ped referenced, speech disabled, both peds look at each other); a skipped ped says 0x58 once in 4096
/// (`rand() & 0xFFF == 3`, return address 0x43B953) and ends the scan. State 1: the player's answer
/// (`p2p_state_opened`); state 2: the ped's final word after 3 s when not talking; state 3: ends after 2.5 s or
/// without a ped. The three aggressive responses build a `CEventScriptCommand` on the original's stack (a zeroed
/// 0x18-byte local here).
#[hook(addr = 0x43B0F0, conv = cdecl, name = "CPedToPlayerConversations::Update", expect = "64 A1 00 00 00 00")]
pub fn p2p_update() {
    let player = find_player_ped(-1);
    if !find_player_vehicle(-1, 0).is_null() {
        let v = find_player_vehicle(-1, 0);
        P2P_LAST_PLAYER_VEHICLE.set(v);
        entity_register_reference(v, P2P_LAST_PLAYER_VEHICLE.ptr().cast());
    }
    match P2P_STATE.get() {
        0 => p2p_state_inactive(player),
        1 => p2p_state_opened(player),
        2 => {
            let p = P2P_PED.get();
            if p.is_null() {
                p2p_null_ped(player);
                return;
            }
            if now() <= P2P_START_TIME.get().wrapping_add(3000) || ped_get_ped_talking(p) != 0 {
                return;
            }
            ped_say(P2P_PED.get(), 0x85, 0, ONE_F, 1, 0, 0);
            p2p_wait_to_finish();
        }
        3 if now() > P2P_START_TIME.get().wrapping_add(2500) || P2P_PED.get().is_null() => {
            p2p_end_conversation();
        }
        _ => {}
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_P2P_CLEAR,
    HOOK_CONV_IS_PLAYER_IN_POSITION_FOR_CONVERSATION,
    HOOK_CONV_FOR_PED_UPDATE,
    HOOK_P2P_UPDATE,
];
