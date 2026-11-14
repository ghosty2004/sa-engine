//! Helpers shared by the PED modules: byte-offset field access and x87 shorthands (re-exported from
//! sa-world's `world::common`), the entity position, the pools, and the `orig!` declarations several
//! modules call.

pub use sa_core::mem::GPtr;
pub use sa_math::x87::X87;
pub use sa_world::collision::fpu::Fpu;
pub use sa_world::physical::shared::lp;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};


/// `CPools::ms_pPedPool` (CPool<CPed, CCopPed>: objects +0, byte map +4, size +8; slot 0x7C4)
pub const PED_POOL: usize = 0xB74490;
/// `CPools::ms_pVehiclePool` (slot 0xA18)
pub const VEHICLE_POOL: usize = 0xB74494;
/// `CPools::ms_pObjectPool`
pub const OBJECT_POOL: usize = 0xB7449C;
pub const PED_SLOT: usize = 0x7C4;
pub const VEHICLE_SLOT: usize = 0xA18;
/// `CWorld::Players` (0x190 per player) and `CWorld::PlayerInFocus`
pub const PLAYERS: usize = 0xB7CD98;
pub const PLAYER_INFO_SIZE: usize = 0x190;
pub const PLAYER_IN_FOCUS: usize = 0xB7CD74;

pub use sa_world::world::common::entity_pos_f as ent_pos;

/// Reads a vector (3 f32).
pub fn vec3(p: GPtr<f32>) -> [f32; 3] {
    [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
}

/// The pool slot `i` (objects base + i * size).
pub fn pool_slot(pool: GPtr<u8>, i: u32, size: usize) -> GPtr<u8> {
    rd::<GPtr<u8>>(pool, 0).byte_add((i as usize * size) as isize)
}

/// The pool's flag byte of slot `i` (bit 7 = free).
pub fn pool_flag(pool: GPtr<u8>, i: u32) -> u8 {
    rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read()
}

/// The pool size.
pub fn pool_size(pool: GPtr<u8>) -> i32 {
    rd::<i32>(pool, 8)
}

pub use sa_stream::bindings::set_model_is_deletable;
pub use sa_world::bindings::stats_increment_stat as increment_stat;
pub use sa_core::bindings::streaming_request_model as request_model;
pub use sa_world::bindings::get_stat_value;
pub use sa_core::bindings::get_weapon_info;
pub use crate::bindings::ped_is_alive;
pub use sa_world::bindings::find_player_wanted;
pub use sa_world::bindings::find_player_ped;
pub use sa_world::bindings::find_player_vehicle;
