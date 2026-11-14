//! `CAEPedSpeechAudioEntity::GetCurrentCJMood` (0x4E53B0, batch `carjack-1`): the player's speech mood, asked when
//! CJ jacks a car and says something. Ported from the 1.0 US bytes (gta-reversed as a reading aid; the bytes win).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, cdecl, ST0)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader` (0x5F6AA0): a 16-bit result
    fn membership_count_excluding_leader(m: GPtr<u8>) -> u32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CAEPedSpeechAudioEntity::IsCJDressedInForGangSpeech()` (0x4E4270, static)
    fn is_cj_dressed_in_for_gang_speech() -> u8 = cdecl 0x4E4270 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// The well-dressed thresholds (stat 0x42 ≥ 650.0, stat 0xE ≥ 10000.0) and the fat ones (stat 0x15 ≥ 600.0, stat
/// 0x15 − 200.0 > stat 0x17).
const DRESSED_RESPECT: usize = 0x8C80F8;
const DRESSED_BUDGET: usize = 0x8C80FC;
const FAT_MIN: usize = 0x8C8100;
const FAT_MARGIN: usize = 0x8C8104;
/// `CPedGroups::ms_groups[0].m_groupMembership` (0x2D4 bytes per group)
const GROUP_MEMBERSHIPS: usize = 0xC09928;
/// The mood time stamp (−1 = none)
const MOOD_STAMP: usize = 0xA476A8;
/// The mood override: end time, basic mood (dword), gang-banging, fat, well-dressed (words, 0xFFFF = none)
const OVERRIDE_TIME: usize = 0xB613E0;
const OVERRIDE_MOOD: usize = 0xB613DC;
const OVERRIDE_GANG: usize = 0xB613D8;
const OVERRIDE_FAT: usize = 0xB613D4;
const OVERRIDE_DRESSED: usize = 0xB613D0;

fn stat(id: u32) -> X87 {
    X87::from_f64_exact(get_stat_value(id))
}

/// The player group's membership (`FindPlayerPed(-1)` → player data +0x480 → group index +0x38).
fn player_membership() -> GPtr<u8> {
    let data = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    GPtr::from_addr(GROUP_MEMBERSHIPS + rd::<u32>(data, 0x38).wrapping_mul(0x2D4) as usize)
}

/// `CAEPedSpeechAudioEntity::GetCurrentCJMood()` (0x4E53B0, static). No player (`FindPlayerPed(-1)`): 5. Well dressed:
/// stat 0x42 ≥ 650.0 and stat 0xE ≥ 10000.0 (`fcomp`, C0 clear). Gang-banging: more than one member besides the
/// leader (16-bit count, signed), or exactly one and any of the 8 member slots holds a ped of type 8 (+0x598) or with
/// speech audio type 3 (+0x326) and voice (+0x328) 0x1A, 0x25 or 0x20; or `IsCJDressedInForGangSpeech()`. Fat: stat
/// 0x15 ≥ 600.0 and `stat 0x15 − 200.0 > stat 0x17` (x87, the muscle stat stored as a float first). Basic mood: wanted
/// level (player data → wanted +0x2C) above 3 → 7, above 1 → 1, else 5 or 9: the stamp (0xA476A8) −1 → 5; the time
/// before the stamp → the stamp becomes the time, 5; else 9 within 180000 ms of the stamp, else 5. An override whose
/// end time is after now (unsigned) replaces the basic mood (whole dword) and the gang/fat/dressed flags (low bytes)
/// that are not 0xFFFF. Result: 1 → 0 gang else 1; 5 → 4 gang, 3 fat, 2 dressed, else 5; 7 → 6 gang else 7; 9 → 8
/// gang else 9; any other mood 5.
#[hook(addr = 0x4E53B0, conv = cdecl, name = "CAEPedSpeechAudioEntity::GetCurrentCJMood", expect = "83 EC 0C 56 57",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn get_current_cj_mood() -> u32 {
    let now = g::<u32>(TIME_MS).read();
    let player = find_player_ped(-1);
    if player.is_null() {
        return 5;
    }
    let ge = |c: Cmp| !matches!(c, Cmp::Less | Cmp::Unordered);
    let mut dressed = ge(stat(0x42).fcom(x(g::<f32>(DRESSED_RESPECT).read())))
        && ge(stat(0xE).fcom(x(g::<f32>(DRESSED_BUDGET).read())));
    let mut gang = false;
    let n = membership_count_excluding_leader(player_membership()) as u16 as i16;
    if n > 1 {
        gang = true;
    } else if n == 1 {
        let m = player_membership();
        for i in 0..8 {
            let p = membership_get_member(m, i);
            if p.is_null() {
                continue;
            }
            if rd::<u32>(p, 0x598) == 8 {
                gang = true;
                continue;
            }
            let voice = rd::<u16>(p, 0x328);
            if matches!(voice, 0x1A | 0x25 | 0x20) && rd::<u16>(p, 0x326) == 3 {
                gang = true;
            }
        }
    }
    if is_cj_dressed_in_for_gang_speech() != 0 {
        gang = true;
    }
    let mut fat = false;
    if ge(stat(0x15).fcom(x(g::<f32>(FAT_MIN).read()))) {
        let muscle = f32_of(stat(0x17));
        fat = stat(0x15).fsub_m(g::<f32>(FAT_MARGIN).read()).fcom(x(muscle)) == Cmp::Greater;
    }
    let data = rd::<GPtr<u8>>(player, 0x480);
    let wanted = if data.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(data, 0) };
    let level = rd::<i32>(wanted, 0x2C);
    let mut mood: u32 = if level > 3 {
        7
    } else if level > 1 {
        1
    } else {
        let stamp = g::<u32>(MOOD_STAMP).read();
        if stamp == u32::MAX {
            5
        } else if now < stamp {
            g::<u32>(MOOD_STAMP).write(now);
            5
        } else if now < stamp.wrapping_add(180_000) {
            9
        } else {
            5
        }
    };
    let (mut gang_b, mut fat_b, mut dressed_b) = (u8::from(gang), u8::from(fat), u8::from(dressed));
    if g::<u32>(OVERRIDE_TIME).read() > now {
        let m = g::<u32>(OVERRIDE_MOOD).read();
        if m as u16 != 0xFFFF {
            mood = m;
        }
        let w = g::<u16>(OVERRIDE_GANG).read();
        if w != 0xFFFF {
            gang_b = w as u8;
        }
        let w = g::<u16>(OVERRIDE_FAT).read();
        if w != 0xFFFF {
            fat_b = w as u8;
        }
        let w = g::<u16>(OVERRIDE_DRESSED).read();
        if w != 0xFFFF {
            dressed_b = w as u8;
        }
    }
    dressed = dressed_b != 0;
    let (gang, fat) = (gang_b != 0, fat_b != 0);
    match (mood as u16 as i16 as i32).wrapping_sub(1) {
        0 => u32::from(!gang),
        4 => {
            if gang {
                4
            } else if fat {
                3
            } else if dressed {
                2
            } else {
                5
            }
        }
        6 => {
            if gang {
                6
            } else {
                7
            }
        }
        8 => {
            if gang {
                8
            } else {
                9
            }
        }
        _ => 5,
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GET_CURRENT_CJ_MOOD];
