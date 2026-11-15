//! FRONTIER-2 turn 2, lane B (batch `frontier2-b5`): world functions on the phase B frontier: garages, pickups,
//! replay, game logic. Ported from the 1.0 US bytes (the `.HOODLUM` bodies followed through the no-CD protection's
//! obfuscated cells and calls, evaluated on the exe image); gta-reversed as a reading aid only (the bytes win).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::{c_str_eq, copy_c_string};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{entity_pos, g, placeable_heading, rd, wr, x};
use super::mvpb_world::{advance_scan_code, cell, scan_rect, sector_index};
use super::mvpd_w1::replay_trigger_playback;
use super::mvpd_w4::{game_logic_restore_player_stuff, game_logic_sort_out_streaming_and_memory, pad_stop_shaking};
use crate::bindings::{
    acquaintance_ped_ctor, add_explosion, automobile_place_on_road_properly, bike_place_on_road_properly,
    camera_set_motion_blur, car_ctrl_clear_interesting_vehicle_list, col_line_ctor, entity_get_col_model,
    event_acquaintance_ped_dtor, event_group_add, find_player_ped, garage_is_entity_touching_3d,
    garage_is_point_inside_r, get_ped_flag, grant_ammo, mark_everything_as_new, pickup_remove, player_info_get_pos,
    set_as_acquaintance, stored_car_restore_car, test_line_sphere, weapon_for_model, world_add,
};
use crate::bindings::{
    automobile_set_panel_damage, damage_get_door_status, dm_get_panel_status, dm_set_door_status,
    dm_set_panel_status, dm_set_wheel_status, find_player_vehicle, get_wheel_status, remove_vehicle_upgrade,
    set_door_damage, vehicle_setup_upgrades_after_load,
};
use sa_core::bindings::{load_all_requested_models, set_model_is_deletable};
use sa_stream::bindings::request_vehicle_upgrade;
use sa_core::bindings::matrix_mul_vec;
use sa_core::bindings::{
    audio_report_frontend_event, audio_set_effects_master_volume, audio_set_music_master_volume,
    clear_exciting_stuff_from_area, file_close, file_read, file_set_dir, file_set_dir_my_documents, file_write,
    game_logic_pass_time, get_pad, get_weapon_info, messages_clear_messages, open_file, open_file_for_writing,
    ped_clear_weapons, ped_give_weapon, references_remove_references_to_player, strncmp,
};
use sa_core::field::addr32;
use sa_stream::bindings::ped_is_player;

orig! {
    /// `CReplay::PlayBackThisFrameInterpolation(CAddressInReplayBuffer&, float, uint32*)` (0x45F380, AL)
    fn replay_play_back_this_frame_interpolation(buf: GPtr<u8>, interp: f32, out: u32) -> u8 = cdecl 0x45F380 => unported;
    /// `CReplay::ProcessLookAroundCam()` (0x45D760)
    fn replay_process_look_around_cam() = cdecl 0x45D760 => unported;
    /// `CReplay::StreamAllNecessaryCarsAndPeds()` (0x45D4B0)
    fn replay_stream_all_necessary_cars_and_peds() = cdecl 0x45D4B0 => unported;
    /// `CPlayerPed::PickWeaponAllowedFor2Player()` (0x609800)
    fn player_ped_pick_weapon_allowed_for_2_player(p: GPtr<u8>) = thiscall 0x609800 => unported;
}

use super::garage_update::now as time_ms;

/// `AudioEngine` (0xB6BC90).
fn audio_engine() -> GPtr<u8> {
    GPtr::from_addr(0xB6_BC90)
}

/// `CGarages::TriggerMessage(const char* tag, int16 n1, uint16 time, int16 n2)` (stub 0x447B80 → .HOODLUM 0x156CC60,
/// cdecl). When `tag` differs from `MessageIDString` (0x96C014, inlined strcmp) or the time (0xB7CB84) lies outside
/// [`MessageStartTime` 0x96C020, `MessageEndTime` 0x96C01C] (unsigned; the end inclusive, gta-reversed excludes it):
/// the tag copied into the string the protection VM 0xCB73E0 returns (descriptor 0x1560058; evaluated on the exe
/// image: 0x96C014) and the start = now. Otherwise, after 500 ms or less nothing changes; later the start (written
/// through the cell decoded from 0x721DC6 ^ 0x1560584 = 0x96C020) = now - 500. Then end = start + time, the numbers
/// 0x96C010 = n1 and 0x96C00C = n2 (sign-extended).
#[hook(addr = 0x447B80, conv = cdecl, name = "CGarages::TriggerMessage", expect = "E9 DB 50 12 01",
       unverified = "frontier2-b5", route = "L2", hazards = "none",
       note = "VM 0xCB73E0 (descriptor 0x1560058) evaluated: the copy destination 0x96C014")]
pub fn garages_trigger_message(tag: GPtr<u8>, n1: u32, time: u32, n2: u32) {
    let id = GPtr::<u8>::from_addr(0x96_C014);
    let now = time_ms();
    let start = g::<u32>(0x96_C020);
    let end = if !c_str_eq(tag, id) || now < start.read() || now > g::<u32>(0x96_C01C).read() {
        copy_c_string(id, tag);
        start.write(now);
        (time & 0xFFFF).wrapping_add(now)
    } else {
        if now.wrapping_sub(start.read()) <= 500 {
            return;
        }
        let s = now.wrapping_sub(500);
        start.write(s);
        (time & 0xFFFF).wrapping_add(s)
    };
    g::<u32>(0x96_C01C).write(end);
    g::<i32>(0x96_C010).write(i32::from(n1 as i16));
    g::<i32>(0x96_C00C).write(i32::from(n2 as i16));
}

/// `CGarages::GivePlayerDetonator()` (stub 0x448660 → .HOODLUM 0x1569B90, cdecl): the player ped gets the detonator
/// (weapon 40, 1 round, `GiveWeapon(.., 1)`); the slot of `GetWeaponInfo(40, 1)` (+0x14): its weapon's +4 (+0x5A4 +
/// 0x1C * slot) = 0, the player data's (+0x480) chosen slot (+0x20) = slot; the ped's +0x70C = 40 unless it is 55.
#[hook(addr = 0x448660, conv = cdecl, name = "CGarages::GivePlayerDetonator", expect = "E9 2B 15 12 01",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn garages_give_player_detonator() {
    let ped = find_player_ped(-1);
    let slot = rd::<u32>(get_weapon_info(0x28, 1), 0x14);
    ped_give_weapon(ped, 0x28, 1, 1);
    wr::<u32>(ped, (slot.wrapping_mul(0x1C) as isize).wrapping_add(0x5A4), 0);
    wr::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x20, slot as u8);
    if rd::<u32>(ped, 0x70C) != 0x37 {
        wr::<u32>(ped, 0x70C, 0x28);
    }
}

/// `CGameLogic::RestorePedsWeapons(CPed*)` (stub 0x441D30 → .HOODLUM 0x1560F90, cdecl): `ClearWeapons`, then for the
/// 13 saved weapons (0x96A9B8, 0x1C each: +0 type, +0xC ammo) the weapon is given back (`GiveWeapon(type, ammo, 1)`)
/// unless its first model (`GetWeaponInfo(type, 1)` +0xC) is set and not loaded (`CStreaming::ms_aInfoForModel`
/// 0x8E4CD0, 20 bytes, +0 load state == 1), or its second model (+0x10) is set and the first model's state (the
/// original indexes with the first model again; gta-reversed checks the second) is not loaded.
#[hook(addr = 0x441D30, conv = cdecl, name = "CGameLogic::RestorePedsWeapons", expect = "E9 5B F2 11 01",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn game_logic_restore_peds_weapons(ped: GPtr<u8>) {
    ped_clear_weapons(ped);
    let loaded = |m: i32| g::<u8>((0x8E_4CD0 + 20 * m as isize) as usize).read() == 1;
    let mut e = 0x96_A9B8usize;
    while e < 0x96_AB24 {
        let w = GPtr::<u8>::from_addr(e);
        let m1 = rd::<i32>(get_weapon_info(rd(w, 0), 1), 0xC);
        let m2 = rd::<i32>(get_weapon_info(rd(w, 0), 1), 0x10);
        if (m1 == -1 || loaded(m1)) && (m2 == -1 || loaded(m1)) {
            ped_give_weapon(ped, rd(w, 0), rd(w, 0xC), 1);
        }
        e += 0x1C;
    }
}

/// `CPickup::ExtractAmmoFromPickup(CPlayerPed*)` (stub 0x454BE0 → .HOODLUM 0x156DB40, thiscall, `ret 4`): the weapon
/// of the pickup object's model (`CPickups::WeaponForModel`); nothing for pickup type 1 or a weapon slot
/// (`GetWeaponInfo(w, 1)` +0x14) other than 3, 4, 5. The ammo is the pickup's (+8), else (bit 1 of +0x1D clear) the
/// on-street ammo of the weapon (word table 0x8A5F50); `GrantAmmo(w, ammo)` (an obfuscated call: target decoded from
/// 0x5026D3 ^ 0x15605AC = 0x5DF220), the frontend audio event 6 (volume 0, speed 1.0); then bit 1 of +0x1D set and
/// +8 cleared (also when the ammo was already taken).
#[hook(addr = 0x454BE0, conv = thiscall, name = "CPickup::ExtractAmmoFromPickup", expect = "E9 5B 8F 11 01",
       this = "CPickup", unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn pickup_extract_ammo_from_pickup(this: GPtr<u8>, ped: GPtr<u8>) {
    let w = weapon_for_model(i32::from(rd::<i16>(rd::<GPtr<u8>>(this, 4), 0x22)));
    let slot = rd::<u32>(get_weapon_info(w, 1), 0x14);
    if rd::<u8>(this, 0x1C) == 1 {
        return;
    }
    if slot != 3 && slot != 4 && slot != 5 {
        return;
    }
    let ammo = rd::<u32>(this, 8);
    let give = if ammo != 0 {
        Some(ammo)
    } else if rd::<u8>(this, 0x1D) & 2 != 0 {
        None
    } else {
        Some(u32::from(g::<u16>((0x8A_5F50 + 2 * w as isize) as usize).read()))
    };
    if let Some(a) = give {
        grant_ammo(ped, w as u32, a);
        audio_report_frontend_event(audio_engine(), 6, 0.0, 1.0);
    }
    wr::<u32>(this, 8, 0);
    wr::<u8>(this, 0x1D, rd::<u8>(this, 0x1D) | 2);
}

/// `CReplay::GoToNextBlock()` (0x45E2A0, cdecl): the end marker (0) at the record position (`Record` 0x97FB70 offset
/// into buffer 0x97FB74); the current slot (byte 0x97FB78) marked 1 in `BufferStatus` (0x97FB7C), the next slot
/// (`(slot + 1) % 8`) marked 2 and made current: its buffer `0x97FB88 + 100000 * slot`, offset 0, first byte 0; then
/// `MarkEverythingAsNew` (tail jump).
#[hook(addr = 0x45E2A0, conv = cdecl, name = "CReplay::GoToNextBlock", expect = "A1 74 FB 97 00",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn replay_go_to_next_block() {
    let buf = g::<u32>(0x97_FB74);
    let off = g::<u32>(0x97_FB70);
    GPtr::<u8>::from_addr(off.read().wrapping_add(buf.read()) as usize).write(0);
    let slot = g::<u8>(0x97_FB78);
    g::<u8>(0x97_FB7C + slot.read() as usize).write(1);
    let next = (slot.read().wrapping_add(1)) & 7;
    slot.write(next);
    g::<u8>(0x97_FB7C + next as usize).write(2);
    let b = 0x97_FB88u32.wrapping_add(u32::from(next).wrapping_mul(100_000));
    buf.write(b);
    off.write(0);
    GPtr::<u8>::from_addr(b as usize).write(0);
    mark_everything_as_new();
}

/// `CReplay::PlayBackThisFrame()` (stub 0x4604A0 → .HOODLUM 0x1562050, cdecl): `PlayBackThisFrameInterpolation(
/// Playback 0x97FB64, 1.0, null)`; when it returns true the effects and music master volumes are set back to the
/// menu's (bytes 0xBA6797 / 0xBA6798), else `ProcessLookAroundCam` and both volumes 0.
#[hook(addr = 0x4604A0, conv = cdecl, name = "CReplay::PlayBackThisFrame", expect = "E9 AB 1B 10 01",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn replay_play_back_this_frame() {
    if replay_play_back_this_frame_interpolation(GPtr::from_addr(0x97_FB64), 1.0, 0) != 0 {
        audio_set_effects_master_volume(audio_engine(), u32::from(g::<u8>(0xBA_6797).read()));
        audio_set_music_master_volume(audio_engine(), u32::from(g::<u8>(0xBA_6798).read()));
    } else {
        replay_process_look_around_cam();
        audio_set_effects_master_volume(audio_engine(), 0);
        audio_set_music_master_volume(audio_engine(), 0);
    }
}

/// The hideout garage's restore pass (stub 0x4485C0 → .HOODLUM 0x1561490, thiscall, `ret 4`, AL; unnamed, called by
/// `CGarage::Update`; `RestoreCarsForThisHideOut` in the mobile symbols): for the first three 0x40-byte stored cars
/// whose model word (+0x12) is set, `CStoredCar::RestoreCar`; a restored vehicle gets +0x42D |= 0x20, is added to the
/// world and placed on the road (`CAutomobile::PlaceOnRoadProperly` for vehicle type 0, `CBike::` for 9), and its
/// slot's model word is cleared. AL = 1 when all three model words are 0 afterwards.
#[hook(addr = 0x4485C0, conv = thiscall, name = "thunk_FUN_01561490", expect = "E9 CB 8E 11 01",
       this = "CGarage", unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn garage_restore_cars_for_this_hide_out(_this: GPtr<u8>, stored: GPtr<u8>) -> u8 {
    for k in 0..3isize {
        let car = stored.byte_add(0x40 * k);
        if rd::<u16>(car, 0x12) == 0 {
            continue;
        }
        let v = stored_car_restore_car(car);
        if v.is_null() {
            continue;
        }
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) | 0x20);
        world_add(v);
        match rd::<u32>(v, 0x594) {
            0 => automobile_place_on_road_properly(v),
            9 => bike_place_on_road_properly(v),
            _ => {}
        }
        wr::<u16>(car, 0x12, 0);
    }
    u8::from((0..3isize).all(|k| rd::<u16>(stored, 0x40 * k + 0x12) == 0))
}

/// A garage's "other vehicle inside" test (stub 0x449100 → .HOODLUM 0x1566680, thiscall, `ret 4`, AL; unnamed,
/// called by `CGarage::Update`): over the vehicle pool from the last slot down, every used vehicle other than
/// `except` and not wrecked (status bits of +0x36 != 5) that `IsEntityTouching3D` the garage: AL = 1 as soon as one of
/// its collision spheres (`GetColModel()` +0x2C: count word +0, spheres +8, 0x14 bytes: centre, radius +0xC) has its
/// centre (through the vehicle's matrix, `CMatrix * CVector`) `IsPointInsideGarage` with the sphere's radius.
#[hook(addr = 0x449100, conv = thiscall, name = "thunk_FUN_01566680", expect = "E9 7B D5 11 01",
       this = "CGarage", unverified = "frontier2-b5", route = "L2", hazards = "none")]
pub fn garage_is_any_other_car_touching(this: GPtr<u8>, except: GPtr<u8>) -> u8 {
    let pool = g::<GPtr<u8>>(0xB7_4494).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let v: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(0xA18)) as usize);
        if v.is_null() || v == except || rd::<u8>(v, 0x36) & 0xF8 == 0x28 {
            continue;
        }
        if garage_is_entity_touching_3d(this, v) == 0 {
            continue;
        }
        let data = rd::<GPtr<u8>>(entity_get_col_model(v), 0x2C);
        let n = rd::<i16>(data, 0);
        let mut k = 0i32;
        while k < i32::from(n) {
            let s = rd::<GPtr<u8>>(data, 8).byte_add(0x14 * k as isize);
            let mut tmp = [0u32; 3];
            let p = matrix_mul_vec(GPtr::from_raw(tmp.as_mut_ptr()).cast(), rd(v, 0x14), s);
            let c = sa_math::vector::CVector::new(rd(p, 0), rd(p, 4), rd(p, 8));
            if garage_is_point_inside_r(this, c, rd(rd::<GPtr<u8>>(data, 8), 0x14 * k as isize + 0xC)) != 0 {
                return 1;
            }
            k += 1;
        }
    }
    0
}

/// `0x449A50` (thiscall, `ret 8`; unnamed, called by `CGarage::Update`): byte for byte the body of
/// `CGarage::StoreAndRemoveCarsForThisHideOut` (0x449900; only the relative branch / call displacements differ), so
/// the same port.
#[hook(addr = 0x449A50, conv = thiscall, name = "FUN_00449a50", expect = "83 EC 0C 8B 54 24 10 53",
       this = "CGarage", unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn garage_store_and_remove_cars_copy(this: GPtr<u8>, stored: GPtr<u8>, max_slot: i32) {
    super::garage_store_cars::store_and_remove_cars_for_this_hide_out(this, stored, max_slot);
}

/// `CCheat::EverybodyAttacksPlayerCheat()` (0x439C70, cdecl, SEH frame not reproduced): the toggle byte 0x96913F
/// flipped; when it turns on, every used ped of the pool (0xB74490, 0x7C4 bytes, from the last slot down) that is
/// not a player gets the player ped type as hate (`SetAsAcquaintance(4, GetPedFlag(0))` on +0x4E0) and a
/// `CEventAcquaintancePedHate` (the `CEventAcquaintancePed` constructor with `FindPlayerPed(-1)`, vtable 0x858E68,
/// the word +0xE = 1000) added to its event group (intelligence +0x47C, +0x68), then destroyed.
#[hook(addr = 0x439C70, conv = cdecl, name = "CCheat::EverybodyAttacksPlayerCheat", expect = "64 A1 00 00 00 00 8A 15 3F 91 96 00",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes")]
pub fn cheat_everybody_attacks_player() {
    let on = u8::from(g::<u8>(0x96_913F).read() == 0);
    g::<u8>(0x96_913F).write(on);
    if on == 0 {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB7_4490).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let ped: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(0x7C4)) as usize);
        if ped.is_null() || ped_is_player(ped) != 0 {
            continue;
        }
        set_as_acquaintance(ped.byte_add(0x4E0), 4, get_ped_flag(0));
        let mut ev = [0u32; 6];
        let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        acquaintance_ped_ctor(addr32(e), addr32(find_player_ped(-1)));
        wr::<u32>(e, 0, 0x85_8E68);
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        wr::<u16>(e, 0xE, 0x3E8);
        event_group_add(intel.byte_add(0x68), e, 0);
        wr::<u32>(e, 0, 0x85_8E68);
        event_acquaintance_ped_dtor(e.cast());
    }
}

/// `CPickup::ProcessGunShot(CVector* start, CVector* end)` (stub 0x4588B0 → .HOODLUM 0x1562550, thiscall, `ret 8`): a
/// `CColLine(start, end)`; with an object (+4) whose position sphere (radius 4.0) the line hits
/// (`CCollision::TestLineSphere`): an explosion of type 8 at the object (no victim / creator, no lifetime, sound, the
/// camera shake -1.0 decoded from 0x1560490 + 0x682CB3 + 1, visibility from the protection VM (descriptor 0x155FF2C,
/// evaluated: 0)) and the pickup is removed.
#[hook(addr = 0x4588B0, conv = thiscall, name = "CPickup::ProcessGunShot", expect = "E9 9B 9C 10 01",
       this = "CPickup", unverified = "frontier2-b5", route = "L2", hazards = "heap-writes",
       note = "VM 0xCB73E0 (descriptor 0x155FF2C) evaluated: 0 (AddExplosion's last argument)")]
pub fn pickup_process_gun_shot(this: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) {
    let mut line = [0u32; 8];
    let l = GPtr::from_raw(line.as_mut_ptr()).cast::<u8>();
    col_line_ctor(l, start, end);
    let obj = rd::<GPtr<u8>>(this, 4);
    if obj.is_null() {
        return;
    }
    let p = entity_pos(obj);
    let mut sphere = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), 0x4080_0000, 0];
    if test_line_sphere(l.cast(), GPtr::from_raw(sphere.as_mut_ptr()).cast()) == 0 {
        return;
    }
    let p = entity_pos(rd::<GPtr<u8>>(this, 4));
    add_explosion(GPtr::NULL, GPtr::NULL, 8, rd(p, 0), rd(p, 4), rd(p, 8), 0, 1, f32::from_bits(0xBF80_0000), 0);
    pickup_remove(this);
}

/// `CReplay::SaveReplayToHD()` (0x45C340, cdecl): in the user's documents `replay.rep` (0x859C60) opened for writing
/// (a negative handle skips to the end): the 8-byte header (0x859C54), then from the first slot after the current one
/// (`BufferStatus` 0x97FB7C == 2, searched from 0; `(i + 1) % 8`) the first slot with status 1 or 2 and every slot
/// after it up to and including the current one, 100000 bytes each (0x97FB88); the file closed; the directory set
/// back to the game's (0x858B54).
#[hook(addr = 0x45C340, conv = cdecl, name = "CReplay::SaveReplayToHD", expect = "57 E8 1A C5 0D 00",
       unverified = "frontier2-b5", route = "split", hazards = "none", note = "writes replay.rep in the user's documents")]
pub fn replay_save_replay_to_hd() {
    file_set_dir_my_documents();
    let f = open_file_for_writing(GPtr::from_addr(0x85_9C60));
    if addr32(f) as i32 >= 0 {
        file_write(f, GPtr::from_addr(0x85_9C54), 8);
        let status = |i: i32| g::<u8>((0x97_FB7C + i) as usize).read();
        let next = |i: i32| (i + 1) % 8;
        let mut cur = 0i32;
        while cur < 8 && status(cur) != 2 {
            cur += 1;
        }
        let mut i = next(cur);
        while status(i) != 2 && status(i) != 1 {
            i = next(i);
        }
        let buf = |i: i32| GPtr::<u8>::from_addr((0x97_FB88 + 100_000 * i) as usize);
        file_write(f, buf(i), 100_000);
        while status(i) != 2 {
            i = next(i);
            file_write(f, buf(i), 100_000);
        }
        file_close(f);
    }
    file_set_dir(GPtr::from_addr(0x85_8B54));
}

/// `CReplay::PlayReplayFromHD()` (0x460390, cdecl): `replay.rep` (0x859C60, mode 0x859C6C) opened in the user's
/// documents; nothing without a file. The 8-byte header read into `gString` (0xB71670); a header other than 0x859C54
/// (`strncmp`, 8): file closed, game directory set back. Else 100000-byte blocks read into the slots from 0x97FB88
/// while `Read` returns non-zero (no bound on the count), every slot read so far marked 1; the status before the
/// first unread slot = 2 (0x97FB7B when nothing was read) and the rest of the 8 statuses 0; file closed, directory
/// set back, `TriggerPlayback(0, (0, 0, 0), false)`, `bPlayingBackFromFile` (0x97FAE1) and 0x97FAE0 = 1,
/// `StreamAllNecessaryCarsAndPeds`; the directory set back again.
#[hook(addr = 0x460390, conv = cdecl, name = "CReplay::PlayReplayFromHD", expect = "55 E8 CA 84 0D 00",
       unverified = "frontier2-b5", route = "split", hazards = "heap-writes", note = "reads replay.rep from the user's documents")]
pub fn replay_play_replay_from_hd() {
    file_set_dir_my_documents();
    let f = open_file(GPtr::from_addr(0x85_9C60), GPtr::from_addr(0x85_9C6C));
    if f.is_null() {
        file_set_dir(GPtr::from_addr(0x85_8B54));
        return;
    }
    file_read(f, GPtr::from_addr(0xB7_1670), 8);
    if strncmp(GPtr::from_addr(0xB7_1670), GPtr::from_addr(0x85_9C54), 8) != 0 {
        file_close(f);
        file_set_dir(GPtr::from_addr(0x85_8B54));
        return;
    }
    let mut n = 0u32;
    let mut buf = 0x97_FB88u32;
    if file_read(f, GPtr::from_addr(buf as usize), 100_000) != 0 {
        loop {
            buf = buf.wrapping_add(100_000);
            g::<u8>((0x97_FB7C + n) as usize).write(1);
            n += 1;
            if file_read(f, GPtr::from_addr(buf as usize), 100_000) == 0 {
                break;
            }
        }
    }
    g::<u8>((0x97_FB7Bu32.wrapping_add(n)) as usize).write(2);
    if (n as i32) < 8 {
        for k in n..8 {
            g::<u8>((0x97_FB7C + k) as usize).write(0);
        }
    }
    file_close(f);
    file_set_dir(GPtr::from_addr(0x85_8B54));
    replay_trigger_playback(0, 0.0, 0.0, 0.0, 0);
    g::<u8>(0x97_FAE1).write(1);
    g::<u8>(0x97_FAE0).write(1);
    replay_stream_all_necessary_cars_and_peds();
    file_set_dir(GPtr::from_addr(0x85_8B54));
}

/// `CGameLogic::DoWeaponStuffAtStartOf2PlayerGame(bool shareWeapons)` (stub 0x4428B0 → .HOODLUM 0x156AA30, cdecl): the
/// 13 weapons of `FindPlayerPed(0)` (+0x5A0, 0x1C bytes) saved to 0x96A9B8; with `shareWeapons`, every one whose type
/// is set given to `FindPlayerPed(1)` (`GiveWeapon(type, ammo +0xC, 1)`), `PickWeaponAllowedFor2Player` on the player
/// the protection VM names (descriptor 0x1560004 plus the key 0x14AA974D it rewrites: evaluated, player 0), and
/// player 1's chosen slot (player data +0x480, +0x20) = player 0's.
#[hook(addr = 0x4428B0, conv = cdecl, name = "CGameLogic::DoWeaponStuffAtStartOf2PlayerGame", expect = "E9 7B 81 12 01",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes",
       note = "VM 0xCB73E0 (descriptor 0x1560004) evaluated: FindPlayerPed(0)")]
pub fn game_logic_do_weapon_stuff_at_start_of_2_player_game(share: u32) {
    let p0 = find_player_ped(0).byte_add(0x5A0);
    for k in 0..13isize {
        for d in 0..7isize {
            wr::<u32>(GPtr::from_addr(0x96_A9B8), 0x1C * k + 4 * d, rd(p0, 0x1C * k + 4 * d));
        }
    }
    if share as u8 == 0 {
        return;
    }
    let mut o = 0isize;
    while o < 0x16C {
        if rd::<u32>(find_player_ped(0), o + 0x5A0) != 0 {
            let ammo = rd::<u32>(find_player_ped(0), o + 0x5AC);
            let ty = rd::<u32>(find_player_ped(0), o + 0x5A0);
            ped_give_weapon(find_player_ped(1), ty, ammo, 1);
        }
        o += 0x1C;
    }
    player_ped_pick_weapon_allowed_for_2_player(find_player_ped(0));
    let a = rd::<GPtr<u8>>(find_player_ped(0), 0x480);
    let b = rd::<GPtr<u8>>(find_player_ped(1), 0x480);
    wr::<u8>(b, 0x20, rd(a, 0x20));
}

/// `CGameLogic::ResetStuffUponResurrection()` (0x442980, cdecl): for the focused player (0xB7CD98 + 0x190 * byte
/// 0xB7CD74): messages cleared, the interesting vehicle list cleared, exciting stuff cleared within 4000 of the
/// player's position (`GetPos`), 720 minutes passed; `RestorePlayerStuffDuringResurrection(ped, position, +0x558 *
/// [0x859878])`, `SortOutStreamingAndMemory(position, heading)` (heading: FPATAN of the matrix's forward, or +0x10),
/// TheCamera +0x12C = 0, `SetMotionBlur(0, 0, 0, 0, 0)`, pad 0 stops shaking, references to the player removed, the
/// bytes 0x9690C0 and 0xC0EC23 = 10, the focused pad's +0x10E = 0, the ped's weapons cleared, `GameState` 0x96A8B0
/// and `TimeOfLastEvent` 0x96A8AC = 0.
#[hook(addr = 0x442980, conv = cdecl, name = "CGameLogic::ResetStuffUponResurrection", expect = "83 EC 10 56 0F B6 35 74 CD B7 00",
       unverified = "frontier2-b5", route = "L2", hazards = "heap-writes,x87-transcendental", note = "FPATAN heading")]
pub fn game_logic_reset_stuff_upon_resurrection() {
    let info = GPtr::<u8>::from_addr((u32::from(g::<u8>(0xB7_CD74).read()) * 0x190 + 0xB7_CD98) as usize);
    messages_clear_messages(0);
    car_ctrl_clear_interesting_vehicle_list();
    let mut tmp = [0u32; 3];
    let pos = player_info_get_pos(info, GPtr::from_raw(tmp.as_mut_ptr()));
    clear_exciting_stuff_from_area(pos.cast(), 4000.0, 1);
    game_logic_pass_time(0x2D0);
    let ped = rd::<GPtr<u8>>(info, 0);
    let p = entity_pos(ped);
    let h = x(rd::<f32>(ped, 0x558)).fmul_m(g::<f32>(0x85_9878).read()).to_f32();
    game_logic_restore_player_stuff(ped, rd(p, 0), rd(p, 4), rd(p, 8), h);
    let ped = rd::<GPtr<u8>>(info, 0);
    let heading = placeable_heading(ped).to_f32();
    game_logic_sort_out_streaming_and_memory(entity_pos(ped), heading);
    g::<u32>(0xB6_F154).write(0);
    camera_set_motion_blur(GPtr::from_addr(0xB6_F028), 0, 0, 0, 0, 0);
    pad_stop_shaking(get_pad(0), 0);
    references_remove_references_to_player();
    g::<u8>(0x96_90C0).write(10);
    g::<u8>(0xC0_EC23).write(10);
    wr::<u16>(get_pad(i32::from(g::<u8>(0xB7_CD74).read())), 0x10E, 0);
    ped_clear_weapons(rd::<GPtr<u8>>(info, 0));
    g::<u8>(0x96_A8B0).write(0);
    g::<u32>(0x96_A8AC).write(0);
}

orig! {
    /// `CWorld::FindMissionEntitiesIntersectingCubeSectorList(list, min, max, count, maxCount, out, vehicles, peds,
    /// objects)` (0x565300; the port reads the first eight)
    fn find_mission_entities_intersecting_cube_sector_list(list: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>, count: GPtr<u8>, max_count: u32, out: GPtr<u8>, v: u32, p: u32, o: u32) = cdecl 0x565300 => unported;
    /// `CWorld::FindObjectsIntersectingCubeSectorList(list, min, max, count, maxCount, out)` (0x5650E0)
    fn find_objects_intersecting_cube_sector_list(list: GPtr<u8>, min: GPtr<u8>, max: GPtr<u8>, count: GPtr<u8>, max_count: u32, out: GPtr<u8>) = cdecl 0x5650E0 => unported;
}

/// The cube scans' sector rectangle (`floor(v * 0.02 + 60)` of the corners' x / y), then the scan code advance.
fn cube_rect(min: GPtr<u8>, max: GPtr<u8>) -> (i32, i32, i32, i32) {
    let x0 = sector_index(x(rd::<f32>(min, 0)));
    let y0 = sector_index(x(rd::<f32>(min, 4)));
    let x1 = sector_index(x(rd::<f32>(max, 0)));
    let y1 = sector_index(x(rd::<f32>(max, 4)));
    advance_scan_code();
    (x0, y0, x1, y1)
}

/// `CWorld::FindMissionEntitiesIntersectingCube(const CVector& min, const CVector& max, int16* count, int16 maxCount,
/// CEntity** out, bool vehicles, bool peds, bool objects)` (0x569240, cdecl): the sector rectangle of the corners,
/// the scan code advanced, the count zeroed; per cell (y outer, x inner) the repeat sector's (0xB992B8, `& 15`, not
/// clamped) vehicle / ped / object lists through `FindMissionEntitiesIntersectingCubeSectorList` with the matching
/// flag. Kept original detail: the loop indices live in the caller's argument slots (not modelled).
#[hook(addr = 0x569240, conv = cdecl, name = "CWorld::FindMissionEntitiesIntersectingCube", expect = "83 EC 0C 53 55 56 57",
       unverified = "frontier2-b5", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) argument slots with loop indices")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_mission_entities_intersecting_cube(min: GPtr<u8>, max: GPtr<u8>, count: GPtr<u8>, max_count: u32, out: GPtr<u8>, v: u32, p: u32, o: u32) {
    let (x0, y0, x1, y1) = cube_rect(min, max);
    wr::<u16>(count, 0, 0);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let rep = GPtr::<u8>::from_addr(0xB9_92B8 + 12 * (((ys & 0xF) << 4) + (xs & 0xF)) as usize);
            if v as u8 != 0 {
                find_mission_entities_intersecting_cube_sector_list(rep, min, max, count, max_count, out, 1, 0, 0);
            }
            if p as u8 != 0 {
                find_mission_entities_intersecting_cube_sector_list(rep.byte_add(4), min, max, count, max_count, out, 0, 1, 0);
            }
            if o as u8 != 0 {
                find_mission_entities_intersecting_cube_sector_list(rep.byte_add(8), min, max, count, max_count, out, 0, 0, 1);
            }
        }
    }
}

/// `CWorld::FindObjectsIntersectingCube(const CVector& min, const CVector& max, int16* count, int16 maxCount, CEntity**
/// out, bool buildings, bool vehicles, bool peds, bool objects, bool dummies)` (0x568DD0, cdecl): as
/// `FindMissionEntitiesIntersectingCube` over `FindObjectsIntersectingCubeSectorList`, with the sector's (0xB7D0B8,
/// clamped to 0..119) building (+0) and dummy (+4) lists and the repeat sector's vehicle / ped / object lists.
#[hook(addr = 0x568DD0, conv = cdecl, name = "CWorld::FindObjectsIntersectingCube", expect = "83 EC 14 53 55 56 57",
       unverified = "frontier2-b5", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) argument slots with loop indices")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_objects_intersecting_cube(min: GPtr<u8>, max: GPtr<u8>, count: GPtr<u8>, max_count: u32, out: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32) {
    let (x0, y0, x1, y1) = cube_rect(min, max);
    wr::<u16>(count, 0, 0);
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let (sector, rep) = cell(xs, ys);
            if b as u8 != 0 {
                find_objects_intersecting_cube_sector_list(sector, min, max, count, max_count, out);
            }
            if v as u8 != 0 {
                find_objects_intersecting_cube_sector_list(rep, min, max, count, max_count, out);
            }
            if p as u8 != 0 {
                find_objects_intersecting_cube_sector_list(rep.byte_add(4), min, max, count, max_count, out);
            }
            if o as u8 != 0 {
                find_objects_intersecting_cube_sector_list(rep.byte_add(8), min, max, count, max_count, out);
            }
            if d as u8 != 0 {
                find_objects_intersecting_cube_sector_list(sector.byte_add(4), min, max, count, max_count, out);
            }
        }
    }
}

/// The 25 damage slots `CShopping::StoreVehicleMods` / `RestoreVehicleMods` save (0xA97258 + j, the switch index
/// `j - 2`, the jump tables 0x49B384 / 0x49B5A4): wheel 2, 3, 0, 1 at 0, 2, 3, 5; panels 5, 6 at 10, 11; doors 0, 1
/// at 14, 15; the others are not saved (stored as 0, skipped on restore).
#[derive(Clone, Copy)]
enum ModSlot {
    Wheel(i32),
    Panel(u32),
    Door(u32),
    None,
}

fn mod_slot(k: i32) -> ModSlot {
    match k {
        0 => ModSlot::Wheel(2),
        2 => ModSlot::Wheel(3),
        3 => ModSlot::Wheel(0),
        5 => ModSlot::Wheel(1),
        10 => ModSlot::Panel(5),
        11 => ModSlot::Panel(6),
        14 => ModSlot::Door(0),
        15 => ModSlot::Door(1),
        _ => ModSlot::None,
    }
}

/// `CShopping::StoreVehicleMods()` (0x49B280, cdecl): the player vehicle's 15 upgrade words (+0x43A) saved to
/// 0xA97274; for an automobile (+0x590 == 0) the 25 damage bytes (0xA97258) from its damage manager (+0x5A0) per
/// [`mod_slot`] (wheel / panel / door status, 0 for the others).
#[hook(addr = 0x49B280, conv = cdecl, name = "CShopping::StoreVehicleMods", expect = "57 6A 00 6A FF E8 46 2E 0D 00",
       unverified = "frontier2-b5", route = "L2", hazards = "none")]
pub fn shopping_store_vehicle_mods() {
    let v = find_player_vehicle(-1, 0);
    for k in 0..15isize {
        wr::<u16>(GPtr::from_addr(0xA9_7274), 2 * k, rd(v, 0x43A + 2 * k));
    }
    if rd::<u32>(v, 0x590) != 0 {
        return;
    }
    let dm = v.byte_add(0x5A0);
    for j in 0..25i32 {
        let s = match mod_slot(j - 2) {
            ModSlot::Wheel(w) => get_wheel_status(dm, w as u32) as u8,
            ModSlot::Panel(p) => dm_get_panel_status(dm, p) as u8,
            ModSlot::Door(d) => damage_get_door_status(dm, d as i32) as u8,
            ModSlot::None => 0,
        };
        g::<u8>((0xA9_7258 + j) as usize).write(s);
    }
}

/// `CShopping::RestoreVehicleMods()` (0x49B3C0, cdecl): on the player vehicle every fitted upgrade (+0x43A words, -1 =
/// none) removed, the saved word (0xA97274) put back and, when set, its model requested (flags 0xA); all requested
/// models loaded, `SetupUpgradesAfterLoad`, each saved upgrade model made deletable. For an automobile (+0x590 == 0)
/// every saved damage byte (0xA97258) above 0 re-applied per [`mod_slot`]: the wheel status; the panel status then
/// `SetPanelDamage(panel, true)`; the door status then `SetDoorDamage(door, true)`.
#[hook(addr = 0x49B3C0, conv = cdecl, name = "CShopping::RestoreVehicleMods", expect = "53 56 57 6A 00 6A FF",
       unverified = "frontier2-b5", route = "split", hazards = "heap-writes", note = "loads the requested upgrade models")]
pub fn shopping_restore_vehicle_mods() {
    let v = find_player_vehicle(-1, 0);
    let saved = GPtr::<u8>::from_addr(0xA9_7274);
    for k in 0..15isize {
        let cur = rd::<u16>(v, 0x43A + 2 * k);
        if cur != 0xFFFF {
            remove_vehicle_upgrade(v, i32::from(cur as i16));
        }
        wr::<u16>(v, 0x43A + 2 * k, rd(saved, 2 * k));
        let s = rd::<u16>(saved, 2 * k);
        if s != 0xFFFF {
            request_vehicle_upgrade(i32::from(s as i16), 0xA);
        }
    }
    load_all_requested_models(0);
    vehicle_setup_upgrades_after_load(v);
    for k in 0..15isize {
        let s = rd::<u16>(saved, 2 * k);
        if s != 0xFFFF {
            set_model_is_deletable(i32::from(s as i16));
        }
    }
    if rd::<u32>(v, 0x590) != 0 {
        return;
    }
    let dm = v.byte_add(0x5A0);
    for j in 0..25i32 {
        let st = g::<i8>((0xA9_7258 + j) as usize).read();
        match mod_slot(j - 2) {
            ModSlot::None => {}
            _ if st <= 0 => {}
            ModSlot::Wheel(w) => dm_set_wheel_status(dm, w, st as u8),
            ModSlot::Panel(p) => {
                dm_set_panel_status(dm, p, i32::from(st) as u32);
                automobile_set_panel_damage(v, p, 1);
            }
            ModSlot::Door(d) => {
                dm_set_door_status(dm, d, i32::from(st) as u32);
                set_door_damage(v, d, 1);
            }
        }
    }
}

orig! {
    /// `CWorld::CallOffChaseForAreaSectorListVehicles(list, x1, y1, x2, y2, minX, minY, maxX, maxY)` (0x563A80)
    fn call_off_chase_for_area_sector_list_vehicles(list: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, a: f32, b: f32, c: f32, d: f32) = cdecl 0x563A80 => crate::world::frontier_b3::call_off_chase_sector_list_vehicles;
    /// `CWorld::CallOffChaseForAreaSectorListPeds(list, x1, y1, x2, y2, minX, minY, maxX, maxY)` (0x563D00)
    fn call_off_chase_for_area_sector_list_peds(list: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, a: f32, b: f32, c: f32, d: f32) = cdecl 0x563D00 => crate::world::frontier_b3::call_off_chase_sector_list_peds;
}

/// `CWorld::CallOffChaseForArea(float x1, float y1, float x2, float y2)` (0x566A60, cdecl): the rectangle grown by
/// 10 (0x85862C; `x1 - 10`, `y1 - 10`, `x2 + 10`, `y2 + 10`), its sector rectangle, the scan code advanced; per
/// repeat sector (y outer, x inner, not clamped) the vehicle list then the ped list through the sector-list
/// functions with the original and the grown corners. Kept original detail: a loop value lives in the caller's `y2`
/// argument slot (not modelled).
#[hook(addr = 0x566A60, conv = cdecl, name = "CWorld::CallOffChaseForArea", expect = "83 EC 20 D9 44 24 24",
       unverified = "frontier2-b7", route = "L2", hazards = "none",
       note = "overwrites its own (caller-owned) y2 argument slot with a loop value")]
pub fn world_call_off_chase_for_area(x1: f32, y1: f32, x2: f32, y2: f32) {
    let ten = g::<f32>(0x85_862C).read();
    let a = x(x1).fsub_m(ten).to_f32();
    let c = x(x2).fadd_m(ten).to_f32();
    let b = x(y1).fsub_m(ten).to_f32();
    let d = x(y2).fadd_m(ten).to_f32();
    let sx0 = sector_index(x(a));
    let sy0 = sector_index(x(b));
    let sx1 = sector_index(x(c));
    let sy1 = sector_index(x(d));
    advance_scan_code();
    for ys in sy0..=sy1 {
        for xs in sx0..=sx1 {
            let rep = GPtr::<u8>::from_addr(0xB9_92B8 + 12 * (((ys & 0xF) << 4) + (xs & 0xF)) as usize);
            call_off_chase_for_area_sector_list_vehicles(rep, x1, y1, x2, y2, a, b, c, d);
            call_off_chase_for_area_sector_list_peds(rep.byte_add(4), x1, y1, x2, y2, a, b, c, d);
        }
    }
}

orig! {
    /// `CWorld::FindNearestObjectOfTypeSectorList(model, list, pos, radius, b2D, CEntity** nearest, float* dist)`
    /// (0x565450)
    fn find_nearest_object_of_type_sector_list(model: i32, list: GPtr<u8>, pos: GPtr<u8>, r: f32, two_d: u32, nearest: GPtr<u8>, dist: GPtr<u8>) = cdecl 0x565450 => unported;
}

/// `CWorld::FindNearestObjectOfType(int32 model, const CVector& pos, float radius, bool b2D, bool buildings, bool
/// vehicles, bool peds, bool objects, bool dummies)` (0x5693F0, cdecl): the sector rectangle of `pos ± radius`, the
/// scan code advanced; the nearest distance starts at the radius (kept in the caller's `pos` argument slot, not
/// modelled), the result at null; per cell (y outer, x inner) the selected lists (sector buildings +0, repeat-sector
/// vehicles / peds / objects, sector dummies +4, the sector clamped to 0..119) through
/// `FindNearestObjectOfTypeSectorList`. Returns the nearest entity.
#[hook(addr = 0x5693F0, conv = cdecl, name = "CWorld::FindNearestObjectOfType", expect = "83 EC 24 53 55 56 57",
       unverified = "frontier2-b7", route = "L2", hazards = "none",
       note = "the nearest distance lives in the caller's pos argument slot")]
#[allow(clippy::too_many_arguments)]
pub fn world_find_nearest_object_of_type(model: i32, pos: GPtr<u8>, r: f32, two_d: u32, b: u32, v: u32, p: u32, o: u32, d: u32) -> GPtr<u8> {
    let (x0, y0, x1, y1) = scan_rect(pos, r);
    let mut dist = [r];
    let mut found = [0u32];
    let dp = GPtr::from_raw(dist.as_mut_ptr()).cast::<u8>();
    let fp = GPtr::from_raw(found.as_mut_ptr()).cast::<u8>();
    for ys in y0..=y1 {
        for xs in x0..=x1 {
            let (sector, rep) = cell(xs, ys);
            if b as u8 != 0 {
                find_nearest_object_of_type_sector_list(model, sector, pos, r, two_d, fp, dp);
            }
            if v as u8 != 0 {
                find_nearest_object_of_type_sector_list(model, rep, pos, r, two_d, fp, dp);
            }
            if p as u8 != 0 {
                find_nearest_object_of_type_sector_list(model, rep.byte_add(4), pos, r, two_d, fp, dp);
            }
            if o as u8 != 0 {
                find_nearest_object_of_type_sector_list(model, rep.byte_add(8), pos, r, two_d, fp, dp);
            }
            if d as u8 != 0 {
                find_nearest_object_of_type_sector_list(model, sector.byte_add(4), pos, r, two_d, fp, dp);
            }
        }
    }
    rd::<GPtr<u8>>(fp, 0)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_WORLD_FIND_NEAREST_OBJECT_OF_TYPE,
    HOOK_WORLD_CALL_OFF_CHASE_FOR_AREA,
    HOOK_SHOPPING_STORE_VEHICLE_MODS,
    HOOK_SHOPPING_RESTORE_VEHICLE_MODS,
    HOOK_WORLD_FIND_MISSION_ENTITIES_INTERSECTING_CUBE,
    HOOK_WORLD_FIND_OBJECTS_INTERSECTING_CUBE,
    HOOK_CHEAT_EVERYBODY_ATTACKS_PLAYER,
    HOOK_PICKUP_PROCESS_GUN_SHOT,
    HOOK_REPLAY_SAVE_REPLAY_TO_HD,
    HOOK_REPLAY_PLAY_REPLAY_FROM_HD,
    HOOK_GAME_LOGIC_DO_WEAPON_STUFF_AT_START_OF_2_PLAYER_GAME,
    HOOK_GAME_LOGIC_RESET_STUFF_UPON_RESURRECTION,
    HOOK_GARAGES_TRIGGER_MESSAGE,
    HOOK_GARAGES_GIVE_PLAYER_DETONATOR,
    HOOK_GAME_LOGIC_RESTORE_PEDS_WEAPONS,
    HOOK_PICKUP_EXTRACT_AMMO_FROM_PICKUP,
    HOOK_REPLAY_GO_TO_NEXT_BLOCK,
    HOOK_REPLAY_PLAY_BACK_THIS_FRAME,
    HOOK_GARAGE_RESTORE_CARS_FOR_THIS_HIDE_OUT,
    HOOK_GARAGE_IS_ANY_OTHER_CAR_TOUCHING,
    HOOK_GARAGE_STORE_AND_REMOVE_CARS_COPY,
];
