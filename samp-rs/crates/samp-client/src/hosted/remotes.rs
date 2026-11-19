//! The other players beyond walking, on the hosted client: driving and riding (200 / 211), aiming (203) and
//! shooting (206), the vehicles they push (209) and tow (210), and their radar markers (InitGame's marker mode, 208,
//! the player colours). The rules are the DLL client's (`crate::vehicle`, `crate::combat`, `crate::markers`,
//! `crate::remote`); the game side goes through the adapter in GTA terms: warps into seats, a vehicle's matrix and
//! speeds, the gun-aim task, a ped's shot at a point, blips on peds and points.

use std::collections::HashMap;

use samp_proto::raknet::Reliability;
use samp_proto::rpcs::player;
use samp_proto::session::InitGame;
use samp_proto::sync::IncomingSync;
use samp_proto::sync::player::{AimSync, RemoteBulletSync, RemoteFootSync, hit_type};
use samp_proto::sync::vehicle::{RemoteTrailerSync, RemoteUnoccupiedSync};
use samp_proto::time::Instant;

use sa_samp_api::hooks::HookAction;
use sa_samp_api::peds::{DamageSource, PedTask};
use sa_samp_api::vehicles::VehicleSnapshot;
use sa_samp_api::{GameAdapter, Matrix, PedRef, Vec3, VehicleRef, WeaponType};

use super::{HostedClient, report};
use crate::combat;
use crate::footsync::{Rotation, quat_from_matrix};
use crate::markers::{self, PlayerMarker};
use crate::remote::{self, FootData, accept_timestamp, normalise, rows_from_quat, slerp, state};
use crate::vehicle::{self as vm, DriverData, PassengerData, VehCorrection};

/// The aim task runs this long after its last renewal, so a lost aim sync ends it.
const AIM_TASK_MS: i32 = 2000;
/// The aim task is renewed this often while the player aims, and at once when the aim point moves farther.
const AIM_RENEW_MS: u128 = 1500;
const AIM_RENEW_DISTANCE: f32 = 1.0;
/// The lengths an aim sync's camera front has when it is a direction.
const AIM_FRONT_LENGTH: std::ops::RangeInclusive<f32> = 0.9..=1.1;
/// A ped shooting with the server's shots hits where the shot did: no spread.
const EXACT_ACCURACY: u8 = 100;
/// A 206 without a hit entity is replayed toward its hit position only when that lies within this distance of the
/// shooter (on a miss the client that sent it did not set the position).
const MISS_RANGE: f32 = 300.0;
/// The weapon types that fire instant hits (pistols to the sniper rifle, the minigun).
const INSTANT_HIT_WEAPONS: std::ops::RangeInclusive<u8> = 22..=34;
const MINIGUN: u8 = 38;
/// The body part RPC 115 reports when the hosted client cannot tell (the torso).
const BODY_TORSO: u32 = 3;

/// What the hosted client keeps of the other players beyond their on-foot sync.
#[derive(Debug, Default)]
pub(super) struct Others {
    players: HashMap<u16, Other>,
    /// InitGame's player marker mode (0: no markers).
    marker_mode: u32,
    /// The lag-comp selector (`combat::lag_comp_selector`).
    lag_comp: u8,
    /// The local player's health and armour last frame, for RPC 115.
    local_health: Option<f32>,
}

#[derive(Debug, Default)]
struct Other {
    state: u8,
    last_ts: Option<u32>,
    driver: Option<DriverData>,
    driver_new: bool,
    passenger: Option<PassengerData>,
    passenger_new: bool,
    /// The keys of the last on-foot sync.
    keys: u16,
    /// The weapon the ped was last armed with.
    weapon: Option<u8>,
    aim: Option<AimSync>,
    /// The aim task given: its point and when.
    aim_task: Option<(Vec3, Instant)>,
    marker: PlayerMarker,
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

fn arr(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn rows(m: &Matrix) -> Rotation {
    Rotation { right: arr(m.right), forward: arr(m.forward), up: arr(m.up) }
}

fn with_rows(m: &Matrix, r: &Rotation) -> Matrix {
    Matrix { right: v3(r.right), forward: v3(r.forward), up: v3(r.up), pos: m.pos }
}

fn combat_matrix(m: &Matrix) -> combat::Matrix {
    combat::Matrix { right: arr(m.right), forward: arr(m.forward), up: arr(m.up), pos: arr(m.pos) }
}

fn fires_instant_hits(weapon: u8) -> bool {
    INSTANT_HIT_WEAPONS.contains(&weapon) || weapon == MINIGUN
}

impl HostedClient {
    /// InitGame: the marker mode and the lag-comp selector.
    pub(super) fn others_init_game(&mut self, g: &InitGame) {
        self.others.marker_mode = g.player_marker_mode;
        self.others.lag_comp = combat::lag_comp_selector(g.lag_compensation);
    }

    /// The blip colour of player `id`: its colour (the player colour table, as samp's radar colour hook reads it).
    fn blip_colour(&self, id: u16) -> u32 {
        let colours = &self.ui.core.colours;
        markers::rgba_blip_colour(crate::worldents::radar_trace_colour(u32::from(id), |i| colours.rgba(i)))
    }

    /// The server vehicle id of a game vehicle.
    fn vehicle_id_of(&self, v: VehicleRef) -> Option<u16> {
        self.vehicles.iter().find(|(_, r)| **r == v).map(|(&id, _)| id)
    }

    /// A stream-in made the player's ped: the markers, an exact aim for the shots it replays, and the seat its last
    /// in-car sync names.
    pub(super) fn other_streamed_in(&mut self, id: u16, game: &mut dyn GameAdapter) {
        let Some(&ped) = self.players.get(&id) else { return };
        let colour = self.blip_colour(id);
        let mode = self.others.marker_mode;
        let o = self.others.players.entry(id).or_default();
        o.weapon = None;
        o.aim_task = None;
        let r = o.marker.on_spawn(game, mode, ped, colour);
        report(self, "marker blip", r);
        report(self, "set_accuracy", game.set_accuracy(ped, EXACT_ACCURACY));
        let Some(o) = self.others.players.get(&id) else { return };
        let seat = match (o.state, o.driver, o.passenger) {
            (vm::state::DRIVER, Some(d), _) => Some((d.vehicle_id, 0)),
            (vm::state::PASSENGER, _, Some(p)) if p.seat > 0 => Some((p.vehicle_id, p.seat)),
            _ => None,
        };
        if let Some((vehicle, seat)) = seat
            && self.seat_other(ped, vehicle, seat, game)
            && let Some(o) = self.others.players.get_mut(&id)
        {
            o.driver_new = o.driver.is_some();
        }
    }

    /// The player's ped goes (stream-out): the marker resets, the ped's blip went with it.
    pub(super) fn other_streamed_out(&mut self, id: u16, game: &mut dyn GameAdapter) {
        if let Some(o) = self.others.players.get_mut(&id) {
            o.marker.reset(game);
            o.marker.ped_destroyed();
            o.state = state::NONE;
            o.aim_task = None;
        }
    }

    /// The player left (or the players are deleted): its markers go.
    pub(super) fn other_left(&mut self, id: u16, game: &mut dyn GameAdapter) {
        // the ped's blip goes with the ped
        if let Some(mut o) = self.others.players.remove(&id) {
            o.marker.reset(game);
        }
    }

    /// Every player's markers and state (server restart, lost connection).
    pub(super) fn others_reset(&mut self, game: &mut dyn GameAdapter) {
        let ids: Vec<u16> = self.others.players.keys().copied().collect();
        for id in ids {
            self.other_left(id, game);
        }
        self.others.local_health = None;
    }

    /// RPC 72 (also SetPlayerMarkerForPlayer): the blips take the player's new colour.
    pub(super) fn other_recoloured(&mut self, id: u16, game: &mut dyn GameAdapter) {
        let colour = self.blip_colour(id);
        if let Some(o) = self.others.players.get(&id) {
            report(self, "set_blip_colour", o.marker.recolour(game, colour));
        }
    }

    /// Warps the ped into `seat` (0 driver) of server vehicle `vehicle` when the vehicle takes it there.
    fn seat_other(&mut self, ped: PedRef, vehicle: u16, seat: u8, game: &mut dyn GameAdapter) -> bool {
        let Some(&v) = self.vehicles.get(&vehicle) else { return false };
        let Ok(s) = game.vehicle_snapshot(v) else { return false };
        let kind = s.class.map_or(vm::Kind::Other, vm::kind_from_class);
        if !vm::seat_allowed(kind, s.health, seat, s.max_passengers, s.driver.is_some()) {
            return false;
        }
        let r = game.warp_into_vehicle(ped, v, (seat != 0).then(|| i32::from(seat) - 1));
        let ok = r.is_ok();
        report(self, "warp_into_vehicle", r);
        self.note(format!("remote ped {:#x} warped into vehicle {vehicle} seat {seat}", ped.0));
        ok
    }

    /// Whether the other player's ped is near enough to the local player for the relayed enter / exit (samp's
    /// relay range).
    fn within_relay_range(&self, ped: PedRef, game: &dyn GameAdapter) -> bool {
        let pos = |p: PedRef| game.ped_snapshot(p).ok().map(|s| arr(s.matrix.pos));
        match (pos(ped), game.local_player().and_then(pos)) {
            (Some(a), Some(b)) => vm::dist3d(a, b) < vm::RELAY_RANGE,
            _ => false,
        }
    }

    /// RPC 26 relayed: the other player's ped walks to the vehicle and gets in (the game's enter task, samp's
    /// time limit; a passenger takes any free seat). Mirrors the DLL client's `Vehicles::remote_enter`.
    pub(super) fn other_enters(&mut self, id: u16, vehicle: u16, passenger: bool, game: &mut dyn GameAdapter) {
        let (Some(&ped), Some(&v)) = (self.players.get(&id), self.vehicles.get(&vehicle)) else { return };
        if !self.within_relay_range(ped, game) || game.ped_snapshot(ped).map_or(true, |s| s.vehicle.is_some()) {
            return;
        }
        if let Some(o) = self.others.players.get_mut(&id) {
            o.keys = 0;
        }
        let task = if passenger {
            PedTask::EnterCarAsPassenger { vehicle: v, time_ms: vm::ENTER_TASK_MS, seat: -1 }
        } else {
            PedTask::EnterCarAsDriver { vehicle: v, time_ms: vm::ENTER_TASK_MS }
        };
        report(self, "enter task", game.start_task(ped, task));
    }

    /// RPC 154 relayed: the other player's ped leaves whatever vehicle it is in. Mirrors the DLL client's
    /// `Vehicles::remote_exit`.
    pub(super) fn other_exits(&mut self, id: u16, game: &mut dyn GameAdapter) {
        let Some(&ped) = self.players.get(&id) else { return };
        let Some(v) = game.ped_snapshot(ped).ok().and_then(|s| s.vehicle) else { return };
        if !self.within_relay_range(ped, game) {
            return;
        }
        let model = game.vehicle_snapshot(v).map_or(0, |s| s.model.0 as u32);
        if !vm::remote_exit_applies(model) || self.vehicle_id_of(v).is_none() {
            return;
        }
        if let Some(o) = self.others.players.get_mut(&id) {
            o.keys = 0;
        }
        report(self, "leave task", game.start_task(ped, PedTask::LeaveCar { vehicle: v }));
    }

    /// The syncs of the other players beyond on foot.
    pub(super) fn other_sync(&mut self, s: IncomingSync, game: &mut dyn GameAdapter) {
        let local = self.net.player_id();
        match s {
            IncomingSync::Vehicle { timestamp, sync } if Some(sync.player_id) != local => {
                self.store_driver(sync.player_id, timestamp, DriverData::from_sync(&sync), game);
            }
            IncomingSync::Passenger(p) if Some(p.player_id) != local => {
                self.store_passenger(p.player_id, PassengerData::from_sync(&p.sync), game);
            }
            IncomingSync::Aim(a) if self.players.contains_key(&a.player_id) => {
                self.others.players.entry(a.player_id).or_default().aim = Some(a.sync);
            }
            IncomingSync::Bullet(b) => self.apply_bullet(&b, game),
            IncomingSync::Markers(m) => {
                for rec in m.markers.iter().filter(|r| r.player_id < markers::MAX_PLAYER_ID) {
                    let colour = self.blip_colour(rec.player_id);
                    let streamed = self.players.contains_key(&rec.player_id);
                    let o = self.others.players.entry(rec.player_id).or_default();
                    let r = o.marker.on_record(game, rec.player_id, rec.position, streamed, colour);
                    report(self, "marker blip", r);
                }
            }
            IncomingSync::Unoccupied(u) => self.apply_unoccupied(&u, game),
            IncomingSync::Trailer(t) => self.apply_trailer(&t, game),
            _ => {}
        }
    }

    /// A 207 of a player with a ped (before its position is applied): out of a vehicle unless entering or leaving
    /// one (special actions 3 / 4), the synced weapon, and the gun-aim task while it aims.
    pub(super) fn other_on_foot(&mut self, sync: &RemoteFootSync, ped: PedRef, game: &mut dyn GameAdapter) {
        let f = FootData::from_sync(sync);
        let o = self.others.players.entry(sync.player_id).or_default();
        o.state = state::ON_FOOT;
        o.driver = None;
        o.passenger = None;
        o.keys = f.keys;
        if let Ok(s) = game.ped_snapshot(ped)
            && s.vehicle.is_some()
            && f.special_action != 3
            && f.special_action != 4
        {
            report(self, "warp_out_of_vehicle", game.warp_out_of_vehicle(ped, s.matrix.pos));
        }
        self.arm_other(sync.player_id, ped, f.weapon, game);
        self.aim_other(sync.player_id, ped, game);
    }

    /// The synced weapon on the ped: armed when it has it, else given with samp's ammo and armed. Mirrors samp.dll
    /// 0x100AF7D0 (as the DLL client's `players::apply_weapon`).
    fn arm_other(&mut self, id: u16, ped: PedRef, weapon: u8, game: &mut dyn GameAdapter) {
        let weapon = weapon & 0x3F;
        let Some(o) = self.others.players.get_mut(&id) else { return };
        if o.weapon == Some(weapon) {
            return;
        }
        o.weapon = Some(weapon);
        let armed = |game: &dyn GameAdapter| game.ped_snapshot(ped).is_ok_and(|s| s.weapon.0 == weapon);
        if armed(game) {
            return;
        }
        let _ = game.set_current_weapon(ped, WeaponType(weapon));
        if armed(game) || crate::game::weapon_model(u32::from(weapon)).is_none() {
            return;
        }
        report(self, "give_weapon", game.give_weapon(ped, WeaponType(weapon), combat::REMOTE_WEAPON_AMMO));
        report(self, "set_current_weapon", game.set_current_weapon(ped, WeaponType(weapon)));
    }

    /// While the player holds the aim key with a gun, the ped aims at the point its camera looks at (the aim sync's
    /// camera position and front, as far out as samp's head look: GTA aims a player's gun along its camera), or
    /// straight ahead of the ped when the sync's front is no direction (open.mp's NPCs send a camera at the
    /// origin); the task ends by itself once the aim stops being renewed.
    fn aim_other(&mut self, id: u16, ped: PedRef, game: &mut dyn GameAdapter) {
        let Some(o) = self.others.players.get_mut(&id) else { return };
        let gun = o.weapon.is_some_and(fires_instant_hits);
        let (Some(a), true, true) = (o.aim, o.keys & remote::KEY_AIM != 0, gun) else {
            if o.aim_task.take().is_some() {
                report(self, "stop aiming", game.stop_anims(ped));
                self.note(format!("player {id} stops aiming"));
            }
            return;
        };
        let (mut f, mut c) = (a.cam_front, a.cam_pos);
        if !(AIM_FRONT_LENGTH).contains(&(f.x * f.x + f.y * f.y + f.z * f.z).sqrt())
            && let Ok(s) = game.ped_snapshot(ped)
        {
            let (m, p) = (s.matrix.forward, s.matrix.pos);
            (f, c) = (samp_proto::types::Vec3 { x: m.x, y: m.y, z: m.z }, samp_proto::types::Vec3 { x: p.x, y: p.y, z: p.z });
        }
        let target = Vec3::new(
            c.x + f.x * remote::LOOK_DISTANCE,
            c.y + f.y * remote::LOOK_DISTANCE,
            c.z + f.z * remote::LOOK_DISTANCE,
        );
        let now = Instant::now();
        let renew = o.aim_task.is_none_or(|(t, at)| {
            let d = ((t.x - target.x).powi(2) + (t.y - target.y).powi(2) + (t.z - target.z).powi(2)).sqrt();
            d > AIM_RENEW_DISTANCE || now.duration_since(at).as_millis() >= AIM_RENEW_MS
        });
        if renew {
            let first = o.aim_task.replace((target, now)).is_none();
            report(self, "aim task", game.start_task(ped, PedTask::AimGunAt { target, time_ms: AIM_TASK_MS }));
            if first {
                self.note(format!(
                    "player {id} aims at ({:.1},{:.1},{:.1}): camera ({:.1},{:.1},{:.1}) front ({:.2},{:.2},{:.2})",
                    target.x, target.y, target.z, c.x, c.y, c.z, f.x, f.y, f.z
                ));
            }
        }
    }

    /// Stores a 200, seats the ped when it is in no vehicle yet, then the car-jacked rule. Mirrors samp.dll's 200
    /// store (as the DLL client's `Vehicles::store_driver`).
    fn store_driver(&mut self, id: u16, timestamp: Option<u32>, d: DriverData, game: &mut dyn GameAdapter) {
        let o = self.others.players.entry(id).or_default();
        if !accept_timestamp(o.last_ts, timestamp) {
            return;
        }
        if timestamp.is_some() {
            o.last_ts = timestamp;
        }
        let old_state = o.state;
        o.state = vm::state::DRIVER;
        o.driver = Some(d);
        o.driver_new = true;
        o.passenger = None;
        let tag = self.remotes.entry(id).or_default();
        (tag.health, tag.armour, tag.synced) = (d.health, d.armour, Some(Instant::now()));
        if let Some(&ped) = self.players.get(&id)
            && game.ped_snapshot(ped).is_ok_and(|s| s.vehicle.is_none())
        {
            self.seat_other(ped, d.vehicle_id, 0, game);
        }
        self.car_jacked(old_state, d.vehicle_id, game);
    }

    /// "Car Jacked": the local ped driving the vehicle a remote player starts driving is warped out, 1 m up.
    fn car_jacked(&mut self, old_state: u8, vehicle_id: u16, game: &mut dyn GameAdapter) {
        let Some(me) = game.local_player() else { return };
        let Ok(s) = game.ped_snapshot(me) else { return };
        let driven = s
            .vehicle
            .filter(|&v| game.vehicle_snapshot(v).is_ok_and(|vs| vs.driver == Some(me)))
            .and_then(|v| self.vehicle_id_of(v));
        if !vm::car_jacked(old_state, driven, vehicle_id) {
            return;
        }
        let p = s.matrix.pos;
        report(self, "warp_out_of_vehicle", game.warp_out_of_vehicle(me, Vec3::new(p.x, p.y, p.z + 1.0)));
        let r = game.big_message(vm::CAR_JACKED_TEXT, vm::CAR_JACKED_MS, vm::CAR_JACKED_STYLE as u16);
        report(self, "big_message", r);
        self.note(format!("car jacked (vehicle {vehicle_id})"));
    }

    /// Stores a 211 and seats the ped when it is in no vehicle yet.
    fn store_passenger(&mut self, id: u16, d: PassengerData, game: &mut dyn GameAdapter) {
        let o = self.others.players.entry(id).or_default();
        o.state = vm::state::PASSENGER;
        o.passenger = Some(d);
        o.passenger_new = true;
        o.driver = None;
        let tag = self.remotes.entry(id).or_default();
        (tag.health, tag.armour, tag.synced) = (d.health, d.armour, Some(Instant::now()));
        if let Some(&ped) = self.players.get(&id)
            && d.seat > 0
            && game.ped_snapshot(ped).is_ok_and(|s| s.vehicle.is_none())
        {
            self.seat_other(ped, d.vehicle_id, d.seat, game);
        }
    }

    /// Per frame, every seated remote driver and passenger: the one-shot apply of a new 200 (position correction,
    /// siren, health, weapon, trailer) or 211 (weapon), then every frame a driver's turn clamp and rotation slerp.
    /// Mirrors the driver / passenger part of samp.dll 0x100166B0.
    pub(super) fn others_tick(&mut self, game: &mut dyn GameAdapter) {
        let seated: Vec<(u16, PedRef)> = self.players.iter().map(|(&id, &p)| (id, p)).collect();
        for (id, ped) in seated {
            let Some(o) = self.others.players.get(&id) else { continue };
            let (st, driver, passenger) = (o.state, o.driver, o.passenger);
            let in_vehicle = game.ped_snapshot(ped).ok().and_then(|s| s.vehicle);
            match (st, driver, passenger) {
                (vm::state::DRIVER, Some(d), _) => {
                    let Some(&v) = self.vehicles.get(&d.vehicle_id) else { continue };
                    if in_vehicle != Some(v) {
                        continue;
                    }
                    let npc = self.remotes.get(&id).is_some_and(|t| t.npc);
                    self.driver_frame(id, ped, v, &d, npc, game);
                }
                (vm::state::PASSENGER, _, Some(d)) if in_vehicle.is_some() => {
                    if let Some(o) = self.others.players.get_mut(&id)
                        && std::mem::take(&mut o.passenger_new)
                    {
                        self.arm_other(id, ped, d.weapon, game);
                    }
                }
                _ => {}
            }
        }
        self.local_damage(game);
    }

    fn driver_frame(&mut self, id: u16, ped: PedRef, v: VehicleRef, d: &DriverData, npc: bool, game: &mut dyn GameAdapter) {
        let Ok(s) = game.vehicle_snapshot(v) else { return };
        let model = s.model.0 as u32;
        let train = vm::TRAIN_SYNC_MODELS.contains(&model);
        let fresh = self.others.players.get_mut(&id).is_some_and(|o| std::mem::take(&mut o.driver_new));
        if fresh {
            self.driver_motion(v, &s, d, train, game);
            if s.siren != d.siren {
                report(self, "set_siren", game.set_siren(v, d.siren));
            }
            report(self, "set_vehicle_health", game.set_vehicle_health(v, d.vehicle_health));
            self.arm_other(id, ped, d.weapon, game);
            if (d.trailer_id == 0 || d.trailer_id >= vm::MAX_VEHICLES)
                && let Some(cab) = self.vehicle_id_of(v)
                && s.trailer.is_some_and(|t| self.vehicle_id_of(t).is_some())
            {
                self.detach_trailer(cab, game);
            }
        }
        if train {
            return;
        }
        let Ok(s) = game.vehicle_snapshot(v) else { return };
        let cur = quat_from_matrix(&rows(&s.matrix));
        let q = normalise(slerp(d.rotation, cur, remote::SLERP_KEEP));
        report(self, "set_vehicle_matrix", game.set_vehicle_matrix(v, &with_rows(&s.matrix, &rows_from_quat(q))));
        let speed = if npc && d.velocity == [0.0; 3] { Vec3::default() } else { s.move_speed };
        let turn = v3(vm::clamp_turn(arr(s.turn_speed)));
        report(self, "set_vehicle_speed", game.set_vehicle_speed(v, speed, turn));
    }

    /// The one-shot position apply of a new 200: trains through their own position and speed, other vehicles
    /// through the driver correction (nudge or teleport).
    fn driver_motion(&mut self, v: VehicleRef, s: &VehicleSnapshot, d: &DriverData, train: bool, game: &mut dyn GameAdapter) {
        let cur = arr(s.matrix.pos);
        if train {
            if vm::train_needs_teleport(cur, d.position) {
                report(self, "set_train_position", game.set_train_position(v, v3(d.position)));
            }
            report(self, "set_vehicle_speed", game.set_vehicle_speed(v, v3(d.velocity), s.turn_speed));
            let speed = d.extra_f32();
            if speed.abs() <= 100.0 {
                report(self, "set_train_speed", game.set_train_speed(v, speed));
            }
            return;
        }
        let kind = s.class.map_or(vm::Kind::Other, vm::kind_from_class);
        let speed = match vm::correct_driver(cur, d.position, d.velocity, s.moving, vm::z_threshold(kind)) {
            VehCorrection::None => d.velocity,
            VehCorrection::Speed(speed) => speed,
            VehCorrection::Teleport => {
                let m = Matrix { pos: v3(d.position), ..s.matrix };
                report(self, "set_vehicle_matrix", game.set_vehicle_matrix(v, &m));
                d.velocity
            }
        };
        report(self, "set_vehicle_speed", game.set_vehicle_speed(v, v3(speed), s.turn_speed));
    }

    /// A 209, at once: a vehicle no player drives and no driven cab tows takes the rotation and the speeds, and
    /// the position when it is far off. Mirrors samp.dll 0x100158D0.
    fn apply_unoccupied(&mut self, u: &RemoteUnoccupiedSync, game: &mut dyn GameAdapter) {
        let s = &u.sync;
        if !vm::unoccupied_valid(s) {
            return;
        }
        let Some(&v) = self.vehicles.get(&s.vehicle_id) else { return };
        let Ok(vs) = game.vehicle_snapshot(v) else { return };
        let me = game.local_player();
        let player_driver = vs.driver.is_some_and(|d| Some(d) == me || self.players.values().any(|&p| p == d));
        let towed = vs.tractor.is_some_and(|t| game.vehicle_snapshot(t).is_ok_and(|c| c.driver.is_some()));
        if !vm::unoccupied_applies(player_driver, towed) {
            return;
        }
        let rows = Rotation { right: vm::vec3(s.roll), forward: vm::vec3(s.direction), up: arr(vs.matrix.up) };
        let target = vm::vec3(s.position);
        let turn = v3(vm::vec3(s.turn_velocity));
        match vm::correct_unoccupied(arr(vs.matrix.pos), target, vm::vec3(s.velocity)) {
            VehCorrection::None => {}
            VehCorrection::Speed(speed) => {
                report(self, "set_vehicle_matrix", game.set_vehicle_matrix(v, &with_rows(&vs.matrix, &rows)));
                report(self, "set_vehicle_speed", game.set_vehicle_speed(v, v3(speed), turn));
            }
            VehCorrection::Teleport => {
                let m = Matrix { pos: v3(target), ..with_rows(&vs.matrix, &rows) };
                report(self, "set_vehicle_matrix", game.set_vehicle_matrix(v, &m));
                report(self, "set_vehicle_speed", game.set_vehicle_speed(v, v3(vm::vec3(s.velocity)), turn));
            }
        }
    }

    /// A 210, at once: the trailer goes on the sender's cab, then the nudge or teleport. Mirrors samp.dll
    /// 0x10015C90.
    fn apply_trailer(&mut self, t: &RemoteTrailerSync, game: &mut dyn GameAdapter) {
        let s = &t.sync;
        let Some(cab_id) = self.others.players.get(&t.player_id).and_then(|o| o.driver).map(|d| d.vehicle_id) else {
            return;
        };
        let (Some(&cab), Some(&trailer)) = (self.vehicles.get(&cab_id), self.vehicles.get(&s.trailer_id)) else {
            return;
        };
        let model = self.vehicle_state.get(&s.trailer_id).map_or(0, |h| h.model);
        if s.trailer_id == 0 || !vm::is_towable_list(model) {
            return;
        }
        let Ok(ts) = game.vehicle_snapshot(trailer) else { return };
        if ts.tractor != Some(cab) {
            report(self, "set_trailer", game.set_trailer(cab, Some(trailer)));
        }
        if let Some(h) = self.vehicle_state.get_mut(&cab_id)
            && h.trailer.replace(s.trailer_id) != Some(s.trailer_id)
        {
            self.note(format!("trailer {} attached to {cab_id} by trailer sync", s.trailer_id));
        }
        let target = vm::vec3(s.position);
        let rows = rows_from_quat(normalise(s.rotation));
        let turn = v3(vm::vec3(s.turn_velocity));
        match vm::correct_trailer(arr(ts.matrix.pos), target, arr(ts.move_speed)) {
            VehCorrection::None => {}
            VehCorrection::Speed(speed) => {
                report(self, "set_vehicle_matrix", game.set_vehicle_matrix(trailer, &with_rows(&ts.matrix, &rows)));
                report(self, "set_vehicle_speed", game.set_vehicle_speed(trailer, v3(speed), turn));
            }
            VehCorrection::Teleport => {
                let m = Matrix { pos: v3(target), ..with_rows(&ts.matrix, &rows) };
                report(self, "set_vehicle_matrix", game.set_vehicle_matrix(trailer, &m));
                report(self, "set_vehicle_speed", game.set_vehicle_speed(trailer, v3(vm::vec3(s.velocity)), turn));
            }
        }
    }

    /// A 206, at once: the shooter's ped, armed with the shot's weapon, fires at the point the shot hit: the hit
    /// entity's matrix and the offset as the lag-comp selector reads them (samp's line-of-sight redirect), else
    /// the reported hit position. Mirrors samp.dll 0x10016370.
    fn apply_bullet(&mut self, b: &RemoteBulletSync, game: &mut dyn GameAdapter) {
        let s = &b.sync;
        if b.player_id >= markers::MAX_PLAYER_ID || Some(b.player_id) == self.net.player_id() {
            return;
        }
        if !combat::remote_bullet_valid(s) {
            return;
        }
        let Some(&shooter) = self.players.get(&b.player_id) else { return };
        if !game.ped_in_world(shooter) {
            return;
        }
        let entity = match s.hit_type {
            hit_type::PLAYER if Some(s.hit_id) == self.net.player_id() => {
                game.local_player().and_then(|me| game.ped_snapshot(me).ok()).map(|p| p.matrix)
            }
            hit_type::PLAYER if s.hit_id == b.player_id => None,
            hit_type::PLAYER => self.players.get(&s.hit_id).and_then(|&p| game.ped_snapshot(p).ok()).map(|p| p.matrix),
            hit_type::VEHICLE => self.vehicles.get(&s.hit_id).and_then(|&v| game.vehicle_snapshot(v).ok()).map(|v| v.matrix),
            _ => None,
        };
        let offset = [s.offset.x, s.offset.y, s.offset.z];
        let hit_pos = [s.hit_pos.x, s.hit_pos.y, s.hit_pos.z];
        let target = match entity {
            Some(m) if self.others.lag_comp == 0 => combat_matrix(&m).transform(offset),
            Some(m) if self.others.lag_comp == 1 => combat::add(arr(m.pos), offset),
            _ => hit_pos,
        };
        let Ok(at) = game.ped_snapshot(shooter).map(|p| p.matrix.pos) else { return };
        if entity.is_none() && vm::dist3d(arr(at), target) > MISS_RANGE {
            return;
        }
        self.arm_other(b.player_id, shooter, s.weapon_id, game);
        report(self, "fire_weapon_at", game.fire_weapon_at(shooter, v3(target)));
    }

    /// `BreakTowLink` of a vehicle: the game keeps a tow the server made (`vm::tow_link_kept`); the client's own
    /// detach forgets the trailer first.
    pub(super) fn tow_link(&self, trailer: VehicleRef, game: &dyn GameAdapter) -> HookAction {
        let Ok(t) = game.vehicle_snapshot(trailer) else { return HookAction::RunOriginal };
        let cab_id = t.tractor.and_then(|c| self.vehicle_id_of(c));
        let cab = cab_id.map(|id| self.vehicle_state.get(&id).is_some_and(|h| h.trailer.is_some()));
        let me = game.local_player();
        let local_drives = t.tractor.is_some_and(|c| game.vehicle_snapshot(c).is_ok_and(|s| me.is_some() && s.driver == me));
        if vm::tow_link_kept(false, t.health <= 0.0, t.tractor.is_some(), cab, local_drives) {
            HookAction::Skip
        } else {
            HookAction::RunOriginal
        }
    }

    /// RPC 115 (taking) when the local player lost health or armour to a remote player's ped this frame: that
    /// player, the loss and the weapon the game recorded.
    fn local_damage(&mut self, game: &mut dyn GameAdapter) {
        let Some(me) = game.local_player() else { return };
        let Ok(s) = game.ped_snapshot(me) else { return };
        let now = s.health + s.armour;
        let before = self.others.local_health.replace(now);
        let Some(before) = before.filter(|b| *b > now && self.spawned) else { return };
        let Ok(damage) = game.ped_damage(me) else { return };
        let DamageSource::Ped(p) = damage.source else { return };
        let Some((&id, _)) = self.players.iter().find(|(_, q)| **q == p) else { return };
        let rpc = player::OnPlayerGiveTakeDamage {
            taking: true,
            player_id: id,
            damage: before - now,
            weapon_id: u32::from(damage.weapon.0),
            bodypart: BODY_TORSO,
        };
        self.net.send_rpc(&rpc, Reliability::ReliableOrdered);
        self.note(format!("took {:.1} damage from player {id} (weapon {})", before - now, damage.weapon.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_hit_weapons() {
        assert!(fires_instant_hits(24) && fires_instant_hits(34) && fires_instant_hits(38));
        assert!(!fires_instant_hits(35) && !fires_instant_hits(0) && !fires_instant_hits(16));
    }
}
