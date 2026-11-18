//! The game events (`sa_mod_api::event`), derived at each frame boundary from what the game state was at the last
//! one. Deriving them from the state instead of from inside the game's functions keeps the game untouched: the
//! host reads, it never sits in a port's body.

use sa_mod_api::event;
use sa_samp_api::{GameAdapter, PedRef, VehicleRef};

/// `ePedState` of a dying and of a dead ped.
const PED_STATE_DIE: u32 = 54;
/// See [`PED_STATE_DIE`].
const PED_STATE_DEAD: u32 = 55;

/// The events of state, input and key kinds, as masks.
const STATE_EVENTS: i32 = (1 << event::GAME_STATE)
    | (1 << event::PLAYER_SPAWN)
    | (1 << event::PLAYER_DEATH)
    | (1 << event::VEHICLE_ENTER)
    | (1 << event::VEHICLE_EXIT)
    | (1 << event::AREA);
const KEY_EVENTS: i32 = (1 << event::KEY_DOWN) | (1 << event::KEY_UP);

/// What the events are derived from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tracked {
    game_state: i32,
    player: Option<PedRef>,
    alive: bool,
    vehicle: Option<VehicleRef>,
    area: i32,
    keys: [u64; 4],
}

impl Tracked {
    /// Reads the game (only what the `wanted` events need), appends the events since the last call to `out` and
    /// keeps the new state. The first call compares with the state before the game starts (no player, state 0).
    pub fn update(&mut self, game: &dyn GameAdapter, wanted: i32, out: &mut Vec<(i32, i32, i32)>) {
        if wanted & STATE_EVENTS != 0 {
            self.update_state(game, out);
        }
        if wanted & KEY_EVENTS != 0 {
            self.update_keys(game, out);
        }
    }

    fn update_state(&mut self, game: &dyn GameAdapter, out: &mut Vec<(i32, i32, i32)>) {
        let game_state = game.game_state().raw();
        if game_state != self.game_state {
            out.push((event::GAME_STATE, game_state, self.game_state));
        }
        let player = game.local_player();
        let alive = player.and_then(|p| game.ped_snapshot(p).ok()).is_some_and(|s| {
            s.health > 0.0 && s.ped_state != PED_STATE_DIE && s.ped_state != PED_STATE_DEAD
        });
        let handle = |p: Option<PedRef>| p.map_or(-1, |p| p.0 as i32);
        match (self.player == player, self.alive, alive) {
            (true, true, false) => out.push((event::PLAYER_DEATH, handle(player), 0)),
            (false, _, true) | (true, false, true) => out.push((event::PLAYER_SPAWN, handle(player), 0)),
            _ => {}
        }
        let vehicle = game.local_vehicle();
        if vehicle != self.vehicle {
            if let Some(v) = self.vehicle {
                out.push((event::VEHICLE_EXIT, v.0 as i32, handle(self.player)));
            }
            if let Some(v) = vehicle {
                out.push((event::VEHICLE_ENTER, v.0 as i32, handle(player)));
            }
        }
        let area = game.area_raw();
        if area != self.area {
            out.push((event::AREA, area, self.area));
        }
        self.game_state = game_state;
        self.player = player;
        self.alive = alive;
        self.vehicle = vehicle;
        self.area = area;
    }

    fn update_keys(&mut self, game: &dyn GameAdapter, out: &mut Vec<(i32, i32, i32)>) {
        for vk in 1..=255u16 {
            let (w, bit) = (usize::from(vk / 64), 1u64 << (vk % 64));
            let down = game.key_held(vk);
            if down != (self.keys[w] & bit != 0) {
                out.push((if down { event::KEY_DOWN } else { event::KEY_UP }, i32::from(vk), 0));
                self.keys[w] ^= bit;
            }
        }
    }
}
