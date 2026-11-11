//! `CCarCtrl`, `CCarAI`, `CAutoPilot` (PORT-POP-2): the police mission / speed choices, the "tell car to"
//! helpers, node validity, steering helpers. Ported from the 1.0 US bytes; gta-reversed and
//! `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne_mask, jp5, rand_at, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, b: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90; its own rand, ST0)
    fn random_float_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// `CLoadedCarGroup::PickLeastUsedModel(int32)` (0x611E90, `ret 4`)
    fn loaded_car_group_pick_least_used_model(this: GPtr<u8>, n: i32) -> i32 = thiscall 0x611E90 => unported;
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854) is loaded.
fn area_loaded(area: u32) -> bool {
    g::<u32>((0x96F854 + area * 4) as usize).read() != 0
}

fn wanted_level() -> i32 {
    rd::<i32>(find_player_wanted(-1), 0x2C)
}

/// `CCarAI::FindSwitchDistanceClose(CVehicle*)` (0x41BF50): `(float)` autopilot +0x3DD.
#[hook(addr = 0x41BF50, conv = cdecl, name = "CCarAI::FindSwitchDistanceClose", expect = "8B 44 24 04 0F B6 88 DD 03 00 00")]
pub fn find_switch_distance_close(veh: GPtr<u8>) -> X87 {
    xi(i32::from(rd::<u8>(veh, 0x3DD)))
}

/// `CCarAI::FindSwitchDistanceFar(CVehicle*)` (0x41BF70): 50 for law enforcers (+0x428 bit 0), else +0x3DD + 5.
// Unhookable: 0x40248F holds 0x41BF79, inside the patched entry (sa-hook patch_refs test).
pub fn find_switch_distance_far(veh: GPtr<u8>) -> X87 {
    if rd::<u8>(veh, 0x428) & 1 != 0 {
        return x(50.0);
    }
    xi(i32::from(rd::<u8>(veh, 0x3DD))).fadd_m(5.0)
}

/// `CCarAI::FindPoliceCarSpeedForWantedLevel(CVehicle*)` (0x41CAA0): wanted 0 → random 12..16, 1 → 25,
/// 2 → 34, 3..6 → handling +0x88 (max velocity) × 54 / 72 / 75 / 78 (`_ftol`, EAX), else 0.
// Unhookable: protection code reads its entry bytes as data (0x14B1FBF, 0x14B351B, ...; sa-hook every_hook_installs_on_the_exe).
pub fn find_police_car_speed_for_wanted_level(veh: GPtr<u8>) -> i32 {
    let w = wanted_level() as u32;
    let k = match w {
        0 => return ftol32(X87::from_f64_exact(random_float_in_range(12.0, 16.0))),
        1 => return 25,
        2 => return 34,
        3 => 54.0,
        4 => 72.0,
        5 => 75.0,
        6 => 78.0,
        _ => return 0,
    };
    let h = rd::<GPtr<u8>>(veh, 0x384);
    ftol32(x(rd::<f32>(h, 0x88)).fmul_m(k))
}

/// `CCarAI::FindPoliceCarMissionForWantedLevel()` (0x41C9D0): wanted 0, 1 and > 6 → 4 (ram); 2, 3, 4..6 →
/// 4 or 2 (block) by `rand() & 3` < 3 / < 2 / < 1 (AL).
// Unhookable: 0x4049D5 holds 0x41C9D5, inside the patched entry (sa-hook patch_refs test).
pub fn find_police_car_mission_for_wanted_level() -> u8 {
    let (site, lim) = match wanted_level() as u32 {
        2 => (0x41C9EE, 3),
        3 => (0x41C9FF, 2),
        4..=6 => (0x41CA10, 1),
        _ => return 4,
    };
    let r = (rand_at(site) & 3) as u8;
    if r < lim { 4 } else { 2 }
}

/// `CCarAI::FindPoliceBoatMissionForWantedLevel()` (stub 0x41CA50 → 0x156FFB0): with wanted level 2..6, 0x15
/// in a vehicle else 0x40; otherwise 4 in a vehicle else 0x40 (`FindPlayerVehicle(-1, false)`; the bool
/// is pushed through an obfuscated `[0x15605D0] - [0x6D161D]` = 0 on the first path).
#[hook(addr = 0x41CA50, conv = cdecl, name = "CCarAI::FindPoliceBoatMissionForWantedLevel", expect = "E9 5B 35 15 01")]
pub fn find_police_boat_mission_for_wanted_level() -> u8 {
    let w = wanted_level();
    let in_veh = !find_player_vehicle(-1, 0).is_null();
    if (2..=6).contains(&w) {
        if in_veh { 0x15 } else { 0x40 }
    } else if in_veh {
        4
    } else {
        0x40
    }
}

/// `CCarAI::MellowOutChaseSpeedBoat(CVehicle*)` (0x41CB70): cruise speed by wanted level.
#[hook(addr = 0x41CB70, conv = cdecl, name = "CCarAI::MellowOutChaseSpeedBoat", expect = "6A FF E8 B9 16 15 00")]
pub fn mellow_out_chase_speed_boat(veh: GPtr<u8>) {
    const SPEED: [u8; 7] = [8, 10, 15, 20, 25, 30, 40];
    let w = wanted_level() as u32;
    if w <= 6 {
        wr::<u8>(veh, 0x3D0, SPEED[w as usize]);
    }
}

/// The shared body of `TellCarTo{Ram,Block,Follow}OtherCar`: target entity (+0x41C, referenced), join the
/// road system, mission, "avoid" bit 4 of +0x428 = !(+0x42D bit 1), cruise speed at least 6.
fn tell_car(veh: GPtr<u8>, target: GPtr<u8>, mission: u8) {
    let field = veh.byte_add(0x41C);
    field.cast::<GPtr<u8>>().write(target);
    entity_register_reference(target, field);
    join_car_with_road_system(veh);
    let b = rd::<u8>(veh, 0x428);
    wr::<u8>(veh, 0x3BA, mission);
    wr::<u8>(veh, 0x428, if rd::<u8>(veh, 0x42D) & 2 != 0 { b & 0xEF } else { b | 0x10 });
    let s = rd::<u8>(veh, 0x3D0);
    wr::<u8>(veh, 0x3D0, if s > 6 { s } else { 6 });
}

/// `CCarAI::TellCarToRamOtherCar(CVehicle*, CVehicle*)` (0x41C8A0): mission 0xF.
#[hook(addr = 0x41C8A0, conv = cdecl, name = "CCarAI::TellCarToRamOtherCar", expect = "8B 4C 24 08 56")]
pub fn tell_car_to_ram_other_car(veh: GPtr<u8>, target: GPtr<u8>) {
    tell_car(veh, target, 0xF);
}

/// `CCarAI::TellCarToBlockOtherCar(CVehicle*, CVehicle*)` (0x41C900): mission 0x11.
#[hook(addr = 0x41C900, conv = cdecl, name = "CCarAI::TellCarToBlockOtherCar", expect = "8B 4C 24 08 56")]
pub fn tell_car_to_block_other_car(veh: GPtr<u8>, target: GPtr<u8>) {
    tell_car(veh, target, 0x11);
}

/// `CCarAI::TellCarToFollowOtherCar(CVehicle*, CVehicle*, float radius)` (0x41C960): mission 0x34,
/// +0x3DE = `_ftol(radius)`.
// Unhookable: protection code reads and writes its entry bytes as data (0x14B4FBF, 0x14B5219, ...; sa-hook every_hook_installs_on_the_exe).
pub fn tell_car_to_follow_other_car(veh: GPtr<u8>, target: GPtr<u8>, radius: f32) {
    tell_car(veh, target, 0x34);
    wr::<u8>(veh, 0x3DE, ftol32(x(radius)) as u8);
}

/// `CCarCtrl::ChooseBoatModel()` (0x421970): `m_LoadedBoats.PickLeastUsedModel(1)`.
#[hook(addr = 0x421970, conv = cdecl, name = "CCarCtrl::ChooseBoatModel", expect = "6A 01 B9 98 E9 C0 00")]
pub fn choose_boat_model() -> i32 {
    loaded_car_group_pick_least_used_model(g(0xC0E998), 1)
}

/// `CCarCtrl::FindIntersection2Lines(x1, y1, dx1, dy1, x2, y2, dx2, dy2, float* ox, float* oy)` (0x4226F0):
/// `t = ((x2-x1)*dy2 - (y2-y1)*dx2) / (dx1*dy2 - dy1*dx2)` (0 when the determinant is 0),
/// `o = p1 + t*d1`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x4226F0, conv = cdecl, name = "CCarCtrl::FindIntersection2Lines", expect = "D9 44 24 0C D8 4C 24 20", preserve = "edx")]
pub fn find_intersection_2_lines(
    x1: f32,
    y1: f32,
    dx1: f32,
    dy1: f32,
    x2: f32,
    y2: f32,
    dx2: f32,
    dy2: f32,
    ox: GPtr<f32>,
    oy: GPtr<f32>,
) {
    let det = x(dx1).fmul_m(dy2).fsub(x(dy1).fmul_m(dx2));
    // `test ah, 0x44; jnp`: exactly one of C3/C2, i.e. det == 0 (ordered)
    let t = if !det.fcom(x(0.0)).test_parity(0x44) {
        x(0.0)
    } else {
        let a = x(x2).fsub_m(x1).fmul_m(dy2);
        let b = x(y2).fsub_m(y1).fmul_m(dx2);
        a.fsub(b).fdiv(det)
    };
    ox.write(x(dx1).fmul(t).fadd_m(x1).to_f32());
    oy.write(t.fmul_m(dy1).fadd_m(y1).to_f32());
}

/// `CCarCtrl::FindMaxSteerAngle(CVehicle*)` (0x427FE0): speed > 0.7 → 0.2, else `min(0.7, 0.9 - speed)` as
/// the x87 compares order it.
#[hook(addr = 0x427FE0, conv = cdecl, name = "CCarCtrl::FindMaxSteerAngle", expect = "8B 44 24 04 D9 40 4C")]
pub fn find_max_steer_angle(veh: GPtr<u8>) -> X87 {
    let vx = x(rd::<f32>(veh, 0x44));
    let vy = x(rd::<f32>(veh, 0x48));
    let vz = x(rd::<f32>(veh, 0x4C));
    let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt();
    let c = f32::from_bits(0x3F33_3333);
    if !jne_mask(s, x(c), 0x41) {
        return x(f32::from_bits(0x3E4C_CCCD));
    }
    let r = s.fsubr_m(f32::from_bits(0x3F66_6666));
    if jp5(x(c), r) { r } else { x(c) }
}

/// `CCarCtrl::StopCarIfNodesAreInvalid(CVehicle*)` (stub 0x422590 → 0x1567AD0): any of the autopilot's
/// link addresses (+0x3A4, +0x3A6) or node addresses (+0x390, +0x394) unset or in an unloaded area → set
/// +0x3DC bit 0 and return true.
#[hook(addr = 0x422590, conv = cdecl, name = "CCarCtrl::StopCarIfNodesAreInvalid", expect = "E9 3B 55 14 01")]
pub fn stop_car_if_nodes_are_invalid(veh: GPtr<u8>) -> u8 {
    let link_ok = |off: isize| {
        let a = rd::<u16>(veh, off);
        a != 0xFFFF && area_loaded(u32::from(a) >> 10)
    };
    let node_ok = |off: isize| {
        let a = rd::<u16>(veh, off);
        a != 0xFFFF && area_loaded(u32::from(a))
    };
    if link_ok(0x3A4) && link_ok(0x3A6) && node_ok(0x390) && node_ok(0x394) {
        return 0;
    }
    wr::<u8>(veh, 0x3DC, rd::<u8>(veh, 0x3DC) | 1);
    1
}

/// `CAutoPilot::RemoveOnePathNode()` (stub 0x41B950 → 0x1562000): shift the node list (+0x68) down by one.
#[hook(addr = 0x41B950, conv = thiscall, name = "CAutoPilot::RemoveOnePathNode", expect = "E9 AB 66 14 01")]
pub fn auto_pilot_remove_one_path_node(this: GPtr<u8>) {
    let n = rd::<i16>(this, 0x88).wrapping_sub(1);
    wr::<i16>(this, 0x88, n);
    let mut i: i16 = 0;
    if n <= 0 {
        return;
    }
    loop {
        let v = rd::<u32>(this, 0x6C + 4 * isize::from(i));
        wr::<u32>(this, 0x68 + 4 * isize::from(i), v);
        i = i.wrapping_add(1);
        if i >= rd::<i16>(this, 0x88) {
            break;
        }
    }
}

/// `thunk_FUN_01563560` (stub 0x421120 → 0x1563560): `CAutoPilot` speed step +0x50 (int8) + 1, at most 120.
#[hook(addr = 0x421120, conv = thiscall, name = "thunk_FUN_01563560", expect = "E9 3B 24 14 01")]
pub fn auto_pilot_speed_step(this: GPtr<u8>) {
    let v = i32::from(rd::<i8>(this, 0x50)) + 1;
    wr::<u8>(this, 0x50, if v >= 120 { 120 } else { v as u8 });
}

orig! {
    /// `CColStore::HasCollisionLoaded(const CVector&, int32 area)` (0x410CE0)
    fn has_collision_loaded(pos: GPtr<f32>, area: i32) -> u8 = cdecl 0x410CE0 => unported;
    /// `CDamageManager::SetWheelStatus(int32, eCarWheelStatus)` (0x6C21A0, `ret 8`)
    fn damage_set_wheel_status(this: GPtr<u8>, wheel: i32, status: i32) = thiscall 0x6C21A0 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(this: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `CBike::PlaceOnRoadProperly()` (0x6BEEB0)
    fn bike_place_on_road_properly(this: GPtr<u8>) = thiscall 0x6BEEB0 => unported;
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    fn find_player_centre_of_world(id: i32) -> GPtr<f32> = cdecl 0x56E250 => unported;
    /// `CCarCtrl::PossiblyRemoveVehicle(CVehicle*)` (0x424F80)
    fn possibly_remove_vehicle(v: GPtr<u8>) = cdecl 0x424F80 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CRoadBlocks::GenerateRoadBlockPedsForCar(CVehicle*, int32, ePedType)` (0x461170)
    fn generate_road_block_peds_for_car(v: GPtr<u8>, n: i32, pt: i32) = cdecl 0x461170 => unported;
    /// `CGangWars::DontCreateCivilians()` (0x4439C0)
    fn dont_create_civilians() -> u8 = cdecl 0x4439C0 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CCarCtrl::GenerateOneRandomCar()` (0x430050)
    fn generate_one_random_car() = cdecl 0x430050 => unported;
    /// `CTrain::DoTrainGenerationAndRemoval()` (0x6F7900)
    fn do_train_generation_and_removal() = cdecl 0x6F7900 => unported;
    /// `CPlane::DoPlaneGenerationAndRemoval()` (0x6CD2F0)
    fn do_plane_generation_and_removal() = cdecl 0x6CD2F0 => unported;
    /// `CCarCtrl::GenerateEmergencyServicesCar()` (0x42F9C0)
    fn generate_emergency_services_car_o() = cdecl 0x42F9C0 => unported;
    /// `CWanted::AreSwatRequired()` (0x561F40)
    fn are_swat_required(w: GPtr<u8>) -> u8 = thiscall 0x561F40 => unported;
    /// `CWanted::AreFbiRequired()` (0x561F60)
    fn are_fbi_required(w: GPtr<u8>) -> u8 = thiscall 0x561F60 => unported;
    /// `CWanted::AreArmyRequired()` (0x561F80)
    fn are_army_required(w: GPtr<u8>) -> u8 = thiscall 0x561F80 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180; its own rand)
    fn random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CStreaming::GetDefaultCopCarModel(int32)` (0x407C50)
    fn get_default_cop_car_model(ignore_lvpd: u32) -> i32 = cdecl 0x407C50 => unported;
}

/// The entity position: the matrix position or the placement's.
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

/// `CCarCtrl::FindSpeedMultiplier(float angle, float minAngle, float maxAngle, float coef)` (0x4224E0): the
/// angle wrapped into [-π, π] by repeated ±2π, `v = max(|a| - min, 0)`; `v <= max - min` →
/// `1 - v / (max - min) * (1 - coef)` (stored as f32 first), else `coef`.
#[hook(addr = 0x4224E0, conv = cdecl, name = "CCarCtrl::FindSpeedMultiplier", expect = "D9 44 24 04 D8 15 C0 8C 85 00")]
pub fn find_speed_multiplier(angle: f32, min: f32, max: f32, coef: f32) -> X87 {
    let npi = f32::from_bits(0xC049_0FDB);
    let twopi = f32::from_bits(0x40C9_0FDB);
    let pi = f32::from_bits(0x4049_0FDB);
    let mut a = x(angle);
    if !jp5(a, x(npi)) {
        loop {
            a = a.fadd_m(twopi);
            if jp5(a, x(npi)) {
                break;
            }
        }
    }
    if !jne_mask(a, x(pi), 0x41) {
        loop {
            a = a.fsub_m(twopi);
            if jne_mask(a, x(pi), 0x41) {
                break;
            }
        }
    }
    if !jp5(a, x(0.0)) {
        a = a.fchs();
    }
    let mut v = a.fsub_m(min);
    if !jne_mask(x(0.0), v, 0x41) {
        v = x(0.0);
    }
    let d = x(max).fsub_m(min);
    let r = v.fdiv(d).fmul(x(1.0).fsub_m(coef)).fsubr_m(1.0).to_f32();
    if jne_mask(v, d, 0x41) { x(r) } else { x(coef) }
}

/// `CCarCtrl::SwitchBetweenPhysicsAndGhost(CVehicle*)` (0x4222A0): mission vehicles (+0x40 bit 14,
/// created by 2) that are not boats/trains/heli-like subclasses (+0x594 3..5) switch physics (status 3) →
/// ghost (12) where collision is not loaded (automobiles also get the 4 wheels status 0), and back where it
/// is (re-placed on the road: automobile / bike).
#[hook(addr = 0x4222A0, conv = cdecl, name = "CCarCtrl::SwitchBetweenPhysicsAndGhost", expect = "56 8B 74 24 08")]
pub fn switch_between_physics_and_ghost(veh: GPtr<u8>) {
    if rd::<u32>(veh, 0x40) & 0x4000 == 0 || rd::<u8>(veh, 0x4A4) != 2 {
        return;
    }
    let sub = rd::<i32>(veh, 0x594);
    if sub == 5 || sub == 4 || sub == 3 {
        return;
    }
    let st = rd::<u8>(veh, 0x36) >> 3;
    if st == 3 {
        if has_collision_loaded(entity_pos(veh), 0) != 0 {
            return;
        }
        wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x60);
        if rd::<i32>(veh, 0x594) != 0 {
            return;
        }
        let dm = veh.byte_add(0x5A0);
        for i in 0..=3 {
            damage_set_wheel_status(dm, i, 0);
        }
    } else if st == 12 {
        if has_collision_loaded(entity_pos(veh), 0) == 0 {
            return;
        }
        wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
        match rd::<i32>(veh, 0x590) {
            0 => automobile_place_on_road_properly(veh),
            9 => bike_place_on_road_properly(veh),
            _ => {}
        }
    }
}

/// `CCarCtrl::RemoveDistantCars()` (0x42CD10): every vehicle (pool walked downwards) through
/// `PossiblyRemoveVehicle`; road-block vehicles (+0x42A bit 4) within 54.5 (2D, `sqrt(dy² + dx²)`) of the
/// centre of the world get their road block peds (cops 6, else 7) and lose the bit.
#[hook(addr = 0x42CD10, conv = cdecl, name = "CCarCtrl::RemoveDistantCars", expect = "83 EC 10 55 57")]
pub fn remove_distant_cars() {
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
        possibly_remove_vehicle(veh);
        if rd::<u8>(veh, 0x42A) & 0x10 == 0 {
            continue;
        }
        let c = find_player_centre_of_world(i32::from(g::<u8>(0xB7CD74).read()));
        let (cx, cy) = (c.read(), c.wrapping_index(1).read());
        let p = entity_pos(veh);
        let dy = x(p.wrapping_index(1).read()).fsub_m(cy);
        let dx = x(p.read()).fsub_m(cx);
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if jp5(d, x(54.5)) {
            continue;
        }
        let pt = if is_law_enforcement_vehicle(veh) != 0 { 6 } else { 7 };
        generate_road_block_peds_for_car(veh, i32::from(rd::<u8>(veh, 0x4AD)), pt);
        wr::<u8>(veh, 0x42A, rd::<u8>(veh, 0x42A) & 0xEF);
    }
}

/// `CCarCtrl::IsAnyoneParking()` (stub 0x42C250 → 0x15636D0): any vehicle on a parking mission
/// (0x2D, 0x2E, 0x31, 0x32).
#[hook(addr = 0x42C250, conv = cdecl, name = "CCarCtrl::IsAnyoneParking", expect = "E9 7B 74 13 01")]
pub fn is_anyone_parking() -> u8 {
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
        if matches!(rd::<u8>(veh, 0x3BA), 0x2D | 0x2E | 0x31 | 0x32) {
            return 1;
        }
    }
    0
}

/// `CCarCtrl::GenerateRandomCars()` (0x4341C0).
#[hook(addr = 0x4341C0, conv = cdecl, name = "CCarCtrl::GenerateRandomCars", expect = "A0 51 F8 B5 00")]
pub fn generate_random_cars() {
    if g::<u8>(0xB5F851).read() != 0 {
        g::<u8>(0x9690C0).write(2);
        return;
    }
    if dont_create_civilians() != 0 || g::<u32>(0xB72914).read() != 0 {
        return;
    }
    let mad = g::<f32>(0x9690BC);
    if la_riots_active_here() != 0 && !jne_mask(x(mad.read()), x(480.0), 0x41) {
        mad.write(random_float_in_range(240.0, 480.0) as f32);
    }
    let ts = g::<f32>(0xB7CB5C).read();
    mad.write(x(ts).fmul_m(f32::from_bits(0x3CA3_D70A)).fsubr_m(mad.read()).to_f32());
    if g::<i32>(0x969094).read() < 0x2D {
        let c = g::<u8>(0x9690C0);
        let n = c.read();
        if n != 0 {
            c.write(n - 1);
            if n - 1 == 0 {
                for _ in 0..100 {
                    generate_one_random_car();
                }
                g::<u8>(0xC279D0).write(20);
            }
        } else {
            generate_one_random_car();
            generate_one_random_car();
        }
    }
    do_train_generation_and_removal();
    do_plane_generation_and_removal();
    if (g::<u32>(0xB7CB78).read() ^ g::<u32>(0xB7CB84).read()) & 0xFFFF_F000 != 0 {
        generate_emergency_services_car_o();
    }
}

/// `CCarCtrl::ChoosePoliceCarModel(uint32 ignoreLvpd)` (0x421980): SWAT (enforcer 427 with a 1-in-4 draw),
/// FBI (490), army (432/433 by `rand() < 0x3FFF`) when required and loaded, else the default cop car.
#[hook(addr = 0x421980, conv = cdecl, name = "CCarCtrl::ChoosePoliceCarModel", expect = "53 6A FF E8 A8 C8 14 00")]
pub fn choose_police_car_model(ignore_lvpd: u32) -> i32 {
    let loaded = |a: usize| g::<u8>(a).read() == 1;
    if are_swat_required(find_player_wanted(-1)) != 0 && loaded(0x8E6E2C) && loaded(0x8E6314) {
        if random_int_in_range(0, 3) == 2 {
            return 0x1AB;
        }
        return get_default_cop_car_model(ignore_lvpd);
    }
    if are_fbi_required(find_player_wanted(-1)) != 0 && loaded(0x8E7318) && loaded(0x8E6328) {
        return 0x1EA;
    }
    if are_army_required(find_player_wanted(-1)) != 0 && loaded(0x8E6E90) && loaded(0x8E6EA4) && loaded(0x8E633C) {
        return 0x1B0 + i32::from(rand_at(0x421A1E) < 0x3FFF);
    }
    get_default_cop_car_model(ignore_lvpd)
}

orig! {
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CCranes::IsThisCarBeingTargettedByAnyCrane(CVehicle*)` (0x6F4000)
    fn is_this_car_being_targetted_by_any_crane(v: GPtr<u8>) -> u8 = cdecl 0x6F4000 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// vtable slot 0 (scalar deleting destructor, `ret 4`)
    fn delete_entity(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CVehicle::SetUpDriver(int32, bool, bool)` (0x6D1A50, `ret 0xC`)
    fn vehicle_set_up_driver(v: GPtr<u8>, pt: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x6D1A50 => unported;
    /// `CVehicle::SetupPassenger(int32, int32, bool, bool)` (0x6D1AA0, `ret 0x10`)
    fn vehicle_setup_passenger(v: GPtr<u8>, i: i32, pt: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x6D1AA0 => unported;
    /// `CPed::GiveObjectToPedToHold(int32, uint8)` (0x5E4390, `ret 8`)
    fn ped_give_object_to_hold(p: GPtr<u8>, mi: i32, b: u32) = thiscall 0x5E4390 => unported;
    /// `CPed::UpdateStatEnteringVehicle()` (0x5E01A0)
    fn ped_update_stat_entering_vehicle(p: GPtr<u8>) = thiscall 0x5E01A0 => unported;
    /// `CModelInfo::IsCarModel(int32)` (0x4C5AA0)
    fn is_car_model(mi: i32) -> u8 = cdecl 0x4C5AA0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn get_animation_block_index(name: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CBaseModelInfo` vtable +0x38 (`GetAnimFileIndex`)
    fn model_info_anim_file_index(mi: GPtr<u8>) -> i32 = thiscall vslot 0x38 => unported;
}

/// `CCarCtrl::FindPercDependingOnDistToLink(CVehicle*, CCarPathLinkAddress)` (0x422620): 2D distance to
/// the navi node (`(dy² + dx²)`, compressed coordinates × 0.125): < 5 → 0.5, < 15 (the distance stored as
/// f32) → `(d - 5) * 0.05 + 0.5`, else 1.
#[hook(addr = 0x422620, conv = cdecl, name = "CCarCtrl::FindPercDependingOnDistToLink", expect = "0F B7 44 24 08")]
pub fn find_perc_depending_on_dist_to_link(veh: GPtr<u8>, link: u16) -> X87 {
    let a = u32::from(link);
    let navi = g::<GPtr<u8>>((0x96F974 + (a >> 10) * 4) as usize).read().byte_add(((a & 0x3FF) * 14) as isize);
    let ny = xi(i32::from(rd::<i16>(navi, 2))).fmul_m(0.125);
    let nx = xi(i32::from(rd::<i16>(navi, 0))).fmul_m(0.125);
    let p = entity_pos(veh);
    let dy = ny.fsub_m(p.wrapping_index(1).read());
    let dx = nx.fsub_m(p.read());
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    if !jp5(d, x(5.0)) {
        return x(0.5);
    }
    let df = d.to_f32();
    if jp5(x(df), x(15.0)) {
        return x(1.0);
    }
    x(df).fsub_m(5.0).fmul_m(f32::from_bits(0x3D4C_CCCD)).fadd_m(0.5)
}

/// `CCarCtrl::RemoveCarsIfThePoolGetsFull()` (0x4322B0): on frames with `(counter & 7) == 3`, when fewer than
/// 8 vehicle slots are free, deletes the deletable vehicle (not of interest, not +0x428 bit 3, not on a crane)
/// nearest to the camera (3D, `(dz² + dx²) + dy²`; pool walked
/// downwards, strict `<`, start 1e7).
#[hook(addr = 0x4322B0, conv = cdecl, name = "CCarCtrl::RemoveCarsIfThePoolGetsFull", expect = "A1 4C CB B7 00")]
pub fn remove_cars_if_the_pool_gets_full() {
    if g::<u32>(0xB7CB4C).read() & 7 != 3 {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let size = rd::<i32>(pool, 8);
    let mut used = 0;
    for i in 0..size {
        if rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() & 0x80 == 0 {
            used += 1;
        }
    }
    if size - used >= 8 {
        return;
    }
    let mut best = 10_000_000.0f32;
    let mut res = GPtr::<u8>::NULL;
    let mut i = size;
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
        if (0..2).any(|k| g::<GPtr<u8>>(0x969084 + 4 * k).read() == veh) {
            continue;
        }
        if rd::<u8>(veh, 0x428) & 8 != 0 || vehicle_can_be_deleted(veh) == 0 {
            continue;
        }
        if is_this_car_being_targetted_by_any_crane(veh) != 0 {
            continue;
        }
        let p = entity_pos(veh);
        let cm = g::<GPtr<u8>>(0xB6F03C).read();
        let c: GPtr<f32> = if cm.is_null() { g(0xB6F02C) } else { cm.byte_add(0x30).cast() };
        let dx = x(c.read()).fsub_m(p.read());
        let dy = x(c.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read());
        let dz = x(c.wrapping_index(2).read()).fsub_m(p.wrapping_index(2).read());
        let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt();
        if !jp5(d, x(best)) {
            best = d.to_f32();
            res = veh;
        }
    }
    if !res.is_null() {
        world_remove(res);
        delete_entity(res, 1);
    }
}

/// `CCarCtrl::SetUpDriverAndPassengersForVehicle(CVehicle*, int32 pedType, int32 minPassengers, bool, bool,
/// int32 maxPassengers)` (0x4217C0): the driver, then `min(+0x488, max)` passengers where each one above
/// the minimum is added with `rand() * (1/32767.5) < 0.125`; one at most for car models whose anim file is
/// the 0x858EE0 block; ped types 14..23 hold object `[0x8CD72C]` half of the time.
#[hook(addr = 0x4217C0, conv = cdecl, name = "CCarCtrl::SetUpDriverAndPassengersForVehicle", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn set_up_driver_and_passengers_for_vehicle(veh: GPtr<u8>, pt: i32, min: i32, a: u32, b: u32, max: i32) {
    vehicle_set_up_driver(veh, pt, a, b);
    let gang_like = (14..=23).contains(&pt);
    if gang_like && rand_at(0x4217ED) < 0x3FFF {
        ped_give_object_to_hold(rd::<GPtr<u8>>(veh, 0x460), i32::from(g::<u16>(0x8CD72C).read()), 1);
    }
    let cap = i32::from(rd::<u8>(veh, 0x488));
    let lim = if cap < max { cap } else { max };
    let mut n = min;
    if min < lim {
        for _ in 0..(lim - min) {
            let r = rand_at(0x42182D);
            if !jp5(xi(r).fmul_m(f32::from_bits(0x3800_0100)), x(0.125)) {
                n += 1;
            }
        }
        if n >= lim {
            n = lim;
        }
    } else {
        n = lim;
    }
    let model = i32::from(rd::<i16>(veh, 0x22));
    if is_car_model(model) != 0 {
        let blk = get_animation_block_index(0x858EE0);
        if model_info_anim_file_index(crate::common::model_info(model)) == blk && n >= 1 {
            n = 1;
        }
    }
    for i in 0..n {
        let p = vehicle_setup_passenger(veh, i, pt, a, b);
        if p.is_null() {
            continue;
        }
        ped_update_stat_entering_vehicle(p);
        if gang_like && rand_at(0x4218D1) < 0x3FFF {
            ped_give_object_to_hold(p, i32::from(g::<u16>(0x8CD72C).read()), 1);
        }
    }
}

orig! {
    /// `CTrafficLights::ShouldCarStopForLight(CVehicle*, bool)` (0x49D610)
    fn should_car_stop_for_light(v: GPtr<u8>, b: u32) -> u8 = cdecl 0x49D610 => unported;
    /// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420)
    fn should_car_stop_for_bridge(v: GPtr<u8>) -> u8 = cdecl 0x49D420 => unported;
    /// `CCarAI::CarHasReasonToStop(CVehicle*)` (0x41C050)
    fn car_has_reason_to_stop(v: GPtr<u8>) = cdecl 0x41C050 => unported;
    /// `CCarCtrl::FindMaximumSpeedForThisCarInTraffic(CVehicle*)` (0x434400, ST0)
    fn find_maximum_speed_for_this_car_in_traffic(v: GPtr<u8>) -> f64 = cdecl 0x434400 => unported;
    /// `CAutoPilot::ModifySpeed(float)` (0x41B980, `ret 4`)
    fn auto_pilot_modify_speed(ap: GPtr<u8>, s: f32) = thiscall 0x41B980 => unported;
    /// `CGangWars::GangWarFightingGoingOn()` (0x443AC0)
    fn gang_war_fighting_going_on() -> u8 = cdecl 0x443AC0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CAccidentManager::GetInstance()` (0x56CF20)
    fn accident_manager_get_instance() -> GPtr<u8> = cdecl 0x56CF20 => unported;
    /// `CAccidentManager::GetNumberOfFreeAccidents()` (0x56CEE0)
    fn accident_get_number_of_free_accidents(m: GPtr<u8>) -> i32 = thiscall 0x56CEE0 => unported;
    /// `CAccidentManager::GetNearestFreeAccident(CVector&, bool)` (0x56D050, `ret 8`)
    fn accident_get_nearest_free_accident(m: GPtr<u8>, pos: GPtr<f32>, b: u32) -> GPtr<u8> = thiscall 0x56D050 => unported;
    /// `CStreaming::StreamAmbulanceAndMedic(bool)` (0x40A2A0)
    fn stream_ambulance_and_medic(b: u32) -> u8 = cdecl 0x40A2A0 => unported;
    /// `CStreaming::StreamFireEngineAndFireman(bool)` (0x40A400)
    fn stream_fire_engine_and_fireman(b: u32) -> u8 = cdecl 0x40A400 => unported;
    /// `thunk_FUN_015674b0` (0x407D30): the level's ambulance model (`[0x8A5AB4 + 4 * level]`)
    fn ambulance_model() -> i32 = cdecl 0x407D30 => unported;
    /// `thunk_FUN_0156ce80` (0x407DC0): the level's fire engine model
    fn fire_engine_model() -> i32 = cdecl 0x407DC0 => unported;
    /// `CCarCtrl::GenerateOneEmergencyServicesCar(uint32 model, CVector)` (0x42B7D0; the vector by value)
    fn generate_one_emergency_services_car(model: i32, x: f32, y: f32, z: f32) -> u32 = cdecl 0x42B7D0 => unported;
    /// `CFireManager::GetNumOfNonScriptFires()` (0x538F10)
    fn fire_get_num_of_non_script_fires(fm: GPtr<u8>) -> u32 = thiscall 0x538F10 => unported;
    /// `CFireManager::FindNearestFire(const CVector&, bool, bool)` (0x538F40; callee-cleaned, 3 arguments)
    fn fire_find_nearest_fire(fm: GPtr<u8>, pos: GPtr<f32>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x538F40 => unported;
}

/// `CCarCtrl::SlowCarOnRailsDownForTrafficAndLights(CVehicle*)` (0x434790): every 4th frame (by +0x20 +
/// frame counter) the target speed (+0x3CC) is 0 when the car must stop for a light or a bridge, else the
/// traffic speed; the autopilot speed (+0x3C8) then moves towards it by `0.7 * timestep` down (below 0.1:
/// straight to 0) or `0.05 * timestep` up (`CAutoPilot::ModifySpeed`).
#[hook(addr = 0x434790, conv = cdecl, name = "CCarCtrl::SlowCarOnRailsDownForTrafficAndLights", expect = "8B 15 4C CB B7 00")]
pub fn slow_car_on_rails_down_for_traffic_and_lights(veh: GPtr<u8>) {
    let fc = g::<u32>(0xB7CB4C).read();
    if u32::from(rd::<u8>(veh, 0x20)).wrapping_add(fc) & 3 == 0 {
        if should_car_stop_for_light(veh, 0) != 0 || should_car_stop_for_bridge(veh) != 0 {
            car_has_reason_to_stop(veh);
            wr::<f32>(veh, 0x3CC, 0.0);
        } else {
            let s = find_maximum_speed_for_this_car_in_traffic(veh);
            wr::<f32>(veh, 0x3CC, s as f32);
        }
    }
    let target = rd::<f32>(veh, 0x3CC);
    let ap = veh.byte_add(0x390);
    let cur = || x(rd::<f32>(veh, 0x3C8));
    let is_zero = |v: X87| !v.fcom(x(0.0)).test_parity(0x44);
    let ts = || x(g::<f32>(0xB7CB5C).read());
    if !jp5(x(target), cur()) {
        if is_zero(cur()) {
            return;
        }
        if !jp5(cur(), x(f32::from_bits(0x3DCC_CCCD))) {
            if is_zero(cur()) {
                return;
            }
            auto_pilot_modify_speed(ap, 0.0);
            return;
        }
        let v = ts().fmul_m(f32::from_bits(0x3F33_3333)).fsubr_m(rd::<f32>(veh, 0x3C8));
        if jne_mask(v, x(target), 0x41) {
            auto_pilot_modify_speed(ap, target);
        } else {
            auto_pilot_modify_speed(ap, v.to_f32());
        }
    } else {
        if jne_mask(x(target), cur(), 0x41) {
            return;
        }
        let v = ts().fmul_m(f32::from_bits(0x3D4C_CCCD)).fadd_m(rd::<f32>(veh, 0x3C8));
        if jp5(v, x(target)) {
            auto_pilot_modify_speed(ap, target);
        } else {
            auto_pilot_modify_speed(ap, v.to_f32());
        }
    }
}

/// `CCarCtrl::GenerateEmergencyServicesCar()` (0x42F9C0): when allowed, no gang war fight, wanted level <= 3,
/// outside interiors, a level set (0xBA6718) and the car counts below `MaxNumberOfCarsInUse`: an ambulance
/// to the nearest free accident (2+ free accidents, the medic streamed, 30 s since the last), then a fire
/// engine to the nearest fire (3+ fires, the fireman streamed, 35 s since the last); otherwise the models
/// are released (`Stream*(false)`).
#[hook(addr = 0x42F9C0, conv = cdecl, name = "CCarCtrl::GenerateEmergencyServicesCar", expect = "A0 28 5B 8A 00")]
pub fn generate_emergency_services_car() {
    if g::<u8>(0x8A5B28).read() == 0 || gang_war_fighting_going_on() != 0 {
        return;
    }
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<i32>(wanted, 0x2C) > 3 {
        return;
    }
    if g::<u32>(0xB72914).read() != 0 || g::<u32>(0xBA6718).read() == 0 {
        return;
    }
    let c = |a: usize| g::<i32>(a).read();
    let amb = c(0x9690A8);
    let sum = c(0x9690AC)
        .wrapping_add(amb)
        .wrapping_add(c(0x9690A0))
        .wrapping_add(c(0x96909C))
        .wrapping_add(c(0x969098))
        .wrapping_add(c(0x969094));
    if sum > c(0x8A5B24) {
        return;
    }
    let mut v = [0f32; 3];
    if amb == 0 {
        if accident_get_number_of_free_accidents(accident_manager_get_instance()) < 2 {
            stream_ambulance_and_medic(0);
        } else {
            let pos = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
            let acc = accident_get_nearest_free_accident(accident_manager_get_instance(), pos, 0);
            if !acc.is_null()
                && stream_ambulance_and_medic(1) != 0
                && g::<u32>(0xB7CB84).read() > g::<u32>(0x9690B0).read().wrapping_add(30_000)
            {
                let p = entity_pos(rd::<GPtr<u8>>(acc, 0));
                let (px, py, pz) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
                let m = ambulance_model();
                if generate_one_emergency_services_car(m, px, py, pz) != 0 {
                    g::<u32>(0x9690B0).write(g::<u32>(0xB7CB84).read());
                }
            }
        }
    }
    if c(0x9690AC) != 0 {
        return;
    }
    if (fire_get_num_of_non_script_fires(g(0xB71F80)) as u16) < 3 {
        stream_fire_engine_and_fireman(0);
        return;
    }
    let pos = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    let fire = fire_find_nearest_fire(g(0xB71F80), pos, 1, 1);
    if fire.is_null() || stream_fire_engine_and_fireman(1) == 0 {
        return;
    }
    if g::<u32>(0xB7CB84).read() <= g::<u32>(0x9690B4).read().wrapping_add(35_000) {
        return;
    }
    let p = fire.byte_add(4).cast::<f32>();
    let (px, py, pz) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
    let m = fire_engine_model();
    if generate_one_emergency_services_car(m, px, py, pz) != 0 {
        g::<u32>(0x9690B4).write(g::<u32>(0xB7CB84).read());
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SLOW_CAR_ON_RAILS_DOWN_FOR_TRAFFIC_AND_LIGHTS,
    HOOK_GENERATE_EMERGENCY_SERVICES_CAR,
    HOOK_FIND_PERC_DEPENDING_ON_DIST_TO_LINK,
    HOOK_REMOVE_CARS_IF_THE_POOL_GETS_FULL,
    HOOK_SET_UP_DRIVER_AND_PASSENGERS_FOR_VEHICLE,
    HOOK_FIND_SPEED_MULTIPLIER,
    HOOK_SWITCH_BETWEEN_PHYSICS_AND_GHOST,
    HOOK_REMOVE_DISTANT_CARS,
    HOOK_IS_ANYONE_PARKING,
    HOOK_GENERATE_RANDOM_CARS,
    HOOK_CHOOSE_POLICE_CAR_MODEL,
    HOOK_FIND_SWITCH_DISTANCE_CLOSE,
    HOOK_FIND_POLICE_BOAT_MISSION_FOR_WANTED_LEVEL,
    HOOK_MELLOW_OUT_CHASE_SPEED_BOAT,
    HOOK_TELL_CAR_TO_RAM_OTHER_CAR,
    HOOK_TELL_CAR_TO_BLOCK_OTHER_CAR,
    HOOK_CHOOSE_BOAT_MODEL,
    HOOK_FIND_INTERSECTION_2_LINES,
    HOOK_FIND_MAX_STEER_ANGLE,
    HOOK_STOP_CAR_IF_NODES_ARE_INVALID,
    HOOK_AUTO_PILOT_REMOVE_ONE_PATH_NODE,
    HOOK_AUTO_PILOT_SPEED_STEP,
];
