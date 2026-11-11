//! Helpers shared by the AI modules: byte-offset field access and x87 shorthands (re-exported from
//! sa-world's `world::common`), the task and event pools' `operator new`/`operator delete`, and the
//! `orig!` declarations several modules call.

pub use sa_core::mem::GPtr;
pub use sa_math::x87::X87;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

use sa_core::orig;

/// `CPools::ms_pTaskPool` (CPool<CTask>, slot 0x80)
pub const TASK_POOL: usize = 0xB744A8;
/// `CPools::ms_pEventPool` (CPool<CEvent>, slot 0x44)
pub const EVENT_POOL: usize = 0xB744AC;

orig! {
    /// `CTask::operator new(uint32 size)` (0x61A5A0): a slot of the task pool (0x80 bytes).
    pub fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTask::operator delete(void*)` (0x61A5B0): frees a task pool slot.
    pub fn task_delete(p: GPtr<u8>) = cdecl 0x61A5B0 => unported;
    /// `CEvent::operator new(uint32 size)` (0x4B5620): a slot of the event pool (0x44 bytes).
    pub fn event_new(size: u32) -> GPtr<u8> = cdecl 0x4B5620 => unported;
    /// `CEvent::operator delete(void*)` (0x4B5630): frees an event pool slot (the scalar deleting
    /// destructors inline the same code).
    pub fn event_delete(p: GPtr<u8>) = cdecl 0x4B5630 => unported;
    /// CRT `rand()` (0x821B1E); name the original return address with `sa_core::orig::at_site`.
    pub fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
}
