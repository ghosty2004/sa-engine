//! WRITER-VEH-BIG (batch `vehbig-1`): `CGarage::Update(int32 index)`, the per-frame state machine
//! of one garage (Pay 'n' Spray, bomb shops, mission garages, collect / crusher garages, hideouts,
//! tuning shops). Ported from the 1.0 US bytes; gta-reversed has no body for it.
//!
//! Fields used (see `garages.rs`): +0x8 base z, +0x1C top z, +0x28 / +0x2C min / max x, +0x30 /
//! +0x34 min / max y, +0x38 door position, +0x3C timer, +0x40 target vehicle, +0x4C type, +0x4D door
//! state (0 closed, 1 open, 2 closing, 3 opening, 4 open with car / refused, 5 closed after a
//! crush), +0x4E flags.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x, xi};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => crate::world::cworld::find_player_ped;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => crate::world::cworld::find_player_vehicle;
    /// `CWorld::FindPlayerCoors(CVector*, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => crate::world::cworld::find_player_coors;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => crate::world::cworld::find_player_wanted;
    /// `CGarage::IsEntityEntirelyInside3D(CEntity*, float)` (0x448BE0)
    fn is_entity_entirely_inside_3d(gp: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 = thiscall 0x448BE0 => crate::world::mvpb_world3::garage_is_entity_entirely_inside_3d;
    /// `CGarage::IsEntityEntirelyOutside(CEntity*, float)` (0x448D30)
    fn is_entity_entirely_outside(gp: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 = thiscall 0x448D30 => crate::world::mvpb_world2::garage_is_entity_entirely_outside;
    /// `CGarage::IsStaticPlayerCarEntirelyInside()` (0x44A830)
    fn is_static_player_car_entirely_inside(gp: GPtr<u8>) -> u8 = thiscall 0x44A830 => crate::world::mvpb_world3::garage_is_static_player_car_entirely_inside;
    /// `thunk_FUN_01569180(float margin)` (0x448E50): the player (vehicle) entirely outside
    fn is_player_entirely_outside(gp: GPtr<u8>, margin: f32) -> u32 = thiscall 0x448E50 => crate::world::mvpb_world::garage_is_player_entirely_outside;
    /// `CGarage::IsEntityTouching3D(CEntity*)` (0x448EE0)
    fn is_entity_touching_3d(gp: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x448EE0 => unported;
    /// `CGarage::IsGarageEmpty()` (0x44A9C0)
    fn is_garage_empty(gp: GPtr<u8>) -> u8 = thiscall 0x44A9C0 => unported;
    /// `FUN_00447d80(float x, float y)` (0x447D80): squared 2D distance to the garage box, ST0
    fn dist_sq_to_box(gp: GPtr<u8>, px: f32, py: f32) -> f64 = thiscall 0x447D80 => dist_sq_to_box_port;
    /// `FUN_0044a750()` (0x44A750): slides the door shut, true once closed
    fn slide_door_closed(gp: GPtr<u8>) -> u8 = thiscall 0x44A750 => unported;
    /// `FUN_0044a660()` (0x44A660): slides the door open, true once open
    fn slide_door_open(gp: GPtr<u8>) -> u8 = thiscall 0x44A660 => unported;
    /// `FUN_00449220(CVehicle*)` (0x449220): pushes cars near the door out of the garage
    fn throw_cars_near_door_out(gp: GPtr<u8>, veh: GPtr<u8>) = thiscall 0x449220 => unported;
    /// `thunk_FUN_0156d610()` (0x4494F0): another car blocks the garage
    fn is_any_car_blocking(gp: GPtr<u8>) -> u8 = thiscall 0x4494F0 => unported;
    /// `thunk_FUN_01561400(CVehicle*)` (0x4495F0): the count of cars inside except the given one
    fn count_cars_inside(gp: GPtr<u8>, veh: GPtr<u8>) -> i32 = thiscall 0x4495F0 => unported;
    /// `thunk_FUN_01566680(CPed*)` (0x449100): another ped touches the garage
    fn is_any_other_ped_touching(gp: GPtr<u8>, ped: u32) -> u8 = thiscall 0x449100 => unported;
    /// `thunk_FUN_01565260(CVehicle*)` (0x447720): the vehicle suits a tuning garage
    fn vehicle_suits_garage(gp: GPtr<u8>, veh: GPtr<u8>) -> u8 = thiscall 0x447720 => crate::world::garages::vehicle_suits_garage;
    /// `FUN_00449ff0(CObject**, CObject**)` (0x449FF0): the garage's door objects
    fn find_doors(gp: GPtr<u8>, a: GPtr<u32>, b: GPtr<u32>) = thiscall 0x449FF0 => unported;
    /// `FUN_00449a50(CStoredCar*, int32)` (0x449A50)
    fn hideout_store_cars(gp: GPtr<u8>, stored: GPtr<u8>, max: i32) = thiscall 0x449A50 => unported;
    /// `FUN_00448330(CStoredCar*)` (0x448330)
    fn hideout_prepare(gp: GPtr<u8>, stored: GPtr<u8>) = thiscall 0x448330 => unported;
    /// `thunk_FUN_01561490(CStoredCar*)` (0x4485C0)
    fn hideout_restore(gp: GPtr<u8>, stored: GPtr<u8>) -> u8 = thiscall 0x4485C0 => unported;
    /// `thunk_FUN_0156d8d0(CStoredCar*)` (0x448550)
    fn hideout_restore_cars(gp: GPtr<u8>, stored: GPtr<u8>) -> u8 = thiscall 0x448550 => unported;
    /// `CGarage::RemoveCarsBlockingDoorNotInside()` (0x449690)
    fn remove_cars_blocking_door_not_inside(gp: GPtr<u8>) = thiscall 0x449690 => unported;
    /// `CGarage::StoreAndRemoveCarsForThisHideOut(CStoredCar*, int32)` (0x449900)
    fn store_and_remove_cars(gp: GPtr<u8>, stored: GPtr<u8>, max: i32) = thiscall 0x449900 => unported;
    /// `CGarages::FindSafeHouseIndexForGarageType(eGarageType)` (0x4489F0)
    fn find_safe_house_index(ty: i32) -> i32 = cdecl 0x4489F0 => crate::world::misc2::find_safe_house_index_for_garage_type;
    /// `CGarages::CountCarsInHideoutGarage(eGarageType)` (0x44A210)
    fn count_cars_in_hideout(ty: u8) -> i32 = cdecl 0x44A210 => crate::world::misc2::count_cars_in_hideout_garage;
    /// `CGarages::GivePlayerDetonator()` (0x448660)
    fn give_player_detonator() = cdecl 0x448660 => unported;
    /// `CGarages::IsCarSprayable(CVehicle*)` (0x4479A0)
    fn is_car_sprayable(veh: GPtr<u8>) -> u8 = cdecl 0x4479A0 => crate::world::misc2::is_car_sprayable;
    /// `CGarages::TriggerMessage(const char*, int16, uint16, int16)` (0x447B80)
    fn trigger_message(msg: u32, a: i32, time: u32, b: i32) = cdecl 0x447B80 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0) on `AudioEngine`
    fn report_frontend_audio_event(ae: GPtr<u8>, ev: u32, vol: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40), ST0
    fn get_stat_value(stat: u16) -> f64 = cdecl 0x558E40 => crate::world::restart::stat_value_port;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180)
    fn increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => crate::world::mvpb_stats::increment_stat;
    /// `CStats::SetStatValue(eStats, float)` (0x55A070)
    fn set_stat_value(stat: u16, v: f32) = cdecl 0x55A070 => crate::world::stats::set_stat_value;
    /// `CWanted::ClearWantedLevelAndGoOnParole()` (0x5625A0)
    fn clear_wanted_level_and_go_on_parole(w: GPtr<u8>) = thiscall 0x5625A0 => unported;
    /// `std::max<float>` (0x420800), ST0
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => crate::world::entity_prerender::max_f_port;
    /// `CVehicle::GetRemapIndex()` (0x6D0B70)
    fn get_remap_index(veh: GPtr<u8>) -> i32 = thiscall 0x6D0B70 => unported;
    /// `CVehicle::SetRemap(int32)` (0x6D0C00)
    fn set_remap(veh: GPtr<u8>, idx: i32) = thiscall 0x6D0C00 => unported;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500)
    fn choose_vehicle_colour(mi: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>, shift: i32) = thiscall 0x4C8500 => sa_stream::modelinfo::vehicle::choose_vehicle_colour;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, lastFactor, life)` (0x4AB290)
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, gg: f32, b: f32, a: f32, size: f32, last: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<f32>, vel: GPtr<f32>, time: f32, mult: GPtr<u8>, brightness: f32, zf: f32, camf: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90), ST0
    fn random_float_in_range(lo: f32, hi: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CVehicle::DestroyVehicleAndDriverAndPassengers(CVehicle*)` (0x6D2250)
    fn destroy_vehicle_and_occupants(veh: GPtr<u8>) = cdecl 0x6D2250 => unported;
    /// `CWorld::CallOffChaseForArea(float, float, float, float)` (0x566A60)
    fn call_off_chase_for_area(x1: f32, y1: f32, x2: f32, y2: f32) = cdecl 0x566A60 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CText::Get(const char*)` (0x6A0050) on `TheText`
    fn text_get(t: GPtr<u8>, key: u32) -> u32 = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0)
    fn set_help_message(text: u32, quick: u32, permanent: u32, brief: u32) = cdecl 0x588BE0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CVehicle::Fix()` (vtable +0xC8)
    fn v_fix(veh: GPtr<u8>) = thiscall vslot 0xC8 => unported;
}

/// The f64 face of `garage_dist_sq_to_box` for `orig!` (ST0 returns cross as f64).
pub fn dist_sq_to_box_port(gp: GPtr<u8>, px: f32, py: f32) -> f64 {
    super::mvpb_world::garage_dist_sq_to_box(gp, px, py).to_f64()
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// `CTimer::m_FrameCounter`
const FRAME_COUNTER: usize = 0xB7_CB4C;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6_BC90;
/// `TheText`
const THE_TEXT: usize = 0xC1_B340;
/// `CGarages::aCarsInSafeHouse` (0x100 bytes per safe house)
const CARS_IN_SAFE_HOUSE: usize = 0x96_ABD8;
/// `CGarages::LastGaragePlayerWasIn`
const LAST_GARAGE: usize = 0x96_BFDC;
/// `CGarages::LastTimeHelpMessage`
const LAST_HELP_TIME: usize = 0x96_BFE0;
/// `CGarages::RespraysAreFree`, `CGarages::BombsAreFree`, `CGarages::NoResprays`
const RESPRAYS_FREE: usize = 0x96_C009;
const BOMBS_FREE: usize = 0x96_C00A;
const NO_RESPRAYS: usize = 0x96_C008;

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

fn now() -> u32 {
    g::<u32>(TIME_MS).read()
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn player_vehicle() -> GPtr<u8> {
    find_player_vehicle(-1, 0)
}

/// `FindPlayerCoors(&tmp, -1)` copied out.
fn coors() -> [f32; 3] {
    let mut t = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(t.as_mut_ptr()), -1);
    [p.read(), p.byte_add(4).read(), p.byte_add(8).read()]
}

/// The player money of the current player (`CWorld::Players[PlayerInFocus].m_nMoney`).
fn money() -> GPtr<i32> {
    let idx = usize::from(g::<u8>(0xB7_CD74).read());
    GPtr::from_addr(0xB7_CE50 + idx * 0x190)
}

/// `money = max(money - cost, 0)` when positive (the `setl` / `dec` / `and` sequence).
fn charge(cost: i32) {
    let m = money();
    let v = m.read();
    if v > 0 {
        let r = v.wrapping_sub(cost);
        m.write(if r < 0 { 0 } else { r });
    }
}

fn audio(ev: u32) {
    report_frontend_audio_event(GPtr::from_addr(AUDIO_ENGINE), ev, 0.0, 1.0);
}

fn message(msg: u32) {
    trigger_message(msg, -1, 4000, -1);
}

fn help(key: u32) {
    let t = text_get(GPtr::from_addr(THE_TEXT), key);
    set_help_message(t, 0, 0, 1);
}

/// `GetPad(0)->DisablePlayerControls` bit 2 (+0x10E).
fn pad_lock(on: bool) {
    let p = get_pad(0);
    let v = rd::<u8>(p, 0x10E);
    wr::<u8>(p, 0x10E, if on { v | 4 } else { v & 0xFB });
}

/// `FindPlayerWanted(-1)` +0x1E bit 1 (police back off while in the garage).
fn wanted_hold(on: bool) {
    let w = find_player_wanted(-1).byte_add(0x1E);
    let v = rd::<u8>(w, 0);
    wr::<u8>(w, 0, if on { v | 2 } else { v & 0xFD });
}

fn state(gp: GPtr<u8>) -> u8 {
    rd::<u8>(gp, 0x4D)
}

fn set_state(gp: GPtr<u8>, s: u8) {
    wr::<u8>(gp, 0x4D, s);
}

fn flags(gp: GPtr<u8>) -> u8 {
    rd::<u8>(gp, 0x4E)
}

fn set_flags(gp: GPtr<u8>, f: u8) {
    wr::<u8>(gp, 0x4E, f);
}

fn safe_house(ty: u8) -> GPtr<u8> {
    let i = find_safe_house_index(i32::from(ty));
    GPtr::from_addr((i << 8) as u32 as usize + CARS_IN_SAFE_HOUSE)
}

/// The squared player distance to the garage centre, from four `FindPlayerCoors` calls:
/// `(y3 - cy) * (y4 - cy) + (x2 - cx) * (x1 - cx)`, the first factor rounded to f32.
fn centre_dist_sq(gp: GPtr<u8>) -> X87 {
    let c1 = coors();
    let c2 = coors();
    let c3 = coors();
    let cy = || ld(gp, 0x30).fadd_m(rd::<f32>(gp, 0x34)).fmul_m(0.5);
    let l1e8 = cy().fsubr_m(c3[1]).to_f32();
    let c4 = coors();
    let yy = cy().fsubr_m(c4[1]).fmul_m(l1e8);
    let cx = || ld(gp, 0x2C).fadd_m(rd::<f32>(gp, 0x28)).fmul_m(0.5);
    let xa = cx().fsubr_m(c2[0]);
    let xb = cx().fsubr_m(c1[0]);
    yy.fadd(xa.fmul(xb))
}

fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

/// The squared box distance of the player vehicle (two position reads: x of the second, y of the
/// first).
fn vehicle_box_dist(gp: GPtr<u8>) -> X87 {
    let p1 = pos_ptr(player_vehicle());
    let p2 = pos_ptr(player_vehicle());
    X87::from_f64_exact(dist_sq_to_box(gp, rd::<f32>(p2, 0), rd::<f32>(p1, 4)))
}

/// The squared box distance of the player (x of the first `FindPlayerCoors`, y of the second).
fn player_box_dist(gp: GPtr<u8>) -> X87 {
    let c1 = coors();
    let c2 = coors();
    X87::from_f64_exact(dist_sq_to_box(gp, c1[0], c2[1]))
}

/// `CGarage::Update(int32 garageId)` (0x44AA50).
///
/// Except for type 13 and while the player ped exists, the garage is not inactive (+0x4E bit 5) and
/// the state is at most 5: the player ped (or the kart it drives, model 0x23B) entirely inside
/// (margin 0.25) sets `CGarages::bCamShouldBeOutside` / the camera garage (0xB6F844); a vehicle
/// not entirely outside becomes 0xB6F848; a Dumper-sized model 0x1A7 within 0.5 of the box also
/// sets the camera flags. Inactive closed garages stop there. Flag 8 garages keep bit 6 clear
/// while open or opening past 0.4. Then the type switch:
/// - 5 (Pay 'n' Spray, below z 950): open: a sprayable, paid-for car inside closes the door
///   (controls locked, +0x4B0 cleared), else a refusal message and state 4; the wanted level is
///   held while inside; within 8 of the box `CallOffChaseForArea` (box +-10). Closing: throws cars
///   out, once shut the spray statistics and a 2 s timer; the car gets +0x8E4 = 0 and +0x42E bit 7.
///   Closed: after the timer opens, clears the wanted level (parole), repairs (health at least
///   1000, `Fix`, upside-down cars flipped), repaints with fresh colours and paint particles,
///   charges 100 unless free and shows the result message (random of two when only repainted).
///   Opening: unlocks past 0.5. State 4: open again once the player is outside.
/// - 2 / 3 / 4 (bomb shops): open: a car without a bomb is fitted (state 2) or refused (no money,
///   already fitted, bikes); closing / closed: fits the bomb type (`type - 1`), owner, detonator for
///   type 4, charges 500 unless free, the help text by pad mode; type 4 streams model 0x16C.
/// - 1, 14, 21, 20, 36..38, 43 (mission / collect / tuning garages), 11, 15, 19, 23: the door
///   cycles around the target vehicle +0x40 (crushers destroy it; type 14 re-opens after 2 s).
/// - 16..18, 24..32, 39..42, 44, 45 (hideouts): closes behind the player when inside and nothing
///   blocks, stores the cars (4 slots) once shut, opens when the player arrives (help message when
///   a full garage's door is within 5, every 18 s).
/// - 33..35: hideouts that store 3 cars, opened from below the roof within 60 / closed beyond 65.
///
/// Kept from the bytes: the closing hideout ignores the door's slide result; distances mix two
/// `FindPlayerCoors` / `FindPlayerVehicle` reads.
#[hook(addr = 0x44aa50, conv = thiscall, name = "CGarage::Update", this = "CGarage",
       expect = "81 EC E8 01 00 00 53",
       unverified = "vehbig-2", route = "split", hazards = "heap-writes",
       recs = "driving",
       note = "rand through at_site(0x44B36C); the PrtMult / colour locals live on the Rust stack")]
pub fn garage_update(gp: GPtr<u8>, garage_id: i32) {
    let ty = rd::<u8>(gp, 0x4C);
    if ty != 0xD && state(gp) <= 5 && !find_player_ped(-1).is_null() && flags(gp) & 0x20 == 0 {
        camera_checks(gp);
    }
    let fl = flags(gp);
    if fl & 2 != 0 && state(gp) == 0 {
        return;
    }
    if fl & 8 != 0 {
        let s = state(gp);
        let open = (s == 3 && !le(ld(gp, 0x38), c(0x3ECC_CCCD))) || s == 1;
        set_flags(gp, if open { fl & 0xBF } else { fl | 0x40 });
    }
    match ty {
        5 => {
            if ge(x(coors()[2]), c(0x446D_8000)) {
                return;
            }
            match state(gp) {
                0 => spray_closed(gp),
                1 => spray_open(gp, garage_id),
                2 => spray_closing(gp),
                3 => opening_unlock(gp),
                4 if is_player_entirely_outside(gp, 0.0) & 0xFF != 0 => set_state(gp, 1),
                _ => {}
            }
        }
        2..=4 => match state(gp) {
            0 => bomb_closed(gp, ty),
            1 => bomb_open(gp),
            2 => {
                let v = player_vehicle();
                if !v.is_null() {
                    throw_cars_near_door_out(gp, player_vehicle());
                }
                if slide_door_closed(gp) != 0 {
                    set_state(gp, 0);
                    wr::<u32>(gp, 0x3C, now().wrapping_add(2000));
                }
                if rd::<u8>(gp, 0x4C) == 4 {
                    request_model(0x16C, 2);
                }
            }
            3 => opening_unlock(gp),
            4 if is_player_entirely_outside(gp, 0.0) & 0xFF != 0 => set_state(gp, 1),
            _ => {}
        },
        1 => match state(gp) {
            0 => target_arrives(gp, 0x4280_0000),
            1 => mission_open(gp),
            2 => mission_closing(gp, false),
            3 => open_door(gp),
            _ => {}
        },
        11 => match state(gp) {
            1 => {
                if is_garage_empty(gp) != 0 {
                    set_state(gp, 2);
                }
            }
            2 => {
                if slide_door_closed(gp) != 0 {
                    set_state(gp, 0);
                }
                if is_garage_empty(gp) == 0 {
                    set_state(gp, 3);
                }
            }
            3 => open_door(gp),
            _ => {}
        },
        14 | 21 => match state(gp) {
            0 => target_arrives(gp, 0x4390_8000),
            1 => collect_open(gp),
            2 => mission_closing(gp, true),
            3 => open_door(gp),
            5 if ty == 0xE && now() > rd::<u32>(gp, 0x3C) => set_state(gp, 3),
            _ => {}
        },
        15 => {
            if state(gp) == 3 {
                open_door(gp);
            }
        }
        19 => match state(gp) {
            2 => close_door(gp),
            3 => open_door(gp),
            _ => {}
        },
        20 => match state(gp) {
            0 => target_arrives(gp, 0x4280_0000),
            1 => {
                if le(centre_dist_sq(gp), c(0x4461_0000)) {
                    return;
                }
                let t = rd::<GPtr<u8>>(gp, 0x40);
                if t.is_null() || is_entity_entirely_outside(gp, t, 0.0) == 0 {
                    return;
                }
                set_flags(gp, flags(gp) | 1);
                set_state(gp, 2);
            }
            2 => {
                let t = rd::<GPtr<u8>>(gp, 0x40);
                if !t.is_null() {
                    throw_cars_near_door_out(gp, t);
                }
                close_door(gp);
            }
            3 => open_door(gp),
            _ => {}
        },
        23 => match state(gp) {
            1 => {
                let t = rd::<GPtr<u8>>(gp, 0x40);
                if t.is_null()
                    || is_entity_entirely_inside_3d(gp, t, 0.0) == 0
                    || is_any_car_blocking(gp) != 0
                    || is_player_entirely_outside(gp, 0.0) & 0xFF == 0
                {
                    return;
                }
                pad_lock(true);
                set_flags(gp, flags(gp) & 0xFE);
                set_state(gp, 2);
            }
            2 => {
                if slide_door_closed(gp) != 0 {
                    set_state(gp, 0);
                    pad_lock(false);
                }
            }
            3 => open_door(gp),
            _ => {}
        },
        33..=35 => match state(gp) {
            0 => small_hideout_closed(gp),
            1 | 2 => small_hideout_open(gp),
            _ => {}
        },
        36..=38 => match state(gp) {
            0 => {
                if vehicle_suits_garage(gp, player_vehicle()) == 0 {
                    return;
                }
                let p1 = pos_ptr(player_vehicle());
                let p2 = pos_ptr(player_vehicle());
                let d = X87::from_f64_exact(dist_sq_to_box(gp, rd::<f32>(p2, 0), rd::<f32>(p1, 4)));
                if !ge(d, c(0x4280_0000)) {
                    set_state(gp, 3);
                }
            }
            1 => {
                if le(centre_dist_sq(gp), c(0x4461_0000)) || g::<u32>(FRAME_COUNTER).read() & 0x1F != 0 {
                    return;
                }
                if vehicle_suits_garage(gp, player_vehicle()) != 0
                    && is_entity_touching_3d(gp, player_vehicle()) != 0
                {
                    return;
                }
                if is_any_other_ped_touching(gp, 0) != 0 {
                    return;
                }
                set_flags(gp, flags(gp) | 1);
                set_state(gp, 2);
            }
            2 => {
                if !player_vehicle().is_null() {
                    throw_cars_near_door_out(gp, player_vehicle());
                }
                close_door(gp);
            }
            3 => open_door(gp),
            _ => {}
        },
        43 => match state(gp) {
            1 => {
                if le(centre_dist_sq(gp), c(0x4461_0000)) || is_any_other_ped_touching(gp, 0) != 0 {
                    return;
                }
                set_state(gp, 2);
            }
            2 => {
                if !player_vehicle().is_null() {
                    throw_cars_near_door_out(gp, player_vehicle());
                }
                close_door(gp);
            }
            3 => open_door(gp),
            _ => {}
        },
        16..=18 | 24..=32 | 39..=42 | 44 | 45 => match state(gp) {
            0 => hideout_closed(gp),
            1 => hideout_open(gp),
            2 => hideout_closing(gp),
            3 => open_door(gp),
            _ => {}
        },
        _ => {}
    }
}

/// 0x44AA69..0x44ABB6: the camera-garage checks.
fn camera_checks(gp: GPtr<u8>) {
    let veh = player_vehicle();
    let mut ent = find_player_ped(-1);
    if rd::<u32>(find_player_ped(-1), 0x46C) & 0x100 != 0
        && !rd::<GPtr<u8>>(find_player_ped(-1), 0x58C).is_null()
        && rd::<u16>(rd::<GPtr<u8>>(find_player_ped(-1), 0x58C), 0x22) == 0x23B
    {
        ent = rd::<GPtr<u8>>(find_player_ped(-1), 0x58C);
    }
    if is_entity_entirely_inside_3d(gp, ent, 0.25) == 1 {
        g::<u8>(0x96_BFE4).write(1);
        g::<u32>(0xB6_F844).write(gp.addr() as u32);
    }
    if veh.is_null() {
        return;
    }
    if is_entity_entirely_outside(gp, veh, 0.0) == 0 {
        g::<u32>(0xB6_F848).write(gp.addr() as u32);
    }
    if rd::<u16>(veh, 0x22) != 0x1A7 {
        return;
    }
    let p = pos_ptr(veh);
    let (px, py) = (rd::<f32>(p, 0), rd::<f32>(p, 4));
    if ge(ld(gp, 0x28).fsub_m(0.5), x(px))
        || le(ld(gp, 0x2C).fadd_m(0.5), x(px))
        || ge(ld(gp, 0x30).fsub_m(0.5), x(py))
        || le(ld(gp, 0x34).fadd_m(0.5), x(py))
    {
        return;
    }
    g::<u8>(0x96_BFE4).write(1);
    g::<u32>(0xB6_F844).write(gp.addr() as u32);
}

/// State 3 of the shops (0x44B79C): opening; past 0.5 the controls and the wanted level free up.
fn opening_unlock(gp: GPtr<u8>) {
    if slide_door_open(gp) != 0 {
        set_state(gp, 4);
    }
    if le(ld(gp, 0x38), c(0x3F00_0000)) {
        return;
    }
    pad_lock(false);
    wanted_hold(false);
}

/// 0x44BCBE: open the door; state 1 once open.
fn open_door(gp: GPtr<u8>) {
    if slide_door_open(gp) != 0 {
        set_state(gp, 1);
    }
}

/// 0x44C69D: close the door; state 0 once shut.
fn close_door(gp: GPtr<u8>) {
    if slide_door_closed(gp) != 0 {
        set_state(gp, 0);
    }
}

/// 0x44B3C1: `CWorld::CallOffChaseForArea` over the box grown by 10.
fn call_off_chase(gp: GPtr<u8>) {
    let y2 = ld(gp, 0x34).fadd_m(10.0).to_f32();
    let x2 = ld(gp, 0x2C).fadd_m(10.0).to_f32();
    let y1 = ld(gp, 0x30).fsub_m(10.0).to_f32();
    let x1 = ld(gp, 0x28).fsub_m(10.0).to_f32();
    call_off_chase_for_area(x1, y1, x2, y2);
}

/// Type 5, state 1 (0x44AC50).
fn spray_open(gp: GPtr<u8>, garage_id: i32) {
    if g::<u8>(NO_RESPRAYS).read() != 0 {
        return;
    }
    if is_static_player_car_entirely_inside(gp) != 0 {
        if is_car_sprayable(player_vehicle()) != 0 {
            if money().read() < 100 && g::<u8>(RESPRAYS_FREE).read() == 0 {
                message(0x85_9A58);
                set_state(gp, 4);
                audio(0xE);
            } else {
                set_state(gp, 2);
                pad_lock(true);
                wr::<u32>(player_vehicle(), 0x4B0, 0);
            }
        } else {
            let msg = if rd::<u32>(player_vehicle(), 0x594) == 0xA {
                0x85_9A50
            } else {
                0x85_9A48
            };
            message(msg);
            set_state(gp, 4);
            audio(0xF);
        }
        wanted_hold(true);
        g::<i32>(LAST_GARAGE).write(garage_id);
    } else if is_player_entirely_outside(gp, 0.0) & 0xFF == 0 {
        wanted_hold(true);
        g::<i32>(LAST_GARAGE).write(garage_id);
    } else if garage_id == g::<i32>(LAST_GARAGE).read() {
        wanted_hold(false);
    }
    if player_vehicle().is_null() {
        return;
    }
    if ge(vehicle_box_dist(gp), c(0x4280_0000)) {
        return;
    }
    call_off_chase(gp);
}

/// Type 5, state 2 (0x44AE0C).
fn spray_closing(gp: GPtr<u8>) {
    if !player_vehicle().is_null() {
        throw_cars_near_door_out(gp, player_vehicle());
    }
    if slide_door_closed(gp) != 0 {
        set_state(gp, 0);
        audio(0x10);
        wr::<u32>(gp, 0x3C, now().wrapping_add(2000));
        // the float result is stored into the pushed argument slot IncrementStat reads
        let v = X87::from_f64_exact(get_stat_value(0xB0)).to_f32();
        increment_stat(0xB1, v);
        set_stat_value(0xB0, 0.0);
    }
    let v = player_vehicle();
    if !v.is_null() {
        wr::<u32>(player_vehicle(), 0x8E4, 0);
        let v = player_vehicle();
        wr::<u8>(v, 0x42E, rd::<u8>(v, 0x42E) | 0x80);
    }
    call_off_chase(gp);
}

/// Type 5, state 0 (0x44AEC0): the respray.
fn spray_closed(gp: GPtr<u8>) {
    if g::<u8>(NO_RESPRAYS).read() != 0 {
        return;
    }
    if now() <= rd::<u32>(gp, 0x3C) {
        call_off_chase(gp);
        return;
    }
    set_state(gp, 3);
    // L1e1: charge, L1dc: was wanted, bl: repainted
    let mut charge_it = false;
    let wanted = rd::<u32>(find_player_wanted(-1), 0x2C) != 0;
    if wanted {
        charge_it = true;
        clear_wanted_level_and_go_on_parole(find_player_wanted(-1));
    }
    let repainted;
    let v = player_vehicle();
    if !v.is_null() && (rd::<u32>(player_vehicle(), 0x590) == 0 || rd::<u32>(player_vehicle(), 0x590) == 9) {
        if !ge(ld(player_vehicle(), 0x4C0), c(0x4472_8000)) {
            charge_it = true;
        }
        let h = X87::from_f64_exact(max_f(1000.0, rd::<f32>(player_vehicle(), 0x4C0))).to_f32();
        wr::<f32>(player_vehicle(), 0x4C0, h);
        if rd::<u32>(player_vehicle(), 0x590) == 0 {
            wr::<u32>(player_vehicle(), 0x8E4, 0);
        } else {
            wr::<u32>(player_vehicle(), 0x7BC, 0);
        }
        v_fix(player_vehicle());
        increment_stat(0x8A, 1.0);
        if !ge(ld(rd::<GPtr<u8>>(player_vehicle(), 0x14), 0x28), X87::zero()) {
            for off in [0x20isize, 0x24, 0x28, 0, 4, 8] {
                let src = rd::<GPtr<u8>>(player_vehicle(), 0x14);
                let dst = rd::<GPtr<u8>>(player_vehicle(), 0x14);
                wr::<f32>(dst, off, ld(src, off).fchs().to_f32());
            }
        }
        let veh = player_vehicle();
        repainted = repaint(veh);
        wr::<u32>(veh, 0x4B0, 0);
        wr::<u8>(veh, 0x42E, rd::<u8>(veh, 0x42E) & 0x7F);
    } else {
        repainted = wanted;
    }
    if flags(gp) & 0x80 != 0 {
        message(0x85_9A34);
    } else if charge_it && g::<u8>(RESPRAYS_FREE).read() == 0 {
        charge(100);
        increment_stat(0x10, 100.0);
        message(if wanted { 0x85_9A2C } else { 0x85_9A24 });
    } else if repainted {
        let r = sa_core::orig::at_site(0x44_B36C, sa_core::crt::rand);
        message(if r & 1 != 0 { 0x85_9A1C } else { 0x85_9A14 });
    }
    set_flags(gp, flags(gp) | 4);
    if !player_vehicle().is_null() {
        let v = player_vehicle();
        wr::<u8>(v, 0x42F, rd::<u8>(v, 0x42F) | 1);
    }
    call_off_chase(gp);
}

/// 0x44B0D7..0x44B2BA: new colours for a vehicle without a fixed paint job or remap; true when
/// they differ (then ten paint particles in the new primary colour).
fn repaint(veh: GPtr<u8>) -> bool {
    if rd::<u8>(veh, 0x868) & 2 != 0 || get_remap_index(veh) >= 0 {
        return false;
    }
    let mut col = [0u8; 4];
    let p = GPtr::from_raw(col.as_mut_ptr());
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * (i32::from(rd::<i16>(veh, 0x22)) as usize)).read();
    choose_vehicle_colour(mi, p, p.byte_add(1), p.byte_add(2), p.byte_add(3), 1);
    let changed = (0..4).any(|k| rd::<u8>(veh, 0x434 + k as isize) != col[k]);
    for (k, v) in col.iter().enumerate() {
        wr::<u8>(veh, 0x434 + k as isize, *v);
    }
    set_remap(veh, -1);
    if !changed {
        return false;
    }
    let mut mult = [0u32; 8];
    let mp = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(
        mp,
        1.0,
        0.0,
        0.0,
        f32::from_bits(0x3F19_999A),
        f32::from_bits(0x3F33_3333),
        1.0,
        f32::from_bits(0x3ECC_CCCD),
    );
    let rgb = g::<u32>(0xB4_E480 + 4 * usize::from(rd::<u8>(veh, 0x434))).read();
    let k = f32::from_bits(0x3B80_8081);
    wr::<f32>(mp, 0, xi((rgb & 0xFF) as i32).fmul_m(k).to_f32());
    wr::<f32>(mp, 4, xi(((rgb >> 8) & 0xFF) as i32).fmul_m(k).to_f32());
    wr::<f32>(mp, 8, xi(((rgb >> 16) & 0xFF) as i32).fmul_m(k).to_f32());
    let mut vel = [0f32; 3];
    for _ in 0..10 {
        let p = pos_ptr(veh);
        let mut pos = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
        pos[0] = X87::from_f64_exact(random_float_in_range(-3.0, 3.0))
            .fadd_m(pos[0])
            .to_f32();
        pos[1] = X87::from_f64_exact(random_float_in_range(-3.0, 3.0))
            .fadd_m(pos[1])
            .to_f32();
        vel[2] = X87::from_f64_exact(random_float_in_range(0.0, f32::from_bits(0x3D4C_CCCD))).to_f32();
        fx_add_particle(
            g::<GPtr<u8>>(0xA9_AE20).read(),
            GPtr::from_raw(pos.as_mut_ptr()),
            GPtr::from_raw(vel.as_mut_ptr()),
            0.0,
            mp,
            -1.0,
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
    true
}

/// Types 2..4, state 1 (0x44B42D): the bomb fitting starts or is refused.
fn bomb_open(gp: GPtr<u8>) {
    if is_static_player_car_entirely_inside(gp) == 0 || player_vehicle().is_null() {
        return;
    }
    let st = rd::<u32>(player_vehicle(), 0x594);
    if st == 9 || st == 0xA {
        return;
    }
    if rd::<u8>(player_vehicle(), 0x4A8) & 7 != 0 {
        message(0x85_9A04);
        set_state(gp, 4);
        audio(0x11);
        return;
    }
    if g::<u8>(BOMBS_FREE).read() == 0 && money().read() < 500 {
        message(0x85_9A0C);
        set_state(gp, 4);
        audio(0xE);
        return;
    }
    set_state(gp, 2);
    pad_lock(true);
    wanted_hold(true);
}

/// Types 2..4, state 0 (0x44B5CA): the bomb is fitted when the timer opens the door.
fn bomb_closed(gp: GPtr<u8>, ty: u8) {
    if now() <= rd::<u32>(gp, 0x3C) {
        return;
    }
    if ty == 4 && g::<u8>(0x8E_6940).read() != 1 {
        request_model(0x16C, 2);
        return;
    }
    match ty {
        2 => audio(0x12),
        3 => audio(0x13),
        4 => audio(0x14),
        _ => {}
    }
    set_state(gp, 3);
    if g::<u8>(BOMBS_FREE).read() == 0 {
        charge(500);
    }
    let v = player_vehicle();
    if !v.is_null() && (rd::<u32>(player_vehicle(), 0x590) == 0 || rd::<u32>(player_vehicle(), 0x590) == 9) {
        let v = player_vehicle();
        let b = rd::<u8>(v, 0x4A8);
        wr::<u8>(v, 0x4A8, (b & !7) | (rd::<u8>(gp, 0x4C).wrapping_sub(1) & 7));
        let ped = find_player_ped(-1);
        wr::<GPtr<u8>>(player_vehicle(), 0x4CC, ped);
        if rd::<u8>(gp, 0x4C) == 4 {
            give_player_detonator();
        }
        increment_stat(0x7F, 10.0);
    }
    let pad_mode = || i32::from(rd::<u16>(get_pad(0), 0x10A));
    let key = match rd::<u8>(gp, 0x4C) {
        2 => match pad_mode() {
            0..=2 => 0x85_99DC,
            3 => 0x85_99E4,
            _ => return,
        },
        3 => match pad_mode() {
            0..=2 => 0x85_99EC,
            3 => 0x85_99F4,
            _ => return,
        },
        4 => 0x85_99FC,
        _ => return,
    };
    help(key);
}

/// State 0 of types 1 / 14 / 21 / 20 (0x44B9AB / 0x44BBFB): the target vehicle, driven by the
/// player, within `limit` (squared) of the box opens the door.
fn target_arrives(gp: GPtr<u8>, limit: u32) {
    let t = rd::<GPtr<u8>>(gp, 0x40);
    if player_vehicle() != t || t.is_null() {
        return;
    }
    if !ge(vehicle_box_dist(gp), c(limit)) {
        set_state(gp, 3);
    }
}

/// Type 1, state 1 (0x44B802): far away, the door closes on an untouched garage; near, it closes
/// once the target is parked inside and the player is 2 outside.
fn mission_open(gp: GPtr<u8>) {
    if !le(centre_dist_sq(gp), c(0x4461_0000)) {
        if g::<u32>(FRAME_COUNTER).read() & 0x1F != 0 {
            return;
        }
        let t = rd::<GPtr<u8>>(gp, 0x40);
        if !t.is_null() && is_entity_touching_3d(gp, t) != 0 {
            return;
        }
        set_flags(gp, flags(gp) | 1);
        set_state(gp, 2);
        return;
    }
    let t = rd::<GPtr<u8>>(gp, 0x40);
    if player_vehicle() == t || t.is_null() || is_entity_entirely_inside_3d(gp, t, 0.0) == 0 {
        return;
    }
    let who = if !player_vehicle().is_null() {
        player_vehicle()
    } else {
        find_player_ped(-1)
    };
    if is_entity_entirely_outside(gp, who, 2.0) == 0 {
        return;
    }
    pad_lock(true);
    wanted_hold(true);
    set_flags(gp, flags(gp) & 0xFE);
    set_state(gp, 2);
}

/// Types 14 / 21, state 1 (0x44BA70): far away the door closes; near, it closes on the target
/// parked by the player with nothing blocking.
fn collect_open(gp: GPtr<u8>) {
    if !le(centre_dist_sq(gp), c(0x4461_0000)) || rd::<u32>(gp, 0x40) == 0 {
        set_flags(gp, flags(gp) | 1);
        set_state(gp, 2);
        return;
    }
    if rd::<GPtr<u8>>(gp, 0x40) != player_vehicle() {
        return;
    }
    if is_static_player_car_entirely_inside(gp) == 0 || is_any_car_blocking(gp) != 0 {
        return;
    }
    pad_lock(true);
    wanted_hold(true);
    set_flags(gp, flags(gp) & 0xFE);
    set_state(gp, 2);
}

/// State 2 of types 1 (crusher: destroys the target) and 14 / 21 (2 s timer) (0x44B963 /
/// 0x44BB83).
fn mission_closing(gp: GPtr<u8>, collect: bool) {
    let t = rd::<GPtr<u8>>(gp, 0x40);
    if !t.is_null() {
        throw_cars_near_door_out(gp, t);
    }
    if slide_door_closed(gp) == 0 {
        return;
    }
    if flags(gp) & 1 != 0 {
        set_state(gp, 0);
        return;
    }
    let t = rd::<GPtr<u8>>(gp, 0x40);
    if !t.is_null() {
        set_state(gp, 5);
        if collect {
            wr::<u32>(gp, 0x3C, now().wrapping_add(2000));
        } else {
            destroy_vehicle_and_occupants(t);
        }
        wr::<u32>(gp, 0x40, 0);
    } else {
        set_state(gp, 0);
    }
    pad_lock(false);
    wanted_hold(false);
}

/// Hideouts, state 1 (0x44BCF2): closes behind the player.
fn hideout_open(gp: GPtr<u8>) {
    let d = player_box_dist(gp);
    let l1e0 = d.to_f32();
    let check_blocking = if !le(d, c(0x4361_0000)) {
        true
    } else if le(x(l1e0), c(0x4180_0000)) {
        false
    } else {
        let v = player_vehicle();
        v.is_null() || rd::<u32>(player_vehicle(), 0x594) == 0xA
    };
    if check_blocking && is_any_car_blocking(gp) == 0 {
        set_state(gp, 2);
        return;
    }
    if !player_vehicle().is_null() {
        let need = if rd::<u8>(gp, 0x4C) != 0x10 { 4 } else { 2 };
        if count_cars_inside(gp, player_vehicle()) >= need && is_player_entirely_outside(gp, 0.25) & 0xFF != 0
        {
            set_state(gp, 2);
            return;
        }
    }
    if le(x(l1e0), c(0x4599_2000)) {
        return;
    }
    set_state(gp, 2);
    remove_cars_blocking_door_not_inside(gp);
}

/// Hideouts, state 2 (0x44BDF1): once shut with the player outside, the cars are stored.
fn hideout_closing(gp: GPtr<u8>) {
    slide_door_closed(gp);
    if is_player_entirely_outside(gp, 0.0) & 0xFF == 0 {
        set_state(gp, 3);
        return;
    }
    if ld(gp, 0x38).fcom(X87::zero()).test_parity(0x44) {
        return;
    }
    set_state(gp, 0);
    let ty = rd::<u8>(gp, 0x4C);
    if ty == 0x2C {
        return;
    }
    store_and_remove_cars(gp, safe_house(ty), 4);
}

/// The squared 2D distance of a door object to the player vehicle (y term first).
fn door_dist(door: GPtr<u8>) -> X87 {
    let d1 = pos_ptr(door);
    let v1 = pos_ptr(player_vehicle());
    let d2 = pos_ptr(door);
    let v2 = pos_ptr(player_vehicle());
    let d3 = pos_ptr(door);
    let v3 = pos_ptr(player_vehicle());
    let yy = ld(d3, 4)
        .fsub_m(rd::<f32>(v3, 4))
        .fmul(ld(d2, 4).fsub_m(rd::<f32>(v2, 4)));
    let xx = ld(d2, 0)
        .fsub_m(rd::<f32>(v1, 0))
        .fmul(ld(d1, 0).fsub_m(rd::<f32>(v1, 0)));
    yy.fadd(xx)
}

/// Hideouts, state 0 (0x44BE56): opens for the arriving player unless full.
fn hideout_closed(gp: GPtr<u8>) {
    if ge(x(coors()[2]), c(0x446D_8000)) {
        return;
    }
    let d = player_box_dist(gp);
    // not below 12.25: also within 10 and not on a bike
    if d.fcom(c(0x4144_0000)).test_parity(5)
        && (ge(d, c(0x42C8_0000)) || player_vehicle().is_null() || rd::<u32>(player_vehicle(), 0x594) == 0xA)
    {
        return;
    }
    let ty = rd::<u8>(gp, 0x4C);
    if !player_vehicle().is_null() && ty != 0x2C {
        let need = if ty != 0x10 { 4 } else { 2 };
        if count_cars_in_hideout(ty) >= need {
            full_hideout_help(gp);
            return;
        }
    }
    let ty = rd::<u8>(gp, 0x4C);
    if ty != 0x2C && hideout_restore_cars(gp, safe_house(ty)) == 0 {
        return;
    }
    set_state(gp, 3);
}

/// 0x44BF37..0x44C1C0: a full hideout shows its help text when a door is within 5 of the
/// player's vehicle, at most every 18 s, unless the vehicle is a bike / boat appearance (3, 5).
fn full_hideout_help(gp: GPtr<u8>) {
    let mut a = 0u32;
    let mut b = 0u32;
    find_doors(gp, GPtr::from_raw(&mut a), GPtr::from_raw(&mut b));
    let near = |d: u32| d != 0 && !ge(door_dist(GPtr::from_addr(d as usize)), c(0x41C8_0000));
    if !near(a) && !near(b) {
        return;
    }
    if now().wrapping_sub(g::<u32>(LAST_HELP_TIME).read()) <= 18000 {
        return;
    }
    let ap = get_vehicle_appearance(player_vehicle());
    if ap == 3 || get_vehicle_appearance(player_vehicle()) == 5 {
        return;
    }
    help(0x85_99C0);
    g::<u32>(LAST_HELP_TIME).write(now());
}

/// Whether the player stands below the roof (top - 2) and above the base.
fn player_below_roof(gp: GPtr<u8>) -> bool {
    let z = coors()[2];
    if le(ld(gp, 0x1C).fsub_m(2.0), x(z)) {
        return false;
    }
    let z = coors()[2];
    !le(x(z), ld(gp, 8))
}

/// Types 33..35, states 1 / 2 (0x44C3AE): beyond 65 or outside the height range the cars are
/// stored (3 slots) and the door shuts.
fn small_hideout_open(gp: GPtr<u8>) {
    let d = player_box_dist(gp).to_f32();
    let inside = player_below_roof(gp);
    if le(x(d), c(0x4584_0800)) && inside && state(gp) != 2 {
        return;
    }
    set_state(gp, 0);
    hideout_store_cars(gp, safe_house(rd::<u8>(gp, 0x4C)), 3);
}

/// Types 33..35, state 0 (0x44C477): within 60 and below the roof the cars come back.
fn small_hideout_closed(gp: GPtr<u8>) {
    let d = player_box_dist(gp).to_f32();
    let inside = player_below_roof(gp);
    if ge(x(d), c(0x4561_0000)) || !inside {
        return;
    }
    hideout_prepare(gp, safe_house(rd::<u8>(gp, 0x4C)));
    if hideout_restore(gp, safe_house(rd::<u8>(gp, 0x4C))) != 0 {
        set_state(gp, 1);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_GARAGE_UPDATE];
