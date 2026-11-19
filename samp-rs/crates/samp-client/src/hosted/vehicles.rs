//! The hosted client's vehicles: the vehicle RPCs on the adapter, with the DLL client's rules (`crate::vehicle`,
//! `crate::vehops`), and the per-frame engine and lights of SetVehicleParams.

use samp_proto::rpcs::vehicle::{self as rv, Incoming as V};

use sa_samp_api::vehicles::{VehicleClass, VehicleDamage, VehicleSpawn};
use sa_samp_api::{AreaCode, GameAdapter, ModelId, Vec3, VehicleRef};

use super::{HostedClient, report};
use crate::vehicle as vm;

/// What the client keeps of a server vehicle besides its game vehicle.
#[derive(Clone, Debug, Default)]
pub(super) struct HostedVehicle {
    model: u32,
    /// SetVehicleParams' engine and lights: -1 unset, 0 off, 1 on.
    engine: i32,
    lights: i32,
    /// The trailer attached by RPC 148.
    trailer: Option<u16>,
    /// RPC 123's plate text (the engine draws the model's own plate).
    plate: Vec<u8>,
}

impl HostedVehicle {
    fn new(model: u32) -> Self {
        Self { model, engine: -1, lights: -1, ..Self::default() }
    }
}

/// The vehicle's class, when the adapter tells it.
fn class(game: &dyn GameAdapter, v: VehicleRef) -> Option<VehicleClass> {
    game.vehicle_snapshot(v).ok().and_then(|s| s.class)
}

impl HostedClient {
    pub(super) fn on_vehicle_rpc(&mut self, rpc: rv::Incoming, game: &mut dyn GameAdapter) {
        match rpc {
            V::StreamInVehicle(v) => self.stream_in_vehicle(&v, game),
            V::StreamOutVehicle(v) => {
                if let Some(r) = self.vehicles.remove(&v.vehicle_id) {
                    for cab in self.cabs_of(v.vehicle_id) {
                        self.detach_trailer(cab, game);
                    }
                    report(self, "destroy_vehicle", game.destroy_vehicle(r));
                    self.vehicle_state.remove(&v.vehicle_id);
                    self.note(format!("vehicle {} streamed out", v.vehicle_id));
                }
            }
            V::PutPlayerInVehicle(p) => {
                let (Some(me), Some(&v)) = (game.local_player(), self.vehicles.get(&p.vehicle_id)) else {
                    self.note(format!("PutPlayerInVehicle {}: unknown vehicle", p.vehicle_id));
                    return;
                };
                let seat = (p.seat != 0).then(|| i32::from(p.seat) - 1);
                report(self, "warp_into_vehicle", game.warp_into_vehicle(me, v, seat));
                self.note(format!("PutPlayerInVehicle {} seat {}", p.vehicle_id, p.seat));
            }
            V::RemovePlayerFromVehicle(_) => {
                if let Some(me) = game.local_player()
                    && let Ok(s) = game.ped_snapshot(me)
                {
                    let p = s.matrix.pos;
                    report(self, "warp_out_of_vehicle", game.warp_out_of_vehicle(me, Vec3::new(p.x + 2.0, p.y, p.z)));
                    self.note("RemovePlayerFromVehicle".into());
                }
            }
            V::SetVehicleHealth(h) => {
                if let Some(&v) = self.vehicles.get(&h.vehicle_id) {
                    report(self, "set_vehicle_health", game.set_vehicle_health(v, h.health));
                }
            }
            V::SetVehicleDamageStatus(d) => {
                if let Some(&v) = self.vehicles.get(&d.vehicle_id) {
                    let damage = VehicleDamage { panels: d.panels, doors: d.doors, lights: d.lights, tyres: d.tyres };
                    self.apply_damage(game, v, damage, true);
                    self.note(format!("SetVehicleDamageStatus {} {:?}", d.vehicle_id, damage));
                }
            }
            V::RemoveVehicleComponent(c) => {
                if let Some(&v) = self.vehicles.get(&c.vehicle_id) {
                    report(self, "remove_component", game.remove_component(v, ModelId(i32::from(c.component))));
                }
            }
            V::LinkVehicleToInterior(l) => {
                if let Some(&v) = self.vehicles.get(&l.vehicle_id) {
                    report(self, "set_vehicle_area", game.set_vehicle_area(v, AreaCode(l.interior)));
                }
            }
            V::SetVehicleVelocity(s) => self.set_local_vehicle_velocity(&s, game),
            V::ScmEvent(e) => self.scm_event(&e, game),
            V::SetVehiclePlate(p) => {
                if p.plate.len() <= 32
                    && let Some(st) = self.vehicle_state.get_mut(&p.vehicle_id)
                {
                    st.plate = p.plate.clone();
                    let r = self.vehicles.get(&p.vehicle_id).map(|&v| game.set_plate_text(v, &super::text(&p.plate)));
                    if let Some(r) = r {
                        report(self, "set_plate_text", r);
                    }
                }
            }
            V::AttachTrailer(a) => {
                let (Some(&t), Some(&c)) = (self.vehicles.get(&a.trailer_id), self.vehicles.get(&a.vehicle_id)) else {
                    return;
                };
                if let Some(st) = self.vehicle_state.get_mut(&a.vehicle_id) {
                    st.trailer = Some(a.trailer_id);
                }
                report(self, "set_trailer", game.set_trailer(c, Some(t)));
                self.note(format!("AttachTrailer {} to {}", a.trailer_id, a.vehicle_id));
            }
            V::DetachTrailer(d) => self.detach_trailer(d.vehicle_id, game),
            V::SetVehiclePosition(p) => {
                let Some(&v) = self.vehicles.get(&p.vehicle_id) else { return };
                let pos = Vec3::new(p.position.x, p.position.y, p.position.z);
                let model = self.vehicle_state.get(&p.vehicle_id).map_or(0, |s| s.model);
                if vm::TRAIN_SYNC_MODELS.contains(&model) {
                    report(self, "set_train_position", game.set_train_position(v, pos));
                } else if let Ok(s) = game.vehicle_snapshot(v) {
                    let mut m = s.matrix;
                    m.pos = pos;
                    report(self, "set_vehicle_matrix", game.set_vehicle_matrix(v, &m));
                }
            }
            V::SetVehicleZAngle(a) => {
                if let Some(&v) = self.vehicles.get(&a.vehicle_id) {
                    report(self, "set_vehicle_heading", game.set_vehicle_heading(v, a.angle));
                }
            }
            V::SetVehicleParams(p) => self.set_vehicle_params(p.vehicle_id, &p.params, game),
            other => self.note(format!("vehicle rpc {} (not applied natively yet)", other.id())),
        }
    }

    /// RPC 164: the vehicle with samp's creation state (heading, no petrol-tank weak point, no hydraulics, tyres
    /// that do not burst, doors unlocked), then health, interior, damage and tyres, the components and the paintjob
    /// of an automobile, and the body colours. Mirrors the DLL client's `vehicles/stream.rs`.
    fn stream_in_vehicle(&mut self, s: &rv::StreamInVehicle, game: &mut dyn GameAdapter) {
        if s.vehicle_id >= vm::MAX_VEHICLES || !(vm::MODEL_MIN..=vm::MODEL_MAX).contains(&s.model) {
            return;
        }
        if let Some(old) = self.vehicles.remove(&s.vehicle_id) {
            self.debug_line(&format!("Warning: vehicle {} was not deleted", s.vehicle_id));
            report(self, "destroy_vehicle", game.destroy_vehicle(old));
        }
        if vm::is_rail(s.model) {
            // samp creates trains as mission trains, which the adapter has no way to create
            self.note(format!("vehicle {} (model {}): trains are not created on this host", s.vehicle_id, s.model));
            return;
        }
        let model = ModelId(s.model as i32);
        report(self, "request_model", game.request_model(model));
        report(self, "load_all_now", game.load_all_now());
        let default = rv::StreamInVehicle::COLOUR_DEFAULT;
        let spawn = VehicleSpawn {
            model,
            pos: Vec3::new(s.position.x, s.position.y, s.position.z),
            heading_deg: s.angle,
            colours: vm::stream_in_colours(s.colour1, s.colour2).unwrap_or([default; 2]),
            paintjob: s.paintjob,
            area: AreaCode(s.interior),
        };
        let v = match game.create_vehicle(&spawn) {
            Ok(v) => v,
            Err(e) => {
                self.note(format!("vehicle {} (model {}): {e}", s.vehicle_id, s.model));
                self.debug_line(&format!("Warning: couldn't create vehicle type: {}", s.model));
                return;
            }
        };
        report(self, "set_petrol_tank_weakpoint", game.set_petrol_tank_weakpoint(v, false));
        report(self, "set_hydraulics", game.set_hydraulics(v, false));
        report(self, "set_tyres_can_burst", game.set_tyres_can_burst(v, false));
        report(self, "set_doors_locked", game.set_doors_locked(v, false));
        report(self, "set_vehicle_health", game.set_vehicle_health(v, s.health));
        if s.interior != 0 {
            report(self, "set_vehicle_area", game.set_vehicle_area(v, AreaCode(s.interior)));
        }
        let damage =
            VehicleDamage { panels: s.panel_damage, doors: s.door_damage, lights: s.light_damage, tyres: s.tyre_damage };
        self.apply_damage(game, v, damage, false);
        if class(game, v) == Some(VehicleClass::Automobile) {
            for slot in 0..s.mods.len() {
                if let Some(c) = s.component(slot) {
                    let r = crate::vehops::add_component(game, v, ModelId(i32::from(c)), &|| {});
                    report(self, "add_component", r);
                }
            }
            if let Some(p) = s.paintjob()
                && p < vm::PAINTJOBS
            {
                report(self, "set_paintjob", game.set_paintjob(v, p));
            }
        }
        if let Some(c) = vm::body_colours(s.body_colour1, s.body_colour2) {
            report(self, "set_vehicle_colours", game.set_vehicle_colours(v, c));
        }
        self.vehicles.insert(s.vehicle_id, v);
        self.vehicle_state.insert(s.vehicle_id, HostedVehicle::new(s.model));
        self.note(format!(
            "vehicle {} (model {}) streamed in at ({:.1},{:.1},{:.1}) as {:#x}",
            s.vehicle_id, s.model, s.position.x, s.position.y, s.position.z, v.0
        ));
    }

    /// samp's damage setter: all-zero panels, doors and lights repair a damaged automobile or plane (RPC 106; on
    /// stream-in they leave it as created), else they are set; the tyres are set in both cases.
    fn apply_damage(&self, game: &mut dyn GameAdapter, v: VehicleRef, d: VehicleDamage, repair: bool) {
        if vm::damage_repairs(d.panels, d.doors, d.lights) {
            let damaged = game.vehicle_damage(v).is_ok_and(|c| c.panels != 0 || c.doors != 0 || c.lights != 0);
            let parts = matches!(class(game, v), Some(VehicleClass::Automobile | VehicleClass::Plane));
            if repair && damaged && parts {
                report(self, "fix_vehicle", game.fix_vehicle(v));
            }
            if d.tyres == 0 && !repair {
                return;
            }
        }
        report(self, "set_vehicle_damage", game.set_vehicle_damage(v, d));
    }

    /// RPC 91: the move (kind 0) or turn (kind 1) speed of the server vehicle the local player is in.
    fn set_local_vehicle_velocity(&mut self, s: &rv::SetVehicleVelocity, game: &mut dyn GameAdapter) {
        let Some(v) = game.local_player().and_then(|me| game.ped_snapshot(me).ok()).and_then(|p| p.vehicle) else {
            return;
        };
        if !self.vehicles.values().any(|r| *r == v) {
            return;
        }
        let Ok(snap) = game.vehicle_snapshot(v) else { return };
        let speed = Vec3::new(s.velocity.x, s.velocity.y, s.velocity.z);
        let r = match s.kind {
            0 => game.set_vehicle_speed(v, speed, snap.turn_speed),
            1 => game.set_vehicle_speed(v, snap.move_speed, speed),
            _ => return,
        };
        report(self, "set_vehicle_speed", r);
    }

    /// RPC 96 (S→C), after the first spawn: paintjob and component of an automobile, colours. The mod-shop follow
    /// (event 4) needs the remote players' vehicles, which this client does not drive.
    fn scm_event(&mut self, e: &rv::ScmEvent, game: &mut dyn GameAdapter) {
        if !self.spawned_once || e.vehicle_id >= u32::from(vm::MAX_VEHICLES) {
            return;
        }
        let Some(&v) = self.vehicles.get(&(e.vehicle_id as u16)) else { return };
        let automobile = class(game, v) == Some(VehicleClass::Automobile);
        match e.event_type {
            1 if automobile && e.arg1 < u32::from(vm::PAINTJOBS) => {
                report(self, "set_paintjob", game.set_paintjob(v, e.arg1 as u8));
            }
            2 if automobile => {
                let r = crate::vehops::add_component(game, v, ModelId(e.arg1 as i32), &|| {});
                report(self, "add_component", r);
            }
            3 => report(self, "set_vehicle_colours", game.set_vehicle_colours(v, [e.arg1 as u8, e.arg2 as u8])),
            _ => {}
        }
        self.note(format!("ScmEvent {} vehicle {} ({}, {})", e.event_type, e.vehicle_id, e.arg1, e.arg2));
    }

    /// RPC 24: the engine and lights are kept for the per-frame pass; alarm and siren, the door lock, the doors of
    /// an automobile and the windows apply now.
    fn set_vehicle_params(&mut self, id: u16, p: &rv::VehicleParams, game: &mut dyn GameAdapter) {
        let Some(&v) = self.vehicles.get(&id) else { return };
        if let Some(st) = self.vehicle_state.get_mut(&id) {
            if let Some(on) = vm::param_bit(p.engine) {
                st.engine = i32::from(on);
            }
            if let Some(on) = vm::param_bit(p.lights) {
                st.lights = i32::from(on);
            }
        }
        if let Some(on) = vm::param_bit(p.alarm) {
            report(self, "set_siren", game.set_siren(v, on));
        }
        if let Some(locked) = vm::param_bit(p.doors) {
            report(self, "set_doors_locked", game.set_doors_locked(v, locked));
        }
        if let Some(on) = vm::param_bit(p.siren) {
            report(self, "set_siren", game.set_siren(v, on));
        }
        if class(game, v) == Some(VehicleClass::Automobile) {
            for (b, _, door) in vm::param_doors(p) {
                if let Some(open) = vm::param_bit(b) {
                    report(self, "set_door", game.set_door(v, door, if open { 1.0 } else { 0.0 }));
                }
            }
        }
        for (b, node) in vm::param_windows(p) {
            if let Some(closed) = vm::param_bit(b) {
                report(self, "set_window_open", game.set_window_open(v, node as u8, !closed));
            }
        }
        self.note(format!("SetVehicleParams {id}"));
    }

    /// RPC 98 (session): the tyres, the other damage as it is.
    pub(super) fn set_vehicle_tyres(&mut self, id: u16, tyres: u8, game: &mut dyn GameAdapter) {
        let Some(&v) = self.vehicles.get(&id) else { return };
        if let Ok(d) = game.vehicle_damage(v) {
            report(self, "set_vehicle_damage", game.set_vehicle_damage(v, VehicleDamage { tyres, ..d }));
        }
    }

    /// RPC 161 (session): the old params' door lock.
    pub(super) fn set_vehicle_doors_old(&mut self, id: u16, doors: u8, game: &mut dyn GameAdapter) {
        if let Some(&v) = self.vehicles.get(&id) {
            report(self, "set_doors_locked", game.set_doors_locked(v, doors != 0));
        }
    }

    fn cabs_of(&self, trailer: u16) -> Vec<u16> {
        self.vehicle_state.iter().filter(|(_, s)| s.trailer == Some(trailer)).map(|(&id, _)| id).collect()
    }

    fn detach_trailer(&mut self, cab: u16, game: &mut dyn GameAdapter) {
        let Some(&c) = self.vehicles.get(&cab) else { return };
        if self.vehicle_state.get_mut(&cab).and_then(|s| s.trailer.take()).is_some() {
            report(self, "set_trailer", game.set_trailer(c, None));
            self.note(format!("DetachTrailer from {cab}"));
        }
    }

    /// Per frame: SetVehicleParams' engine and lights of the vehicles that have one set, or every vehicle in the
    /// server's manual engine and lights mode (the DLL client's pool pass). Lights the server left unset keep the
    /// game's own state, so only vehicles with both are written.
    pub(super) fn vehicle_params_tick(&mut self, game: &mut dyn GameAdapter) {
        for (id, st) in &self.vehicle_state {
            if !self.manual_engine && st.engine < 0 && st.lights < 0 {
                continue;
            }
            let Some(&v) = self.vehicles.get(id) else { continue };
            let seated = game.vehicle_snapshot(v).is_ok_and(|s| s.driver.is_some());
            let engine = vm::engine_wanted(self.manual_engine, st.engine, seated);
            if let Some(lights) = vm::lights_wanted(self.manual_engine, st.lights) {
                report(self, "set_engine_lights", game.set_engine_lights(v, engine, lights));
            }
        }
    }
}
