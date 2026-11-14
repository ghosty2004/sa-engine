//! Helpers shared by the PORT-WORLD modules (zones, pickups, garages, ...): raw field access at byte
//! offsets, x87 shorthands and the `orig!` declarations several modules call.

use sa_core::mem::GPtr;
use sa_math::x87::X87;

/// Raw field and global access (ADR-003 §3.2: one definition, in sa-core).
pub use sa_core::field::{at, g, rd, wr};
pub use sa_math::fpu::x;
pub use sa_math::fpu::xi;
pub use sa_math::fpu::ftol32;
/// The f32 a `fstp dword` stores.
#[inline]
pub fn f32_of(v: X87) -> f32 {
    v.to_f32()
}

sa_core::globals! {
    list GLOBALS;
    // foreign (owner: CCamera / CWorld), declared here so the hosted L1 specs can set them
    /// `TheCamera` +0x4: the vector `FindPlayerCentreOfWorld` / `CPickup::IsVisible` fall back to
    pub static CAM_VECTOR @ 0xB6F02C: [f32; 3] = [0.0; 3];
    /// `TheCamera` +0x14: the entity whose matrix position they use when set
    pub static CAM_ENTITY @ 0xB6F03C: GPtr<u8> = GPtr::NULL;
    /// `CWorld::Players[0]` +4 (`m_pWanted`... the wanted pointer `FindPlayerWanted` returns)
    pub static PLAYER0_WANTED @ 0xB7CD9C: GPtr<u8> = GPtr::NULL;
    /// `CWorld::Players[1]` +4
    pub static PLAYER1_WANTED @ 0xB7CF2C: GPtr<u8> = GPtr::NULL;
}

/// An entity's position as the original reads it inline: `m_matrix ? &m_matrix->pos : &m_placement.pos`
/// (+0x14 matrix, its +0x30; else +0x4).
pub fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// [`entity_pos`] as a pointer to the three f32.
pub fn entity_pos_f(e: GPtr<u8>) -> GPtr<f32> {
    entity_pos(e).cast()
}

/// [`entity_pos`] as a pointer to the three dwords (bit copies).
pub fn entity_pos_u(e: GPtr<u8>) -> GPtr<u32> {
    entity_pos(e).cast()
}

/// The entity's position vector, read (three f32).
pub fn entity_pos3(e: GPtr<u8>) -> [f32; 3] {
    sa_core::field::rd3f(entity_pos(e))
}

/// `CEntity::m_nType` (the low 3 bits of +0x36).
pub fn entity_type(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// The camera position the original reads: `[0xB6F03C] + 0x30` when set, else the vector at 0xB6F02C.
pub fn cam_pos() -> GPtr<f32> {
    let e = CAM_ENTITY.get();
    if e.is_null() { CAM_VECTOR.ptr().cast() } else { e.byte_add(0x30).cast() }
}

pub use sa_core::bindings::load_data_from_work_buffer;
pub use sa_core::bindings::save_data_to_work_buffer;
pub use crate::bindings::find_player_coors;
pub use crate::bindings::find_player_ped;
pub use crate::bindings::find_player_vehicle;
