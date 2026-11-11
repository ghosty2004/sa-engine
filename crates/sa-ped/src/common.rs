//! Helpers shared by the PED modules: byte-offset field access and x87 shorthands (re-exported from
//! sa-world's `world::common`), the entity position, the pools, and the `orig!` declarations several
//! modules call.

pub use sa_core::mem::GPtr;
pub use sa_math::x87::X87;
pub use sa_world::collision::fpu::Fpu;
pub use sa_world::physical::shared::lp;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

use sa_core::orig;

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

/// The position an entity's placement gives: `m_matrix ? &m_matrix->pos (+0x30) : &m_placement.pos (+4)`.
pub fn ent_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

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

orig! {
    /// `FindPlayerPed(int32 playerId)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `FindPlayerVehicle(int32 playerId, bool bIncludeRemote)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `FindPlayerWanted(int32 playerId)` (0x56E230)
    pub fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => sa_world::world::cworld::find_player_wanted;
    /// `CStats::IncrementStat(uint16 stat, float value)` (0x55C180)
    pub fn increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u32 = thiscall 0x5E0170 => crate::ped::is_alive;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40)
    pub fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CStreaming::RequestModel(int32 model, int32 flags)` (0x4087E0)
    pub fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::SetModelIsDeletable(int32 model)` (0x409C10)
    pub fn set_model_is_deletable(model: i32) = cdecl 0x409C10 => unported;
}
