//! Helpers shared by the PORT-HUD modules: raw field access (sa-world's), x87 shorthands and the CRT
//! entry points several modules call.

use sa_core::mem::GPtr;
use sa_core::orig;

pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

orig! {
    /// CRT `strlen` (0x826330)
    pub fn strlen(s: GPtr<u8>) -> u32 = cdecl 0x826330 => unported;
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, a, b)` (0x821BB5)
    pub fn sprintf2(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32, b: i32) -> i32 = cdecl 0x821BB5 => unported;
}
