//! WRITER-WORLD-2 (batch `world6-4`): more cheat handlers: love conquers all, suicide and ninja.
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};

use crate::bindings::damage_calc_compute_damage_response as compute_damage_response;
use crate::bindings::damage_calc_dtor;
use crate::bindings::event_damage_affects_ped;
use crate::bindings::event_damage_dtor;
use crate::bindings::event_group_add;
use crate::bindings::ped_set_current_weapon as set_current_weapon;
use crate::bindings::set_texture_and_model;
use sa_core::bindings::clothes_rebuild_player as rebuild_player;
use sa_core::bindings::find_player_ped;
use sa_core::bindings::load_all_requested_models;
use sa_core::bindings::streaming_request_model as request_model;
use sa_stream::bindings::reclassify_loaded_cars;
use sa_stream::bindings::stream_peds_into_random_slots;
orig! {
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, e: u32, dmg: f32, wt: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32, eWeaponType, ePedPieceTypes, uint8, bool, bool)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, src: u32, time: u32, wt: u32, piece: u32, dir: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CCheat::BeachPartyCheat()` (0x439230)
    fn beach_party_cheat() = cdecl 0x439230 => super::cheat_clothes_themes::beach_party_cheat;
    /// `CCheat::FunhouseCheat()` (0x439720)
    fn funhouse_cheat() = cdecl 0x439720 => super::cheat_clothes_themes::funhouse_cheat;
    /// `CCheat::CountrysideInvasionCheat()` (0x439F60)
    fn countryside_invasion_cheat() = cdecl 0x439F60 => super::cheat_clothes_themes::countryside_invasion_cheat;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080)
    fn give_weapon(ped: GPtr<u8>, ty: i32, ammo: u32, b: u32) -> i32 = thiscall 0x5E6080 => unported;
}

/// Toggles a cheat's state byte (`test dl, dl; sete al`), returning the new state.
fn toggle(addr: usize) -> bool {
    let on = g::<u8>(addr).read() == 0;
    g::<u8>(addr).write(u8::from(on));
    on
}

/// `CCheat::LoveConquersAllCheat()` (0x4393F0): toggles the cheat (0x96915D); on: streams models
/// 249, 63, 64, 75, 238, 85, 207 (slot 8: -2), dresses the player in "gimpleg" (0x8590D4, part 17)
/// and rebuilds the player unless driving.
#[hook(addr = 0x4393F0, conv = cdecl, name = "CCheat::LoveConquersAllCheat",
       expect = "8A 15 5D 91 96 00",
       unverified = "world6-4", route = "L2", hazards = "heap-writes",
       recs = "world-cheats")]
pub fn love_conquers_all_cheat() {
    if !toggle(0x96_915D) {
        return;
    }
    let mut models: [i32; 8] = [0xF9, 0x3F, 0x40, 0x4B, 0xEE, 0x55, 0xCF, -2];
    stream_peds_into_random_slots(GPtr::from_raw(&mut models as *mut [i32; 8]).cast());
    let clothes = rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 4);
    set_texture_and_model(clothes, 0x85_90D4, 0x85_90D4, 0x11);
    if rd::<u32>(find_player_ped(-1), 0x530) != 0x32 {
        rebuild_player(find_player_ped(-1), 0);
    }
}

/// `CCheat::SuicideCheat()` (0x438FF0): a 1000-damage unarmed torso hit on the player through the
/// damage calculator and a `CEventDamage` (no source, now) added to the player's event group.
#[hook(addr = 0x438FF0, conv = cdecl, name = "CCheat::SuicideCheat",
       expect = "6A FF 68 20 75 83 00",
       unverified = "world6-4", route = "L2", hazards = "heap-writes",
       recs = "world-cheats")]
pub fn suicide_cheat() {
    let mut calc = [0u32; 8];
    let mut ev = [0u32; 0x20];
    let c = GPtr::from_raw(&mut calc as *mut [u32; 8]).cast::<u8>();
    let e = GPtr::from_raw(&mut ev as *mut [u32; 0x20]).cast::<u8>();
    damage_calc_ctor(c, 0, 1000.0, 0, 3, 0);
    event_damage_ctor(e, 0, g::<u32>(0xB7_CB84).read(), 0, 3, 0, 0, 0);
    if event_damage_affects_ped(e, find_player_ped(-1)) != 0 {
        compute_damage_response(c, find_player_ped(-1), e.byte_add(0x38), 1);
    } else {
        wr::<u8>(e, 0x42, 1);
    }
    event_group_add(rd::<GPtr<u8>>(find_player_ped(-1), 0x47C).byte_add(0x68), e, 0);
    event_damage_dtor(e);
    damage_calc_dtor(c);
}

/// `CCheat::NinjaCheat()` (0x439E50): toggles the cheat (0x96915C); on: streams models 117, 118,
/// 120 (others -2), switches the beach party, funhouse and countryside cheats off through their
/// handlers (clearing 0x96915E / 0x96915F), reclassifies the loaded cars, turns 0x969151 on (and
/// 0x969150 off) when it was off, and gives the player a katana (model 339 loaded, weapon 8, ammo 0)
/// as the current weapon; off: clears 0x969151.
#[hook(addr = 0x439E50, conv = cdecl, name = "CCheat::NinjaCheat",
       expect = "8A 15 5C 91 96 00",
       unverified = "world6-4", route = "L2", hazards = "heap-writes",
       recs = "world-cheats")]
pub fn ninja_cheat() {
    if !toggle(0x96_915C) {
        if g::<u8>(0x96_9151).read() != 0 {
            g::<u8>(0x96_9151).write(0);
        }
        return;
    }
    let mut models: [i32; 8] = [0x75, 0x76, 0x78, -2, -2, -2, -2, -2];
    stream_peds_into_random_slots(GPtr::from_raw(&mut models as *mut [i32; 8]).cast());
    if g::<u8>(0x96_9159).read() != 0 {
        beach_party_cheat();
    }
    if g::<u8>(0x96_915E).read() != 0 {
        g::<u8>(0x96_915E).write(0);
    }
    if g::<u8>(0x96_915F).read() != 0 {
        g::<u8>(0x96_915F).write(0);
    }
    if g::<u8>(0x96_9176).read() != 0 {
        funhouse_cheat();
    }
    if g::<u8>(0x96_917B).read() != 0 {
        countryside_invasion_cheat();
    }
    reclassify_loaded_cars();
    if g::<u8>(0x96_9151).read() == 0 {
        g::<u8>(0x96_9151).write(1);
        g::<u8>(0x96_9150).write(0);
    }
    request_model(0x153, 2);
    load_all_requested_models(0);
    give_weapon(find_player_ped(-1), 8, 0, 1);
    set_current_weapon(find_player_ped(-1), 8);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_LOVE_CONQUERS_ALL_CHEAT, HOOK_SUICIDE_CHEAT, HOOK_NINJA_CHEAT];
