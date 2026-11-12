//! `CGangs`, `CGangWars`, `CDarkel`, `CTagManager`, `CTrafficLights`, `CRoadBlocks`, car generators,
//! `CAccident*` (PORT-POP-2). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as
//! reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jne_mask, jp5, rand_at, rd, wr, x, xi};
use crate::gangs::{GANG, GANG_ATTACK_WITH_COPS, GANG_WARS_ACTIVE, NUM_SPECIFIC_ZONES, PROVOCATION, SPECIFIC_ZONES};
use crate::misc::{NUM_TAGS, TAG_DESC};
use crate::population::TRANSLATION_ARRAY;

/// Globals owned by this module: none (they belong to the classes declared in gangs.rs and friends).
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CTheZones::FillZonesWithGangColours(bool)` (0x572440)
    fn fill_zones_with_gang_colours(b: u32) = cdecl 0x572440 => unported;
    /// `CGangWars::SetGangWarsActive(bool)` (0x446570)
    fn set_gang_wars_active_o(b: u32) = cdecl 0x446570 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CGangWars::EndGangWar(bool)` (0x4464C0)
    fn end_gang_war_o(b: u32) = cdecl 0x4464C0 => unported;
    /// `CGangWars::ReleasePedsInAttackWave(bool, bool)` (0x445C30)
    fn release_peds_in_attack_wave(a: u32, b: u32) -> i32 = cdecl 0x445C30 => unported;
    /// `CGangWars::MakeEnemyGainInfluenceInZone(int32 gang, int32 amount)` (0x445FD0)
    fn make_enemy_gain_influence_in_zone_o(gang: i32, amount: i32) = cdecl 0x445FD0 => unported;
    /// `CGangWars::ReleaseCarsInAttackWave()` (0x445E20)
    fn release_cars_in_attack_wave_o() = cdecl 0x445E20 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70)
    fn vehicle_set_created_by(veh: GPtr<u8>, by: i32) = thiscall 0x6D5D70 => unported;
}

/// `CGangWars::TimeTillNextAttack` (0x96AB48)
const TIME_TILL_NEXT_ATTACK: usize = 0x96AB48;
/// `CGangWars::State2` (attack state, 0x96AB4C)
const STATE2: usize = 0x96AB4C;
/// `CGangWars::State` (0x96AB64)
const STATE: usize = 0x96AB64;
/// `CGangWars::Gang1` (0x96AB84)
const GANG1: usize = 0x96AB84;
/// `CGangWars::pZoneInfoToFightOver` (0x96AB8C)
const ZONE_INFO_TO_FIGHT_OVER: usize = 0x96AB8C;
/// `CGangWars::bPlayerIsCloseby` area byte read by AddKillToProvocation (0x96AB93)
const AKP_FLAG: usize = 0x96AB93;
/// `PickStreamedInPedForThisGang`'s rolling index (0x96ABD4)
const PICK_STREAMED_INDEX: usize = 0x96ABD4;
/// `CPopulation::m_nNumPedsInGroup` (i16 per group, 0xC0ECC0)
const NUM_PEDS_IN_GROUP: usize = 0xC0ECC0;
/// `CPopulation::m_PedGroups` (u16[21] per group, 0xC0F358)
const PED_GROUPS: usize = 0xC0F358;
/// `CStreaming::ms_aInfoForModel[0].m_nLoadState` (0x8E4CC0 + 0x10; 0x14 per model)
const MODEL_INFO_STREAMING: usize = 0x8E4CD0;

/// `rand() * (1/32767.5) * 0.9 * 1080000 + 648000` truncated by `_ftol` and stored back as an unsigned
/// float into `CGangWars::TimeTillNextAttack` (the 3 copies of `CalculateTimeTillNextAttack`'s body).
fn time_till_next_attack(r: i32) -> u64 {
    xi(r)
        .fmul_m(f32::from_bits(0x3800_0100))
        .fmul_m(f32::from_bits(0x3F66_6666))
        .fmul_m(1_080_000.0)
        .fadd_m(648_000.0)
        .ftol()
}

/// `fild eax; jge; fadd 2^32` (u32 → x87), stored with `fstp dword`.
fn store_u32_as_f32(addr: usize, v: u32) {
    let mut f = xi(v as i32);
    if (v as i32) < 0 {
        f = f.fadd_m(4_294_967_296.0);
    }
    g::<f32>(addr).write(f.to_f32());
}

fn gang_base(gang: i16) -> GPtr<u8> {
    GANG.ptr().cast::<u8>().byte_add(isize::from(gang) << 4)
}

/// `CGangs::SetWillAttackPlayerWithCops(ePedType, bool)` (0x5DE4E0): ped types 7..16 only.
#[hook(addr = 0x5DE4E0, conv = cdecl, name = "CGangs::SetWillAttackPlayerWithCops", expect = "8B 44 24 04 83 F8 07")]
pub fn set_will_attack_player_with_cops(pt: i32, b: u8) {
    if (7..=16).contains(&pt) {
        GANG_ATTACK_WITH_COPS.ptr().cast::<u8>().wrapping_index(pt - 7).write(b);
    }
}

/// `CGangs::GetWillAttackPlayerWithCops(ePedType)` (0x5DE500).
#[hook(addr = 0x5DE500, conv = cdecl, name = "CGangs::GetWillAttackPlayerWithCops", expect = "8B 44 24 04 83 F8 07")]
pub fn get_will_attack_player_with_cops(pt: i32) -> u8 {
    if (7..=16).contains(&pt) {
        GANG_ATTACK_WITH_COPS.ptr().cast::<u8>().wrapping_index(pt - 7).read()
    } else {
        0
    }
}

/// `CGangInfo::CGangInfo()` (0x5DE520): model override 0xFF, weapons 0; returns `this`.
#[hook(addr = 0x5DE520, conv = thiscall, name = "CGangInfo", expect = "8B C1 33 C9 C6 00 FF")]
pub fn gang_info_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0, 0xFF);
    wr::<i32>(this, 4, 0);
    wr::<i32>(this, 8, 0);
    wr::<i32>(this, 0xC, 0);
    this
}

/// `CGangs::SetGangWeapons(int16 gang, int32, int32, int32)` (0x5DE550): negative weapons are kept.
#[hook(addr = 0x5DE550, conv = cdecl, name = "CGangs::SetGangWeapons", expect = "0F BF 44 24 04")]
pub fn set_gang_weapons(gang: i16, w1: i32, w2: i32, w3: i32) {
    let p = gang_base(gang);
    if w1 >= 0 {
        wr(p, 4, w1);
    }
    if w2 >= 0 {
        wr(p, 8, w2);
    }
    if w3 >= 0 {
        wr(p, 0xC, w3);
    }
}

/// `CGangs::SetGangPedModelOverride(int16 gang, int8)` (0x5DE580).
#[hook(addr = 0x5DE580, conv = cdecl, name = "CGangs::SetGangPedModelOverride", expect = "0F BF 44 24 04")]
pub fn set_gang_ped_model_override(gang: i16, v: u8) {
    wr(gang_base(gang), 0, v);
}

/// `__init_CGangs__Gang` (0x850CE0): `eh vector constructor iterator(Gang, 0x10, 10, CGangInfo ctor, dtor)`,
/// `atexit(__dtor_CGangs__Gang)`.
#[hook(addr = 0x850CE0, conv = cdecl, name = "__init_CGangs__Gang", expect = "68 40 E5 5D 00")]
pub fn init_gangs_gang() {
    eh_vector_ctor(GANG.ptr().cast(), 0x10, 10, 0x5DE520, 0x5DE540);
    atexit(0x856760);
}

/// `__dtor_CGangs__Gang` (0x856760): `eh vector destructor iterator(Gang, 0x10, 10, ~CGangInfo)`.
#[hook(addr = 0x856760, conv = cdecl, name = "__dtor_CGangs__Gang", expect = "68 40 E5 5D 00")]
pub fn dtor_gangs_gang() {
    eh_vector_dtor(GANG.ptr().cast(), 0x10, 10, 0x5DE540);
}

/// `CGangWars::GangWarGoingOn()` (0x443AA0): `State != 0 || State2 != 0`, full EAX 0/1.
#[hook(addr = 0x443AA0, conv = cdecl, name = "CGangWars::GangWarGoingOn", expect = "A1 64 AB 96 00")]
pub fn gang_war_going_on() -> u32 {
    u32::from(g::<i32>(STATE).read() != 0 || g::<i32>(STATE2).read() != 0)
}

/// The "player controls the zone" test of a `CZoneInfo` (gang densities: +0 gang 0, +1 gang 1 (the
/// player's), +2 gang 2): `d1 != 0 && d1 >= d0 + d2`.
fn player_controls(z: GPtr<u8>) -> bool {
    let d1 = i32::from(rd::<u8>(z, 1));
    d1 != 0 && d1 >= i32::from(rd::<u8>(z, 2)) + i32::from(rd::<u8>(z, 0))
}

/// `CGangWars::DoesPlayerControlThisZone(CZoneInfo*)` (0x443AE0).
#[hook(addr = 0x443AE0, conv = cdecl, name = "CGangWars::DoesPlayerControlThisZone", expect = "8B 44 24 04 0F B6 48 01")]
pub fn does_player_control_this_zone(z: GPtr<u8>) -> u8 {
    u8::from(player_controls(z))
}

/// `CGangWars::CalculateTimeTillNextAttack()` (0x443DB0): returns `_ftol`'s EDX:EAX.
#[hook(addr = 0x443DB0, conv = cdecl, name = "CGangWars::CalculateTimeTillNextAttack", expect = "51 E8 68 DD 3D 00")]
pub fn calculate_time_till_next_attack() -> u64 {
    time_till_next_attack(rand_at(0x443DB6))
}

/// `CGangWars::SetSpecificZoneToTriggerGangWar(int32 zone)` (0x444010): appends the zone, then tail-calls
/// `FillZonesWithGangColours(false)`.
#[hook(addr = 0x444010, conv = cdecl, name = "CGangWars::SetSpecificZoneToTriggerGangWar", expect = "A1 94 AB 96 00")]
pub fn set_specific_zone_to_trigger_gang_war(zone: i32) {
    let n = NUM_SPECIFIC_ZONES.get();
    SPECIFIC_ZONES.ptr().cast::<i32>().wrapping_index(n).write(zone);
    NUM_SPECIFIC_ZONES.set(n.wrapping_add(1));
    fill_zones_with_gang_colours(0);
}

/// `CGangWars::SetGangWarsActive(bool)` (0x446570).
#[hook(addr = 0x446570, conv = cdecl, name = "CGangWars::SetGangWarsActive", expect = "A0 90 AB 96 00")]
pub fn set_gang_wars_active(b: u8) {
    if b == GANG_WARS_ACTIVE.get() {
        return;
    }
    fill_zones_with_gang_colours(u32::from(b == 0));
    let t = time_till_next_attack(rand_at(0x446591));
    store_u32_as_f32(TIME_TILL_NEXT_ATTACK, t as u32);
    if b == 0 {
        end_gang_war_o(0);
    }
    GANG_WARS_ACTIVE.set(b);
}

/// `CGangWars::SwitchGangWarsActive()` (0x4465F0).
#[hook(addr = 0x4465F0, conv = cdecl, name = "CGangWars::SwitchGangWarsActive", expect = "8A 15 90 AB 96 00")]
pub fn switch_gang_wars_active() {
    set_gang_wars_active_o(u32::from(GANG_WARS_ACTIVE.get() == 0));
}

/// `CGangWars::AddKillToProvocation(ePedType)` (0x443950): ped types 7 and 9; with specific zones, only
/// when one of them has a nonzero density of that type (byte `zoneinfo + type + 9`).
#[hook(addr = 0x443950, conv = cdecl, name = "CGangWars::AddKillToProvocation", expect = "A0 93 AB 96 00")]
pub fn add_kill_to_provocation(pt: i32) {
    if g::<u8>(AKP_FLAG).read() == 0 && is_player_on_a_mission() != 0 {
        return;
    }
    if pt != 7 && pt != 9 {
        return;
    }
    let n = NUM_SPECIFIC_ZONES.get();
    if n != 0 {
        let mut i = 0;
        loop {
            if i >= n {
                return;
            }
            let z = SPECIFIC_ZONES.ptr().cast::<i32>().wrapping_index(i).read();
            let info = i32::from(g::<u16>(0xBA37B4usize.wrapping_add((z << 5) as u32 as usize)).read());
            let d = g::<u8>((0xBA1DE9u32.wrapping_add((info * 17) as u32).wrapping_add(pt as u32)) as usize).read();
            if d != 0 {
                break;
            }
            i += 1;
        }
    }
    PROVOCATION.set(x(PROVOCATION.get()).fadd_m(1.0).to_f32());
}

fn gang_group(gang: i32) -> i32 {
    TRANSLATION_ARRAY.ptr().cast::<i32>().wrapping_index(54 + gang.wrapping_mul(3)).read()
}

fn group_count(grp: i32) -> i32 {
    i32::from(g::<i16>(NUM_PEDS_IN_GROUP.wrapping_add((grp * 2) as u32 as usize)).read())
}

/// `cmp byte [model*20 + 0x8E4CD0], 1` (0x443A06 / 0x443A68): the model's load state is LOADED.
fn model_loaded(model: u32) -> bool {
    g::<u8>(MODEL_INFO_STREAMING.wrapping_add((model * 20) as usize)).read() == 1
}

/// `CGangWars::PedStreamedInForThisGang(int32 gang)` (0x4439D0): any member of the gang's zone-0 group
/// loaded.
#[hook(addr = 0x4439D0, conv = cdecl, name = "CGangWars::PedStreamedInForThisGang", expect = "8B 44 24 04 8D 04 40",
       unverified = "popw4-partial-1", route = "L2", hazards = "none",
       note = "model_loaded fixed (load state at 0x8E4CD0 + model * 20, not +0x10); re-verify")]
pub fn ped_streamed_in_for_this_gang(gang: i32) -> u8 {
    let grp = gang_group(gang);
    let n = group_count(grp);
    for i in 0..n {
        let m = g::<u16>(PED_GROUPS.wrapping_add((grp * 0x2A + i * 2) as u32 as usize)).read();
        if model_loaded(u32::from(m)) {
            return 1;
        }
    }
    0
}

/// `CGangWars::PickStreamedInPedForThisGang(int32 gang, int32* outModel)` (0x443A20): walks the group from
/// the rolling index + 1, writing every candidate to `*out`.
#[hook(addr = 0x443A20, conv = cdecl, name = "CGangWars::PickStreamedInPedForThisGang", expect = "8B 44 24 04 53",
       unverified = "popw4-partial-1", route = "L2", hazards = "none",
       note = "model_loaded fixed (load state at 0x8E4CD0 + model * 20, not +0x10); re-verify")]
pub fn pick_streamed_in_ped_for_this_gang(gang: i32, out: GPtr<i32>) -> u8 {
    let mut grp = gang_group(gang);
    let mut n = group_count(grp);
    if n <= 0 {
        return 0;
    }
    let mut idx = g::<i32>(PICK_STREAMED_INDEX).read();
    let mut k = 0;
    loop {
        idx = idx.wrapping_add(1).wrapping_rem(n);
        let m = g::<u16>(PED_GROUPS.wrapping_add((grp.wrapping_mul(21).wrapping_add(idx) * 2) as u32 as usize)).read();
        out.write(i32::from(m));
        if model_loaded(u32::from(m)) {
            g::<i32>(PICK_STREAMED_INDEX).write(idx);
            return 1;
        }
        grp = gang_group(gang);
        n = group_count(grp);
        k += 1;
        if k >= n {
            break;
        }
    }
    g::<i32>(PICK_STREAMED_INDEX).write(idx);
    0
}

/// `CGangWars::MakeEnemyGainInfluenceInZone(int32 gang, int32 amount)` (0x445FD0).
#[hook(addr = 0x445FD0, conv = cdecl, name = "CGangWars::MakeEnemyGainInfluenceInZone", expect = "A1 8C AB 96 00")]
pub fn make_enemy_gain_influence_in_zone(gang: i32, amount: i32) {
    let z = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
    let before = player_controls(z);
    let sum = i32::from(rd::<u8>(z, 0)) + i32::from(rd::<u8>(z, 1)) + i32::from(rd::<u8>(z, 2));
    if sum == 0 {
        return;
    }
    let p = z.byte_add(gang as isize);
    p.write(p.read().wrapping_add(amount as u8));
    if !before {
        return;
    }
    let z = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
    if !player_controls(z) {
        increment_stat(0xEB, 1.0);
    }
}

/// `CGangWars::MakePlayerGainInfluenceInZone(float removal)` (0x445E80): every other gang's density times
/// `1 - removal` (below 4 → 0), the difference moved to the player's gang; returns (full EAX) whether no
/// enemy density is left.
#[hook(addr = 0x445E80, conv = cdecl, name = "CGangWars::MakePlayerGainInfluenceInZone", expect = "83 EC 08 55 57")]
pub fn make_player_gain_influence_in_zone(removal: f32) -> u32 {
    let mut z = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
    let before = player_controls(z);
    let mut acc: i32 = 0;
    for i in 0..10isize {
        if i == 1 {
            continue;
        }
        let b = rd::<u8>(z, i);
        let v = x(1.0).fsub_m(removal).fmul(xi(i32::from(b)));
        wr::<u8>(z, i, v.ftol() as u8);
        let mut z2 = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
        if rd::<u8>(z2, i) < 4 {
            wr::<u8>(z2, i, 0);
            z2 = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
        }
        let c = rd::<u8>(z2, i);
        wr::<u8>(z2, 1, rd::<u8>(z2, 1).wrapping_add(b.wrapping_sub(c)));
        z = g::<GPtr<u8>>(ZONE_INFO_TO_FIGHT_OVER).read();
        acc = acc.wrapping_add(i32::from(rd::<u8>(z, i)));
    }
    if !before && player_controls(z) {
        increment_stat(0xEA, 1.0);
    }
    u32::from(acc == 0)
}

/// `CGangWars::ReleaseCarsInAttackWave()` (0x445E20): vehicles (pool walked downwards) with +0x42D bit 3
/// lose it and become random vehicles (`SetVehicleCreatedBy(1)`).
#[hook(addr = 0x445E20, conv = cdecl, name = "CGangWars::ReleaseCarsInAttackWave", expect = "53 8B 1D 94 44 B7 00")]
pub fn release_cars_in_attack_wave() {
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flags = rd::<GPtr<u8>>(pool, 4);
        i -= 1;
        if (flags.wrapping_index(i).read() as i8) < 0 {
            continue;
        }
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add((i as isize).wrapping_mul(0xA18));
        if veh.is_null() {
            continue;
        }
        let b = rd::<u8>(veh, 0x42D);
        if b & 8 != 0 {
            wr::<u8>(veh, 0x42D, b & 0xF7);
            vehicle_set_created_by(veh, 1);
        }
    }
}

/// `CGangWars::EndGangWar(bool)` (0x4464C0; its first read of `State2` goes through an obfuscated pointer
/// (`[0x451F3F] ^ [0x156063C]` = 0x96AB4C).
// Unhookable: its entry bytes (0x4464C1..) are a code key read by protection code at 0x40589D (`sub edi, [0x4464C1]`; sa-hook keycells test).
// A phase B native (STD-8).
#[sa_core::native(addr = 0x4464C0, conv = cdecl, name = "CGangWars::EndGangWar")]
pub fn end_gang_war(b: u32) {
    let s2 = g::<i32>(STATE2).read();
    g::<i32>(STATE).write(0);
    if s2 != 0 {
        g::<i32>(STATE2).write(0);
        let t = time_till_next_attack(rand_at(0x4464E3));
        store_u32_as_f32(TIME_TILL_NEXT_ATTACK, t as u32);
        let n = release_peds_in_attack_wave(1, 0);
        make_enemy_gain_influence_in_zone_o(g::<i32>(GANG1).read(), n.wrapping_mul(3));
    }
    PROVOCATION.set(0.0);
    fill_zones_with_gang_colours(0);
    release_peds_in_attack_wave(1, b);
    release_cars_in_attack_wave_o();
}

orig! {
    /// `CStats::SetStatValue(uint16, float)` (0x55A070)
    fn set_stat_value(stat: u32, v: f32) = cdecl 0x55A070 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: u32) -> u32 = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageQ(const char*, uint32, uint16, bool)` (0x69F0B0)
    fn add_message_q(text: u32, time: u32, flag: u32, b: u32) = cdecl 0x69F0B0 => unported;
    /// `CMessages::AddToPreviousBriefArray(const char*, int32 x6, const char*)` (0x69DD50)
    #[allow(clippy::too_many_arguments)]
    fn add_to_previous_brief_array(text: u32, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, s: u32) = cdecl 0x69DD50 => unported;
    /// `CGangWars::TellGangMembersTo(bool)` (0x444530)
    fn tell_gang_members_to(b: u32) = cdecl 0x444530 => unported;
    /// `CGangWars::CheerVictory()` (0x444040)
    fn cheer_victory() = cdecl 0x444040 => unported;
    /// `CGangWars::ReleaseCarsInAttackWave()` (0x445E20)
    fn release_cars_in_attack_wave_o2() = cdecl 0x445E20 => unported;
}

/// `CGangWars::UpdateTerritoryUnderControlPercentage()` (0x443DE0): counts the navigation zones the player's
/// gang controls (`d1 >= d0 + d2`) or gang 0 / gang 2 lead; stats 0xEC (current) and 0xED (maximum);
/// `TerritoryUnderControlPercentage` (0x96AB9C); the three (count, gang) pairs at 0x96AB2C / 0x96AB38
/// sorted by count, descending, with the bytes' bubble sort (signed compares, restart after the second swap).
#[hook(addr = 0x443DE0, conv = cdecl, name = "CGangWars::UpdateTerritoryUnderControlPercentage", expect = "83 EC 0C 0F B7 05 94 37 BA 00")]
pub fn update_territory_under_control_percentage() {
    let n = i32::from(g::<u16>(0xBA3794).read());
    let (mut player, mut a, mut b) = (0i32, 0i32, 0i32);
    for j in 0..n {
        let zi = g::<u16>((0xBA37B4 + j * 0x20) as usize).read();
        if zi == 0 {
            continue;
        }
        let info = (0xBA1DF0 + u32::from(zi) * 17) as usize;
        let d0 = g::<u8>(info).read();
        let d1 = i32::from(g::<u8>(info + 1).read());
        let d2 = g::<u8>(info + 2).read();
        let s = i32::from(d0) + i32::from(d2);
        if s == 0 && d1 == 0 {
            continue;
        }
        if s <= d1 {
            player += 1;
        } else if d0 > d2 {
            a += 1;
        } else {
            b += 1;
        }
    }
    let f = xi(player).to_f32();
    set_stat_value(0xEC, f);
    let m = if crate::common::jp5(X87::from_f64_exact(get_stat_value(0xED)), x(f)) {
        get_stat_value(0xED) as f32
    } else {
        f
    };
    set_stat_value(0xED, m);
    let total = b.wrapping_add(a).wrapping_add(player);
    if total == 0 {
        g::<i32>(0x96AB9C).write(0);
        return;
    }
    g::<f32>(0x96AB9C).write(xi(total).fdivr_m(f).to_f32());
    // counts (esi, eax, ebx) and gangs (edi, ecx, ebp) as the bytes keep them
    let (mut c0, mut c1, mut c2) = (player, a, b);
    let (mut g0, mut g1, mut g2) = (1i32, 0i32, 2i32);
    loop {
        let mut swapped = false;
        if c0 < c1 {
            core::mem::swap(&mut c0, &mut c1);
            core::mem::swap(&mut g0, &mut g1);
            swapped = true;
        }
        if c1 < c2 {
            core::mem::swap(&mut c1, &mut c2);
            core::mem::swap(&mut g1, &mut g2);
            continue;
        }
        if !swapped {
            break;
        }
    }
    for (k, (c, gg)) in [(c0, g0), (c1, g1), (c2, g2)].into_iter().enumerate() {
        g::<i32>(0x96AB2C + 4 * k).write(c);
        g::<i32>(0x96AB38 + 4 * k).write(gg);
    }
}

/// `CGangWars::ClearTheStreets()` (stub 0x4444B0 → 0x1564200): every non-player ped of type 4 or 5 (pool
/// walked downwards) with an active task 0x390 gets its +0xC set to 7.
#[hook(addr = 0x4444B0, conv = cdecl, name = "CGangWars::ClearTheStreets", expect = "E9 4B FD 11 01")]
pub fn clear_the_streets() {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flags = rd::<GPtr<u8>>(pool, 4);
        i -= 1;
        if (flags.wrapping_index(i).read() as i8) < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add((i as isize).wrapping_mul(0x7C4));
        if ped.is_null() || ped_is_player(ped) != 0 {
            continue;
        }
        let pt = rd::<i32>(ped, 0x598);
        if pt != 4 && pt != 5 {
            continue;
        }
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        let t = find_active_task_by_type(tm, 0x390);
        if !t.is_null() {
            wr::<i32>(t, 0xC, 7);
        }
    }
}

/// `CGangWars::DoStuffWhenPlayerVictorious()` (0x446400): release the wave, cheer, the "territory taken"
/// message (key 0x85992C) and brief, gang members told to (1), stat 0x44 += 45, colours refilled; the next
/// attack 240 s sooner but at least after 30 s.
#[hook(addr = 0x446400, conv = cdecl, name = "CGangWars::DoStuffWhenPlayerVictorious", expect = "6A 00 6A 01 E8 27 F8 FF FF")]
pub fn do_stuff_when_player_victorious() {
    release_peds_in_attack_wave(1, 0);
    release_cars_in_attack_wave_o2();
    cheer_victory();
    g::<i32>(STATE).write(0);
    let t = text_get(g(0xC1B340), 0x85992C);
    add_message_q(t, 0x1194, 1, 1);
    let t = text_get(g(0xC1B340), 0x85992C);
    add_to_previous_brief_array(t, -1, -1, -1, -1, -1, -1, 0);
    PROVOCATION.set(0.0);
    tell_gang_members_to(1);
    increment_stat(0x44, 45.0);
    fill_zones_with_gang_colours(0);
    let v = x(g::<f32>(TIME_TILL_NEXT_ATTACK).read()).fsub_m(240_000.0);
    if crate::common::jne_mask(v, x(30_000.0), 0x41) {
        g::<f32>(TIME_TILL_NEXT_ATTACK).write(30_000.0);
    } else {
        g::<f32>(TIME_TILL_NEXT_ATTACK).write(v.to_f32());
    }
}

// ---- CDarkel ---------------------------------------------------------------------------------------

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPedGroupMembership::IsMember(CPed*)` (0x5F6A10)
    fn group_is_member(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CStats::DecrementStat(uint16, float)` (0x559FA0)
    fn decrement_stat(stat: u32, v: f32) = cdecl 0x559FA0 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980)
    fn display_script_stat_update_message(a: u32, stat: u32, v: f32) = cdecl 0x55B980 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(int32, float, float)` (0x506EA0)
    fn report_frontend_audio_event(this: GPtr<u8>, ev: i32, vol: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn is_law_enforcement_vehicle(veh: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CDarkel::ThisVehicleShouldBeKilledForFrenzy(CVehicle*)` (0x43D350)
    fn this_vehicle_should_be_killed_for_frenzy(veh: GPtr<u8>) -> u8 = cdecl 0x43D350 => unported;
    /// `CPlayerPed::SetWantedLevel(int32)` (0x609F10)
    fn set_wanted_level(ped: GPtr<u8>, lvl: i32) = thiscall 0x609F10 => unported;
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0)
    fn set_help_message(text: u32, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CDarkel::DealWithWeaponChangeAtEndOfFrenzy()` (0x43D7A0)
    fn deal_with_weapon_change_at_end_of_frenzy() = cdecl 0x43D7A0 => unported;
}

/// `CDarkel::Status` (int16)
const D_STATUS: usize = 0x96A704;
/// `CDarkel::TimeOfFrenzyStart`
const D_START: usize = 0x96A6E0;
/// `CDarkel::PreviousTime` (seconds shown)
const D_PREV: usize = 0x96A6E4;
/// `CDarkel::TimeLimit`
const D_LIMIT: usize = 0x96A6E8;
/// `CDarkel::KillsNeeded`
const D_KILLS_NEEDED: usize = 0x96A6EC;
/// `CDarkel::ModelToKill[4]` (in the order the bytes test them)
const D_MODELS: [usize; 4] = [0x96A6FC, 0x96A6F8, 0x96A6F4, 0x96A6F0];
/// `CDarkel::RegisteredKills[800][2]` (int16)
const D_KILLS: usize = 0x969A50;
/// `CPopulation::m_AllRandomPedsThisType`
const ALL_RANDOM_PEDS_THIS_TYPE: usize = 0x8D2534;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;

fn d_status() -> u16 {
    g::<u16>(D_STATUS).read()
}

/// `CDarkel::CalcFade(uint32 t, uint32 begin, uint32 end)` (0x43D740): 0 outside, ramps over 500 ms at both
/// ends, else 0xFF (AL).
#[hook(addr = 0x43D740, conv = cdecl, name = "CDarkel::CalcFade", expect = "8B 4C 24 04 8B 44 24 08")]
pub fn calc_fade(t: u32, begin: u32, end: u32) -> u8 {
    if t < begin || t > end {
        return 0;
    }
    if t < begin.wrapping_add(500) {
        return (t.wrapping_sub(begin).wrapping_mul(255) / 500) as u8;
    }
    if t > end.wrapping_sub(500) {
        return (end.wrapping_sub(t).wrapping_mul(255) / 500) as u8;
    }
    0xFF
}

/// `CDarkel::CheckDamagedWeaponType(int32 weapon, int32 type)` (0x43D9E0): type 56 (melee) matches weapons
/// 0..15; type 57 (any weapon) matches 0..45 and 51..52 (the jump tables 0x43DA30 / 0x43DA78).
#[hook(addr = 0x43D9E0, conv = cdecl, name = "CDarkel::CheckDamagedWeaponType", expect = "8B 44 24 04 83 F8 FF", preserve = "edx")]
pub fn check_damaged_weapon_type(weapon: i32, ty: i32) -> u8 {
    if weapon == -1 {
        return 0;
    }
    let w = weapon as u32;
    u8::from(match ty {
        0x38 => w < 16,
        0x39 => w <= 45 || w == 51 || w == 52,
        _ => false,
    })
}

/// `CDarkel::FailKillFrenzy()` (0x43DC60).
#[hook(addr = 0x43DC60, conv = cdecl, name = "CDarkel::FailKillFrenzy", expect = "66 83 3D 04 A7 96 00 04")]
pub fn fail_kill_frenzy() {
    if d_status() == 4 {
        let now = g::<u32>(TIME_MS).read();
        g::<u8>(0x96A8B0).write(3);
        g::<u32>(0x96A8AC).write(now);
    }
    set_help_message(0, 1, 0, 0);
    let st = d_status();
    if st == 1 || st == 4 {
        let now = g::<u32>(TIME_MS).read();
        g::<u16>(D_STATUS).write(3);
        g::<i32>(ALL_RANDOM_PEDS_THIS_TYPE).write(-1);
        g::<u32>(D_START).write(now);
        deal_with_weapon_change_at_end_of_frenzy();
    }
}

fn kills_cell(player: i32, m: u32) -> GPtr<u16> {
    g::<u16>((D_KILLS as u32).wrapping_add(m.wrapping_mul(4)).wrapping_add((player as u32).wrapping_mul(2)) as usize)
}

/// `CDarkel::FindTotalPedsKilledByPlayer(int32 player)` (stub 0x43D6E0 → 0x1569900): the sum over 400 models.
#[hook(addr = 0x43D6E0, conv = cdecl, name = "CDarkel::FindTotalPedsKilledByPlayer", expect = "E9 1B C2 12 01")]
pub fn find_total_peds_killed_by_player(player: i32) -> i32 {
    let mut s: i32 = 0;
    for m in 0..400u32 {
        s = s.wrapping_add(i32::from(kills_cell(player, m).read() as i16));
    }
    s
}

/// `CDarkel::ResetModelsKilledByPlayer(int32 player)` (stub 0x43D6A0 → 0x1561630): 800 entries.
#[hook(addr = 0x43D6A0, conv = cdecl, name = "CDarkel::ResetModelsKilledByPlayer", expect = "E9 8B 3F 12 01")]
pub fn reset_models_killed_by_player(player: i32) {
    for m in 0..800u32 {
        kills_cell(player, m).write(0);
    }
}

/// `CDarkel::ThisPedShouldBeKilledForFrenzy(CPed*)` (stub 0x43D2F0 → 0x1567170).
#[hook(addr = 0x43D2F0, conv = cdecl, name = "CDarkel::ThisPedShouldBeKilledForFrenzy", expect = "E9 7B 9E 12 01")]
pub fn this_ped_should_be_killed_for_frenzy(ped: GPtr<u8>) -> u8 {
    let st = d_status();
    if st != 1 && st != 4 {
        return 0;
    }
    if g::<i32>(D_MODELS[0]).read() != -1 {
        let m = i32::from(rd::<i16>(ped, 0x22));
        if !D_MODELS.iter().any(|a| g::<i32>(*a).read() == m) {
            return 0;
        }
    }
    let pt = rd::<i32>(ped, 0x598);
    u8::from(pt != 0 && pt != 1)
}

/// `CDarkel::RegisterKillNotByPlayer(const CPed*)` (0x43D210).
#[hook(addr = 0x43D210, conv = cdecl, name = "CDarkel::RegisterKillNotByPlayer", expect = "56 6A FF E8 F8 0F 13 00")]
pub fn register_kill_not_by_player(ped: GPtr<u8>) {
    if ped != find_player_ped(-1) {
        let p = find_player_ped(-1);
        let grp = rd::<i32>(rd::<GPtr<u8>>(p, 0x480), 0x38);
        let m = g::<u8>((grp.wrapping_mul(0x2D4) as u32).wrapping_add(0xC09928) as usize);
        if group_is_member(m, ped) != 0 {
            decrement_stat(0x44, 2.0);
            increment_stat(0xA3, 1.0);
            display_script_stat_update_message(0, 0x150, 1.0);
        }
    }
    increment_stat(0x78, 1.0);
    let pt = rd::<i32>(ped, 0x598).wrapping_sub(7) as u32;
    if pt <= 9 {
        increment_stat(if pt == 1 { 0xF0 } else { 0xEF }, 1.0);
    }
}

/// `CDarkel::RegisterCarBlownUpByPlayer(CVehicle*, int32 player)` (0x43DF20).
#[hook(addr = 0x43DF20, conv = cdecl, name = "CDarkel::RegisterCarBlownUpByPlayer", expect = "56 8B 74 24 08 56")]
pub fn register_car_blown_up_by_player(veh: GPtr<u8>, player: i32) {
    if this_vehicle_should_be_killed_for_frenzy(veh) != 0 {
        let k = g::<i32>(D_KILLS_NEEDED);
        k.write(k.read().wrapping_sub(1));
        report_frontend_audio_event(g(AUDIO_ENGINE), 0x2B, 0.0, 1.0);
    }
    let model = i32::from(rd::<i16>(veh, 0x22));
    let c = kills_cell(player, model as u32);
    c.write(c.read().wrapping_add(1));
    match get_vehicle_appearance(veh).wrapping_sub(1) {
        0 | 1 => increment_stat(0x7A, 1.0),
        2 | 4 => increment_stat(0x7C, 1.0),
        3 => increment_stat(0x7B, 1.0),
        _ => {}
    }
    let st = d_status();
    if st == 1 || st == 4 {
        if rd::<i16>(veh, 0x22) == 0x1B0 {
            increment_stat(0xD1, 1.0);
        } else if is_law_enforcement_vehicle(veh) != 0 {
            increment_stat(0xD0, 1.0);
        } else {
            increment_stat(0xCF, 1.0);
        }
    }
}

/// `CDarkel::Update()` (0x43DAC0). After a time-out (status 3) the kills-needed test still runs and can end
/// the frenzy as a success too, as the bytes do.
#[hook(addr = 0x43DAC0, conv = cdecl, name = "CDarkel::Update", expect = "53 66 8B 1D 04 A7 96 00")]
pub fn darkel_update() {
    let mut bx = d_status();
    if bx != 1 && bx != 4 {
        return;
    }
    let mut now = g::<u32>(TIME_MS).read();
    let limit = g::<i32>(D_LIMIT).read();
    let rem = g::<i32>(D_START).read().wrapping_sub(now as i32).wrapping_add(limit);
    let timed_out = if rem > 0 || limit < 0 { bx == 4 && g::<i32>(0xB7CF28).read() == 0 } else { true };
    if timed_out {
        if bx == 4 {
            g::<u8>(0x96A8B0).write(3);
            g::<u32>(0x96A8AC).write(now);
        }
        g::<u16>(D_STATUS).write(3);
        g::<i32>(ALL_RANDOM_PEDS_THIS_TYPE).write(-1);
        g::<u32>(D_START).write(now);
        deal_with_weapon_change_at_end_of_frenzy();
        bx = d_status();
        now = g::<u32>(TIME_MS).read();
    } else {
        let secs = rem / 1000;
        let prev = g::<i32>(D_PREV).read();
        if secs != prev {
            if prev < 12 {
                report_frontend_audio_event(g(AUDIO_ENGINE), 0x21, 0.0, 1.0);
                bx = d_status();
                now = g::<u32>(TIME_MS).read();
            }
            g::<i32>(D_PREV).write(secs);
        }
    }
    if g::<i32>(D_KILLS_NEEDED).read() > 0 {
        return;
    }
    if bx == 4 {
        g::<u8>(0x96A8B0).write(bx as u8);
        g::<u32>(0x96A8AC).write(now);
    }
    g::<u16>(D_STATUS).write(2);
    g::<i32>(ALL_RANDOM_PEDS_THIS_TYPE).write(-1);
    if g::<u8>(0x969A48).read() != 0 {
        increment_stat(0xA8, 1.0);
        now = g::<u32>(TIME_MS).read();
    }
    g::<u32>(D_START).write(now);
    set_wanted_level(find_player_ped(-1), 0);
    deal_with_weapon_change_at_end_of_frenzy();
}

// ---- CTagManager -----------------------------------------------------------------------------------

orig! {
    /// `CVisibilityPlugins::GetUserValue(RpAtomic*)` (0x7323A0)
    fn get_user_value(atomic: GPtr<u8>) -> u32 = cdecl 0x7323A0 => unported;
    /// `CVisibilityPlugins::SetUserValue(RpAtomic*, uint16)` (0x732380)
    fn set_user_value(atomic: GPtr<u8>, v: u32) = cdecl 0x732380 => unported;
    /// `CGarages::TriggerMessage(const char*, int16, uint16, int16)` (0x447B80)
    fn garages_trigger_message(text: u32, n1: i32, time: u32, n2: i32) = cdecl 0x447B80 => unported;
    /// An `RpAtomic` render callback (`call [atomic + 0x48]`)
    fn atomic_render_cb(f: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

/// The tag entries: `{CEntity* entity; uint8 alpha; pad}` x 150 at 0xA9A8C0.
fn tag(i: i32) -> GPtr<u8> {
    TAG_DESC.ptr().cast::<u8>().byte_add((i as isize).wrapping_mul(8))
}

/// The entity position the tag loops use: the matrix position, or the placement's.
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

/// `CTagManager::FindTagDesc(CEntity*)` (0x49CCB0): searched from the last entry down.
#[hook(addr = 0x49CCB0, conv = cdecl, name = "CTagManager::FindTagDesc", expect = "A1 70 AD A9 00")]
pub fn find_tag_desc(e: GPtr<u8>) -> GPtr<u8> {
    let mut i = NUM_TAGS.get();
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(tag(i), 0) == e {
            return tag(i);
        }
    }
    GPtr::NULL
}

/// `CTagManager::GetAlpha(CEntity*)` (0x49CF90): the atomic's user value, else the entry's alpha (a
/// missing entry reads address 4, as the bytes).
#[hook(addr = 0x49CF90, conv = cdecl, name = "CTagManager::GetAlpha-Entity", expect = "8B 4C 24 04 8B 41 18")]
pub fn get_alpha_entity(e: GPtr<u8>) -> u8 {
    let a = rd::<GPtr<u8>>(e, 0x18);
    if !a.is_null() {
        return get_user_value(a) as u8;
    }
    rd::<u8>(find_tag_desc_inline(e), 4)
}

fn find_tag_desc_inline(e: GPtr<u8>) -> GPtr<u8> {
    let mut i = NUM_TAGS.get();
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(tag(i), 0) == e {
            return tag(i);
        }
    }
    GPtr::NULL
}

/// `CTagManager::GetNearestTag(const CVector&)` (0x49D160): 2D squared distance, strict `<`, last entry
/// first; with no tag it returns entry -1's entity (`[0xA9A8B8]`).
#[hook(addr = 0x49D160, conv = cdecl, name = "CTagManager::GetNearestTag", expect = "51 8B 0D 70 AD A9 00")]
pub fn get_nearest_tag(pos: GPtr<f32>) -> GPtr<u8> {
    let mut best_i: i32 = -1;
    let mut best = f32::MAX;
    let mut i = NUM_TAGS.get();
    while i != 0 {
        i -= 1;
        let p = entity_pos(rd::<GPtr<u8>>(tag(i), 0));
        let dx = x(pos.read()).fsub_m(p.read());
        let dy = x(pos.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
        let d = dy.fmul(dy).fadd(dx.fmul(dx));
        if !crate::common::jp5(d, x(best)) {
            best = d.to_f32();
            best_i = i;
        }
    }
    rd::<GPtr<u8>>(tag(best_i), 0)
}

/// The rectangle test of `GetPercentageTaggedInArea` / `SetAlphaInArea`: `r[0] <= x <= r[2]`,
/// `r[3] <= y <= r[1]` (x87 compares as the bytes, unordered passing the `<=` tests).
fn in_rect(p: GPtr<f32>, r: GPtr<f32>) -> bool {
    let px = x(p.read());
    if !crate::common::jp5(px, x(r.read())) {
        return false;
    }
    if !crate::common::jne_mask(px, x(r.wrapping_index(2).read()), 0x41) {
        return false;
    }
    let py = x(p.wrapping_index(1).read());
    if !crate::common::jp5(py, x(r.wrapping_index(3).read())) {
        return false;
    }
    crate::common::jne_mask(py, x(r.wrapping_index(1).read()), 0x41)
}

/// `CTagManager::GetPercentageTaggedInArea(CRect*)` (0x49D0B0): `_ftol(tagged / total * 100)` (EDX:EAX).
#[hook(addr = 0x49D0B0, conv = cdecl, name = "CTagManager::GetPercentageTaggedInArea", expect = "83 EC 0C 8B 0D 70 AD A9 00")]
pub fn get_percentage_tagged_in_area(r: GPtr<f32>) -> u64 {
    let (mut total, mut tagged) = (0i32, 0i32);
    let mut i = NUM_TAGS.get();
    while i != 0 {
        i -= 1;
        let p = entity_pos(rd::<GPtr<u8>>(tag(i), 0));
        if in_rect(p, r) {
            total += 1;
            if rd::<u8>(tag(i), 4) > 0xE4 {
                tagged += 1;
            }
        }
    }
    xi(tagged).fdiv(xi(total)).fmul_m(100.0).ftol()
}

/// `CTagManager::GetTagPos(int32)` (0x49CEA0).
#[hook(addr = 0x49CEA0, conv = cdecl, name = "CTagManager::GetTagPos", expect = "8B 44 24 04 8B 04 C5 C0 A8 A9 00")]
pub fn get_tag_pos(i: i32) -> GPtr<f32> {
    entity_pos(rd::<GPtr<u8>>(tag(i), 0))
}

/// `CTagManager::UpdateNumTagged()` (stub 0x49CDE0 → 0x156B330): entries with alpha > 0xE4.
#[hook(addr = 0x49CDE0, conv = cdecl, name = "CTagManager::UpdateNumTagged", expect = "E9 4B E5 0C 01")]
pub fn update_num_tagged() {
    count_tagged();
}

fn count_tagged() -> i32 {
    let mut n = 0;
    crate::misc::NUM_TAGGED.set(0);
    let mut i = NUM_TAGS.get();
    if i == 0 {
        return 0;
    }
    while i != 0 {
        i -= 1;
        if rd::<u8>(tag(i), 4) > 0xE4 {
            n += 1;
        }
    }
    crate::misc::NUM_TAGGED.set(n);
    n
}

/// `CTagManager::SetAlpha(RpAtomic*, uint8)` (0x49CD30): tail call of `SetUserValue` (the original stores
/// EAX with `movzx ax` over the argument slot; the callee reads the word).
#[hook(addr = 0x49CD30, conv = cdecl, name = "CTagManager::SetAlpha-RpAtomic", expect = "66 0F B6 44 24 08")]
pub fn set_alpha_atomic(atomic: GPtr<u8>, a: u8) {
    set_user_value(atomic, u32::from(a));
}

/// `CTagManager::SetAlpha(CEntity*, uint8)` (0x49CEC0): sets the atomic's user value and the entry's
/// alpha; when the alpha crosses 0xE4 upwards (and no other message is shown, 0xB6F065) a garage-style
/// message reports the count ("TAG_ALL" 0x85A5A4 / "TAG_UPD" 0x85A59C).
#[hook(addr = 0x49CEC0, conv = cdecl, name = "CTagManager::SetAlpha-Entity", expect = "51 53 8A 5C 24 10")]
pub fn set_alpha_entity(e: GPtr<u8>, a: u8) {
    let atomic = rd::<GPtr<u8>>(e, 0x18);
    if !atomic.is_null() {
        set_user_value(atomic, u32::from(a));
    }
    let n = NUM_TAGS.get();
    let d = find_tag_desc_inline(e);
    let mut became = false;
    if a > 0xE4 && rd::<u8>(d, 4) <= 0xE4 {
        became = true;
    }
    wr::<u8>(d, 4, a);
    let tagged = count_tagged();
    if became && g::<u8>(0xB6F065).read() == 0 {
        if tagged == n {
            garages_trigger_message(0x85A5A4, -1, 5000, -1);
        } else {
            garages_trigger_message(0x85A59C, tagged, 5000, n);
        }
    }
}

/// `CTagManager::SetAlphaInArea(CRect*, uint8)` (0x49CFE0).
#[hook(addr = 0x49CFE0, conv = cdecl, name = "CTagManager::SetAlphaInArea", expect = "83 EC 08 53 56")]
pub fn set_alpha_in_area(r: GPtr<f32>, a: u8) {
    let mut i = NUM_TAGS.get();
    while i != 0 {
        i -= 1;
        let e = rd::<GPtr<u8>>(tag(i), 0);
        if in_rect(entity_pos(e), r) {
            let atomic = rd::<GPtr<u8>>(e, 0x18);
            if !atomic.is_null() {
                set_user_value(atomic, u32::from(a));
            }
            wr::<u8>(tag(i), 4, a);
        }
    }
    count_tagged();
}

/// `CTagManager::RenderTagForPC(RpAtomic*)` (0x49CE40): the first material's colour alpha becomes
/// `(uint8)uv * (uint16)uv / 255` (kept in the material), then the atomic's render callback.
#[hook(addr = 0x49CE40, conv = cdecl, name = "CTagManager::RenderTagForPC", expect = "53 56 8B 74 24 0C")]
pub fn render_tag_for_pc(atomic: GPtr<u8>) {
    let mat = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x18), 0x20), 4);
    let col = rd::<u32>(mat, 4);
    let a = get_user_value(atomic) & 0xFF;
    let b = get_user_value(atomic) & 0xFFFF;
    let v = (a.wrapping_mul(b) as i32) / 255;
    wr::<u32>(mat, 4, (col & 0x00FF_FFFF) | ((v as u32 & 0xFF) << 24));
    atomic_render_cb(rd::<u32>(atomic, 0x48), atomic);
}

// ---- CTrafficLights, CAccidentManager, car generators, road blocks ----------------------------------

orig! {
    /// `CTrafficLights::FindOrientationForTrafficLightType(CEntity*)` (0x49D520)
    fn find_orientation_for_traffic_light_type(e: GPtr<u8>) -> f64 = cdecl 0x49D520 => unported;
    /// `CTrafficLights::FindTrafficLightTypeFromOrientation(float)` (0x49D4D0)
    fn find_traffic_light_type_from_orientation(a: f32) -> i32 = cdecl 0x49D4D0 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CAccidentManager::CAccidentManager()` (0x56CE30)
    fn accident_manager_ctor_o(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x56CE30 => unported;
    /// `CAccidentManager::GetNearestFreeAccidentExceptThisOne(CVector&, CAccident*, bool)` (0x56CF90)
    fn get_nearest_free_accident_except_this_one_o(this: GPtr<u8>, pos: GPtr<f32>, except: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x56CF90 => unported;
    /// `CCarGenerator::CheckIfWithinRangeOfAnyPlayers()` (0x6F2F40)
    fn cargen_check_if_within_range(this: GPtr<u8>) -> u8 = thiscall 0x6F2F40 => unported;
    /// `CCarGenerator::DoInternalProcessing()` (0x6F34D0)
    fn cargen_do_internal_processing(this: GPtr<u8>) = thiscall 0x6F34D0 => unported;
    /// `CCarGenerator::Process()` (0x6F3E90)
    fn cargen_process_o(this: GPtr<u8>) = thiscall 0x6F3E90 => unported;
    /// `CWorld::FindPlayerTrain(int32)` (0x56E160)
    fn find_player_train(id: i32) -> GPtr<u8> = cdecl 0x56E160 => unported;
    /// `CFileMgr::OpenFile(const char*, const char*)` (0x538900)
    fn file_open(path: u32, mode: u32) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::Read(FILESTREAM, void*, int32)` (0x538950)
    fn file_read(f: i32, buf: GPtr<u8>, n: i32) -> i32 = cdecl 0x538950 => unported;
    /// `CFileMgr::CloseFile(FILESTREAM)` (0x5389D0)
    fn file_close(f: i32) -> i32 = cdecl 0x5389D0 => unported;
}

/// `CTrafficLights::FindTrafficLightType(CEntity*)` (0x49D580).
#[hook(addr = 0x49D580, conv = cdecl, name = "CTrafficLights::FindTrafficLightType", expect = "8B 44 24 04 50")]
pub fn find_traffic_light_type(e: GPtr<u8>) -> i32 {
    let a = find_orientation_for_traffic_light_type(e);
    find_traffic_light_type_from_orientation(a as f32)
}

/// `CTrafficLights::LightForPeds()` (0x49D400): `t = (now >> 1) & 0x3FFF`: < 12000 → 2 (don't walk),
/// < 15384 → 0, else 1.
#[hook(addr = 0x49D400, conv = cdecl, name = "CTrafficLights::LightForPeds", expect = "A1 84 CB B7 00")]
pub fn light_for_peds() -> u8 {
    let t = (g::<u32>(TIME_MS).read() >> 1) & 0x3FFF;
    if t < 0x2EE0 {
        2
    } else if t < 0x3C18 {
        0
    } else {
        1
    }
}

/// `ThePaths.m_pNaviNodes[area]` + 14 * index + 0xC: the navi node flags byte of a link address.
fn navi_flags(addr: u16) -> u8 {
    let a = u32::from(addr);
    let base = g::<GPtr<u8>>((0x96F974 + (a >> 10) * 4) as usize).read();
    rd::<u8>(base, ((a & 0x3FF) * 14 + 0xC) as isize)
}

/// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420): not boats/planes (appearance 3, 5);
/// both autopilot links (+0x3A6, +0x3A4) loaded, the current one a bridge (navi flags bit 2) and the next
/// one not.
#[hook(addr = 0x49D420, conv = cdecl, name = "CTrafficLights::ShouldCarStopForBridge", expect = "53 56 8B 74 24 0C")]
pub fn should_car_stop_for_bridge(veh: GPtr<u8>) -> u8 {
    if get_vehicle_appearance(veh) == 3 || get_vehicle_appearance(veh) == 5 {
        return 0;
    }
    let cur = rd::<u16>(veh, 0x3A6);
    if cur == 0xFFFF || g::<u32>((0x96F854 + (u32::from(cur) >> 10) * 4) as usize).read() == 0 {
        return 0;
    }
    let next = rd::<u16>(veh, 0x3A4);
    if next == 0xFFFF || g::<u32>((0x96F854 + (u32::from(next) >> 10) * 4) as usize).read() == 0 {
        return 0;
    }
    if navi_flags(cur) & 4 == 0 {
        return 0;
    }
    u8::from(navi_flags(next) & 4 == 0)
}

orig! {
    /// `CNodeAddress::operator==` (0x420920, thiscall, `ret 4`)
    fn node_address_eq(a: GPtr<u8>, b: GPtr<u8>) -> u8 = thiscall 0x420920 => unported;
    /// `CNodeAddress::operator!=` (0x420950, thiscall, `ret 4`)
    fn node_address_ne(a: GPtr<u8>, b: GPtr<u8>) -> u8 = thiscall 0x420950 => unported;
    /// `CTrafficLights::LightForCars1()` (0x49D2D0)
    fn light_for_cars1() -> u8 = cdecl 0x49D2D0 => unported;
    /// `CTrafficLights::LightForCars2()` (0x49D310)
    fn light_for_cars2() -> u8 = cdecl 0x49D310 => unported;
}

/// One link of `ShouldCarStopForLight`: `None` = go on with the next link, `Some(r)` = return r.
#[allow(clippy::too_many_arguments)]
fn stop_for_light_link(veh: GPtr<u8>, ignore_lights: u8, link_off: isize, node_off: isize, dir_off: isize, lo: f32, hi: f32, needs_physics: bool) -> Option<u8> {
    let link = rd::<u16>(veh, link_off);
    if link == 0xFFFF {
        return if needs_physics { Some(0) } else { None };
    }
    let a = u32::from(link);
    if g::<u32>((0x96F854 + (a >> 10) * 4) as usize).read() == 0 {
        return if needs_physics { Some(0) } else { None };
    }
    let navi = g::<GPtr<u8>>((0x96F974 + (a >> 10) * 4) as usize).read().byte_add(((a & 0x3FF) * 14) as isize);
    if needs_physics && rd::<u8>(veh, 0x36) & 0xF8 != 0x18 {
        return Some(0);
    }
    let ty = rd::<u8>(navi, 0xC) & 3;
    let skip = if needs_physics { Some(0) } else { None };
    if ty == 0 {
        return skip;
    }
    let bl = (rd::<u8>(navi, 0xB) >> 6) & 1;
    let node = veh.byte_add(node_off);
    if bl != 0 && node_address_eq(navi.byte_add(4), node) == 0 {
        return skip;
    }
    if bl == 0 && node_address_ne(navi.byte_add(4), node) == 0 {
        return skip;
    }
    if ignore_lights == 0
        && !(ty == 1 && light_for_cars1() != 0)
        && (rd::<u8>(navi, 0xC) & 3 != 2 || light_for_cars2() == 0)
    {
        return Some(0);
    }
    let p = entity_pos(veh);
    let dy = xi(i32::from(rd::<i16>(navi, 2))).fmul_m(0.125).fsubr_m(p.wrapping_index(1).read());
    let ky = xi(i32::from(rd::<i8>(navi, 9))).fmul_m(f32::from_bits(0x3C23_D70A));
    let dx = xi(i32::from(rd::<i16>(navi, 0))).fmul_m(0.125).fsubr_m(p.read());
    let kx = xi(i32::from(rd::<i8>(navi, 8))).fmul_m(f32::from_bits(0x3C23_D70A));
    let v = dy.fmul(ky).fadd(dx.fmul(kx));
    if rd::<u8>(veh, dir_off) == 0xFF {
        if jne_mask(v, x(0.0), 0x41) {
            return skip;
        }
        if jp5(v, x(lo)) {
            return skip;
        }
    } else {
        if jp5(v, x(0.0)) {
            return skip;
        }
        if jne_mask(v, x(hi), 0x41) {
            return skip;
        }
    }
    Some(1)
}

/// `CTrafficLights::ShouldCarStopForLight(CVehicle*, bool ignoreLights)` (0x49D610): for the autopilot's
/// current, next and (physical vehicles only) previous links (+0x3A4/+0x3A6/+0x3A8 with nodes
/// +0x390/+0x394/+0x398 and directions +0x3B5/+0x3B6/+0x3B4): a traffic light node passed in the right
/// direction whose light is red (unless ignored) stops the car when the signed distance along the link
/// direction is within (0, 12) / (-12, 0) (previous link: 6 / -6). A red-light test that fails returns
/// false at once, as the bytes do.
#[hook(addr = 0x49D610, conv = cdecl, name = "CTrafficLights::ShouldCarStopForLight", expect = "83 EC 08 53 56")]
pub fn should_car_stop_for_light(veh: GPtr<u8>, ignore_lights: u8) -> u8 {
    if get_vehicle_appearance(veh) == 3 || get_vehicle_appearance(veh) == 5 {
        return 0;
    }
    if let Some(r) = stop_for_light_link(veh, ignore_lights, 0x3A4, 0x390, 0x3B5, 12.0, -12.0, false) {
        return r;
    }
    if let Some(r) = stop_for_light_link(veh, ignore_lights, 0x3A6, 0x394, 0x3B6, 12.0, -12.0, false) {
        return r;
    }
    stop_for_light_link(veh, ignore_lights, 0x3A8, 0x398, 0x3B4, 6.0, -6.0, true).unwrap_or(0)
}

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data_to_work_buffer(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data_from_work_buffer(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => unported;
    /// `CGangWarsSaveStructure::Construct()` (0x5D2620)
    fn gang_wars_save_construct(s: GPtr<u8>) = thiscall 0x5D2620 => gang_wars_save_structure_construct;
    /// `CGangWarsSaveStructure::Extract()` (0x5D2740)
    fn gang_wars_save_extract(s: GPtr<u8>) = thiscall 0x5D2740 => gang_wars_save_structure_extract;
}

/// `CGangs::Save()` (0x5D3A60): the 10 `CGangInfo` (0x10 each).
#[hook(addr = 0x5D3A60, conv = cdecl, name = "CGangs::Save", expect = "56 57 BE F0 91 C0 00")]
pub fn gangs_save() -> u8 {
    for k in 0..10isize {
        save_data_to_work_buffer(GANG.ptr().cast::<u8>().byte_add(16 * k), 0x10);
    }
    1
}

/// `CGangs::Load()` (0x5D3A90).
#[hook(addr = 0x5D3A90, conv = cdecl, name = "CGangs::Load", expect = "56 57 BE F0 91 C0 00")]
pub fn gangs_load() -> u8 {
    for k in 0..10isize {
        load_data_from_work_buffer(GANG.ptr().cast::<u8>().byte_add(16 * k), 0x10);
    }
    1
}

/// The dword fields of `CGangWarsSaveStructure` after the two zone indices (structure offset, global), in
/// the bytes' order.
const GANG_WARS_SAVE_DWORDS: [(isize, usize); 15] = [
    (0x14, 0x96ABBC),
    (0x18, 0x96ABC0),
    (0x1C, 0x96ABC4),
    (0x20, 0x96AB84),
    (0x24, 0x96AB50),
    (0x28, 0x96AB58),
    (0x2C, 0x96AB54),
    (0x30, 0x96AB4C),
    (0x34, 0x96AB48),
    (0x38, 0x96ABC8),
    (0x3C, 0x96ABCC),
    (0x40, 0x96ABD0),
    (0x44, 0x96AB44),
    (0x48, 0x96AB98),
    (0x50, 0x96AB9C),
];

/// `CGangWarsSaveStructure::Construct()` (0x5D2620, thiscall; docs/re/specs/SAVE/CGangWarsSaveStructure.md):
/// +0 `bGangWarsActive`, +4 `State`, +8 `TimeStarted`, +0xC / +0x10 the indices of the zone info / navigation
/// zone to fight over (-1 when null or not found; linear searches over the counts, read once), the dwords of
/// [`GANG_WARS_SAVE_DWORDS`], +0x4C `bPlayerIsCloseby`, +0x54 `Difficulty`. +1..+3 and +0x4D..+0x4F are not
/// written.
#[hook(addr = 0x5D2620, conv = thiscall, name = "CGangWarsSaveStructure::Construct", expect = "A0 90 AB 96 00 88 01",
       unverified = "save-pop-1", route = "L2", hazards = "none", recs = "save-load")]
pub fn gang_wars_save_structure_construct(this: GPtr<u8>) {
    let gw = |a: usize| GPtr::<u32>::from_addr(a).read();
    this.write(GPtr::<u8>::from_addr(0x96AB90).read());
    this.byte_add(4).cast::<u32>().write(gw(0x96AB64));
    this.byte_add(8).cast::<u32>().write(gw(0x96AB60));
    for (field, ptr, count, base, stride) in [(0xCisize, 0x96AB8Cusize, 0xBA1DE8usize, 0xBA1DF0u32, 0x11u32), (0x10, 0x96AB88, 0xBA3794, 0xBA3798, 0x20)] {
        this.byte_add(field).cast::<i32>().write(-1);
        let p = gw(ptr);
        if p == 0 {
            continue;
        }
        let n = i32::from(GPtr::<u16>::from_addr(count).read());
        let mut i = 0i32;
        while i < n {
            if p == base.wrapping_add((i as u32).wrapping_mul(stride)) {
                this.byte_add(field).cast::<i32>().write(i);
                break;
            }
            i += 1;
        }
    }
    for (o, a) in GANG_WARS_SAVE_DWORDS {
        if o == 0x50 {
            this.byte_add(0x4C).write(GPtr::<u8>::from_addr(0x96AB92).read());
        }
        this.byte_add(o).cast::<u32>().write(gw(a));
    }
    this.byte_add(0x54).cast::<u32>().write(gw(0x96AB80));
}

/// `CGangWarsSaveStructure::Extract()` (0x5D2740, thiscall): the inverse of `Construct` in the same order; the
/// zone pointers from the indices (`0xBA1DF0 + 0x11 * i` / `0xBA3798 + (i << 5)`, null for -1, no range
/// checks).
#[hook(addr = 0x5D2740, conv = thiscall, name = "CGangWarsSaveStructure::Extract", expect = "8A 01 A2 90 AB 96 00",
       unverified = "save-pop-1", route = "L2", hazards = "none", recs = "save-load")]
pub fn gang_wars_save_structure_extract(this: GPtr<u8>) {
    let set = |a: usize, v: u32| GPtr::<u32>::from_addr(a).write(v);
    GPtr::<u8>::from_addr(0x96AB90).write(this.read());
    set(0x96AB64, this.byte_add(4).cast::<u32>().read());
    set(0x96AB60, this.byte_add(8).cast::<u32>().read());
    let zi = this.byte_add(0xC).cast::<i32>().read();
    set(0x96AB8C, if zi == -1 { 0 } else { (zi as u32).wrapping_mul(0x11).wrapping_add(0xBA1DF0) });
    let nz = this.byte_add(0x10).cast::<i32>().read();
    set(0x96AB88, if nz == -1 { 0 } else { (nz as u32).wrapping_shl(5).wrapping_add(0xBA3798) });
    for (o, a) in GANG_WARS_SAVE_DWORDS {
        if o == 0x50 {
            GPtr::<u8>::from_addr(0x96AB92).write(this.byte_add(0x4C).read());
        }
        set(a, this.byte_add(o).cast::<u32>().read());
    }
    set(0x96AB80, this.byte_add(0x54).cast::<u32>().read());
}

/// `CGangWars::Save()` (0x5D5530): size 0x58, then the constructed `CGangWarsSaveStructure`. The structure
/// is an uninitialised local (entry ESP -0x58) whose bytes `Construct` skips (+1..+3, +0x4D..+0x4F) reach the
/// save file: SAVE-1 takes them from the footprint snapshot (ADR-001 §5b; docs/arch/save-port.md).
#[hook(addr = 0x5D5530, conv = cdecl, name = "CGangWars::Save", expect = "83 EC 5C 8D 4C 24 04",
       unverified = "save-pop-1", route = "L2", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn gang_wars_save() -> u8 {
    let mut buf = [0u32; 0x17];
    for (k, w) in buf.iter_mut().enumerate().skip(1) {
        let off = 4 * k as i32 - 0x5C;
        *w = u32::from_le_bytes([0, 1, 2, 3].map(|b| sa_core::abi::footprint_residue_u8(off + b, 0x1000)));
    }
    buf[0] = 0x58;
    let p = GPtr::<u8>::from_raw(buf.as_mut_ptr().cast());
    gang_wars_save_construct(p.byte_add(4));
    save_data_to_work_buffer(p, 4);
    save_data_to_work_buffer(p.byte_add(4), 0x58);
    1
}

/// `CGangWars::Load()` (0x5D3EB0).
#[hook(addr = 0x5D3EB0, conv = cdecl, name = "CGangWars::Load", expect = "83 EC 5C 8D 44 24 00")]
pub fn gang_wars_load() -> u8 {
    let mut buf = [0u32; 0x17];
    let p = GPtr::<u8>::from_raw(buf.as_mut_ptr().cast());
    load_data_from_work_buffer(p, 4);
    load_data_from_work_buffer(p.byte_add(4), 0x58);
    gang_wars_save_extract(p.byte_add(4));
    1
}

/// `CTagManager::Save()` (0x5D3D60): the count, then every tag's alpha byte.
#[hook(addr = 0x5D3D60, conv = cdecl, name = "CTagManager::Save", expect = "56 6A 04 68 70 AD A9 00")]
pub fn tag_manager_save() -> u8 {
    save_data_to_work_buffer(NUM_TAGS.ptr().cast(), 4);
    let mut i: u32 = 0;
    while i < NUM_TAGS.get() as u32 {
        save_data_to_work_buffer(tag(i as i32).byte_add(4), 1);
        i += 1;
    }
    1
}

/// `CTagManager::Load()` (0x5D3DA0): the count goes through `SaveDataToWorkBuffer` (original bug, kept),
/// the alphas through `LoadDataFromWorkBuffer`.
#[hook(addr = 0x5D3DA0, conv = cdecl, name = "CTagManager::Load", expect = "56 6A 04 68 70 AD A9 00")]
pub fn tag_manager_load() -> u8 {
    save_data_to_work_buffer(NUM_TAGS.ptr().cast(), 4);
    let mut i: u32 = 0;
    while i < NUM_TAGS.get() as u32 {
        load_data_from_work_buffer(tag(i as i32).byte_add(4), 1);
        i += 1;
    }
    1
}

/// `CAccident::CAccident()` (0x56CDE0): returns `this`.
#[hook(addr = 0x56CDE0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 33 C9 89 08")]
pub fn accident_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u8>(this, 4, 0);
    wr::<u8>(this, 5, 0);
    this
}

/// `CAccidentManager::CAccidentManager()` (0x56CE30): the 16 accidents built by the CRT vector constructor,
/// then cleared again; returns `this`.
#[hook(addr = 0x56CE30, conv = thiscall, name = "FUN_0056ce30", expect = "56 68 F0 CD 56 00")]
pub fn accident_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 8, 16, 0x56CDE0, 0x56CDF0);
    for k in 0..16isize {
        wr::<u32>(this, 8 * k, 0);
        wr::<u8>(this, 8 * k + 4, 0);
        wr::<u8>(this, 8 * k + 5, 0);
    }
    this
}

/// `CAccidentManager::GetInstance()` (0x56CF20): lazily `new CAccidentManager` (0x80 bytes) into
/// `CAccidentManager::mInstance` (0xB9B7D0). The original's SEH frame (handler pushed through an
/// obfuscated `sbb`) has no other effect.
#[hook(addr = 0x56CF20, conv = cdecl, name = "CAccidentManager::GetInstance", expect = "6A FF E9 06 A7 E9 FF")]
pub fn accident_manager_get_instance() -> GPtr<u8> {
    let inst = g::<GPtr<u8>>(0xB9B7D0);
    let cur = inst.read();
    if !cur.is_null() {
        return cur;
    }
    let p = operator_new(0x80);
    let v = if p.is_null() { GPtr::NULL } else { accident_manager_ctor_o(p) };
    inst.write(v);
    v
}

/// `CAccidentManager::GetNearestFreeAccident(CVector&, bool)` (0x56D050).
#[hook(addr = 0x56D050, conv = thiscall, name = "CAccidentManager::GetNearestFreeAccident", expect = "8B 44 24 08 8B 54 24 04")]
pub fn get_nearest_free_accident(this: GPtr<u8>, pos: GPtr<f32>, b: u32) -> GPtr<u8> {
    get_nearest_free_accident_except_this_one_o(this, pos, GPtr::NULL, b)
}

/// `CAccidentManager::GetNearestFreeAccidentExceptThisOne(CVector&, CAccident*, bool)` (0x56CF90): the
/// closest (3D, `(dz² + dx²) + dy²`, strict `<`) accident with a ped that is not yet handled or being
/// treated, not flagged +0x40 bit 8, and (with the bool) not flagged +0x46C bit 15.
#[hook(addr = 0x56CF90, conv = thiscall, name = "CAccidentManager::GetNearestFreeAccidentExceptThisOne", expect = "83 EC 08 53 55")]
pub fn get_nearest_free_accident_except_this_one(this: GPtr<u8>, pos: GPtr<f32>, except: GPtr<u8>, b: u8) -> GPtr<u8> {
    let mut best = f32::MAX;
    let mut res = GPtr::NULL;
    for k in 0..16isize {
        let acc = this.byte_add(8 * k);
        let ped = rd::<GPtr<u8>>(acc, 0);
        if ped.is_null() || rd::<u8>(acc, 4) != 0 || rd::<u8>(acc, 5) != 0 {
            continue;
        }
        if rd::<u32>(ped, 0x40) & 0x100 != 0 {
            continue;
        }
        if b != 0 && (rd::<u32>(ped, 0x46C) >> 8) as u8 & 0x80 != 0 {
            continue;
        }
        if !except.is_null() && except == acc {
            continue;
        }
        let p = entity_pos(ped);
        let dx = x(pos.read()).fsub_m(p.read());
        let dy = x(pos.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
        let dz = x(pos.wrapping_index(2).read()).fsub_m(p.wrapping_index(2).read());
        let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
        if !crate::common::jp5(d, x(best)) {
            best = d.to_f32();
            res = acc;
        }
    }
    res
}

/// `CCarGenerator::CalcNextGen()` (0x6F2E40): now + 4.
#[hook(addr = 0x6F2E40, conv = thiscall, name = "CCarGenerator::CalcNextGen", expect = "A1 84 CB B7 00", this = "CCarGenerator")]
pub fn cargen_calc_next_gen(_this: GPtr<u8>) -> u32 {
    g::<u32>(TIME_MS).read().wrapping_add(4)
}

/// `CCarGenerator::Process()` (0x6F3E90): generate when due (or forced by
/// `GenerateEvenIfPlayerIsCloseCounter` 0xC279D0) and in range; release a generated vehicle once it is no
/// longer standing as created (+0x36 status bits), 60 s later the next one.
#[hook(addr = 0x6F3E90, conv = thiscall, name = "CCarGenerator::Process", expect = "56 8B F1 66 83 7E 18 FF", this = "CCarGenerator")]
pub fn cargen_process(this: GPtr<u8>) {
    if rd::<i16>(this, 0x18) == -1
        && (g::<u8>(0xC279D0).read() != 0 || g::<u32>(TIME_MS).read() >= rd::<u32>(this, 0x14))
        && rd::<u16>(this, 0x1A) != 0
        && cargen_check_if_within_range(this) != 0
    {
        cargen_do_internal_processing(this);
    }
    let h = rd::<i16>(this, 0x18);
    if h == -1 {
        return;
    }
    let h = i32::from(h);
    let idx = h >> 8;
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let ok = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read() == h as u8;
    let veh = if ok { rd::<GPtr<u8>>(pool, 0).byte_add((idx as isize).wrapping_mul(0xA18)) } else { GPtr::NULL };
    if veh.is_null() {
        wr::<i16>(this, 0x18, -1);
        return;
    }
    if rd::<u8>(veh, 0x36) & 0xF8 != 0 {
        return;
    }
    wr::<u32>(this, 0x14, rd::<u32>(this, 0x14).wrapping_add(60_000));
    wr::<i16>(this, 0x18, -1);
    wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
    wr::<u16>(veh, 0x4A6, 0);
    if rd::<i16>(this, 0) < 0 {
        wr::<i16>(this, 0, -1);
    }
}

orig! {
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    fn find_player_centre_of_world(id: i32) -> GPtr<f32> = cdecl 0x56E250 => unported;
    /// `CCamera::IsSphereVisible(const CVector*, float)` (0x420D40, `ret 8`)
    fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `COcclusion::IsPositionOccluded(CVector, float)` (0x7200B0; the vector by value)
    fn is_position_occluded(x: f32, y: f32, z: f32, r: f32) -> u8 = cdecl 0x7200B0 => unported;
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090)
    fn find_player_speed(id: i32) -> GPtr<f32> = cdecl 0x56E090 => unported;
}

/// `CCarGenerator::CheckIfWithinRangeOfAnyPlayers()` (0x6F2F40): within 50 vertically; boat-type models
/// (+0x3C == 5) closer than `240 * LOD multiplier` that are visible and not occluded count as "seen"; within
/// `160 * multiplier` (unless seen) the generator only fires when forced (0xC279D0) or after a reset
/// (+0xD bit 0 clear), on the right side of z 950 for the interior state (0xB72914), and either farther
/// than `limit - 20` or flagged +0xD bit 1, and only while the player moves towards it
/// (`speed · (player - gen) <= 0`); beyond the limit +0xD bit 0 is cleared.
#[hook(addr = 0x6F2F40, conv = thiscall, name = "CCarGenerator::CheckIfWithinRangeOfAnyPlayers", expect = "83 EC 24 0F B6 05 74 CD B7 00", this = "CCarGenerator")]
pub fn cargen_check_if_within_range_of_any_players(this: GPtr<u8>) -> u8 {
    let pc = find_player_centre_of_world(i32::from(g::<u8>(0xB7CD74).read()));
    let gc = |off: isize| xi(i32::from(rd::<i16>(this, off))).fmul_m(0.125).to_f32();
    let gx = gc(4);
    let dxf = x(pc.read()).fsub_m(gx).to_f32();
    let gy = gc(6);
    let dyf = x(pc.wrapping_index(1).read()).fsub_m(gy).to_f32();
    let gz = gc(8);
    let dz = X87::from_f64_exact(x(pc.wrapping_index(2).read()).fsub_m(gz).to_f64().abs());
    if !jne_mask(dz, x(50.0), 0x41) {
        return 0;
    }
    let d = x(dyf).fmul_m(dyf).fadd(x(dxf).fmul_m(dxf)).fsqrt().to_f32();
    let lod = g::<f32>(0xB6F11C);
    let mut seen = false;
    let model = rd::<i16>(this, 0);
    if model > 0
        && rd::<i32>(crate::common::model_info(i32::from(model)), 0x3C) == 5
        && !jne_mask(x(lod.read()).fmul_m(240.0), x(d), 0x41)
    {
        let mut c = [gx, gy, gz];
        if camera_is_sphere_visible(g(0xB6F028), GPtr::from_raw(c.as_mut_ptr()), 0.0) != 0
            && is_position_occluded(gc(4), gc(6), gc(8), 0.0) == 0
        {
            seen = true;
        }
    }
    let lim = x(lod.read()).fmul_m(160.0).to_f32();
    if jp5(x(d), x(lim)) && !seen {
        let f = rd::<u8>(this, 0xD);
        if f & 1 != 0 {
            wr::<u8>(this, 0xD, f & 0xFE);
        }
        return 0;
    }
    if g::<u8>(0xC279D0).read() != 0 {
        return 1;
    }
    let dl = rd::<u8>(this, 0xD);
    if dl & 1 != 0 {
        return 0;
    }
    let zf = xi(i32::from(rd::<i16>(this, 8))).fmul_m(0.125);
    let interior = g::<u32>(0xB72914).read();
    if !zf.fcom(x(950.0)).test_any(1) {
        if interior == 0 {
            return 0;
        }
    } else if !jp5(zf, x(950.0)) && interior != 0 {
        return 0;
    }
    if !jne_mask(x(lim).fsub_m(20.0), x(d), 0x41) && dl & 2 == 0 {
        return 0;
    }
    let sp = find_player_speed(-1);
    let v = x(dyf).fmul_m(sp.wrapping_index(1).read()).fadd(x(dxf).fmul_m(sp.read()));
    if !jne_mask(v, x(0.0), 0x41) {
        return 0;
    }
    1
}

/// `CTheCarGenerators::Process()` (0x6F3F40): unless the player is in a train, a cutscene runs (0xB5F852)
/// or 0xA43088 == 1: every 4th generator from `ProcessCounter` (0xC279D1, cycling 0..3).
#[hook(addr = 0x6F3F40, conv = cdecl, name = "CTheCarGenerators::Process", expect = "6A FF E8 19 A2 E7 FF")]
pub fn the_car_generators_process() {
    if !find_player_train(-1).is_null() || g::<u8>(0xB5F852).read() != 0 || g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let c = g::<u8>(0xC279D1);
    let mut v = c.read().wrapping_add(1);
    c.write(v);
    if v == 4 {
        v = 0;
        c.write(0);
    }
    let mut i = u32::from(v);
    while i < 500 {
        let cg = g::<u8>((0xC27AD0 + i * 0x20) as usize);
        if rd::<u8>(cg, 0x1D) != 0 {
            cargen_process_o(cg);
        }
        i += 4;
    }
    let f = g::<u8>(0xC279D0);
    let n = f.read();
    if n != 0 {
        f.write(n - 1);
    }
}

/// `CTheCarGenerators::Get(uint16)` (0x479D60).
#[hook(addr = 0x479D60, conv = cdecl, name = "CTheCarGenerators::Get", expect = "0F B7 44 24 04")]
pub fn the_car_generators_get(i: u16) -> GPtr<u8> {
    g::<u8>(0xC27AD0 + usize::from(i) * 0x20)
}

/// `CRoadBlocks::Init()` (stub 0x461100 → 0x1568D90): `InOrOut[0x145] = 1`, no script road blocks,
/// `data\paths\roadblox.dat` read into `NumRoadBlocks` (0xA43580) and `RoadBlockNodes` (0xA435A0, 0x514
/// bytes), the 16 script road blocks cleared.
#[hook(addr = 0x461100, conv = cdecl, name = "CRoadBlocks::Init", expect = "E9 8B 7C 10 01")]
pub fn road_blocks_init() {
    for k in 0..0x145 {
        g::<u8>(0xA43438 + k).write(1);
    }
    g::<u8>(0xA43584).write(0);
    let f = file_open(0x859CBC, 0x859C6C);
    file_read(f, g(0xA43580), 4);
    file_read(f, g(0xA435A0), 0x514);
    file_close(f);
    for k in 0..16 {
        g::<u8>(0xA43AD0 + k * 0x1C).write(0);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SET_WILL_ATTACK_PLAYER_WITH_COPS,
    HOOK_GET_WILL_ATTACK_PLAYER_WITH_COPS,
    HOOK_GANG_INFO_CTOR,
    HOOK_SET_GANG_WEAPONS,
    HOOK_SET_GANG_PED_MODEL_OVERRIDE,
    HOOK_INIT_GANGS_GANG,
    HOOK_DTOR_GANGS_GANG,
    HOOK_GANG_WAR_GOING_ON,
    HOOK_DOES_PLAYER_CONTROL_THIS_ZONE,
    HOOK_CALCULATE_TIME_TILL_NEXT_ATTACK,
    HOOK_SET_SPECIFIC_ZONE_TO_TRIGGER_GANG_WAR,
    HOOK_SET_GANG_WARS_ACTIVE,
    HOOK_SWITCH_GANG_WARS_ACTIVE,
    HOOK_ADD_KILL_TO_PROVOCATION,
    HOOK_PED_STREAMED_IN_FOR_THIS_GANG,
    HOOK_PICK_STREAMED_IN_PED_FOR_THIS_GANG,
    HOOK_MAKE_ENEMY_GAIN_INFLUENCE_IN_ZONE,
    HOOK_MAKE_PLAYER_GAIN_INFLUENCE_IN_ZONE,
    HOOK_RELEASE_CARS_IN_ATTACK_WAVE,
    HOOK_UPDATE_TERRITORY_UNDER_CONTROL_PERCENTAGE,
    HOOK_CLEAR_THE_STREETS,
    HOOK_DO_STUFF_WHEN_PLAYER_VICTORIOUS,
    HOOK_CALC_FADE,
    HOOK_CHECK_DAMAGED_WEAPON_TYPE,
    HOOK_FAIL_KILL_FRENZY,
    HOOK_FIND_TOTAL_PEDS_KILLED_BY_PLAYER,
    HOOK_RESET_MODELS_KILLED_BY_PLAYER,
    HOOK_THIS_PED_SHOULD_BE_KILLED_FOR_FRENZY,
    HOOK_REGISTER_KILL_NOT_BY_PLAYER,
    HOOK_REGISTER_CAR_BLOWN_UP_BY_PLAYER,
    HOOK_DARKEL_UPDATE,
    HOOK_FIND_TAG_DESC,
    HOOK_GET_ALPHA_ENTITY,
    HOOK_GET_NEAREST_TAG,
    HOOK_GET_PERCENTAGE_TAGGED_IN_AREA,
    HOOK_GET_TAG_POS,
    HOOK_UPDATE_NUM_TAGGED,
    HOOK_SET_ALPHA_ATOMIC,
    HOOK_SET_ALPHA_ENTITY,
    HOOK_SET_ALPHA_IN_AREA,
    HOOK_RENDER_TAG_FOR_PC,
    HOOK_FIND_TRAFFIC_LIGHT_TYPE,
    HOOK_LIGHT_FOR_PEDS,
    HOOK_SHOULD_CAR_STOP_FOR_LIGHT,
    HOOK_GANGS_SAVE,
    HOOK_GANGS_LOAD,
    HOOK_GANG_WARS_SAVE,
    HOOK_GANG_WARS_SAVE_STRUCTURE_CONSTRUCT,
    HOOK_GANG_WARS_SAVE_STRUCTURE_EXTRACT,
    HOOK_GANG_WARS_LOAD,
    HOOK_TAG_MANAGER_SAVE,
    HOOK_TAG_MANAGER_LOAD,
    HOOK_SHOULD_CAR_STOP_FOR_BRIDGE,
    HOOK_ACCIDENT_CTOR,
    HOOK_ACCIDENT_MANAGER_CTOR,
    HOOK_ACCIDENT_MANAGER_GET_INSTANCE,
    HOOK_GET_NEAREST_FREE_ACCIDENT,
    HOOK_GET_NEAREST_FREE_ACCIDENT_EXCEPT_THIS_ONE,
    HOOK_CARGEN_CALC_NEXT_GEN,
    HOOK_CARGEN_CHECK_IF_WITHIN_RANGE_OF_ANY_PLAYERS,
    HOOK_CARGEN_PROCESS,
    HOOK_THE_CAR_GENERATORS_PROCESS,
    HOOK_THE_CAR_GENERATORS_GET,
    HOOK_ROAD_BLOCKS_INIT,
];
