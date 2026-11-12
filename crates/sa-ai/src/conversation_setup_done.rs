//! `CConversations::DoneSettingUpConversation` (INTRO-2: the intro's first mission sets up a ped conversation).
//! Ported from the 1.0 US bytes; gta-reversed `Conversations.cpp` as a reading aid.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CConversations::m_SettingUpConversationNumNodes`, `m_pSettingUpConversationPed`, `m_bSettingUpConversation`
const NUM_TEMP_NODES: usize = 0x9691C8;
const SETUP_PED: usize = 0x9691CC;
const SETTING_UP: usize = 0x9691D0;
/// `CConversations::m_aConversations` (14 x 0x1C; +8 the ped, the free test)
const CONVERSATIONS: usize = 0x9691D8;
/// `CConversations::m_aTempNodes` (0x2C each: +0 name, +8 yes name, +0x10 no name, +0x18 final index, +0x1C yes
/// index (i16), +0x1E no index (i16), +0x20 / +0x24 / +0x28 speech)
const TEMP_NODES: usize = 0x969360;
/// `CConversations::m_aNodes` (50 x 0x18: +0 name (8), +8 yes (i16), +0xA no (i16), +0xC / +0x10 / +0x14 speech)
const NODES: usize = 0x969570;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// `strcmp(a, b) == 0` as the inlined byte loop compares.
fn same_name(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let mut i = 0;
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

/// `CConversations::DoneSettingUpConversation(bool suppressSubtitles)` (0x43ADB0, cdecl).
///
/// Links the temp nodes: each node's yes / no index (+0x1C / +0x1E, first -1) becomes the last node whose name
/// equals its yes / no name (+8 / +0x10). The conversation is the first of the 14 whose ped (+8) is null (none: null,
/// written through as the original does). Each temp node gets the first free node (empty name; it is marked "X"),
/// else 0, as its final index (+0x18); then each final node takes the temp node's name, the final indices of its
/// yes / no nodes (-1 for none) and its three speech dwords. The conversation: current and first node = the final
/// index of temp node 0, the ped (`RegisterReference`), start time = now, +0x10 = 0, +0x14 = 0, +0x18 = 1, +0x19 =
/// `suppressSubtitles`. The temp node count and the setting-up flag are cleared.
#[hook(addr = 0x43ADB0, conv = cdecl, name = "CConversations::DoneSettingUpConversation", expect = "51 53 8B 1D C8 91 96 00",
       unverified = "intro3-12", route = "L2", hazards = "heap-writes", recs = "intro")]
pub fn conversations_done_setting_up_conversation(suppress: u32) {
    let n = g::<i32>(NUM_TEMP_NODES).read();
    let temp = |i: i32| g::<u8>(TEMP_NODES).byte_add(i as isize * 0x2C);
    for i in 0..n.max(0) {
        let t = temp(i);
        wr::<u16>(t, 0x1E, 0xFFFF);
        wr::<u16>(t, 0x1C, 0xFFFF);
        let mut k = 0;
        while k < g::<i32>(NUM_TEMP_NODES).read() {
            if same_name(t.byte_add(8), temp(k)) {
                wr::<u16>(t, 0x1C, k as u16);
            }
            if same_name(t.byte_add(0x10), temp(k)) {
                wr::<u16>(t, 0x1E, k as u16);
            }
            k += 1;
        }
    }
    let mut conv = GPtr::<u8>::NULL;
    for c in 0..14 {
        let p = g::<u8>(CONVERSATIONS).byte_add(c * 0x1C);
        if rd::<u32>(p, 8) == 0 {
            conv = p;
            break;
        }
    }
    let n = g::<i32>(NUM_TEMP_NODES).read();
    if n > 0 {
        for i in 0..n {
            let mut f = 0u32;
            for c in 0..50u32 {
                let node = g::<u8>(NODES).byte_add(c as isize * 0x18);
                if rd::<u8>(node, 0) == 0 {
                    wr::<u8>(node, 0, b'X');
                    wr::<u8>(node, 1, 0);
                    f = c;
                    break;
                }
            }
            wr::<u32>(temp(i), 0x18, f);
        }
        let n = g::<i32>(NUM_TEMP_NODES).read();
        for i in 0..n.max(0) {
            let t = temp(i);
            let node = g::<u8>(NODES).byte_add(rd::<i32>(t, 0x18) as isize * 0x18);
            let mut j = 0;
            loop {
                let c = rd::<u8>(t, j);
                wr::<u8>(node, j, c);
                j += 1;
                if c == 0 {
                    break;
                }
            }
            for (from, to) in [(0x1C, 8), (0x1E, 0xA)] {
                let k = rd::<i16>(t, from);
                let v = if k < 0 { 0xFFFF } else { rd::<u16>(temp(i32::from(k)), 0x18) };
                wr::<u16>(node, to, v);
            }
            wr::<u32>(node, 0xC, rd::<u32>(t, 0x20));
            wr::<u32>(node, 0x10, rd::<u32>(t, 0x24));
            wr::<u32>(node, 0x14, rd::<u32>(t, 0x28));
        }
    }
    let first = rd::<u32>(temp(0), 0x18);
    wr::<u32>(conv, 0, first);
    wr::<u32>(conv, 4, first);
    let ped = g::<GPtr<u8>>(SETUP_PED).read();
    wr::<GPtr<u8>>(conv, 8, ped);
    entity_register_reference(ped, conv.byte_add(8));
    wr::<u32>(conv, 0xC, g::<u32>(TIME_MS).read());
    wr::<u32>(conv, 0x10, 0);
    wr::<u8>(conv, 0x18, 1);
    wr::<u8>(conv, 0x19, suppress as u8);
    wr::<u32>(conv, 0x14, 0);
    g::<i32>(NUM_TEMP_NODES).write(0);
    g::<u8>(SETTING_UP).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CONVERSATIONS_DONE_SETTING_UP_CONVERSATION];
