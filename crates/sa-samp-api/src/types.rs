//! Shared value types and entity handles (`sa-types`).
//!
//! The original and phase A backends store the game object's address in a handle (`CPed*`, `CVehicle*`,
//! `CObject*`), which is what samp-rs passes around today; the native backend stores a generational pool
//! reference. samp-rs treats them as opaque.

pub use sa_types::*;

/// The 30 tracked weapon/vehicle skill stats the context switch and `/weaponskill` read
/// (`CStats::StatTypesFloat + 0x114` 0xB79494, 11 weapon skills used by samp).
pub const WEAPON_SKILL_STATS: usize = 11;
