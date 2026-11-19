//! The hosted client's trains, with the DLL client's rules (`crate::vehicle`): a locomotive or tram the server
//! streams in is a mission train on the nearest track (every rail model loaded first), its carriages take the vehicle
//! ids after it, and the whole train goes when the locomotive streams out.

use std::collections::{HashMap, HashSet};

use samp_proto::rpcs::vehicle as rv;

use sa_samp_api::{GameAdapter, ModelId, Vec3};

use super::{HostedClient, report};
use crate::vehicle as vm;

/// The server's trains.
#[derive(Debug, Default)]
pub(super) struct Trains {
    /// The locomotives' (and trams') ids.
    locos: HashSet<u16>,
    /// Carriage id → its locomotive's id.
    carriages: HashMap<u16, u16>,
}

impl HostedClient {
    /// RPC 164 for a locomotive or tram (a model with a train type).
    pub(super) fn stream_in_train(&mut self, s: &rv::StreamInVehicle, game: &mut dyn GameAdapter) {
        let Some(kind) = vm::train_type(s.model) else { return };
        for m in vm::RAIL_MODELS {
            report(self, "request_model", game.request_model(ModelId(m as i32)));
        }
        report(self, "load_all_now", game.load_all_now());
        if !vm::RAIL_MODELS.iter().all(|&m| game.model_loaded(ModelId(m as i32))) {
            self.debug_line(&format!("Warning: couldn't create vehicle type: {}", s.model));
            return;
        }
        let pos = Vec3::new(s.position.x, s.position.y, s.position.z);
        let first = match game.create_train(kind, pos, vm::train_clockwise(s.angle)) {
            Ok(v) => v,
            Err(e) => {
                self.note(format!("train {} (model {}): {e}", s.vehicle_id, s.model));
                self.debug_line(&format!("Warning: couldn't create vehicle type: {}", s.model));
                return;
            }
        };
        report(self, "set_vehicle_health", game.set_vehicle_health(first, s.health));
        self.vehicles.insert(s.vehicle_id, first);
        self.trains.locos.insert(s.vehicle_id);
        self.vehicle_state.insert(s.vehicle_id, super::vehicles::HostedVehicle::new(s.model));
        let carriages = game.train_carriages(first).unwrap_or_default();
        for (i, &c) in carriages.iter().enumerate().skip(1).take(usize::from(vm::TRAIN_CARRIAGE_IDS)) {
            let id = s.vehicle_id + i as u16;
            self.vehicles.insert(id, c);
            let model = vm::carriage_of(s.model).unwrap_or(s.model);
            self.vehicle_state.insert(id, super::vehicles::HostedVehicle::new(model));
            self.trains.carriages.insert(id, s.vehicle_id);
        }
        self.note(format!(
            "train {} (model {}) streamed in at ({:.1},{:.1},{:.1}) as {:#x}, {} carriage(s)",
            s.vehicle_id,
            s.model,
            pos.x,
            pos.y,
            pos.z,
            first.0,
            carriages.len().saturating_sub(1)
        ));
    }

    /// RPC 165 for a train: a carriage is forgotten (it goes with its locomotive); a locomotive deletes its train
    /// and forgets its carriages. Returns whether `id` was part of a train.
    pub(super) fn stream_out_train(&mut self, id: u16, game: &mut dyn GameAdapter) -> bool {
        if self.trains.carriages.remove(&id).is_some() {
            self.vehicles.remove(&id);
            self.vehicle_state.remove(&id);
            return true;
        }
        if !self.trains.locos.remove(&id) {
            return false;
        }
        let ids: Vec<u16> = self.trains.carriages.iter().filter(|(_, l)| **l == id).map(|(&c, _)| c).collect();
        for c in ids {
            self.trains.carriages.remove(&c);
            self.vehicles.remove(&c);
            self.vehicle_state.remove(&c);
        }
        self.vehicle_state.remove(&id);
        if let Some(v) = self.vehicles.remove(&id) {
            report(self, "destroy_train", game.destroy_train(v));
            self.note(format!("train {id} streamed out"));
        }
        true
    }

    /// Every train goes (a reset).
    pub(super) fn clear_trains(&mut self, game: &mut dyn GameAdapter) {
        let locos: Vec<u16> = self.trains.locos.iter().copied().collect();
        for id in locos {
            self.stream_out_train(id, game);
        }
    }
}
