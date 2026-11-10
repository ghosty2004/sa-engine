//! `CGangs` and `CGangWars` (PORT-POP), the leaf part. Ported from the 1.0 US bytes; gta-reversed
//! `Gangs.cpp`, `GangWars.cpp` and `docs/re/specs/POP/CGang*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{find_player_coors, jp5, rd, x};
use crate::population::{CURRENT_WORLD_ZONE, NUM_PEDS_IN_GROUP, PED_GROUPS, TRANSLATION_ARRAY};

sa_core::globals! {
    list GLOBALS;
    /// `CGangs::GangAttackWithCops` (`bool[10]`)
    pub static GANG_ATTACK_WITH_COPS @ 0xC091E0: [u8; 10] = [0; 10];
    /// `CGangs::Gang` (`CGangInfo[10]`, 0x10 each: +0 ped model override, +4/+8/+0xC weapons)
    pub static GANG @ 0xC091F0: [u8; 0xA0] = [0; 0xA0];
    /// `CGangWars::ZoneInfoForTraining`
    pub static ZONE_INFO_FOR_TRAINING @ 0x8A5F40: i32 = -1;
    /// `CGangWars::State2` (attack state)
    pub static STATE2 @ 0x96AB4C: i32 = 0;
    /// `CGangWars::Provocation`
    pub static PROVOCATION @ 0x96AB5C: f32 = 0.0;
    /// `CGangWars::State`
    pub static STATE @ 0x96AB64: i32 = 0;
    /// `CGangWars::aSpecificZones`
    pub static SPECIFIC_ZONES @ 0x96AB68: [i32; 6] = [0; 6];
    /// `CGangWars::Gang1`
    pub static GANG1 @ 0x96AB84: i32 = 0;
    /// `CGangWars::bGangWarsActive`
    pub static GANG_WARS_ACTIVE @ 0x96AB90: u8 = 0;
    /// `CGangWars::bTrainingMission`
    pub static TRAINING_MISSION @ 0x96AB91: u8 = 0;
    /// `CGangWars::NumSpecificZones`
    pub static NUM_SPECIFIC_ZONES @ 0x96AB94: i32 = 0;
    /// `CGangWars::bIsPlayerOnAMission`
    pub static IS_PLAYER_ON_A_MISSION @ 0x96ABA0: u8 = 0;
    /// `CGangWars::PointOfAttack`
    pub static POINT_OF_ATTACK @ 0x96ABC8: [f32; 3] = [0.0; 3];
}

/// `m_TranslationArray[18 + gang]` row (the gang popcycle groups).
fn gang_translation(gang: i32, zone: i32) -> i32 {
    TRANSLATION_ARRAY
        .ptr()
        .cast::<i32>()
        .wrapping_index(54 + gang.wrapping_mul(3).wrapping_add(zone))
        .read()
}

fn peds_in_group(pg: i32) -> i32 {
    i32::from(NUM_PEDS_IN_GROUP.ptr().cast::<i16>().wrapping_index(pg).read())
}

fn ped_group_member(pg: i32, i: i32) -> i32 {
    i32::from(
        PED_GROUPS
            .ptr()
            .cast::<u16>()
            .wrapping_index(pg.wrapping_mul(21).wrapping_add(i))
            .read(),
    )
}

/// `CGangs::Initialise()` (0x5DE680): the default gang weapons (gang 3 untouched, as the bytes).
#[hook(addr = 0x5DE680, conv = cdecl, name = "CGangs::Initialise", expect = "33 C0 B9 16 00 00 00")]
pub fn initialise() {
    let g = GANG.ptr().cast::<u8>();
    let w = |off: isize, v: i32| crate::common::wr(g, off, v);
    w(0x04, 0x16);
    w(0x08, 0x1C);
    w(0x0C, 0);
    w(0x14, 0x16);
    w(0x18, 0);
    w(0x1C, 0);
    w(0x24, 0x16);
    w(0x28, 0);
    w(0x2C, 0);
    w(0x44, 0x16);
    w(0x48, 0x1C);
    w(0x4C, 0);
    w(0x54, 0x18);
    w(0x58, 0);
    w(0x5C, 0);
    w(0x64, 0x16);
    w(0x68, 0x1E);
    w(0x6C, 0);
    w(0x74, 0x16);
    w(0x78, 0x1C);
    w(0x7C, 0);
}

orig! {
    /// `CCarCtrl::InitSequence(int32)` (0x421740)
    pub fn init_sequence(n: i32) = cdecl 0x421740 => crate::carctrl::init_sequence;
    /// `CCarCtrl::FindSequenceElement(int32)` (0x421770)
    pub fn find_sequence_element(i: i32) -> i32 = cdecl 0x421770 => crate::carctrl::find_sequence_element;
    /// `CTheZones::FillZonesWithGangColours(bool)` (0x572440)
    pub fn fill_zones_with_gang_colours(disable: u8) = cdecl 0x572440 => sa_world::world::zones::fill_zones_with_gang_colours;
}

/// `CGangs::ChooseGangPedModel(int16 gang)` (0x5DE5A0): with the override byte set (not 0xFF), the first
/// member of the gang's ped group in the current world zone; else a random sequence over the zone-0
/// group size, picking the current zone's member, the first loaded one (`CStreaming` load state 1) or -1.
#[hook(addr = 0x5DE5A0, conv = cdecl, name = "CGangs::ChooseGangPedModel", expect = "55 56 0F BF 74 24 0C")]
pub fn choose_gang_ped_model(gang: i16) -> i32 {
    let gang = i32::from(gang);
    if rd::<u8>(GANG.ptr().cast(), (gang * 16) as isize) != 0xFF {
        let zone = CURRENT_WORLD_ZONE.get() as i32;
        return ped_group_member(gang_translation(gang, zone), 0);
    }
    init_sequence(peds_in_group(gang_translation(gang, 0)));
    let info = sa_stream::shared::INFO_FOR_MODEL.ptr().cast::<u8>();
    let mut i = 0;
    if peds_in_group(gang_translation(gang, 0)) > 0 {
        loop {
            let e = find_sequence_element(i);
            let zone = CURRENT_WORLD_ZONE.get() as i32;
            let mi = ped_group_member(gang_translation(gang, zone), e);
            if rd::<u8>(info, (mi * 0x14 + 0x10) as isize) == 1 {
                return mi;
            }
            i += 1;
            if i >= peds_in_group(gang_translation(gang, 0)) {
                break;
            }
        }
    }
    -1
}

/// `CGangWars::DontCreateCivilians()` (stub 0x4439C0): `State != 0`.
#[hook(addr = 0x4439C0, conv = cdecl, name = "CGangWars::DontCreateCivilians", expect = "E9 2B B6 12 01")]
pub fn dont_create_civilians() -> u8 {
    u8::from(STATE.get() != 0)
}

/// `CGangWars::InitAtStartOfGame()` (stub 0x443920).
#[hook(addr = 0x443920, conv = cdecl, name = "CGangWars::InitAtStartOfGame", expect = "E9 2B 78 12 01")]
pub fn init_at_start_of_game() {
    GANG_WARS_ACTIVE.set(0);
    STATE.set(0);
    STATE2.set(0);
    PROVOCATION.set(0.0);
    NUM_SPECIFIC_ZONES.set(0);
    IS_PLAYER_ON_A_MISSION.set(0);
}

/// `CGangWars::GangWarFightingGoingOn()` (stub 0x443AC0): `State != 0 || State2 == 2`.
#[hook(addr = 0x443AC0, conv = cdecl, name = "CGangWars::GangWarFightingGoingOn", expect = "E9 5B FB 11 01")]
pub fn gang_war_fighting_going_on() -> u8 {
    u8::from(STATE.get() != 0 || STATE2.get() == 2)
}

/// `CGangWars::ClearSpecificZonesToTriggerGangWar()` (stub 0x443FF0).
#[hook(addr = 0x443FF0, conv = cdecl, name = "CGangWars::ClearSpecificZonesToTriggerGangWar", expect = "E9 4B 9D 12 01")]
pub fn clear_specific_zones_to_trigger_gang_war() {
    NUM_SPECIFIC_ZONES.set(0);
    fill_zones_with_gang_colours(0);
}

/// `CGangWars::CanPlayerStartAGangWarHere(CZoneInfo*)` (stub 0x443F80): in a training mission only the
/// training zone info; otherwise any zone when no specific zones are set, else one of them.
#[hook(addr = 0x443F80, conv = cdecl, name = "CGangWars::CanPlayerStartAGangWarHere", expect = "E9 3B 5A 12 01")]
pub fn can_player_start_a_gang_war_here(zi: GPtr<u8>) -> u8 {
    let infos = sa_world::world::zones::ZONE_INFOS.ptr().cast::<u8>();
    if TRAINING_MISSION.get() != 0 {
        let t = infos.byte_add(ZONE_INFO_FOR_TRAINING.get().wrapping_mul(0x11) as isize);
        return u8::from(zi == t);
    }
    let n = NUM_SPECIFIC_ZONES.get();
    if n == 0 {
        return 1;
    }
    let nav = sa_world::world::zones::NAV_ZONES.ptr().cast::<u8>();
    let mut i = 0;
    while i < n {
        let z = SPECIFIC_ZONES.ptr().cast::<i32>().wrapping_index(i).read();
        let idx = rd::<u16>(nav, (z.wrapping_shl(5) + 0x1C) as isize);
        if zi == infos.byte_add(i32::from(idx).wrapping_mul(0x11) as isize) {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CGangWars::TellStreamingWhichGangsAreNeeded(uint32& mask)` (0x443D50): during an attack, within 150
/// (2D) of the point of attack, sets bit `Gang1`.
#[hook(addr = 0x443D50, conv = cdecl, name = "CGangWars::TellStreamingWhichGangsAreNeeded", expect = "A1 4C AB 96 00")]
pub fn tell_streaming_which_gangs_are_needed(mask: GPtr<u32>) {
    if STATE2.get() == 0 {
        return;
    }
    let mut v = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    let poa = POINT_OF_ATTACK.get();
    let dx = x(p.read()).fsub_m(poa[0]);
    let dy = x(p.wrapping_index(1).read()).fsub_m(poa[1]);
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
    if jp5(d, x(150.0)) {
        return;
    }
    let bit = 1u32.wrapping_shl(GANG1.get() as u32);
    mask.write(mask.read() | bit);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_CHOOSE_GANG_PED_MODEL,
    HOOK_DONT_CREATE_CIVILIANS,
    HOOK_INIT_AT_START_OF_GAME,
    HOOK_GANG_WAR_FIGHTING_GOING_ON,
    HOOK_CLEAR_SPECIFIC_ZONES_TO_TRIGGER_GANG_WAR,
    HOOK_CAN_PLAYER_START_A_GANG_WAR_HERE,
    HOOK_TELL_STREAMING_WHICH_GANGS_ARE_NEEDED,
];
