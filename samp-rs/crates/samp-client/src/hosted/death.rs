//! The hosted client's death and respawn, with the DLL client's rules (`crate::death`, `crate::combat`): the death
//! the local tick detects is reported with its killer and reason (RPC 53), and once GTA's wasted sequence has
//! ended the player spawns again with the stored spawn info, or goes back to class selection (RPC 74, F4).

use samp_proto::raknet::Reliability;
use samp_proto::rpcs::player;
use samp_proto::time::Instant;

use sa_samp_api::peds::{DamageSource, PedSnapshot, PedTask};
use sa_samp_api::{GameAdapter, PedRef, Vec3};

use super::{HostedClient, report};
use crate::death::{Life, LifeStep};

/// The restart points `CRestart` holds (`MAX_RESTART_POINTS`); samp adds one per spawn.
const RESTART_POINTS: usize = 10;

/// The local player's life between spawns.
#[derive(Debug, Default)]
pub(super) struct LocalLife {
    /// Died and not spawned since.
    pub(super) dead: bool,
    /// Class selection after the next death (RPC 74).
    force_class_selection: bool,
    /// The restart points added at spawns.
    restarts: Vec<(Vec3, f32)>,
}

impl HostedClient {
    /// RPC 68 / 128's spawn block, kept when its position is in samp's range.
    pub(super) fn store_spawn_info(&mut self, s: player::SpawnInfo) {
        if crate::death::spawn_info_in_range(s.pos.x, s.pos.y, s.pos.z) {
            self.spawn = Some(s);
        } else {
            self.note(format!("spawn info out of range ignored: ({}, {}, {})", s.pos.x, s.pos.y, s.pos.z));
        }
    }

    /// RPC 74: class selection after the next death.
    pub(super) fn force_class_selection(&mut self) {
        self.life.force_class_selection = true;
    }

    /// Whether the local tick runs the death branch (spawned, or dead until the respawn).
    pub(super) fn alive_or_dead(&self) -> bool {
        self.spawned || self.life.dead
    }

    /// The death branch of the local tick: the death, then the respawn or class selection after the wasted
    /// sequence.
    pub(super) fn death_tick(&mut self, game: &mut dyn GameAdapter, now: Instant) {
        let Some(me) = game.local_player() else { return };
        let Ok(s) = game.ped_snapshot(me) else { return };
        let life = Life {
            spawned: self.spawned,
            dead: self.life.dead,
            ped_state: s.ped_state,
            health: s.health,
            spawn_info: self.spawn.is_some(),
            force_class_selection: self.life.force_class_selection,
            connected: self.netgame.connected(),
        };
        match life.step() {
            LifeStep::Died => self.on_local_death(game, me, &s, now),
            LifeStep::Respawn => {
                self.life.dead = false;
                self.note("respawn after death".into());
                self.spawn_local(game);
            }
            LifeStep::ClassSelection => {
                self.life.dead = false;
                let count = self.class_sel.count();
                self.enter_class_selection(game, count);
                self.life.force_class_selection = false;
            }
            LifeStep::Alive | LifeStep::Wasted | LifeStep::Reviving => {}
        }
    }

    /// The death: the phone and the drunk visuals stop, control comes back, a driver's last 200 goes out, then RPC
    /// 53 with the killer and reason samp derives from the ped's last damage; dead from here on.
    fn on_local_death(&mut self, game: &mut dyn GameAdapter, me: PedRef, s: &PedSnapshot, now: Instant) {
        if self.phone {
            self.phone = false;
            report(self, "start_task", game.start_task(me, PedTask::UseMobilePhone(false)));
        }
        report(self, "set_drunkenness", game.set_drunkenness(0));
        report(self, "set_player_control", game.set_player_control(true));
        if let Some(v) = s.vehicle {
            self.last_car_sync = None;
            self.car_sync(game, v, s, now);
        }
        let damage = game.ped_damage(me).unwrap_or_default();
        let source = (damage.source != DamageSource::None).then_some(damage.source);
        let player_of = |src: DamageSource| match src {
            DamageSource::Ped(p) => self.players.iter().find(|(_, r)| **r == p).map(|(&id, _)| id),
            _ => None,
        };
        let vehicle_driver = |src: DamageSource| match src {
            DamageSource::Vehicle(v) if self.vehicles.values().any(|r| *r == v) => {
                let driver = game.vehicle_snapshot(v).ok().and_then(|vs| vs.driver);
                Some(driver.and_then(|d| self.players.iter().find(|(_, r)| **r == d).map(|(&id, _)| id)))
            }
            _ => None,
        };
        let damager = crate::death::damager(source, player_of, vehicle_driver);
        let (reason, killer) = crate::combat::death_reason(u32::from(damage.weapon.0), damager);
        self.note(format!(
            "local death: weapon {} source {:?} -> RPC 53 reason {reason} killer {killer}",
            damage.weapon.0, damage.source
        ));
        self.net.send_rpc(&player::OnPlayerDeath { reason, killer_id: killer }, Reliability::ReliableOrdered);
        self.spawned = false;
        self.life.dead = true;
        report(self, "set_zone_names", game.set_zone_names(false));
    }

    /// The restart point of a spawn (samp's `ADD_HOSPITAL_RESTART` per spawn). The game picks the closest of its
    /// points when the player dies, so a point added before changes nothing; once the game's table is full (it
    /// holds [`RESTART_POINTS`], and the engine refuses more) the next restart is overridden instead.
    pub(super) fn restart_at(&mut self, game: &mut dyn GameAdapter, pos: Vec3, heading: f32) {
        if self.life.restarts.contains(&(pos, heading)) {
            return;
        }
        if self.life.restarts.len() < RESTART_POINTS && game.set_restart_point(pos, heading).is_ok() {
            self.life.restarts.push((pos, heading));
            return;
        }
        report(self, "override_restart", game.override_restart(pos, heading));
    }
}
