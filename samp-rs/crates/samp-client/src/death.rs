//! samp's local death and respawn rules, for every host (docs/re/subsystems/player-state.md, "68 SetSpawnInfo",
//! "53 OnPlayerDeath", "74 ForcePlayerClassSelection"): the spawn info's range check, the death the local tick
//! detects, and what follows GTA's wasted sequence (the respawn, or class selection).

use crate::combat::Damager;

/// `ePedState`: dying (the wasted sequence runs) and dead.
pub const PEDSTATE_DIE: u32 = 54;
pub const PEDSTATE_DEAD: u32 = 55;

/// Whether a spawn position is accepted (samp.dll's validator 0x100B4B90): -20000 < x, y < 20000,
/// -10000 < z < 100000.
pub fn spawn_info_in_range(x: f32, y: f32, z: f32) -> bool {
    x > -20000.0 && x < 20000.0 && y > -20000.0 && y < 20000.0 && z > -10000.0 && z < 100000.0
}

/// What the local tick knows when it checks the local player's life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Life {
    /// Spawned (not dead, not in class selection).
    pub spawned: bool,
    /// Died and not respawned yet.
    pub dead: bool,
    /// The ped's `ePedState`.
    pub ped_state: u32,
    /// The ped's health.
    pub health: f32,
    /// The spawn info is valid.
    pub spawn_info: bool,
    /// Class selection after the next death (RPC 74, F4).
    pub force_class_selection: bool,
    /// The netgame is connected.
    pub connected: bool,
}

/// What the local tick does about it. Mirrors the death branch of samp.dll's local tick (A 0x100074C0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeStep {
    /// Spawned and alive: nothing.
    Alive,
    /// The ped just died: the death notification (RPC 53), then dead.
    Died,
    /// Dead while GTA's wasted sequence runs.
    Wasted,
    /// The wasted sequence ended but the ped has no health yet (the special states are cleared already).
    Reviving,
    /// The wasted sequence ended: spawn again with the stored spawn info.
    Respawn,
    /// The wasted sequence ended: back to class selection (no spawn info, forced, or not connected).
    ClassSelection,
}

impl Life {
    /// The step for this state (only asked while spawned or dead).
    pub fn step(&self) -> LifeStep {
        if self.spawned {
            return if (!self.dead && self.ped_state == PEDSTATE_DIE) || self.health <= 0.0 {
                LifeStep::Died
            } else {
                LifeStep::Alive
            };
        }
        if matches!(self.ped_state, PEDSTATE_DIE | PEDSTATE_DEAD) {
            return LifeStep::Wasted;
        }
        if self.spawn_info && !self.force_class_selection && self.connected {
            return if self.health > 0.0 { LifeStep::Respawn } else { LifeStep::Reviving };
        }
        LifeStep::ClassSelection
    }
}

/// The last damaging entity resolved against the client's players and vehicles: `player_of` names the player a
/// ped stands in for, `vehicle_driver` is `Some` for a vehicle of the server's with its driver's player, if any.
pub fn damager<E: Copy>(
    source: Option<E>,
    player_of: impl Fn(E) -> Option<u16>,
    vehicle_driver: impl Fn(E) -> Option<Option<u16>>,
) -> Damager {
    let Some(e) = source else { return Damager::None };
    if let Some(id) = player_of(e) {
        return Damager::Player(id);
    }
    match vehicle_driver(e) {
        Some(driver) => Damager::Vehicle { driver },
        None => Damager::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn life() -> Life {
        Life {
            spawned: true,
            dead: false,
            ped_state: 1,
            health: 100.0,
            spawn_info: true,
            force_class_selection: false,
            connected: true,
        }
    }

    #[test]
    fn death_then_respawn_or_class_selection() {
        assert_eq!(life().step(), LifeStep::Alive);
        assert_eq!(Life { health: 0.0, ..life() }.step(), LifeStep::Died);
        assert_eq!(Life { ped_state: PEDSTATE_DIE, ..life() }.step(), LifeStep::Died);
        let dead = Life { spawned: false, dead: true, ped_state: PEDSTATE_DEAD, health: 0.0, ..life() };
        assert_eq!(dead.step(), LifeStep::Wasted);
        assert_eq!(Life { ped_state: 1, ..dead }.step(), LifeStep::Reviving);
        assert_eq!(Life { ped_state: 1, health: 100.0, ..dead }.step(), LifeStep::Respawn);
        assert_eq!(Life { ped_state: 1, health: 100.0, force_class_selection: true, ..dead }.step(), LifeStep::ClassSelection);
        assert_eq!(Life { ped_state: 1, spawn_info: false, ..dead }.step(), LifeStep::ClassSelection);
        assert_eq!(Life { ped_state: 1, connected: false, ..dead }.step(), LifeStep::ClassSelection);
    }

    #[test]
    fn spawn_range_and_damagers() {
        assert!(spawn_info_in_range(2495.0, -1687.0, 13.5));
        assert!(!spawn_info_in_range(20000.0, 0.0, 0.0) && !spawn_info_in_range(0.0, 0.0, -10000.0));
        let players = |e: u32| (e == 1).then_some(7);
        let vehicles = |e: u32| match e {
            2 => Some(Some(7)),
            3 => Some(None),
            _ => None,
        };
        assert_eq!(damager(None, players, vehicles), Damager::None);
        assert_eq!(damager(Some(1), players, vehicles), Damager::Player(7));
        assert_eq!(damager(Some(2), players, vehicles), Damager::Vehicle { driver: Some(7) });
        assert_eq!(damager(Some(3), players, vehicles), Damager::Vehicle { driver: None });
        assert_eq!(damager(Some(4), players, vehicles), Damager::Other);
    }
}
