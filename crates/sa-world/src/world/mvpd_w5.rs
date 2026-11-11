//! WRITER-MVP-D (helper-world-2) WORLD ports, batch `mvpd-world-5`: `CSetPiece::Update`, `CPickup::Update`.
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed; its original bugs are kept).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{find_player_coors, find_player_ped, find_player_vehicle, ftol32, g, rd, wr, x, xi};
use super::pickups::{
    frenzy_on_going, get_pad, get_task_jet_pack, get_weapon_info, give_us_a_pick_up_object, is_player_on_a_mission,
    localisation_kill_frenzy, pad_get_target, world_add, DISPLAY_HELP_MESSAGE,
};

orig! {
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn player_ped_get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CWorld::FindPlayerSpeed(int32 playerId)` (0x56E090, cdecl): pointer to the move speed
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56E090 => crate::world::cworld::find_player_speed;
    /// `CSetPiece::GetSpawnCoord1(CVector2D* out)` (0x4997A0, `ret 4`); returns `out`
    fn set_piece_get_spawn_coord1(sp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x4997A0 => unported;
    /// `CSetPiece::GetSpawnCoord2(CVector2D* out)` (0x4997E0, `ret 4`); returns `out`
    fn set_piece_get_spawn_coord2(sp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x4997E0 => unported;
    /// `CSetPiece::GetTargetCoord1(CVector2D* out)` (0x499820, `ret 4`); returns `out`
    fn set_piece_get_target_coord1(sp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x499820 => unported;
    /// `CSetPiece::GetTargetCoord2(CVector2D* out)` (0x499860, `ret 4`); returns `out`
    fn set_piece_get_target_coord2(sp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x499860 => unported;
    /// `CSetPiece::TryToGenerateCopCar(CVector2D spawn, CVector2D target)` (0x4998A0, `ret 0x10`); the four
    /// floats are passed as the dwords the original pushes
    fn set_piece_try_to_generate_cop_car(sp: GPtr<u8>, sx: u32, sy: u32, tx: u32, ty: u32) -> GPtr<u8> = thiscall 0x4998A0 => unported;
    /// `CSetPiece::TryToGenerateCopPed(CVector2D spawn)` (0x499690, `ret 8`)
    fn set_piece_try_to_generate_cop_ped(sp: GPtr<u8>, sx: u32, sy: u32) -> GPtr<u8> = thiscall 0x499690 => unported;
    /// `CCarAI::AddPoliceCarOccupants(CVehicle*, bool)` (0x41C070, cdecl)
    fn car_ai_add_police_car_occupants(v: GPtr<u8>, b: u32) = cdecl 0x41C070 => unported;
    /// `operator*(CVector* out, const CVector&, float)` (0x40FEC0, cdecl); returns `out`
    fn vec_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CWorld::FindGroundZForCoord(float x, float y)` (0x569660, cdecl, float in ST0)
    fn world_find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0, cdecl)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(int32 moveState, const CVector&,
    /// float radius, float moveStateRadius, bool, bool)` (0x668120, `ret 0x18`); the floats as pushed dwords
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, p: GPtr<u8>, r: u32, s: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskComplexBeCop::CTaskComplexBeCop(int32 moveState, bool, CTask*)` (0x4993D0, `ret 0xC`)
    fn task_complex_be_cop_ctor(t: GPtr<u8>, ms: u32, b: u32, sub: GPtr<u8>) -> GPtr<u8> = thiscall 0x4993D0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280, cdecl)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => crate::world::cworld2::remove;
    /// `CEntity` scalar deleting destructor (vtable slot 0, flag 1)
    fn entity_vdelete(e: GPtr<u8>, flag: u32) = thiscall vslot 0x0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const NOW: usize = 0xB7CB84;
/// The set-piece coordinate scale (`int16 * [0x858C84]`, 1/8)
const SCALE: usize = 0x858C84;

fn now() -> u32 {
    g::<u32>(NOW).read()
}

/// `fild` of the `int16` set-piece coordinate at `off`, `fmul [0x858C84]`, stored as a float.
fn coord(sp: GPtr<u8>, off: isize) -> f32 {
    xi(i32::from(rd::<i16>(sp, off))).fmul_m(g::<f32>(SCALE).read()).to_f32()
}

/// `vehicle->SetStatus(STATUS_SIMPLE)`: bits 3..7 of +0x36 set to 3.
fn set_status_simple(v: GPtr<u8>) {
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
}

/// The slow cop car of types 1/8: cruise speed 4 (24 for type 8) also as the float speed +0x3C8, driving
/// style 1, mission 0x18, destination the raw target at `t_off` (z 0), needed for 25 s, then its occupants.
fn setup_slow_car(sp: GPtr<u8>, car: GPtr<u8>, t_off: isize) {
    set_status_simple(car);
    wr::<u8>(car, 0x3D0, 4);
    if rd::<u8>(sp, 0x1C) == 8 {
        wr::<u8>(car, 0x3D0, 0x18);
    }
    let speed = rd::<u8>(car, 0x3D0);
    wr::<u8>(car, 0x3B9, 1);
    wr::<u8>(car, 0x3BA, 0x18);
    wr::<f32>(car, 0x3C8, xi(i32::from(speed)).to_f32());
    let tx = coord(sp, t_off);
    let ty = coord(sp, t_off + 2);
    wr::<f32>(car, 0x3EC, tx);
    wr::<f32>(car, 0x3F0, ty);
    wr::<u32>(car, 0x3F4, 0);
    wr::<u32>(car, 0x4D0, now().wrapping_add(25000));
    car_ai_add_police_car_occupants(car, 0);
}

/// The quick-car approach test (inlined in every quick case): the spawn point from the raw coordinates at
/// `s_off`; `(coors.y - spawn.y)` (stored as a float) `* speed.y + (coors.x - spawn.x) * speed.x` must be below
/// [0x858B50] (`test ah, 5; jp` skips). The calls run FindPlayerCoors, FindPlayerSpeed, FindPlayerCoors,
/// FindPlayerSpeed; x comes from the first coors buffer and the first speed pointer, y from the second ones.
fn approaching(sp: GPtr<u8>, s_off: isize) -> bool {
    let sx = coord(sp, s_off);
    let sy = coord(sp, s_off + 2);
    let mut c1 = [0f32; 3];
    let mut c2 = [0f32; 3];
    let p1 = find_player_coors(GPtr::from_raw(&mut c1).cast(), -1).cast::<u8>();
    let v1 = find_player_speed(-1);
    let p2 = find_player_coors(GPtr::from_raw(&mut c2).cast(), -1).cast::<u8>();
    let dy = x(rd::<f32>(p2, 4)).fsub_m(sy).to_f32();
    let v2 = find_player_speed(-1);
    let a = x(dy).fmul_m(rd::<f32>(v2, 4));
    let b = x(rd::<f32>(p1, 0)).fsub_m(sx).fmul_m(rd::<f32>(v1, 0));
    !a.fadd(b).fcom(x(g::<f32>(0x858B50).read())).test_parity(5)
}

/// The quick cop car of types 2/3/6/7: cruise speed 16, driving style / mission (`style`, `mission`), temp action
/// 9 (go forward) until now + 100, needed for 10 s, move speed `forward * 2/3` (0x40FEC0), then its occupants.
fn setup_quick_car(car: GPtr<u8>, style: u8, mission: u8) {
    set_status_simple(car);
    wr::<u8>(car, 0x3D0, 0x10);
    wr::<u8>(car, 0x3B9, style);
    wr::<u8>(car, 0x3BA, mission);
    wr::<u8>(car, 0x3BB, 9);
    wr::<u32>(car, 0x3BC, now().wrapping_add(100));
    wr::<u32>(car, 0x4D0, now().wrapping_add(10000));
    let m = rd::<GPtr<u8>>(car, 0x14);
    let mut out = [0f32; 3];
    let r = vec_mul_scalar(GPtr::from_raw(&mut out).cast(), m.byte_add(0x10), 0x3F2AAAAB);
    wr::<u32>(car, 0x44, rd::<u32>(r, 0));
    wr::<u32>(car, 0x48, rd::<u32>(r, 4));
    wr::<u32>(car, 0x4C, rd::<u32>(r, 8));
    car_ai_add_police_car_occupants(car, 0);
}

/// One quick car: the approach test on the spawn at `s_off`, then target/spawn coord (`second` picks the
/// `...2` getters), TryToGenerateCopCar, the setup and the generation time.
fn quick_car(sp: GPtr<u8>, s_off: isize, second: bool, style: u8, mission: u8) {
    if !approaching(sp, s_off) {
        return;
    }
    let mut tb = [0f32; 2];
    let mut sb = [0f32; 2];
    let (t, s) = if second {
        let t = set_piece_get_target_coord2(sp, GPtr::from_raw(&mut tb).cast());
        (t, set_piece_get_spawn_coord2(sp, GPtr::from_raw(&mut sb).cast()))
    } else {
        let t = set_piece_get_target_coord1(sp, GPtr::from_raw(&mut tb).cast());
        (t, set_piece_get_spawn_coord1(sp, GPtr::from_raw(&mut sb).cast()))
    };
    let car = set_piece_try_to_generate_cop_car(sp, rd(s, 0), rd(s, 4), rd(t, 0), rd(t, 4));
    if car.is_null() {
        return;
    }
    setup_quick_car(car, style, mission);
    wr::<u32>(sp, 0, now());
}

/// The cop of types 4/5: the ground z under the raw target at `z_off` (`FindGroundZForCoord`, stored as a
/// float), `+0x54C` = now + 10 s, `+0x470 |= 0x40`, then `new CTaskComplexBeCop(4, false, new
/// CTaskComplexGoToPointAndStandStill(6, (target at xy_off, z), [0x86FC84], [0x86FC88], false, false))` (either
/// `new` may fail and pass null) set as primary task 3. `xy_off` is always target 1 in the original: the second
/// cop of type 5 walks to target 1 at target 2's ground height (original bug, kept).
fn setup_cop(sp: GPtr<u8>, cop: GPtr<u8>, z_off: isize, xy_off: isize) {
    let gy = coord(sp, z_off + 2);
    let gx = coord(sp, z_off);
    let z = X87::from_f64_exact(world_find_ground_z_for_coord(gx, gy)).to_f32();
    wr::<u32>(cop, 0x54C, now().wrapping_add(10000));
    wr::<u32>(cop, 0x470, rd::<u32>(cop, 0x470) | 0x40);
    let t = task_new(0x28);
    let inner = if t.is_null() {
        GPtr::NULL
    } else {
        let y = coord(sp, xy_off + 2);
        let xx = coord(sp, xy_off);
        let mut v = [xx, y, z];
        go_to_point_and_stand_still_ctor(t, 6, GPtr::from_raw(&mut v).cast(), g::<u32>(0x86FC84).read(), g::<u32>(0x86FC88).read(), 0, 0)
    };
    let t = task_new(0x50);
    let task = if t.is_null() { GPtr::NULL } else { task_complex_be_cop_ctor(t, 4, 0, inner) };
    task_manager_set_task(rd::<GPtr<u8>>(cop, 0x47C).byte_add(4), task, 3, 0);
}

/// `CSetPiece::Update()` (0x499A80).
///
/// Nothing within 40 s (0x9C40, not 4 s) of the last generation (+0x0, when set). The player (FindPlayerCoors
/// -1) must be inside the area rect (`int16` +4/+6 .. +8/+0xA times 1/8; left/top `<=` via `test ah, 0x41; je`,
/// right/bottom via `test ah, 5; jnp`). By type (+0x1C):
/// - 1/8 (wanted >= 1, on foot): two slow cop cars (spawn/target 1 then target/spawn 2); if the second fails
///   the first is removed and deleted; else both are set up and the time stored.
/// - 2/3 (wanted >= 2, in a vehicle): one quick car (spawn 1) with style/mission 3/0x1A resp. 2/3.
/// - 4 (wanted >= 1 and on foot, gta-reversed has `&&`): one cop to target 1.
/// - 5: two cops (the first gets its task before the second is tried; a failed second removes and deletes
///   the first); the second walks to target 1 at target 2's ground z.
/// - 6/7 (wanted >= 2, in a vehicle): quick cars from spawn 1 and spawn 2 (each tested on its own).
#[hook(addr = 0x499A80, conv = thiscall, name = "CSetPiece::Update", this = "CSetPiece", expect = "55 8B EC 83 E4 F8 6A FF",
       unverified = "mvpd-world-5", route = "L2", hazards = "heap-writes",
       note = "creates cop cars/peds and tasks (pool allocations, writes into them); FindGroundZForCoord result stored as float")]
pub fn set_piece_update(this: GPtr<u8>) {
    let t = rd::<u32>(this, 0);
    if t != 0 && now() <= t.wrapping_add(0x9C40) {
        return;
    }
    let mut pc = [0f32; 3];
    find_player_coors(GPtr::from_raw(&mut pc).cast(), -1);
    let k = g::<f32>(SCALE).read();
    let rc = |off: isize| xi(i32::from(rd::<i16>(this, off))).fmul_m(k);
    if !rc(4).fcom(x(pc[0])).test_any(0x41) {
        return;
    }
    if !rc(8).fcom(x(pc[0])).test_parity(5) {
        return;
    }
    if !rc(6).fcom(x(pc[1])).test_any(0x41) {
        return;
    }
    if !rc(0xA).fcom(x(pc[1])).test_parity(5) {
        return;
    }
    let ty = rd::<u8>(this, 0x1C);
    let wanted = || player_ped_get_wanted_level(find_player_ped(-1));
    let in_vehicle = || !find_player_vehicle(-1, 0).is_null();
    match ty {
        1 | 8 => {
            if wanted() < 1 || in_vehicle() {
                return;
            }
            let mut tb = [0f32; 2];
            let mut sb = [0f32; 2];
            let t1 = set_piece_get_target_coord1(this, GPtr::from_raw(&mut tb).cast());
            let s1 = set_piece_get_spawn_coord1(this, GPtr::from_raw(&mut sb).cast());
            let car1 = set_piece_try_to_generate_cop_car(this, rd(s1, 0), rd(s1, 4), rd(t1, 0), rd(t1, 4));
            if car1.is_null() {
                return;
            }
            let mut tb2 = [0f32; 2];
            let mut sb2 = [0f32; 2];
            let t2 = set_piece_get_target_coord2(this, GPtr::from_raw(&mut tb2).cast());
            let s2 = set_piece_get_spawn_coord2(this, GPtr::from_raw(&mut sb2).cast());
            let car2 = set_piece_try_to_generate_cop_car(this, rd(s2, 0), rd(s2, 4), rd(t2, 0), rd(t2, 4));
            if car2.is_null() {
                world_remove(car1);
                entity_vdelete(car1, 1);
                return;
            }
            setup_slow_car(this, car1, 0x14);
            setup_slow_car(this, car2, 0x18);
            wr::<u32>(this, 0, now());
        }
        2 | 3 => {
            if wanted() < 2 || !in_vehicle() {
                return;
            }
            let (style, mission) = if ty == 2 { (3, 0x1A) } else { (2, 3) };
            quick_car(this, 0xC, false, style, mission);
        }
        4 | 5 => {
            if wanted() < 1 || in_vehicle() {
                return;
            }
            let mut sb = [0f32; 2];
            let s1 = set_piece_get_spawn_coord1(this, GPtr::from_raw(&mut sb).cast());
            let cop1 = set_piece_try_to_generate_cop_ped(this, rd(s1, 0), rd(s1, 4));
            if cop1.is_null() {
                return;
            }
            setup_cop(this, cop1, 0x14, 0x14);
            if ty == 5 {
                let mut sb2 = [0f32; 2];
                let s2 = set_piece_get_spawn_coord2(this, GPtr::from_raw(&mut sb2).cast());
                let cop2 = set_piece_try_to_generate_cop_ped(this, rd(s2, 0), rd(s2, 4));
                if cop2.is_null() {
                    world_remove(cop1);
                    entity_vdelete(cop1, 1);
                    return;
                }
                setup_cop(this, cop2, 0x18, 0x14);
            }
            wr::<u32>(this, 0, now());
        }
        6 | 7 => {
            if wanted() < 2 || !in_vehicle() {
                return;
            }
            let (style, mission) = if ty == 6 { (3, 0x1A) } else { (2, 3) };
            quick_car(this, 0xC, false, style, mission);
            quick_car(this, 0x10, true, style, mission);
        }
        _ => {}
    }
}


// ---------------------------------------------------------------------------------------------------------
// CPickup::Update
// ---------------------------------------------------------------------------------------------------------

orig! {
    /// `CVehicle::IsSphereTouchingVehicle(float x, float y, float z, float radius)` (0x6D84D0, `ret 0x10`); the
    /// floats are the dwords the original pushes
    fn vehicle_is_sphere_touching_vehicle(v: GPtr<u8>, x: u32, y: u32, z: u32, r: u32) -> u8 = thiscall 0x6D84D0 => unported;
    /// `CPlayerPed::CanPlayerStartMission()` (0x609590)
    fn player_ped_can_player_start_mission(p: GPtr<u8>) -> u8 = thiscall 0x609590 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn player_ped_get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPickups::WeaponForModel(int32)` (0x454AE0, cdecl)
    fn weapon_for_model(model: i32) -> i32 = cdecl 0x454AE0 => crate::world::pickups::weapon_for_model;
    /// `thunk_FUN_0156f730` (0x454920, cdecl): true for weapon slots 3, 4, 5
    fn is_slot_3_4_5(slot: i32) -> u32 = cdecl 0x454920 => crate::world::pickups::is_type_3_4_5;
    /// `CPickup::ExtractAmmoFromPickup(CPlayerPed*)` (0x454BE0, `ret 4`)
    fn pickup_extract_ammo_from_pickup(pk: GPtr<u8>, p: GPtr<u8>) = thiscall 0x454BE0 => unported;
    /// `CWeapon::Reload(CPed*)` (0x73AEB0, `ret 4`)
    fn weapon_reload(w: GPtr<u8>, owner: GPtr<u8>) = thiscall 0x73AEB0 => unported;
    /// `CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment(eWeaponType)` (0x4554C0, cdecl)
    fn player_can_pick_up_this_weapon_type_at_this_moment(w: i32) -> u8 = cdecl 0x4554C0 => crate::world::pickups::player_can_pick_up_this_weapon_type_at_this_moment;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0, cdecl)
    fn hud_set_help_message(s: GPtr<u8>, a: u32, b: u32, c: u32) = cdecl 0x588BE0 => unported;
    /// `CHud::HelpMessageDisplayed()` (0x588B50, cdecl)
    fn hud_help_message_displayed() -> u8 = cdecl 0x588B50 => unported;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32)` (0x53F920, `ret 0xC`)
    fn pad_start_shake(pad: GPtr<u8>, time: u32, freq: u32, c: u32) = thiscall 0x53F920 => unported;
    /// `CPad::GetTouchedTimeDelta()` (0x53F210)
    fn pad_get_touched_time_delta(pad: GPtr<u8>) -> u32 = thiscall 0x53F210 => unported;
    /// `CPickups::GivePlayerGoodiesWithPickUpMI(uint16 modelIndex, int32 playerId)` (0x4564F0, cdecl)
    fn pickups_give_player_goodies_with_pick_up_mi(mi: u32, id: i32) -> u8 = cdecl 0x4564F0 => unported;
    /// `CPlayerPed::DoesPlayerWantNewWeapon(eWeaponType, bool)` (0x609710, `ret 8`)
    fn player_ped_does_player_want_new_weapon(p: GPtr<u8>, w: i32, b: u32) -> u8 = thiscall 0x609710 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0, `ret 8`)
    fn ped_give_delayed_weapon(p: GPtr<u8>, w: i32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CPed::GetWeaponSlot(eWeaponType)` (0x5DF200, `ret 4`)
    fn ped_get_weapon_slot(p: GPtr<u8>, w: i32) -> i32 = thiscall 0x5DF200 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0,
    /// `ret 0xC`); the floats as pushed dwords
    fn audio_report_frontend_audio_event(ae: GPtr<u8>, ev: u32, vol: u32, speed: u32) = thiscall 0x506EA0 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`); the float as a pushed dword
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPickup::FindStringForTextIndex(int32)` (0x455540, cdecl)
    fn find_string_for_text_index(i: i32) -> GPtr<u8> = cdecl 0x455540 => crate::world::pickups::find_string_for_text_index;
    /// `CPickups::ModifyStringLabelForControlSetting(char*)` (0x454B70, cdecl)
    fn pickups_modify_string_label_for_control_setting(s: GPtr<u8>) = cdecl 0x454B70 => unported;
    /// `CMessages::InsertNumberInString(const char*, int32 n1..n6, char* out)` (0x69DE90, cdecl)
    fn messages_insert_number_in_string(s: GPtr<u8>, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, out: GPtr<u8>) = cdecl 0x69DE90 => unported;
    /// `CPickup::Remove()` (0x4556C0)
    fn pickup_remove(pk: GPtr<u8>) = thiscall 0x4556C0 => crate::world::pickups::remove;
    /// `CPickup::GetRidOfObjects()` (0x454CF0)
    fn pickup_get_rid_of_objects(pk: GPtr<u8>) = thiscall 0x454CF0 => crate::world::pickups::get_rid_of_objects;
    /// `CPlaceable::GetHeading()` (0x441DB0, float in ST0)
    fn placeable_get_heading(p: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool touchy, CVector* normal)`
    /// (0x6EB690, cdecl); x, y as the pushed dwords
    fn water_level_get_water_level(x: u32, y: u32, z: f32, level: GPtr<u8>, touchy: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    fn entity_update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CEntity* creator, eExplosionType, CVector pos, uint32 time,
    /// bool makeSound, float camShake, bool invisible)` (0x736A50, cdecl); all as pushed dwords
    fn explosion_add_explosion(v: u32, c: u32, ty: u32, x: u32, y: u32, z: u32, time: u32, snd: u32, shake: u32, inv: u32) -> u8 = cdecl 0x736A50 => unported;
    /// `operator+(CVector* out, const CVector&, const CVector&)` (0x40FE30, cdecl); returns `out`
    fn vec_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`)
    fn placeable_set_posn(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
}

/// `TheText` (0xC1B340)
const THE_TEXT: usize = 0xC1B340;
/// `AudioEngine` (0xB6BC90)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CPickups::PlayerOnWeaponPickup` (0x97D640)
const PLAYER_ON_WEAPON_PICKUP: usize = 0x97D640;
/// `CollectPickupBuffer` (0x97D644)
const COLLECT_PICKUP_BUFFER: usize = 0x97D644;
/// `CWorld::Players[0].m_nMoney` (0xB7CE50; players are 0x190 apart)
const PLAYER_MONEY: u32 = 0xB7CE50;
/// The jetpack model (370; the original compares against `bx` = 0x172)
const MODEL_JETPACK: i16 = 0x172;
/// `1.0f`, `2.0f`, `1.5f`, `-1.0f` as the dwords the original pushes
const F1: u32 = 0x3F800000;
const F2: u32 = 0x40000000;
const F1_5: u32 = 0x3FC00000;
const FM1: u32 = 0xBF800000;

/// A `ModelIndices` global (`uint16`, zero-extended as the original's `movzx`).
fn mi_g(addr: usize) -> i32 {
    i32::from(g::<u16>(addr).read())
}

/// The entity position (matrix +0x30 or the simple transform +4).
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The pickup coordinate at `off` (`int16 * [0x858C48]`, stored as a float).
fn pk_coord(pk: GPtr<u8>, off: isize) -> f32 {
    xi(i32::from(rd::<i16>(pk, off))).fmul_m(g::<f32>(0x858C48).read()).to_f32()
}

/// `IsSphereTouchingVehicle` of `veh` at the object's position.
fn sphere_touch(veh: GPtr<u8>, obj: GPtr<u8>, r: u32) -> bool {
    let p = ent_pos(obj);
    vehicle_is_sphere_touching_vehicle(veh, rd(p, 0), rd(p, 4), rd(p, 8), r) != 0
}

/// `AudioEngine.ReportFrontendAudioEvent(ev, 0.0, 1.0)`.
fn frontend_audio(ev: u32) {
    audio_report_frontend_audio_event(GPtr::from_addr(AUDIO_ENGINE), ev, 0, F1);
}

/// `CHud::SetHelpMessage(TheText.Get(key), a, b, c)`.
fn help_text(key: usize, a: u32, b: u32, c: u32) {
    hud_set_help_message(text_get(GPtr::from_addr(THE_TEXT), GPtr::from_addr(key)), a, b, c);
}

/// The on-foot pick-up distance test: `|player.z - obj.z|` (`fchs` unless `test ah, 5; jp` against
/// [0x858B50]) below [0x858CA0], then `dy*dy + dx*dx` below [0x859BB4].
fn near_enough(player: GPtr<u8>, obj: GPtr<u8>) -> bool {
    let pp = ent_pos(player);
    let op = ent_pos(obj);
    let d = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(op, 8));
    let a = if d.fcom(x(g::<f32>(0x858B50).read())).test_parity(5) { d } else { d.fchs() };
    if a.fcom(x(g::<f32>(0x858CA0).read())).test_parity(5) {
        return false;
    }
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(op, 4));
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(op, 0));
    !dy.fmul(dy).fadd(dx.fmul(dx)).fcom(x(g::<f32>(0x859BB4).read())).test_parity(5)
}

/// `GiveDelayedWeapon` with the pickup's ammo (+8; when 0: 0 with flag bit 1, else `AmmoForWeapon_OnStreet`
/// 0x8A5F50), then the chosen slot (player data +0x20) moved to the weapon's slot when it was the unarmed one.
fn give_weapon(pk: GPtr<u8>, player: GPtr<u8>, w: i32) {
    let ammo = rd::<u32>(pk, 8);
    let n = if ammo != 0 {
        ammo
    } else if rd::<u8>(pk, 0x1D) & 2 != 0 {
        0
    } else {
        u32::from(g::<u16>((0x8A5F50u32).wrapping_add((w as u32).wrapping_mul(2)) as usize).read())
    };
    ped_give_delayed_weapon(player, w, n);
    let pd = rd::<GPtr<u8>>(player, 0x480);
    let s0 = ped_get_weapon_slot(player, 0);
    if i32::from(rd::<i8>(pd, 0x20)) == s0 {
        let pd = rd::<GPtr<u8>>(player, 0x480);
        let s = ped_get_weapon_slot(player, w);
        wr::<u8>(pd, 0x20, s as u8);
    }
}

/// `CWaterLevel::GetWaterLevel(x, y, z + [0x858C80], &level, true, null)` at the object's matrix position (no
/// null check of the matrix, as in the original); the level when found.
fn water_level_at(obj: GPtr<u8>) -> Option<f32> {
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let z = x(rd::<f32>(m, 0x38)).fadd_m(g::<f32>(0x858C80).read()).to_f32();
    let mut level = 0u32;
    let ok = water_level_get_water_level(rd(m, 0x30), rd(m, 0x34), z, GPtr::from_raw(&mut level).cast(), 1, 0);
    (ok != 0).then(|| f32::from_bits(level))
}

/// The object's matrix z set to the level (`raw`) or `level + [0x858CC8]` when water was found, then
/// `UpdateRwMatrix` / `UpdateRwFrame` (the object re-read each time).
fn float_on_water(pk: GPtr<u8>, level: Option<f32>, raw: bool) {
    if let Some(l) = level {
        let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(pk, 4), 0x14);
        let z = if raw { l } else { x(l).fadd_m(g::<f32>(0x858CC8).read()).to_f32() };
        wr::<f32>(m, 0x38, z);
    }
    entity_update_rw_matrix(rd::<GPtr<u8>>(pk, 4));
    entity_update_rw_frame(rd::<GPtr<u8>>(pk, 4));
}

/// Walks the vehicle pool (0xB74494, pool pointer read once) from the last slot down, testing every live
/// vehicle (flag byte without bit 7, non-null) against the object's position with radius `r`; true when any
/// touches (no early exit).
fn any_vehicle_touching(pk: GPtr<u8>, r: u32) -> bool {
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let mut i = rd::<i32>(pool, 8);
    let mut hit = false;
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() {
            continue;
        }
        if sphere_touch(v, rd::<GPtr<u8>>(pk, 4), r) {
            hit = true;
        }
    }
    hit
}

/// The mine and floating-package types 9..14 (the first switch, table 0x458894).
fn update_mines(pk: GPtr<u8>, vehicle: GPtr<u8>, removed: &mut u8) {
    let obj = || rd::<GPtr<u8>>(pk, 4);
    match rd::<u8>(pk, 0x1C) {
        9 => {
            if vehicle.is_null() || sphere_touch(vehicle, obj(), F2) {
                return;
            }
            wr::<u8>(pk, 0x1C, 10);
            wr::<u32>(pk, 0xC, now().wrapping_add(10000));
        }
        t @ (10 | 12) => {
            if t == 12 {
                let l = water_level_at(obj());
                float_on_water(pk, l, false);
            }
            let late = now() > rd::<u32>(pk, 0xC);
            let touching = any_vehicle_touching(pk, F1_5);
            if !(late || touching) {
                return;
            }
            let p = ent_pos(obj());
            explosion_add_explosion(0, 0, 8, rd(p, 0), rd(p, 4), rd(p, 8), 0, 1, FM1, 0);
            // the original removes without setting its result flag
            pickup_remove(pk);
        }
        11 => {
            let l = water_level_at(obj());
            float_on_water(pk, l, false);
            if any_vehicle_touching(pk, F2) {
                return;
            }
            wr::<u8>(pk, 0x1C, 12);
            wr::<u32>(pk, 0xC, now().wrapping_add(10000));
        }
        13 => {
            let o = obj();
            let ts = g::<f32>(0xB7CB5C);
            let vz = x(ts.read()).fmul_m(g::<f32>(0x858C58).read()).fsubr_m(rd::<f32>(o, 0x4C)).to_f32();
            wr::<f32>(o, 0x4C, vz);
            let o = obj();
            let pos = ent_pos(o);
            let mut t1 = [0f32; 3];
            let mut t2 = [0f32; 3];
            let a = vec_mul_scalar(GPtr::from_raw(&mut t1).cast(), o.byte_add(0x44), g::<u32>(0xB7CB5C).read());
            let b = vec_add(GPtr::from_raw(&mut t2).cast(), pos, a);
            placeable_set_posn(o, b);
            entity_update_rw_matrix(obj());
            entity_update_rw_frame(obj());
            let Some(l) = water_level_at(obj()) else { return };
            let m = rd::<GPtr<u8>>(obj(), 0x14);
            if !x(l).fcom(x(rd::<f32>(m, 0x38))).test_any(1) {
                wr::<u8>(pk, 0x1C, 0xE);
            }
        }
        14 => {
            let l = water_level_at(obj());
            float_on_water(pk, l, true);
            if vehicle.is_null() || !sphere_touch(vehicle, obj(), F2) {
                return;
            }
            pickup_remove(pk);
            *removed = 1;
            frontend_audio(0xC);
        }
        _ => {}
    }
}

/// The weapon rules of a picked-up pickup (types other than 6, 8..11, 13, 14, 16..19); `false` ends the
/// pick-up (straight to the timeout check), `picked` may be cleared on the way.
fn weapon_checks(pk: GPtr<u8>, player: GPtr<u8>, picked: &mut bool) -> bool {
    let obj = || rd::<GPtr<u8>>(pk, 4);
    let w = weapon_for_model(i32::from(rd::<i16>(obj(), 0x22)));
    if !(0..0x2F).contains(&w) {
        return true;
    }
    if frenzy_on_going() != 0 {
        wr::<u8>(pk, 0x1D, rd::<u8>(pk, 0x1D) & 0xFB);
        return false;
    }
    if w == 0 {
        return true;
    }
    let slot = rd::<i32>(get_weapon_info(w, 1), 0x14);
    let sw = player.byte_add(slot as isize * 0x1C + 0x5A0);
    let cur = rd::<i32>(sw, 0);
    if cur == w {
        return true;
    }
    // `CStreaming::ms_aInfoForModel[mi]` load state (0x8E4CD0 + mi * 20, movsx index)
    let mi = i32::from(rd::<i16>(obj(), 0x22)) as u32;
    if g::<u8>(0x8E4CD0u32.wrapping_add(mi.wrapping_mul(20)) as usize).read() != 1 {
        return false;
    }
    let since_los = || now().wrapping_sub(rd::<u32>(rd::<GPtr<u8>>(player, 0x480), 0x6C));
    if cur != 0 && (rd::<u32>(sw, 0xC) != 0 || is_slot_3_4_5(slot) != 0) {
        g::<u32>(PLAYER_ON_WEAPON_PICKUP).write(6);
        if is_slot_3_4_5(slot) != 0 && matches!(rd::<u8>(pk, 0x1C), 2 | 3 | 4 | 0x16) {
            pickup_extract_ammo_from_pickup(pk, player);
            weapon_reload(sw, GPtr::NULL);
        }
        if player_can_pick_up_this_weapon_type_at_this_moment(w) == 0 {
            *picked = false;
        } else if pad_get_target(get_pad(0)) == 0 && DISPLAY_HELP_MESSAGE.get() != 0 && rd::<u8>(pk, 0x1D) & 4 == 0 {
            let key = match rd::<u16>(get_pad(0), 0x10A) {
                0 | 1 => Some(0x859BAC),
                2 => Some(0x859BA4),
                3 => Some(0x859B9C),
                _ => None,
            };
            if let Some(k) = key {
                help_text(k, 0, 0, 0);
            }
            let d = DISPLAY_HELP_MESSAGE.get();
            if d != 0 {
                DISPLAY_HELP_MESSAGE.set(d - 1);
            }
            wr::<u8>(pk, 0x1D, rd::<u8>(pk, 0x1D) | 4);
        }
        if g::<u32>(COLLECT_PICKUP_BUFFER).read() == 0 {
            *picked = false;
        }
        return since_los() >= 0x5DC;
    }
    if since_los() >= 0x5DC && player_can_pick_up_this_weapon_type_at_this_moment(w) != 0 {
        return true;
    }
    g::<u32>(PLAYER_ON_WEAPON_PICKUP).write(6);
    false
}

/// The collect conditions: body armour / health below the player's maximum minus [0x858CC4]; a bribe needs a
/// wanted level of `FindPlayerPed(0)` (its player data's wanted pointer read without a null check, as in the
/// original); kill frenzy off a mission, no frenzy, `CLocalisation::KillFrenzy`; no jetpack while wearing one;
/// revenue (0x10) at least 10; 2P (0x15) off a mission.
fn can_collect(pk: GPtr<u8>, player: GPtr<u8>, id: i32) -> bool {
    let obj = || rd::<GPtr<u8>>(pk, 4);
    let mi = i32::from(rd::<i16>(obj(), 0x22));
    let row = (id as u32).wrapping_mul(0x190);
    let below_max = |max_addr: u32, cur_off: isize| {
        let max = g::<u8>(max_addr.wrapping_add(row) as usize).read();
        xi(i32::from(max)).fsub_m(g::<f32>(0x858CC4).read()).fcom(x(rd::<f32>(player, cur_off))).test_parity(5)
    };
    if mi == mi_g(0x8CD5C0) && !below_max(0xB7CEE8, 0x548) {
        return false;
    }
    if mi == mi_g(0x8CD5C8) && !below_max(0xB7CEE7, 0x540) {
        return false;
    }
    if mi == mi_g(0x8CD5D0) {
        let pd = rd::<GPtr<u8>>(find_player_ped(0), 0x480);
        let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
        if rd::<u32>(wanted, 0x2C) == 0 {
            return false;
        }
    }
    if i32::from(rd::<i16>(obj(), 0x22)) == mi_g(0x8CD5D4)
        && (is_player_on_a_mission() != 0 || frenzy_on_going() != 0 || localisation_kill_frenzy() == 0)
    {
        return false;
    }
    if rd::<i16>(obj(), 0x22) == MODEL_JETPACK && !get_task_jet_pack(rd::<GPtr<u8>>(player, 0x47C)).is_null() {
        return false;
    }
    let ty = rd::<u8>(pk, 0x1C);
    if ty == 0x10 && !x(rd::<f32>(pk, 0)).fcom(x(g::<f32>(0x85862C).read())).test_parity(5) {
        return false;
    }
    !(ty == 0x15 && is_player_on_a_mission() != 0)
}

/// The collection by type (the second switch, tables 0x45887C / 0x45885C).
fn collect(pk: GPtr<u8>, player: GPtr<u8>, vehicle: GPtr<u8>, id: i32, removed: &mut u8) {
    let obj = || rd::<GPtr<u8>>(pk, 4);
    let mi = || i32::from(rd::<i16>(obj(), 0x22));
    if mi() != mi_g(0x8CD5E8) && mi() != mi_g(0x8CD5EC) && rd::<u8>(pk, 0x1C) != 0x15 {
        pad_start_shake(get_pad(0), 0x78, 0x64, 0);
    }
    let w = weapon_for_model(mi());
    let goodies = || pickups_give_player_goodies_with_pick_up_mi(u32::from(rd::<u16>(obj(), 0x22)), id) != 0;
    match rd::<u8>(pk, 0x1C) {
        2 | 15 => {
            if !goodies() {
                if player_ped_does_player_want_new_weapon(player, w, 0) == 0 {
                    return;
                }
                if w != 0 {
                    give_weapon(pk, player, w);
                    frontend_audio(6);
                } else if mi() == mi_g(0x8CD5D8) && !vehicle.is_null() {
                    frontend_audio(0xB);
                    g::<u8>(0x978620).write(1);
                    g::<GPtr<u8>>(0x97861C).write(find_player_vehicle(-1, 0));
                    let p = ent_pos(obj());
                    g::<u32>(0x97D660).write(rd(p, 0));
                    g::<u32>(0x97D664).write(rd(p, 4));
                    g::<u32>(0x97D668).write(rd(p, 8));
                    g::<u32>(0x978618).write(now());
                }
            }
            let n = now();
            match rd::<u8>(pk, 0x1C) {
                2 => wr::<u32>(pk, 0xC, n.wrapping_add(30000)),
                15 => {
                    let d = if mi() == mi_g(0x8CD5D0) { 300000 } else { 360000 };
                    wr::<u32>(pk, 0xC, n.wrapping_add(d));
                }
                _ => {}
            }
            *removed = 1;
            pickup_get_rid_of_objects(pk);
            wr::<u8>(pk, 0x1D, rd::<u8>(pk, 0x1D) | 1);
        }
        3 | 4 | 5 | 22 => {
            if !goodies() {
                if player_ped_does_player_want_new_weapon(player, w, 0) == 0 {
                    pickup_extract_ammo_from_pickup(pk, player);
                    return;
                }
                if w != 0 {
                    give_weapon(pk, player, w);
                }
                if mi() != mi_g(0x8CD5E4) {
                    hud_set_help_message(GPtr::NULL, 1, 0, 0);
                    frontend_audio(6);
                }
            }
            *removed = 1;
            pickup_remove(pk);
        }
        8 | 19 => {
            let m = g::<i32>(PLAYER_MONEY as usize);
            m.write(m.read().wrapping_add(rd::<i32>(pk, 8)));
            *removed = 1;
            pickup_remove(pk);
            frontend_audio(7);
            ped_say(player, 0xAC, 0, F1, 0, 0, 0);
        }
        16 => {
            let m = g::<i32>(PLAYER_MONEY.wrapping_add((id as u32).wrapping_mul(0x190)) as usize);
            let v = ftol32(xi(m.read()).fadd_m(rd::<f32>(pk, 0)));
            m.write(v);
            wr::<u32>(pk, 0, 0);
            frontend_audio(7);
        }
        17 => {
            let f = rd::<u8>(pk, 0x1D);
            if f & 4 != 0 {
                return;
            }
            wr::<u8>(pk, 0x1D, f | 4);
            let s = find_string_for_text_index(i32::from(((f | 4) >> 4) & 7));
            hud_set_help_message(text_get(GPtr::from_addr(THE_TEXT), s), 0, 0, 0);
        }
        18 => {
            let s = find_string_for_text_index(i32::from((rd::<u8>(pk, 0x1D) >> 4) & 7));
            // strcpy into a frame buffer (unbounded in the original; the keys are at most 8 characters)
            let mut buf = [0u8; 0x30];
            for (i, b) in buf.iter_mut().enumerate().take(0x2F) {
                *b = rd::<u8>(s, i as isize);
                if *b == 0 {
                    break;
                }
            }
            let bp = GPtr::from_raw(&mut buf).cast::<u8>();
            pickups_modify_string_label_for_control_setting(bp);
            let gxt = GPtr::<u8>::from_addr(0xC1B100);
            let t = text_get(GPtr::from_addr(THE_TEXT), bp);
            messages_insert_number_in_string(t, rd::<u32>(pk, 8), 0, 0, 0, 0, 0, gxt);
            if hud_help_message_displayed() == 0 {
                hud_set_help_message(gxt, 0, 0, 0);
            }
            if g::<u32>(COLLECT_PICKUP_BUFFER).read() == 0 {
                return;
            }
            if is_player_on_a_mission() != 0 {
                help_text(0x859B94, 1, 0, 0);
                return;
            }
            let idx = u32::from(g::<u8>(0xB7CD74).read());
            let m = g::<i32>(PLAYER_MONEY.wrapping_add(idx * 0x190) as usize);
            let price = rd::<i32>(pk, 8);
            if m.read() < price {
                help_text(0x859B8C, 1, 0, 0);
                return;
            }
            m.write(m.read().wrapping_sub(price));
            hud_set_help_message(GPtr::NULL, 1, 0, 0);
            *removed = 1;
            pickup_remove(pk);
        }
        21 => {
            if pad_get_touched_time_delta(get_pad(1)) < 0x1F4 {
                g::<u8>(0xBAA45D).write(1);
                frontend_audio(7);
                *removed = 1;
                pickup_remove(pk);
                hud_set_help_message(GPtr::NULL, 1, 0, 0);
                let mut c = [0f32; 3];
                let p = find_player_coors(GPtr::from_raw(&mut c).cast(), -1).cast::<u8>();
                g::<u32>(0x96A9AC).write(rd(p, 0));
                g::<u32>(0x96A9B0).write(rd(p, 4));
                g::<u32>(0x96A9B4).write(rd(p, 8));
                let h = placeable_get_heading(find_player_ped(-1));
                g::<f32>(0x96A840).write(X87::from_f64_exact(h).to_f32());
            } else {
                let f = rd::<u8>(pk, 0x1D);
                if f & 4 != 0 {
                    return;
                }
                wr::<u8>(pk, 0x1D, f | 4);
                help_text(0x859B84, 1, 0, 1);
            }
        }
        _ => {}
    }
}

/// The default types (not 9..14): the pick-up test, the weapon rules, the collect conditions and the
/// collection.
fn update_default(pk: GPtr<u8>, player: GPtr<u8>, vehicle: GPtr<u8>, id: i32, removed: &mut u8) {
    let obj = || rd::<GPtr<u8>>(pk, 4);
    let mi = i32::from(rd::<i16>(obj(), 0x22));
    let mut picked = false;
    if mi == mi_g(0x8CD5D0) {
        // bribes: in a vehicle only the sphere test counts, on foot only the distance test
        picked = if vehicle.is_null() { near_enough(player, obj()) } else { sphere_touch(vehicle, obj(), F2) };
    } else if mi != mi_g(0x8CD5D8) {
        let special = mi == mi_g(0x8CD5E4) || mi == mi_g(0x8CD5F4) || mi == mi_g(0x8CD5F8);
        if (!special || player_ped_can_player_start_mission(player) != 0)
            && vehicle.is_null()
            && ped_is_alive(player) != 0
            && near_enough(player, obj())
        {
            picked = true;
        }
    }
    if rd::<u32>(obj(), 0x140) & 0x2000000 != 0 {
        picked = false;
    }
    if !player_ped_get_pad_from_player(player).is_null()
        && rd::<u16>(player_ped_get_pad_from_player(player), 0x10E) != 0
        && rd::<i16>(obj(), 0x22) == MODEL_JETPACK
    {
        return;
    }
    if !picked {
        return;
    }
    if !matches!(rd::<u8>(pk, 0x1C), 6 | 8 | 9 | 10 | 11 | 13 | 14 | 16 | 17 | 18 | 19) && !weapon_checks(pk, player, &mut picked) {
        return;
    }
    if !picked || !can_collect(pk, player, id) {
        return;
    }
    collect(pk, player, vehicle, id, removed);
}

/// `CPickup::Update(CPlayerPed* player, CVehicle* vehicle, int32 playerId)` (0x457410, `ret 0xC`, bool in AL:
/// "removed").
///
/// The object is moved to the pickup position (`int16 * 1/8`). Revenue pickups (0x10) accumulate
/// `moneyPerDay * elapsed` (u32 product) `* [0x859BB8]` while the player is more than 10 away (3D, `fsqrt`),
/// capped at the ammo (+8), and show `ftol(revenue * 0.2)` (0 below 10) on the object (+0x13E). Disabled
/// pickups (flag bit 0) get their object back after their time when the player's 2D squared distance (four
/// FindPlayerCoors) exceeds 100 (or 5.76 for type 1); that path writes its temporaries into all three argument
/// slots and returns 0. Otherwise a missing object is created (none: return); types 9..14 run the mines /
/// floating packages; the rest test the pick-up, the weapon rules and collect conditions and collect by type.
/// Finally types 4, 5, 8 time out (`Remove`). Differences from gta-reversed (bytes win): regeneration 30 s
/// (type 2) / 300 s bribe, 360 s other (type 15); mine explosions and the timeout `Remove` leave the result
/// unset; for-sale properties are bought here (money taken, `Remove`); bribes in a vehicle need the sphere
/// test only; bribe wanted level of player 0; the 2P help sets flag bit 2.
#[hook(addr = 0x457410, conv = thiscall, name = "CPickup::Update", this = "CPickup", expect = "83 EC 50 53 55 8B D9",
       unverified = "mvpd-world-5", route = "L2", hazards = "stack-residue,heap-writes,x87-transcendental",
       note = "writes temporaries into its own argument slots +0x64/+0x68/+0x6C (disabled path, mine loops, max armour/health); fsqrt only compared (revenue distance)")]
pub fn pickup_update(this: GPtr<u8>, player: GPtr<u8>, vehicle: GPtr<u8>, id: i32) -> u8 {
    let mut removed = 0u8;
    let obj = || rd::<GPtr<u8>>(this, 4);
    let o = obj();
    if !o.is_null() {
        let px = pk_coord(this, 0x10);
        let py = pk_coord(this, 0x12);
        let m = rd::<GPtr<u8>>(o, 0x14);
        let pz = pk_coord(this, 0x14);
        let p = if m.is_null() { o.byte_add(4) } else { m.byte_add(0x30) };
        wr::<f32>(p, 0, px);
        wr::<f32>(p, 4, py);
        wr::<f32>(p, 8, pz);
    }
    if rd::<u8>(this, 0x1C) == 0x10 {
        let n = now();
        let elapsed = n.wrapping_sub(rd::<u32>(this, 0xC));
        wr::<u32>(this, 0xC, n);
        let px = pk_coord(this, 0x10);
        let py = pk_coord(this, 0x12);
        let pz = pk_coord(this, 0x14);
        let mut c = [0f32; 3];
        let cp = find_player_coors(GPtr::from_raw(&mut c).cast(), -1).cast::<u8>();
        let dx = x(rd::<f32>(cp, 0)).fsub_m(px);
        let dy = x(rd::<f32>(cp, 4)).fsub_m(py);
        let dz = x(rd::<f32>(cp, 8)).fsub_m(pz);
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        if !d.fcom(x(g::<f32>(0x85862C).read())).test_any(0x41) {
            let p = u32::from(rd::<u16>(this, 0x16)).wrapping_mul(elapsed) as i32;
            let mut v = xi(p);
            if p < 0 {
                v = v.fadd_m(g::<f32>(0x858C54).read());
            }
            let r = v.fmul_m(g::<f32>(0x859BB8).read()).fadd_m(rd::<f32>(this, 0)).to_f32();
            wr::<f32>(this, 0, r);
        }
        let ammo = xi(rd::<i32>(this, 8));
        let rev = x(rd::<f32>(this, 0));
        let r = if rev.fcom(ammo).test_parity(5) { ammo } else { rev };
        let o = obj();
        wr::<f32>(this, 0, r.to_f32());
        if !o.is_null() {
            if r.fcom(x(g::<f32>(0x85862C).read())).test_parity(5) {
                wr::<u16>(o, 0x13E, ftol32(r.fmul_m(g::<f32>(0x858CC4).read())) as u16);
            } else {
                wr::<u16>(o, 0x13E, 0);
            }
        }
    }
    if rd::<u8>(this, 0x1D) & 1 != 0 {
        if now() <= rd::<u32>(this, 0xC) {
            return removed;
        }
        let px = pk_coord(this, 0x10);
        let py = pk_coord(this, 0x12);
        let mut b = [[0f32; 3]; 4];
        let [b1, b2, b3, b4] = &mut b;
        let c1 = find_player_coors(GPtr::from_raw(b1).cast(), -1).cast::<u8>();
        let c2 = find_player_coors(GPtr::from_raw(b2).cast(), -1).cast::<u8>();
        let c3 = find_player_coors(GPtr::from_raw(b3).cast(), -1).cast::<u8>();
        let dy = x(rd::<f32>(c3, 4)).fsub_m(py).to_f32();
        let c4 = find_player_coors(GPtr::from_raw(b4).cast(), -1).cast::<u8>();
        let yy = x(rd::<f32>(c4, 4)).fsub_m(py).fmul_m(dy);
        let xx = x(rd::<f32>(c2, 0)).fsub_m(px).fmul(x(rd::<f32>(c1, 0)).fsub_m(px));
        let s = yy.fadd(xx);
        let give = if !s.fcom(x(g::<f32>(0x858628).read())).test_any(0x41) {
            true
        } else {
            rd::<u8>(this, 0x1C) == 1 && !s.fcom(x(g::<f32>(0x858FB0).read())).test_any(0x41)
        };
        if give {
            give_us_a_pick_up_object(this, this.byte_add(4).cast(), -1);
            let o = obj();
            if !o.is_null() {
                world_add(o);
                wr::<u8>(this, 0x1D, rd::<u8>(this, 0x1D) & 0xFE);
            }
        }
        return removed;
    }
    if obj().is_null() {
        give_us_a_pick_up_object(this, this.byte_add(4).cast(), -1);
        let o = obj();
        if !o.is_null() {
            world_add(o);
        }
    }
    if obj().is_null() {
        return removed;
    }
    if (9..=14).contains(&rd::<u8>(this, 0x1C)) {
        update_mines(this, vehicle, &mut removed);
    } else {
        update_default(this, player, vehicle, id, &mut removed);
    }
    if rd::<u8>(this, 0x1D) & 1 == 0 && matches!(rd::<u8>(this, 0x1C), 4 | 5 | 8) && now() > rd::<u32>(this, 0xC) {
        // the original removes without setting its result flag
        pickup_remove(this);
    }
    removed
}


// ---------------------------------------------------------------------------------------------------------
// CPickups::RenderPickUpText
// ---------------------------------------------------------------------------------------------------------

orig! {
    /// CRT `sprintf(buf, fmt, int)` (0x821BB5, cdecl)
    fn sprintf_int(buf: GPtr<u8>, fmt: GPtr<u8>, v: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600, cdecl)
    fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// `CFont::SetProportional(bool)` (0x7195B0, cdecl)
    fn font_set_proportional(p: u32) = cdecl 0x7195B0 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0, cdecl)
    fn font_set_background(a: u32, b: u32) = cdecl 0x7195C0 => unported;
    /// `CFont::SetScale(float w, float h)` (0x719380, cdecl); the floats as pushed dwords
    fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => unported;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610, cdecl)
    fn font_set_orientation(o: u32) = cdecl 0x719610 => unported;
    /// `CFont::SetCentreSize(float)` (0x7194E0, cdecl)
    fn font_set_centre_size(s: f32) = cdecl 0x7194E0 => unported;
    /// `CRGBA::CRGBA(uint8 r, g, b, a)` (0x7170C0, `ret 0x10`); returns `this`
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CFont::SetColor(CRGBA)` (0x719430, cdecl)
    fn font_set_color(c: u32) = cdecl 0x719430 => unported;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490, cdecl)
    fn font_set_font_style(s: u32) = cdecl 0x719490 => unported;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700, cdecl); x, y as pushed dwords
    fn font_print_string(x: u32, y: u32, s: GPtr<u8>) = cdecl 0x71A700 => unported;
}

/// `CPickups::aMessages` (0x978680, 0x24 bytes each: x, y, +8, width/height scale +0xC/+0x10, RGBA +0x14,
/// flags +0x18, +0x19, price +0x1C, text +0x20)
const PICKUP_MESSAGES: u32 = 0x978680;
/// `gGxtString` (0xC1B100)
const GXT_STRING: usize = 0xC1B100;

/// `fild` of `RsGlobal.maximumWidth / 640` (stored as a float) against `scale * [0x858F10]`: the smaller (the
/// product when not below, or unordered).
fn text_scale(q: f32, s: f32) -> u32 {
    let p = x(s).fmul_m(g::<f32>(0x858F10).read());
    if x(q).fcom(p).test_parity(5) { p.to_f32().to_bits() } else { q.to_bits() }
}

/// `CPickups::RenderPickUpText()` (0x455000, cdecl).
///
/// Nothing when `NumMessages` (0x978678) is 0 (left as it is). Otherwise every message is drawn: a price (+0x1C)
/// printed with "%d" (0x859B10) into 0xB71670 and converted to `gGxtString`; else its text (+0x20), through
/// `InsertNumberInString(text, 0, ...)` into `gGxtString` when flag bit 1 is set; messages with neither are
/// skipped (the original then stores the entry's +8 into a dead frame slot, indexing it with the loop counter
/// left in ECX; not reproduced). Drawing: proportional, no background, scale `min(maximumWidth / 640, w|h *
/// [0x858F10])`, left aligned, centre size `maximumWidth`, the entry's colour, style 3, at (x, y). The count
/// is re-read only after a drawn message; `NumMessages` is cleared at the end.
#[hook(addr = 0x455000, conv = cdecl, name = "CPickups::RenderPickUpText", expect = "66 8B 15 78 86 97 00",
       unverified = "mvpd-world-5", route = "L2", hazards = "none",
       note = "skipped entries: the original's dead store of entry+8 into its own frame (stale ECX = index) is not reproduced")]
pub fn render_pick_up_text() {
    let num = g::<u16>(0x978678);
    let mut n = num.read();
    if n < 1 {
        return;
    }
    let mut i: i16 = 0;
    loop {
        let e = GPtr::<u8>::from_addr(PICKUP_MESSAGES.wrapping_add((i as u16 as u32) * 0x24) as usize);
        let price = rd::<u32>(e, 0x1C);
        let text = if price != 0 {
            sprintf_int(GPtr::from_addr(0xB71670), GPtr::from_addr(0x859B10), price);
            ascii_to_gxt_char(GPtr::from_addr(0xB71670), GPtr::from_addr(GXT_STRING));
            Some(GPtr::<u8>::from_addr(GXT_STRING))
        } else {
            let t = rd::<GPtr<u8>>(e, 0x20);
            if t.is_null() {
                None
            } else if rd::<u8>(e, 0x18) & 2 == 0 {
                Some(t)
            } else {
                let gxt = GPtr::from_addr(GXT_STRING);
                messages_insert_number_in_string(t, price, 0, 0, 0, 0, 0, gxt);
                Some(gxt)
            }
        };
        if let Some(s) = text {
            font_set_proportional(1);
            font_set_background(0, 0);
            let q = xi(g::<i32>(0xC17044).read() / 640).to_f32();
            let h = text_scale(q, rd::<f32>(e, 0x10));
            let w = text_scale(q, rd::<f32>(e, 0xC));
            font_set_scale(w, h);
            font_set_orientation(0);
            font_set_centre_size(xi(g::<i32>(0xC17044).read()).to_f32());
            let mut c = [0u8; 4];
            let cp = rgba_ctor(
                GPtr::from_raw(&mut c).cast(),
                u32::from(rd::<u8>(e, 0x14)),
                u32::from(rd::<u8>(e, 0x15)),
                u32::from(rd::<u8>(e, 0x16)),
                u32::from(rd::<u8>(e, 0x17)),
            );
            font_set_color(rd::<u32>(cp, 0));
            font_set_font_style(3);
            font_print_string(rd::<u32>(e, 0), rd::<u32>(e, 4), s);
            n = num.read();
        }
        i = i.wrapping_add(1);
        if i32::from(i) >= i32::from(n) {
            break;
        }
    }
    num.write(0);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SET_PIECE_UPDATE, HOOK_PICKUP_UPDATE, HOOK_RENDER_PICK_UP_TEXT];
