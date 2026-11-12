//! PORT-VEH-3 (batch `veh3-1`): train station lookups and skipping, the train list helpers
//! (nearest train, mission/all train removal), the tunnel test, `CTrain::Teleport` and
//! `CTrain::Shutdown`. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_world::world::common::find_player_vehicle;

use crate::common::{b, ftol32, g, rd, wr, x, xi};

orig! {
    /// `operator delete` (0x8213AE)
    fn operator_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `CTrain::FindNextStationPositionInDirection(bool, float, float*, int32*)` (0x6F5F00)
    fn call_find_next_station(clockwise: u32, pos: f32, dist: GPtr<f32>, idx: GPtr<i32>) = cdecl 0x6F5F00 => find_next_station_position_in_direction;
    /// `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300)
    fn get_level_from_position(p: GPtr<u8>) -> i32 = cdecl 0x572300 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40), ST0
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn load_scene(p: GPtr<u8>) = cdecl 0x40EB70 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
    /// `CGameLogic::PassTime(uint32)` (0x4414C0)
    fn game_logic_pass_time(minutes: i32) = cdecl 0x4414C0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// virtual slot 0: the scalar deleting destructor
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    fn set_orientation(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b0: u32, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

/// `CTrain::aStationCoors` (0x8D48F8): 6 `CVector`s.
const STATION_COORS: usize = 0x8D_48F8;
/// The 6 station positions along the track (0xC38034, floats).
const STATION_DIST: usize = 0xC3_8034;
/// `CPools::ms_pVehiclePool` (0xB74494).
const VEHICLE_POOL: usize = 0xB7_4494;

fn station_dist(k: i32) -> f32 {
    g::<f32>(STATION_DIST + 4 * k as usize).read()
}

/// The entity's position (`m_matrix` row 3, or the placeable's `+4` without a matrix).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// Every vehicle of the pool, from the last slot down (the original loops), as `f(vehicle)`.
fn for_each_vehicle_rev(mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() {
            continue;
        }
        f(v);
    }
}

/// `CTrain::Shutdown()` (0x6F58D0): deletes the 4 track node arrays (0xC38024..0xC38034) and clears
/// the pointers.
#[hook(addr = 0x6F58D0, conv = cdecl, name = "CTrain::Shutdown",
       expect = "56 BE 24 80 C3 00",
       unverified = "veh3-1", route = "split", hazards = "heap-writes",
       note = "frees heap memory (one-time shutdown)")]
pub fn train_shutdown() {
    for k in 0..4usize {
        let cell = g::<GPtr<u8>>(0xC3_8024 + 4 * k);
        let p = cell.read();
        if !p.is_null() {
            operator_delete(p);
        }
        cell.write(GPtr::from_addr(0));
    }
}

/// `CTrain::FindNextStationPositionInDirection(bool clockwise, float pos, float* dist, int32* idx)`
/// (0x6F5F00): the first station above `pos` (station 0 when none); anticlockwise the one before it;
/// a station closer than 100 units is skipped in the direction of travel. Writes the index and that
/// station's track position.
/// Not hooked: the station loop branches back into the first 9 bytes (`jl 0x6F5F02` at 0x6F5F18); the Rust function serves the ports' `orig!` calls in phases B/C.
pub fn find_next_station_position_in_direction(clockwise: u32, pos: f32, dist: GPtr<f32>, idx: GPtr<i32>) {
    let cw = clockwise as u8 != 0;
    let mut k = 0i32;
    while k < 6 {
        // `test ah, 0x41; je`: station > pos (not below, not equal, ordered)
        if !x(station_dist(k)).fcom(x(pos)).test_any(0x41) {
            break;
        }
        k += 1;
    }
    if k >= 6 {
        k = 0;
    }
    if !cw {
        k -= 1;
        if k < 0 {
            k += 6;
        }
    }
    if !x(pos).fsub_m(station_dist(k)).fabs().fcom(x(100.0)).test_parity(0x05) {
        k = if cw { k + 1 } else { k - 1 };
        if k < 0 {
            k += 6;
        }
        if k >= 6 {
            k = 0;
        }
    }
    idx.write(k);
    dist.cast::<u32>().write(g::<u32>(STATION_DIST + 4 * k as usize).read());
}

/// The last carriage of a train (`+0x5D0` chain).
fn last_carriage(mut t: GPtr<u8>) -> GPtr<u8> {
    loop {
        let n = rd::<GPtr<u8>>(t, 0x5D0);
        if n.is_null() {
            return t;
        }
        t = n;
    }
}

/// The travel direction bit (+0x5B8 bit 6) as the original passes it (`(b >> 6) & 1`).
fn direction(t: GPtr<u8>) -> u32 {
    u32::from((rd::<u8>(t, 0x5B8) >> 6) & 1)
}

/// `level(station idx) <= CStats::GetStatValue(181) + 1` (the cities unlocked).
fn station_allowed(idx: i32) -> bool {
    let level = xi(get_level_from_position(GPtr::from_addr(STATION_COORS + 12 * idx as usize))).to_f32();
    let s = sa_math::x87::X87::from_f64_exact(get_stat_value(0xB5)).fadd_m(1.0);
    !s.fcom(x(level)).test_any(0x01)
}

/// `CTrain::IsNextStationAllowed(CTrain*)` (0x6F7260): the next station of the last carriage (its
/// track position +0x5A8, direction bit) is in a level the player has unlocked (stat 181 + 1).
#[hook(addr = 0x6F7260, conv = cdecl, name = "CTrain::IsNextStationAllowed",
       expect = "51 8B 44 24 08",
       unverified = "veh3-1", route = "L2", hazards = "none",
       note = "the original keeps the station distance and the level float in its own argument slot")]
pub fn is_next_station_allowed(train: GPtr<u8>) -> u8 {
    let t = last_carriage(train);
    let mut dist = rd::<f32>(t, 0x5A8);
    let mut idx = 0i32;
    let pos = dist;
    call_find_next_station(direction(t), pos, GPtr::from_raw(&mut dist as *mut f32), GPtr::from_raw(&mut idx as *mut i32));
    b(station_allowed(idx))
}

/// `CTrain::SkipToNextAllowedStation(CTrain*)` (0x6F72F0): steps the last carriage's track
/// position from station to station until one is allowed, places it 20 units before that station
/// (speed +0x5A4 = ±0.1 by direction), loads the scene there and passes `ftol(distance * 0.05 + 23)`
/// minutes of game time.
#[hook(addr = 0x6F72F0, conv = cdecl, name = "CTrain::SkipToNextAllowedStation",
       expect = "83 EC 08 56 8B 74 24 10",
       unverified = "veh3-1", route = "split", hazards = "x87-transcendental,heap-writes",
       note = "fsqrt distance; LoadScene / LoadAllRequestedModels stream synchronously (file I/O)")]
pub fn skip_to_next_allowed_station(train: GPtr<u8>) {
    let t = last_carriage(train);
    let mut pos = rd::<f32>(t, 0x5A8);
    let mut idx;
    loop {
        idx = 0i32;
        let p = pos;
        call_find_next_station(direction(t), p, GPtr::from_raw(&mut pos as *mut f32), GPtr::from_raw(&mut idx as *mut i32));
        if station_allowed(idx) {
            break;
        }
    }
    wr::<f32>(t, 0x5A8, pos);
    let np = if rd::<u8>(t, 0x5B8) & 0x40 != 0 {
        wr::<u32>(t, 0x5A4, 0x3DCC_CCCD);
        x(pos).fsub_m(20.0)
    } else {
        wr::<u32>(t, 0x5A4, 0xBDCC_CCCD);
        x(pos).fadd_m(20.0)
    };
    wr::<f32>(t, 0x5A8, np.to_f32());
    let c: GPtr<u8> = GPtr::from_addr(STATION_COORS + 12 * idx as usize);
    load_scene(c);
    load_all_requested_models(0);
    let p = pos_ptr(t);
    let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(p, 4));
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
    game_logic_pass_time(ftol32(d.fmul_m(f32::from_bits(0x3D4C_CCCD)).fadd_m(23.0)));
}

/// `CTrain::RemoveMissionTrains()` (0x6F6A20): deletes every train (+0x590 == 6) with the mission
/// flag (+0x5B8 bit 5) that is not the player's vehicle.
#[hook(addr = 0x6F6A20, conv = cdecl, name = "CTrain::RemoveMissionTrains",
       expect = "53 8B 1D 94 44 B7 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "deletes vehicles (pool, world lists)")]
pub fn remove_mission_trains() {
    for_each_vehicle_rev(|v| {
        if rd::<u32>(v, 0x590) != 6 || v == find_player_vehicle(-1, 0) || rd::<u8>(v, 0x5B8) & 0x20 == 0 {
            return;
        }
        world_remove(v);
        entity_vdelete(v, 1);
    });
}

/// `CTrain::RemoveAllTrains()` (0x6F6AA0): deletes every train carriage whose chains (+0x5D0 and
/// +0x5D4, from the carriage itself) do not contain the player's vehicle.
#[hook(addr = 0x6F6AA0, conv = cdecl, name = "CTrain::RemoveAllTrains",
       expect = "83 EC 08 8B 0D 94 44 B7 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "deletes vehicles (pool, world lists)")]
pub fn remove_all_trains() {
    for_each_vehicle_rev(|v| {
        if rd::<u32>(v, 0x590) != 6 {
            return;
        }
        let mut player = false;
        for link in [0x5D0, 0x5D4] {
            let mut c = v;
            while !c.is_null() {
                if c == find_player_vehicle(-1, 0) {
                    player = true;
                }
                c = rd::<GPtr<u8>>(c, link);
            }
        }
        if !player {
            world_remove(v);
            entity_vdelete(v, 1);
        }
    });
}

/// `CTrain::FindNearestTrain(CVector, bool)` (0x6F7090): the train carriage with the smallest 2D
/// distance (below 9999999.0); with the flag only carriages with +0x5B8 bit 3 (engines) count.
#[hook(addr = 0x6F7090, conv = cdecl, name = "CTrain::FindNearestTrain",
       expect = "51 53 57 8B 3D 94 44 B7 00",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn find_nearest_train(px: f32, py: f32, _pz: f32, engines_only: u8) -> GPtr<u8> {
    let mut best = f32::from_bits(0x4B18_9680);
    let mut res = GPtr::from_addr(0);
    for_each_vehicle_rev(|v| {
        if rd::<u32>(v, 0x590) != 6 {
            return;
        }
        let p = pos_ptr(v);
        let dx = x(rd::<f32>(p, 0)).fsub_m(px);
        let dy = x(rd::<f32>(p, 4)).fsub_m(py);
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(best)).test_parity(0x05) {
            return;
        }
        if engines_only != 0 && rd::<u8>(v, 0x5B8) & 8 == 0 {
            return;
        }
        best = d.to_f32();
        res = v;
    });
    res
}

/// `CTrain::IsInTunnel()` (0x6F6320): a vertical line from the position up to z + 100 hits
/// buildings (`ProcessVerticalLine` with only the building flag).
#[hook(addr = 0x6F6320, conv = thiscall, name = "CTrain::IsInTunnel", this = "CTrain",
       expect = "8B 41 14 83 EC 30",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn train_is_in_tunnel(this: GPtr<u8>) -> u8 {
    let p = pos_ptr(this);
    let mut ent = 0u32;
    let mut cp = [0u32; 11];
    let z = x(rd::<f32>(p, 8)).fadd_m(100.0).to_f32();
    let r = process_vertical_line(
        p,
        z,
        GPtr::from_raw(&mut cp as *mut [u32; 11]).cast(),
        GPtr::from_raw(&mut ent as *mut u32).cast(),
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    b(r != 0)
}

/// `CTrain::Teleport(CVector, bool)` (0x6F7190): out of the world, the new position, with the flag
/// orientation (0, 0, 0), move and turn speeds cleared, back into the world.
#[hook(addr = 0x6F7190, conv = thiscall, name = "CTrain::Teleport", this = "CTrain",
       expect = "56 8B F1 56 E8 E7 C0 E6 FF",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "world remove/add (sector lists); the original zeroes its own argument slots")]
pub fn train_teleport(this: GPtr<u8>, px: u32, py: u32, pz: u32, reset_rot: u8) {
    world_remove(this);
    let p = pos_ptr(this);
    wr::<u32>(p, 0, px);
    wr::<u32>(p, 4, py);
    wr::<u32>(p, 8, pz);
    if reset_rot != 0 {
        set_orientation(this, 0, 0, 0);
    }
    for o in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<u32>(this, o, 0);
    }
    world_add(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_TRAIN_SHUTDOWN,
    HOOK_IS_NEXT_STATION_ALLOWED,
    HOOK_SKIP_TO_NEXT_ALLOWED_STATION,
    HOOK_REMOVE_MISSION_TRAINS,
    HOOK_REMOVE_ALL_TRAINS,
    HOOK_FIND_NEAREST_TRAIN,
    HOOK_TRAIN_IS_IN_TUNNEL,
    HOOK_TRAIN_TELEPORT,
];
