//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `CPickups::GivePlayerGoodiesWithPickUpMI`.
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, xi};

use sa_core::bindings::audio_report_frontend_event as report_frontend_audio_event;
use crate::bindings::event_group_add;
use crate::bindings::event_script_command_ctor;
use crate::bindings::event_script_command_dtor;
use crate::bindings::player_get_wanted_level;
use crate::bindings::task_jetpack_ctor;
use crate::bindings::update_stats_add_to_health;
use crate::bindings::find_player_ped;
use sa_core::bindings::player_ped_reset_sprint_energy;
use sa_core::bindings::set_wanted_level;
use crate::bindings::stats_increment_stat;
use sa_core::bindings::task_new;
orig! {
    /// `CPickups::PickedUpOyster()` (0x4552D0)
    fn picked_up_oyster() = cdecl 0x4552D0 => unported;
    /// `CPickups::PickedUpHorseShoe()` (0x455390)
    fn picked_up_horse_shoe() = cdecl 0x455390 => unported;
}

/// `AudioEngine.ReportFrontendAudioEvent(event, 0.0, 1.0)`.
fn frontend(event: u32) {
    report_frontend_audio_event(GPtr::from_addr(0xB6_BC90), event, f32::from_bits(0), f32::from_bits(0x3F80_0000));
}

/// The int16 model id in a `MI_PICKUP_*` cell.
pub fn mi(cell: usize) -> u16 {
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
/// The armour and health branches index the `CPlayerInfo` maxima with the raw `player` argument, not
/// the resolved slot (original behaviour, kept). The entry is `nop; jmp 0x401F0F` (a protection
/// fragment that only loads `fs:[0]`); `xor` at 0x4016E3 reads the jmp's rel32 (keycells reader).
#[hook(addr = 0x4564F0, conv = cdecl, name = "CPickups::GivePlayerGoodiesWithPickUpMI",
       expect = "90 E9 19 BA FA FF",
       unverified = "world4-1", route = "L2", hazards = "entry-protection-read,heap-writes",
       recs = "grove-walk",
       note = "the jetpack branch builds a CEventScriptCommand on the Rust stack and adds it to the ped's event group (copied there); writes the player ped's health/armour and player data")]
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
        set_wanted_level(find_player_ped(0), w as u32);
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
        event_script_command_dtor(ep.cast::<u32>());
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
