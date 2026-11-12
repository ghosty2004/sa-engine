//! WRITER-MVP-D (helper-world) WORLD ports, batch `mvpd-world-4`: `CGameLogic::Update`. Ported from the 1.0 US
//! bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CStats::UpdateRespectStat(uint8)` (0x55BC50, cdecl)
    fn stats_update_respect_stat(b: u32) = cdecl 0x55BC50 => unported;
    /// `CStats::UpdateSexAppealStat()` (0x55BF20, cdecl)
    fn stats_update_sex_appeal_stat() = cdecl 0x55BF20 => unported;
    /// `CGameLogic::SetPlayerWantedLevelForForbiddenTerritories(bool)` (0x441770, cdecl)
    fn set_player_wanted_level_for_forbidden_territories(b: u32) = cdecl 0x441770 => unported;
    /// `CGameLogic::UpdateSkip()` (0x442480, cdecl)
    fn game_logic_update_skip() = cdecl 0x442480 => crate::world::mvpd_w2::game_logic_update_skip;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0, cdecl)
    fn hud_set_help_message(s: GPtr<u8>, a: u32, b: u32, c: u32) = cdecl 0x588BE0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_manager_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CPlayerPed::ClearAdrenaline()` (0x60A070)
    fn player_ped_clear_adrenaline(p: GPtr<u8>) = thiscall 0x60A070 => unported;
    /// `CPlayerInfo::KillPlayer()` (0x56E580)
    fn player_info_kill_player(i: GPtr<u8>) = thiscall 0x56E580 => unported;
    /// `CPlayerInfo::ArrestPlayer()` (0x56E5D0)
    fn player_info_arrest_player(i: GPtr<u8>) = thiscall 0x56E5D0 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CMessages::AddBigMessage(const char*, uint32, eMessageStyle)` (0x69F2B0, cdecl)
    fn messages_add_big_message(s: GPtr<u8>, t: u32, style: u32) = cdecl 0x69F2B0 => unported;
    /// `CWeaponEffects::ClearCrossHairs()` (0x742C80, cdecl)
    fn weapon_effects_clear_cross_hairs() = cdecl 0x742C80 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00, `ret 0xC`)
    fn camera_set_fade_colour(c: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20, `ret 8`)
    fn camera_fade(c: GPtr<u8>, t: f32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CPed::ClearWeapons()` (0x5E6320)
    fn ped_clear_weapons(p: GPtr<u8>) = thiscall 0x5E6320 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, `ret 4`)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CMessages::ClearMessages(bool)` (0x69DCD0, cdecl)
    fn messages_clear_messages(b: u32) = cdecl 0x69DCD0 => unported;
    /// `CCarCtrl::ClearInterestingVehicleList()` (0x423F00, cdecl)
    fn car_ctrl_clear_interesting_vehicle_list() = cdecl 0x423F00 => unported;
    /// `CGarages::CloseHideOutGaragesBeforeSave()` (0x44A170, cdecl)
    fn garages_close_hide_out_garages_before_save() = cdecl 0x44A170 => unported;
    /// `CPlayerInfo::GetPos(CVector* out)` (0x56DFB0, `ret 4`); returns `out`
    fn player_info_get_pos(i: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x56DFB0 => unported;
    /// `CWorld::ClearExcitingStuffFromArea(const CVector&, float, bool)` (0x56A0D0, cdecl)
    fn world_clear_exciting_stuff_from_area(p: GPtr<u8>, r: f32, b: u8) = cdecl 0x56A0D0 => crate::world::mvpd_w3::world_clear_exciting_stuff_from_area;
    /// `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, `ret 4`)
    fn entry_exit_get_position_relative_to_outside_world(ee: GPtr<u8>, p: GPtr<u8>) = thiscall 0x43EA00 => unported;
    /// `CRestart::FindClosestHospitalRestartPoint(CVector, CVector* out, float* heading)` (0x460850, cdecl)
    fn restart_find_closest_hospital(x: f32, y: f32, z: f32, out: GPtr<u8>, h: GPtr<u8>) = cdecl 0x460850 => unported;
    /// `CRestart::FindClosestPoliceRestartPoint(CVector, CVector* out, float* heading)` (0x460A50, cdecl)
    fn restart_find_closest_police(x: f32, y: f32, z: f32, out: GPtr<u8>, h: GPtr<u8>) = cdecl 0x460A50 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CGameLogic::RestorePedsWeapons(CPed*)` (0x441D30, cdecl)
    fn game_logic_restore_peds_weapons(p: GPtr<u8>) = cdecl 0x441D30 => unported;
    /// `CGameLogic::Remove2ndPlayerIfPresent()` (0x4413C0, cdecl)
    fn game_logic_remove_2nd_player_if_present() = cdecl 0x4413C0 => unported;
    /// `CGameLogic::PassTime(uint32 minutes)` (0x4414C0, cdecl)
    fn game_logic_pass_time(m: u32) = cdecl 0x4414C0 => unported;
    /// `CGameLogic::RestorePlayerStuffDuringResurrection(CPlayerPed*, CVector, float)` (0x442060, cdecl)
    fn game_logic_restore_player_stuff(p: GPtr<u8>, x: f32, y: f32, z: f32, h: f32) = cdecl 0x442060 => unported;
    /// `CGameLogic::SortOutStreamingAndMemory(const CVector&, float)` (0x441440, cdecl)
    fn game_logic_sort_out_streaming_and_memory(p: GPtr<u8>, h: f32) = cdecl 0x441440 => unported;
    /// `CCamera::SetMotionBlur(int32 r, g, b, a, type)` (0x50BF40, `ret 0x14`)
    fn camera_set_motion_blur(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, t: u32) = thiscall 0x50BF40 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StopShaking(int16)` (0x53FB50, `ret 4`)
    fn pad_stop_shaking(p: GPtr<u8>, n: u32) = thiscall 0x53FB50 => unported;
    /// `CReferences::RemoveReferencesToPlayer()` (0x571AD0, cdecl)
    fn references_remove_references_to_player() = cdecl 0x571AD0 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50, cdecl)
    fn the_scripts_is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180, cdecl)
    fn general_get_random_number_in_range_int(a: i32, b: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CGameLogic::StopPlayerMovingFromDirection(int32, CVector)` (0x441290, cdecl)
    fn game_logic_stop_player_moving_from_direction(id: i32, x: f32, y: f32, z: f32) = cdecl 0x441290 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const NOW: usize = 0xB7CB84;
/// `CGameLogic::GameState` (0x96A8B0): 0 normal, 1 wasted, 2 busted, 3/4 respawn
const GAME_STATE: usize = 0x96A8B0;
/// `CGameLogic::TimeOfLastEvent` (0x96A8AC)
const LAST_EVENT: usize = 0x96A8AC;

/// The entity position (matrix +0x30 or simple transform +4).
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The fade-out the three resurrection states start 3 s after the event, while the previous frame was still
/// within those 3 s (`CTimer::m_snPreviousTimeInMilliseconds` 0xB7CB78).
fn fade_out_once() {
    let t = g::<u32>(LAST_EVENT).read();
    if g::<u32>(0xB7CB78).read().wrapping_sub(t) <= 0xBB8 {
        let cam = GPtr::<u8>::from_addr(0xB6F028);
        camera_set_fade_colour(cam, 0, 0, 0);
        camera_fade(cam, 2.0, 0);
    }
}

/// Takes the player out of his vehicle: as the driver (reference cleaned, driver cleared, the vehicle's status
/// set to abandoned unless wrecked) or as a passenger (`RemovePassenger`).
fn take_player_out_of_vehicle() {
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    if rd::<u32>(p0, 0x46C) & 0x100 == 0 {
        return;
    }
    let veh = rd::<GPtr<u8>>(p0, 0x58C);
    if veh.is_null() {
        return;
    }
    let drv = rd::<GPtr<u8>>(veh, 0x460);
    if drv != p0 {
        vehicle_remove_passenger(veh, p0);
        return;
    }
    if !drv.is_null() {
        entity_clean_up_old_reference(drv, veh.byte_add(0x460));
    }
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    wr::<u32>(rd::<GPtr<u8>>(p0, 0x58C), 0x460, 0);
    let v = rd::<GPtr<u8>>(g::<GPtr<u8>>(0xB7CD98).read(), 0x58C);
    let s = rd::<u8>(v, 0x36);
    if s & 0xF8 != 0x28 {
        wr::<u8>(v, 0x36, (s & 7) | 0x20);
    }
}

/// The resurrection shared by the three states after the vehicle exit and messages: clear the area around the
/// player (4000 units), find the restart point (hospital or police) from the player's outside-world position,
/// the coop restart point and the player's weapons in a two-player game, the second player removed,
/// optionally `PassTime(720)` and the player info's +0x144 cleared, the player restored there, streaming
/// sorted out for the player's heading (`atan2(-forward.x, forward.y)`, `fpatan`), motion blur off, pad
/// shaking stopped, references removed, 0x9690C0 / 0xC0EC23 = 10 and the current pad's +0x10E cleared.
fn resurrect(hospital: bool, pass_time: bool, clear_144: bool) {
    let mut tmp = [0f32; 3];
    let pos = player_info_get_pos(GPtr::from_addr(0xB7CD98), GPtr::from_raw(&mut tmp).cast());
    world_clear_exciting_stuff_from_area(pos, 4000.0, 1);
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    let pp = ent_pos(p0);
    let mut v = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let ee = rd::<GPtr<u8>>(p0, 0x78C);
    if !ee.is_null() {
        entry_exit_get_position_relative_to_outside_world(ee, GPtr::from_raw(&mut v).cast());
    }
    let mut r = [0f32; 3];
    let mut heading = 0f32;
    if hospital {
        restart_find_closest_hospital(v[0], v[1], v[2], GPtr::from_raw(&mut r).cast(), GPtr::from_raw(&mut heading).cast());
    } else {
        restart_find_closest_police(v[0], v[1], v[2], GPtr::from_raw(&mut r).cast(), GPtr::from_raw(&mut heading).cast());
    }
    if !g::<GPtr<u8>>(0xB7CD98).read().is_null() && !g::<GPtr<u8>>(0xB7CF28).read().is_null() {
        heading = x(g::<f32>(0x96A840).read()).fmul_m(g::<f32>(0x859878).read()).to_f32();
        r = [g::<f32>(0x96A9AC).read(), g::<f32>(0x96A9B0).read(), g::<f32>(0x96A9B4).read()];
        game_logic_restore_peds_weapons(find_player_ped(0));
    }
    game_logic_remove_2nd_player_if_present();
    if pass_time {
        game_logic_pass_time(0x2D0);
    }
    if clear_144 {
        let idx = g::<u8>(0xB7CD74).read() as usize;
        g::<u16>(0xB7CEDC + 0x190 * idx).write(0);
    }
    game_logic_restore_player_stuff(g::<GPtr<u8>>(0xB7CD98).read(), r[0], r[1], r[2], heading);
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    let m = rd::<GPtr<u8>>(p0, 0x14);
    let h = if m.is_null() {
        rd::<f32>(p0, 0x10)
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))).to_f32()
    };
    game_logic_sort_out_streaming_and_memory(ent_pos(p0), h);
    g::<u32>(0xB6F154).write(0);
    camera_set_motion_blur(GPtr::from_addr(0xB6F028), 0, 0, 0, 0, 0);
    pad_stop_shaking(pad_get_pad(0), 0);
    references_remove_references_to_player();
    g::<u8>(0x9690C0).write(10);
    g::<u8>(0xC0EC23).write(10);
    let pad = pad_get_pad(g::<u8>(0xB7CD74).read() as i32);
    wr::<u16>(pad, 0x10E, 0);
}

/// The fade-in after a resurrection (black, 3 s, in).
fn fade_in() {
    let cam = GPtr::<u8>::from_addr(0xB6F028);
    camera_set_fade_colour(cam, 0, 0, 0);
    camera_fade(cam, 3.0, 1);
}

/// `CGameLogic::Update()` (0x442AD0, cdecl).
///
/// Respect / sex appeal stats and the forbidden-territory wanted level update; nothing else during cutscene
/// processing. With one player the skip logic runs; with two, their 2D distance (`fsqrt`, stored as the
/// previous distance 0x96AB24) drives the coop state (> 100: state 3 once), help texts when moving apart
/// (above 85 and above 0x8A5E4C, at most 5 of the latter, timer 0x96A8B4 refreshed to 30 s unless 0x8A5E50 is
/// 2). Each player who died (`+0x530` 0x37, or 0x36 with `+0x478` bit 5) is killed (state 1), arrested ones
/// (0x3F) are arrested with speech 0xF (state 2; big message for the second player unless 0xBAADC0). Then
/// wasted (1), busted (2) and respawn (3/4) states run their resurrection after 3 s (busted after 4 s, with the
/// mission-less "bribe" draw on 0xB7CEEC/0xB7CEEE), charging hospital (100) or wanted-level fines (100..1500)
/// and removing weapons in single player unless free (0xB7CEEA / 0xB7CEE9). Finally, in coop with 0x8A5E4A,
/// players further apart than 0x8A5E4C are stopped from moving away from each other.
#[hook(addr = 0x442AD0, conv = cdecl, name = "CGameLogic::Update", expect = "83 EC 4C 53 55 56 57",
       route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "fsqrt distances are stored (0x96AB24); fpatan heading feeds SortOutStreamingAndMemory")]
pub fn game_logic_update() {
    stats_update_respect_stat(0);
    stats_update_sex_appeal_stat();
    set_player_wanted_level_for_forbidden_territories(0);
    if g::<u8>(0xB5F852).read() != 0 {
        return;
    }
    let text = GPtr::<u8>::from_addr(0xC1B340);
    let now = || g::<u32>(NOW).read();
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    let p1 = g::<GPtr<u8>>(0xB7CF28).read();
    if p0.is_null() || p1.is_null() {
        game_logic_update_skip();
    } else {
        let a = ent_pos(p0);
        let b = ent_pos(p1);
        let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
        let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let help_timer = g::<u32>(0x96A8B4);
        'msg: {
            if !x(d).fcom(x(g::<f32>(0x858628).read())).test_any(0x41) {
                if g::<u8>(GAME_STATE).read() != 3 {
                    g::<u8>(GAME_STATE).write(3);
                    g::<u32>(LAST_EVENT).write(now());
                }
                break 'msg;
            }
            let k85 = g::<f32>(0x85989C).read();
            if !x(d).fcom(x(k85)).test_any(0x41)
                && !x(g::<f32>(0x96AB24).read()).fcom(x(k85)).test_parity(0x41)
            {
                hud_set_help_message(text_get(text, GPtr::from_addr(0x859894)), 0, 0, 0);
                break 'msg;
            }
            let kfar = g::<f32>(0x8A5E4C).read();
            if x(d).fcom(x(kfar)).test_any(0x41) || g::<u32>(0x96A8B8).read() >= 5 {
                break 'msg;
            }
            if !x(g::<f32>(0x96AB24).read()).fcom(x(kfar)).test_parity(0x41) {
                if now() <= help_timer.read() {
                    break 'msg;
                }
                hud_set_help_message(text_get(text, GPtr::from_addr(0x85988C)), 0, 0, 0);
                help_timer.write(now().wrapping_add(0x3A98));
            } else {
                if now() <= help_timer.read() {
                    break 'msg;
                }
                hud_set_help_message(text_get(text, GPtr::from_addr(0x859884)), 0, 0, 0);
                help_timer.write(now().wrapping_add(0xEA60));
            }
            let c = g::<u32>(0x96A8B8);
            c.write(c.read().wrapping_add(1));
        }
        if g::<u32>(0x8A5E50).read() != 2 && help_timer.read() != 0 {
            help_timer.write(now().wrapping_add(0x7530));
        }
        g::<f32>(0x96AB24).write(d);
    }

    for i in 0..2usize {
        let info = GPtr::<u8>::from_addr(0xB7CD98 + 0x190 * i);
        let ped = rd::<GPtr<u8>>(info, 0);
        if ped.is_null() || rd::<u8>(info, 0xDC) != 0 {
            continue;
        }
        task_manager_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        let ped = rd::<GPtr<u8>>(info, 0);
        let st = rd::<u32>(ped, 0x530);
        if st == 0x37 || (st == 0x36 && rd::<u8>(ped, 0x478) & 0x20 != 0) {
            player_ped_clear_adrenaline(ped);
            player_info_kill_player(info);
            g::<u8>(GAME_STATE).write(1);
            g::<u32>(LAST_EVENT).write(now());
        }
        let ped = rd::<GPtr<u8>>(info, 0);
        if rd::<u32>(ped, 0x530) == 0x3F {
            player_ped_clear_adrenaline(ped);
            player_info_arrest_player(info);
            ped_say(rd::<GPtr<u8>>(info, 0), 0xF, 0x8FC, 1.0, 1, 1, 1);
            g::<u8>(GAME_STATE).write(2);
            g::<u32>(LAST_EVENT).write(now());
            if i == 1 && g::<u8>(0xBAADC0).read() == 0 {
                messages_add_big_message(text_get(text, GPtr::from_addr(0x85987C)), 0x1388, 2);
            }
        }
    }

    match g::<u8>(GAME_STATE).read() {
        1 => 'w: {
            weapon_effects_clear_cross_hairs();
            if now().wrapping_sub(g::<u32>(LAST_EVENT).read()) <= 0xBB8 {
                break 'w;
            }
            fade_out_once();
            if g::<u8>(0xB6F079).read() != 0 {
                break 'w;
            }
            let p0 = g::<GPtr<u8>>(0xB7CD98).read();
            if (p0.is_null() || g::<GPtr<u8>>(0xB7CF28).read().is_null()) && g::<u8>(0x8A5E48).read() != 0 {
                if g::<u8>(0xB7CEEA).read() != 0 {
                    g::<u8>(0xB7CEEA).write(0);
                } else {
                    let m = g::<i32>(0xB7CE50);
                    if m.read() > 0 {
                        m.write((m.read() - 100).max(0));
                    }
                    ped_clear_weapons(p0);
                }
            }
            g::<u8>(0xB7CE74).write(0);
            g::<u8>(GAME_STATE).write(0);
            take_player_out_of_vehicle();
            messages_clear_messages(0);
            hud_set_help_message(GPtr::NULL, 1, 0, 0);
            car_ctrl_clear_interesting_vehicle_list();
            garages_close_hide_out_garages_before_save();
            resurrect(true, true, true);
            if g::<u8>(0xA4325D).read() != 0 {
                fade_in();
            } else {
                g::<u8>(0xA4325D).write(1);
            }
        }
        2 => 'b: {
            weapon_effects_clear_cross_hairs();
            let elapsed = now().wrapping_sub(g::<u32>(LAST_EVENT).read());
            if elapsed > 0xBB8 {
                fade_out_once();
            }
            if the_scripts_is_player_on_a_mission() == 0 && g::<u8>(0xB7CEEC).read() == 0 {
                if general_get_random_number_in_range_int(0, 4) == 0 {
                    let n = g::<u16>(0xB7CEEE).read() as i32;
                    g::<u8>(0xB7CEEC).write(1);
                    g::<u16>(0xB7CEEE).write((n % 0x1C + 1) as u16);
                } else {
                    g::<u8>(0xB7CEEC).write(2);
                }
            }
            if elapsed < 0xFA0 || g::<u8>(0xB6F079).read() != 0 {
                break 'b;
            }
            let p0 = g::<GPtr<u8>>(0xB7CD98).read();
            g::<u8>(0xB7CE74).write(0);
            g::<u8>(GAME_STATE).write(0);
            if p0.is_null() || g::<GPtr<u8>>(0xB7CF28).read().is_null() {
                let lvl = rd::<u32>(g::<GPtr<u8>>(0xB7CD9C).read(), 0x2C);
                let fine = match lvl {
                    2 => 200,
                    3 => 400,
                    4 => 600,
                    5 => 900,
                    6 => 1500,
                    _ => 100,
                };
                if g::<u8>(0x8A5E49).read() != 0 {
                    if g::<u8>(0xB7CEE9).read() != 0 {
                        g::<u8>(0xB7CEE9).write(0);
                    } else {
                        let m = g::<i32>(0xB7CE50);
                        if m.read() > 0 {
                            m.write((m.read() - fine).max(0));
                        }
                        ped_clear_weapons(p0);
                    }
                }
            }
            take_player_out_of_vehicle();
            messages_clear_messages(0);
            hud_set_help_message(GPtr::NULL, 1, 0, 0);
            car_ctrl_clear_interesting_vehicle_list();
            garages_close_hide_out_garages_before_save();
            resurrect(false, true, false);
            if g::<u8>(0xA4325C).read() != 0 {
                fade_in();
            } else {
                g::<u8>(0xA4325C).write(1);
            }
        }
        3 | 4 => 'r: {
            if now().wrapping_sub(g::<u32>(LAST_EVENT).read()) <= 0xBB8 {
                break 'r;
            }
            fade_out_once();
            if g::<u8>(0xB6F079).read() != 0 {
                break 'r;
            }
            g::<u8>(0xB7CE74).write(0);
            g::<u8>(GAME_STATE).write(0);
            take_player_out_of_vehicle();
            messages_clear_messages(0);
            car_ctrl_clear_interesting_vehicle_list();
            garages_close_hide_out_garages_before_save();
            resurrect(false, false, false);
            fade_in();
        }
        _ => {}
    }

    if g::<u8>(0x8A5E4A).read() == 0 {
        return;
    }
    let p0 = g::<GPtr<u8>>(0xB7CD98).read();
    let p1 = g::<GPtr<u8>>(0xB7CF28).read();
    if p0.is_null() || p1.is_null() {
        return;
    }
    let a = ent_pos(p0);
    let b = ent_pos(p1);
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    if d.fcom(x(g::<f32>(0x8A5E4C).read())).test_any(0x41) {
        return;
    }
    let mut dir = [
        x(rd::<f32>(b, 0)).fsub_m(rd::<f32>(a, 0)).to_f32(),
        x(rd::<f32>(b, 4)).fsub_m(rd::<f32>(a, 4)).to_f32(),
        x(rd::<f32>(b, 8)).fsub_m(rd::<f32>(a, 8)).to_f32(),
    ];
    let dp = GPtr::from_raw(&mut dir).cast::<u8>();
    vector_normalise(dp);
    let (nx, ny, nz) = (rd::<f32>(dp, 0), rd::<f32>(dp, 4), rd::<f32>(dp, 8));
    game_logic_stop_player_moving_from_direction(0, nx, ny, nz);
    game_logic_stop_player_moving_from_direction(1, x(nx).fchs().to_f32(), x(ny).fchs().to_f32(), x(nz).fchs().to_f32());
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GAME_LOGIC_UPDATE];
