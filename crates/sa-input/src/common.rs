//! Helpers shared by the INPUT modules: raw field access at byte offsets, foreign globals, x87
//! shorthands.

use sa_core::mem::{GPtr, GameData};
use sa_math::x87::X87;

/// Field `T` at byte offset `off` of the object `p` points at.
#[inline]
pub fn at<T: GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast()
}

/// Reads field `T` at byte offset `off`.
#[inline]
pub fn rd<T: GameData>(p: GPtr<u8>, off: isize) -> T {
    at::<T>(p, off).read()
}

/// Writes field `T` at byte offset `off`.
#[inline]
pub fn wr<T: GameData>(p: GPtr<u8>, off: isize, v: T) {
    at::<T>(p, off).write(v)
}

/// A game global owned by another subsystem, addressed directly (as sa-stream does; its phase B
/// storage comes with that subsystem's port).
#[inline]
pub fn g<T: GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `fld dword` of an f32.
#[inline]
pub fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fild dword` of an int32.
#[inline]
pub fn xi(v: i32) -> X87 {
    X87::from_i32(v)
}

/// `_ftol` (0x821B40) of ST0, low 32 bits (EAX).
#[inline]
pub fn ftol32(v: X87) -> i32 {
    v.ftol() as u32 as i32
}
