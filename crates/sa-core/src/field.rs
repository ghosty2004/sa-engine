//! Raw field and global access by address and byte offset, shared by every port crate (ADR-003 §3.2:
//! one definition, in the lowest layer that owns the concept). Everything goes through [`GPtr`]
//! (ADR-001's audited accessors); these are shorthands for the reads and writes the original makes at
//! fixed offsets that `catalog/layouts.csv` does not name yet. Ports import them (`use
//! sa_core::field::rd;`, or `as` a local name) instead of defining their own.
//!
//! Address arguments come in two widths, as the ports wrote them: `usize` (`g`, `gf`, ...) and `u32`
//! (`gu`, `gfu`, ...); both address the same byte.

use crate::mem::{GPtr, GameData};
use sa_math::x87::X87;

/// Field `T` at byte offset `off` of the object `p` points at.
#[inline]
pub fn at<T>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast()
}

/// Reads field `T` at byte offset `off`.
#[inline]
pub fn rd<T: GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

/// Writes field `T` at byte offset `off`.
#[inline]
pub fn wr<T: GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

/// Reads the u32 at byte offset `off`.
#[inline]
pub fn rd32(p: GPtr<u8>, off: isize) -> u32 {
    rd::<u32>(p, off)
}

/// Writes the u32 at byte offset `off`.
#[inline]
pub fn wr32(p: GPtr<u8>, off: isize, v: u32) {
    wr::<u32>(p, off, v)
}

/// Reads the f32 at byte offset `off`.
#[inline]
pub fn rdf(p: GPtr<u8>, off: isize) -> f32 {
    rd::<f32>(p, off)
}

/// `fld dword [p+off]`: the f32 at byte offset `off` onto the x87 stack.
#[inline]
pub fn ldx(p: GPtr<u8>, off: isize) -> X87 {
    X87::from_f32(rd::<f32>(p, off))
}

/// Reads the pointer at byte offset `off`.
#[inline]
pub fn rdp(p: GPtr<u8>, off: isize) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, off)
}

/// The three f32 of a vector at `p`.
#[inline]
pub fn rd3f(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// The three dwords of a vector at `p` (bit copies).
#[inline]
pub fn rd3u(p: GPtr<u8>) -> [u32; 3] {
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// Copies the 12-byte vector at `src` to `dst`, dword by dword (x, y, z).
#[inline]
pub fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// The 32-bit game address of a pointer (as the original passes it in a dword).
#[inline]
pub fn addr32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// Reads the byte at byte offset `off`.
#[inline]
pub fn rd8(p: GPtr<u8>, off: isize) -> u8 {
    p.byte_add(off).read()
}

/// A game global (or any fixed address), addressed directly.
#[inline]
pub fn g<T>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// [`g`] with a 32-bit address.
#[inline]
pub fn gu<T>(addr: u32) -> GPtr<T> {
    GPtr::from_addr(addr as usize)
}

/// A fixed address as a byte pointer.
#[inline]
pub fn pa(addr: usize) -> GPtr<u8> {
    GPtr::from_addr(addr)
}

/// A 32-bit game address as a byte pointer.
#[inline]
pub fn p8(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}

/// Reads the f32 at a fixed address (an `.rdata` constant or a global).
#[inline]
pub fn gf(addr: usize) -> f32 {
    GPtr::<f32>::from_addr(addr).read()
}

/// [`gf`] with a 32-bit address.
#[inline]
pub fn gfu(addr: u32) -> f32 {
    GPtr::<f32>::from_addr(addr as usize).read()
}

/// `fld dword [addr]`: the f32 at a fixed address onto the x87 stack.
#[inline]
pub fn gx(addr: usize) -> X87 {
    X87::from_f32(gf(addr))
}

/// Reads the u32 at a fixed address.
#[inline]
pub fn gr32(addr: usize) -> u32 {
    GPtr::<u32>::from_addr(addr).read()
}

/// Reads the u32 at a fixed 32-bit address.
#[inline]
pub fn gr32u(addr: u32) -> u32 {
    GPtr::<u32>::from_addr(addr as usize).read()
}

/// Reads the byte at a fixed address.
#[inline]
pub fn gr8(addr: usize) -> u8 {
    GPtr::<u8>::from_addr(addr).read()
}

/// Reads the pointer stored at a fixed address.
#[inline]
pub fn grp(addr: usize) -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(addr).read()
}

/// Writes the u32 at a fixed address.
#[inline]
pub fn gw32(addr: usize, v: u32) {
    GPtr::<u32>::from_addr(addr).write(v)
}

/// Writes the u32 at a fixed 32-bit address.
#[inline]
pub fn gw32u(addr: u32, v: u32) {
    GPtr::<u32>::from_addr(addr as usize).write(v)
}

/// Reads the u16 at a fixed 32-bit address.
#[inline]
pub fn gr16u(addr: u32) -> u16 {
    GPtr::<u16>::from_addr(addr as usize).read()
}

/// Writes the u16 at a fixed 32-bit address.
#[inline]
pub fn gw16u(addr: u32, v: u16) {
    GPtr::<u16>::from_addr(addr as usize).write(v)
}

/// Reads the byte at a fixed 32-bit address.
#[inline]
pub fn gr8u(addr: u32) -> u8 {
    GPtr::<u8>::from_addr(addr as usize).read()
}

/// Writes the byte at a fixed 32-bit address.
#[inline]
pub fn gw8u(addr: u32, v: u8) {
    GPtr::<u8>::from_addr(addr as usize).write(v)
}

/// The address of a Rust-owned local (a stack temporary the original keeps on its frame) as a byte
/// pointer, to pass where the original passes `&local`.
#[inline]
pub fn ptr_of<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn field_access_round_trips() {
        let mut buf = [0u32; 4];
        let p = ptr_of(&mut buf);
        wr32(p, 4, 0x3F80_0000);
        assert_eq!(rd32(p, 4), 0x3F80_0000);
        assert_eq!(rdf(p, 4), 1.0);
        assert_eq!(ldx(p, 4).to_f32(), 1.0);
        wr::<u16>(p, 8, 7);
        assert_eq!(rd::<u16>(p, 8), 7);
        assert_eq!(at::<u32>(p, 4).read(), 0x3F80_0000);
        let a = p.addr();
        assert_eq!(gr32(a + 4), 0x3F80_0000);
        assert_eq!(gf(a + 4), 1.0);
        assert_eq!(gx(a + 4).to_f32(), 1.0);
        gw32(a, 5);
        assert_eq!(buf[0], 5);
        assert_eq!(g::<u32>(a).read(), 5);
        assert_eq!(pa(a), p);
    }
}

pub fn vec3_ptr(v: &mut [u32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}


pub fn g32(a: usize) -> GPtr<u32> {
    GPtr::from_addr(a)
}


pub fn buf_ptr(buf: &mut [u8]) -> GPtr<u8> {
    GPtr::from_raw(buf.as_mut_ptr())
}


pub fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}


pub fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}


pub fn vec_ptr(v: &[u32; 3]) -> GPtr<u32> {
    GPtr::from_raw(v.as_ptr().cast_mut())
}

pub fn copy_c_string(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}


pub fn write3(p: GPtr<u8>, v: [f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        wr::<f32>(p, 4 * i as isize, *c);
    }
}


pub fn vec3(p: GPtr<u8>, off: isize) -> [f32; 3] {
    [rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8)]
}


pub fn copy_dwords(dst: GPtr<u8>, src: GPtr<u8>, n: isize) {
    for i in 0..n {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// The `1.0f` constant the original loads from `.rdata` (0x858624).
pub fn rdata_one() -> f32 {
    g::<f32>(0x858624).read()
}
