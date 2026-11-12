//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `CPickups::GivePlayerGoodiesWithPickUpMI`.
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, xi};

orig! {
    /// `CPlayerPed::ResetSprintEnergy()` (0x60A530)
    fn player_ped_reset_sprint_energy(p: GPtr<u8>) = thiscall 0x60A530 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0) on `AudioEngine` (0xB6BC90)
    fn report_frontend_audio_event(engine: GPtr<u8>, event: u32, volume: u32, speed: u32) = thiscall 0x506EA0 => unported;
    /// `CStats::UpdateStatsAddToHealth(uint32)` (0x55D030)
    fn update_stats_add_to_health(v: i32) = cdecl 0x55D030 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn player_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CPlayerPed::SetWantedLevel(int32)` (0x609F10)
    fn set_wanted_level(p: GPtr<u8>, level: i32) = thiscall 0x609F10 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleJetPack::CTaskSimpleJetPack(const CVector*, float, int32, CEntity*)` (0x67B4E0)
    fn task_jetpack_ctor(t: GPtr<u8>, pos: u32, h: u32, a: u32, e: u32) -> GPtr<u8> = thiscall 0x67B4E0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPickups::PickedUpOyster()` (0x4552D0)
    fn picked_up_oyster() = cdecl 0x4552D0 => unported;
    /// `CPickups::PickedUpHorseShoe()` (0x455390)
    fn picked_up_horse_shoe() = cdecl 0x455390 => unported;
}

/// `AudioEngine.ReportFrontendAudioEvent(event, 0.0, 1.0)`.
fn frontend(event: u32) {
    report_frontend_audio_event(GPtr::from_addr(0xB6_BC90), event, 0, 0x3F80_0000);
}

/// The int16 model id in a `MI_PICKUP_*` cell.
fn mi(cell: usize) -> u16 {
    g::<u16>(cell).read()
}

/// `CPlayerInfo` field `off` of player slot `idx` (0xB7CD98 + idx * 0x190).
fn player_info(idx: i32, off: usize) -> usize {
    (0xB7_CD98usize).wrapping_add((idx as usize).wrapping_mul(0x190)).wrapping_add(off)
}

/// `CPickups::GivePlayerGoodiesWithPickUpMI(int16 model, int32 player)` (0x4564F0): the effect of a
/// collected pickup model on the player's ped (slot `player`, or the focus player for 0/negative):
/// adrenaline (flag + 20 s, sprint energy), body armour (+0x548 = slot's max armour +0x150),
/// three sound-only models, health (+0x540 = slot's max health +0x14F, the difference to the stats), bribe (wanted level - 1, stat 182), jetpack (a
/// script-command event with `CTaskSimpleJetPack`), oyster, horseshoe; each with its frontend
/// sound. False for any other model.
///
/// The entry is `nop; jmp 0x401F0F` (a protection chunk that loads `fs:[0]` and jumps back to
/// 0x4564F6); the hook relocates both instructions. The protection reads the entry's rel32 as a key
/// (`xor eax, [0x4564F2]` at 0x4016E3, value 0xFFFABA19): sa-hook's keycells `READERS` redirect it
/// to a cell with the original dword.
#[hook(addr = 0x4564F0, conv = cdecl, name = "CPickups::GivePlayerGoodiesWithPickUpMI",
       expect = "90 E9 19 BA FA FF",
       unverified = "world5-1", route = "L2", hazards = "entry-protection-read,heap-writes",
       note = "keycells reader 0x4016E3 reads the entry rel32; armour/health use the raw player argument as the slot (original bug); the jetpack case builds a CEventScriptCommand on the stack")]
pub fn give_player_goodies_with_pick_up_mi(model: u16, player: i32) -> u8 {
    let slot = if player > 0 { player } else { i32::from(g::<u8>(0xB7_CD74).read()) };
    let ped = g::<GPtr<u8>>(player_info(slot, 0)).read();
    if model == mi(0x8C_D5BC) {
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        wr::<u32>(pd, 0x34, rd::<u32>(pd, 0x34) | 2);
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x3C, g::<u32>(0xB7_CB84).read().wrapping_add(20000));
        player_ped_reset_sprint_energy(ped);
        frontend(9);
        return 1;
    }
    if model == mi(0x8C_D5C0) {
        let max = g::<u8>(player_info(player, 0x150)).read();
        wr::<f32>(ped, 0x548, xi(i32::from(max)).to_f32());
        frontend(10);
        return 1;
    }
    if model == mi(0x8C_D5C4) || model == mi(0x8C_D5CC) || model == mi(0x8C_D5D4) {
        frontend(11);
        return 1;
    }
    if model == mi(0x8C_D5C8) {
        let max = g::<u8>(player_info(player, 0x14F));
        update_stats_add_to_health(ftol32(xi(i32::from(max.read())).fsub_m(rd::<f32>(ped, 0x540))));
        wr::<f32>(ped, 0x540, xi(i32::from(max.read())).to_f32());
        frontend(8);
        return 1;
    }
    if model == mi(0x8C_D5D0) {
        let w = if player_get_wanted_level(find_player_ped(-1)) - 1 < 0 {
            0
        } else {
            player_get_wanted_level(find_player_ped(-1)) - 1
        };
        set_wanted_level(find_player_ped(0), w);
        stats_increment_stat(0xB6, 1.0);
        frontend(11);
        return 1;
    }
    if model == 0x172 {
        let mem = task_new(0x70);
        let task = if mem.is_null() { mem } else { task_jetpack_ctor(mem, 0, 0x4120_0000, 0, 0) };
        let mut ev = [0u32; 6];
        let ep: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 6]).cast();
        event_script_command_ctor(ep, 3, task, 0);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
        frontend(11);
        event_script_command_dtor(ep);
        return 1;
    }
    if model == mi(0x8C_D750) {
        picked_up_oyster();
        return 1;
    }
    if model == mi(0x8C_D754) {
        picked_up_horse_shoe();
        return 1;
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GIVE_PLAYER_GOODIES_WITH_PICK_UP_MI];
