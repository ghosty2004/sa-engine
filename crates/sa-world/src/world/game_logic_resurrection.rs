//! PORT-VEH-3 (batch `world3-3`, SA:MP-executed rows): `CGameLogic::RestorePlayerStuffDuringResurrection`,
//! the player respawn reset. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes
//! win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x, xi};

use crate::bindings::camera_set_directly_behind;
use crate::bindings::car_ctrl_clear_interesting_vehicle_list;
use crate::bindings::clear_drunk_blur as mblur_clear_drunk_blur;
use crate::bindings::clothes_desc_set_texture_and_model;
use crate::bindings::fire_extinguish;
use crate::bindings::get_ped_type_acquaintances;
use crate::bindings::get_player_info as get_player_info_for_this_player_ped;
use crate::bindings::ped_restart_non_partial_anims;
use crate::bindings::ped_restore_heading_rate;
use crate::bindings::player_clear_weapon_target as player_ped_clear_weapon_target;
use crate::bindings::player_ped_clear_adrenaline;
use crate::bindings::player_ped_set_initial_state;
use crate::bindings::set_stat_value;
use crate::bindings::shopping_remove_loaded_shop;
use crate::bindings::water_find_nearest_water_and_flow as water_level_find_nearest_water_and_its_flow;
use sa_core::bindings::audio_zones_update;
use sa_core::bindings::camera_fade;
use sa_core::bindings::camera_restore_with_jump_cut;
use sa_core::bindings::camera_set_fade_colour;
use sa_core::bindings::clear_exciting_stuff_from_area;
use sa_core::bindings::clothes_rebuild_player;
use sa_core::bindings::clothes_rebuild_player_if_needed;
use sa_core::bindings::entity_clean_up_old_reference as clean_up_old_reference;
use sa_core::bindings::get_pad;
use sa_core::bindings::matrix_set_rotate_z_only;
use sa_core::bindings::player_info_make_player_safe;
use sa_core::bindings::player_ped_reset_sprint_energy;
use sa_core::bindings::post_effects_script_reset_for_effects;
use sa_core::bindings::references_remove_references_to_player;
use sa_core::bindings::stream_zone_models;
use sa_core::bindings::time_cycle_stop_extra_colour;
use sa_core::bindings::world_add;
use sa_core::bindings::world_remove;
use sa_stream::bindings::remove_buildings_not_in_area;
orig! {
    /// `CStats::UpdateStatsOnRespawn()` (0x55CFC0)
    fn stats_update_stats_on_respawn() = cdecl 0x55CFC0 => unported;
    /// `CAEPedAudioEntity::TurnOffJetPack()` (0x4E2A70)
    fn ped_audio_turn_off_jet_pack(a: GPtr<u8>) = thiscall 0x4E2A70 => unported;
    /// `CWanted::Reset()` (0x562400)
    fn wanted_reset(w: GPtr<u8>) = thiscall 0x562400 => unported;
    /// virtual `CEntity::Teleport(CVector, bool)` (vtable +0x38)
    fn v_teleport(e: GPtr<u8>, px: u32, py: u32, pz: u32, reset_rot: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u8>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CGarages::PlayerArrestedOrDied()` (0x449E60)
    fn garages_player_arrested_or_died() = cdecl 0x449E60 => unported;
    /// `CHud::ResetWastedText()` (0x589070)
    fn hud_reset_wasted_text() = cdecl 0x589070 => unported;
    /// `CAudioEngine::StopPoliceScanner(uint8)` (0x507400) on `AudioEngine` (0xB6BC90)
    fn audio_engine_stop_police_scanner(e: GPtr<u8>, b: u32) = thiscall 0x507400 => unported;
}

/// `TheCamera` (0xB6F028).
const CAMERA: usize = 0xB6_F028;

/// `CGameLogic::RestorePlayerStuffDuringResurrection(CPlayerPed*, CVector pos, float headingDeg)`
/// (0x442060): after a death/arrest (game state 0x96A8A0 == 2: fade to black) the player is reset:
/// controls (pad 0 +0x10F bit 0), health = the slot's max health (+0x14F), armour 0, flags, drunk
/// blur, adrenaline, sprint, fire, jetpack audio, vehicle reference, wanted level, anims,
/// MakePlayerSafe(false, 10000), weapon target, initial state, acquaintances; teleported to `pos +
/// (0, 0, 1)` with the heading (degrees x 0.017453292), the area cleared, streaming/camera/garage/
/// stat/shop/hud/time-cycle state reset, clothes rebuilt, audio zones updated at `pos`.
#[hook(addr = 0x442060, conv = cdecl, name = "CGameLogic::RestorePlayerStuffDuringResurrection",
       expect = "A1 A0 A8 96 00",
       unverified = "world3-3", route = "split", hazards = "heap-writes",
       note = "streams zone models and removes buildings (file I/O, pools); the original keeps the heading in its first argument slot")]
pub fn restore_player_stuff_during_resurrection(player: GPtr<u8>, px: u32, py: u32, pz: u32, heading: f32) {
    let cam: GPtr<u8> = GPtr::from_addr(CAMERA);
    if g::<u32>(0x96_A8A0).read() == 2 {
        camera_set_fade_colour(cam, 0, 0, 0);
        camera_fade(cam, 0.5, 1);
    }
    g::<u32>(0x96_A8A0).write(0);
    let pad = get_pad(0);
    wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) & 0xFE);
    // `test ah, 0x41; jp` not taken: exactly one of C0/C3 (health below or equal 0)
    if !x(rd::<f32>(player, 0x540)).fcom(x(0.0)).test_parity(0x41) {
        stats_update_stats_on_respawn();
    }
    let info = get_player_info_for_this_player_ped(player);
    let flags46c = rd::<u32>(player, 0x46C);
    let flags40 = rd::<u32>(player, 0x40);
    let pd = rd::<GPtr<u8>>(player, 0x480);
    let max = rd::<u8>(info, 0x14F);
    let flags1c = rd::<u32>(player, 0x1C);
    wr::<u32>(player, 0x40, flags40 & 0xDFFF_FFFF);
    wr::<u32>(player, 0x548, 0);
    wr::<f32>(player, 0x540, xi(i32::from(max)).to_f32());
    wr::<u32>(player, 0x1C, flags1c | 0x80);
    wr::<u32>(player, 0x750, 0);
    wr::<u32>(player, 0x46C, flags46c & 0xEFFF_FFFF);
    wr::<u8>(pd, 0x40, 0);
    wr::<u8>(rd::<GPtr<u8>>(player, 0x480), 0x41, 0);
    mblur_clear_drunk_blur();
    wr::<u8>(rd::<GPtr<u8>>(player, 0x480), 0x42, 0);
    player_ped_clear_adrenaline(player);
    player_ped_reset_sprint_energy(player);
    let fire = rd::<GPtr<u8>>(player, 0x730);
    if !fire.is_null() {
        fire.write(fire.read() & 0xFD);
        fire_extinguish(rd::<GPtr<u8>>(player, 0x730));
        wr::<u32>(player, 0x730, 0);
    }
    ped_audio_turn_off_jet_pack(player.byte_add(0x138));
    wr::<u32>(player, 0x46C, rd::<u32>(player, 0x46C) & 0xFFFF_FEFF);
    let vref = player.byte_add(0x58C);
    let v = rd::<GPtr<u8>>(player, 0x58C);
    if !v.is_null() {
        clean_up_old_reference(v, vref);
    }
    wr::<u32>(player, 0x58C, 0);
    let pd = rd::<GPtr<u8>>(player, 0x480);
    wanted_reset(if pd.is_null() { GPtr::from_addr(0) } else { rd::<GPtr<u8>>(pd, 0) });
    ped_restart_non_partial_anims(player);
    player_info_make_player_safe(info, 0, 10000.0);
    wr::<u32>(player, 0x1C, rd::<u32>(player, 0x1C) & 0xFFFF_F7FF);
    player_ped_clear_weapon_target(player);
    player_ped_set_initial_state(player, 0);
    let acq = get_ped_type_acquaintances(rd::<u32>(player, 0x598));
    for k in 0..5isize {
        wr::<u32>(player, 0x4E0 + 4 * k, rd::<u32>(acq, 4 * k));
    }
    car_ctrl_clear_interesting_vehicle_list();
    let y = x(f32::from_bits(py)).to_f32().to_bits();
    let z1 = x(f32::from_bits(pz)).fadd_m(1.0).to_f32().to_bits();
    v_teleport(player, px, y, z1, 0);
    let h = x(heading).fmul_m(f32::from_bits(0x3C8E_FA35)).to_f32();
    for o in [0x44, 0x48, 0x4C] {
        wr::<u32>(player, o, 0);
    }
    wr::<f32>(player, 0x558, h);
    wr::<f32>(player, 0x55C, h);
    let m = rd::<GPtr<u8>>(player, 0x14);
    if m.is_null() {
        wr::<f32>(player, 0x10, h);
    } else {
        matrix_set_rotate_z_only(m, h.to_bits());
    }
    let mut pos = [px, py, pz];
    let pp: GPtr<u8> = GPtr::from_raw(&mut pos as *mut [u32; 3]).cast();
    clear_space_for_mission_entity(pp, player);
    clear_exciting_stuff_from_area(pp, 4000.0, 1);
    ped_restore_heading_rate(player);
    wr::<u8>(player, 0x2F, 0);
    wr::<u32>(player, 0x78C, 0);
    g::<u32>(0x96_A7C4).write(0);
    g::<u32>(0xB7_2914).write(0);
    g::<u8>(0xC0_FCB6).write(0);
    remove_buildings_not_in_area(0);
    camera_set_directly_behind(cam);
    camera_restore_with_jump_cut(cam);
    references_remove_references_to_player();
    garages_player_arrested_or_died();
    set_stat_value(0xB0, 0);
    shopping_remove_loaded_shop();
    world_remove(player);
    world_add(player);
    hud_reset_wasted_text();
    stream_zone_models(pp);
    g::<u8>(0xC4_02D3).write(0);
    time_cycle_stop_extra_colour(0);
    post_effects_script_reset_for_effects();
    let cd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(player, 0x480), 4);
    if rd::<u32>(cd, 0x24) != 0 {
        clothes_desc_set_texture_and_model(cd, 0, 0, 0x11);
        clothes_rebuild_player(player, 0);
    } else {
        clothes_rebuild_player_if_needed(player);
    }
    audio_zones_update(1, pos[0], pos[1], pos[2]);
    audio_engine_stop_police_scanner(GPtr::from_addr(0xB6_BC90), 1);
    water_level_find_nearest_water_and_its_flow();
    g::<u32>(0xB6_F10C).write(0x47C3_4FF3);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RESTORE_PLAYER_STUFF_DURING_RESURRECTION];
