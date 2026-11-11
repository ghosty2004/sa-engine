//! Helpers shared by the PORT-RW-1 modules: raw field access, x87 shorthands, `RwGlobals` (`[0xC97B24]`)
//! and its memory function table, engine-plugin module bases, and the RW error calls.

use sa_core::mem::{GPtr, GameData};
use sa_core::orig;
use sa_math::x87::X87;

/// `RwEngineInstance` (the `RwGlobals*`, `G` in the specs).
pub const RW_ENGINE_INSTANCE: usize = 0xC97B24;

/// Field `T` at byte offset `off` of `p`.
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

/// A game global addressed directly.
#[inline]
pub fn g<T: GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// A pointer from a 32-bit game address.
#[inline]
pub fn p8(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}

/// `fld dword` of an f32.
#[inline]
pub fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// The f32 a `fstp dword` stores.
#[inline]
pub fn f32_of(v: X87) -> f32 {
    v.to_f32()
}

/// `G` = `[0xC97B24]`.
#[inline]
pub fn rw_globals() -> GPtr<u8> {
    p8(g::<u32>(RW_ENGINE_INSTANCE).read())
}

/// An engine-plugin module: `G + [offset_global]`.
#[inline]
pub fn module(offset_global: usize) -> GPtr<u8> {
    let off = g::<u32>(offset_global).read();
    p8(g::<u32>(RW_ENGINE_INSTANCE).read().wrapping_add(off))
}

orig! {
    /// `G->rwmalloc(size, hint)` (`RwGlobals` +0x134)
    pub fn rw_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x134 => unported;
    /// `G->rwfree(p)` (+0x138)
    pub fn rw_free(p: GPtr<u8>) = cdecl table 0xC97B24 0x138 => unported;
    /// `G->rwrealloc(p, size, hint)` (+0x13C)
    pub fn rw_realloc(p: GPtr<u8>, size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x13C => unported;
    /// `G->rwcalloc(n, size, hint)` (+0x140)
    pub fn rw_calloc(n: u32, size: u32, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x140 => unported;
    /// `G->memoryAlloc(freelist, hint)` (+0x144, `_rwFreeListAllocReal` by default)
    pub fn rw_memory_alloc(fl: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x144 => unported;
    /// `G->memoryFree(freelist, p)` (+0x148, `_rwFreeListFreeReal` by default)
    pub fn rw_memory_free(fl: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0x148 => unported;
    /// `_rwerror(code)` (0x8088D0, varargs; one argument)
    pub fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code, a)` (0x8088D0, varargs; two arguments)
    pub fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `RwErrorSet(const RwError*)` (0x808820)
    pub fn rw_error_set(e: GPtr<RwError>) -> GPtr<RwError> = cdecl 0x808820 => unported;
}

/// `RwError` {pluginID, errorCode}.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RwError {
    pub plugin_id: i32,
    pub error_code: i32,
}

// SAFETY: two integers.
#[allow(unsafe_code)]
unsafe impl GameData for RwError {}

/// `RwErrorSet({1, _rwerror(code)})` as the originals build it on their stack.
pub fn error1(code: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: 0,
    };
    e.error_code = rw_error1(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `RwErrorSet({1, _rwerror(code, a)})`.
pub fn error2(code: u32, a: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: 0,
    };
    e.error_code = rw_error2(code, a) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `RwLLLink` / `RwLinkList` helpers (next +0, prev +4; a list head is a link to itself when empty).
pub mod llink {
    use super::{p8, rd, wr};
    use sa_core::mem::GPtr;

    pub fn next(l: GPtr<u8>) -> GPtr<u8> {
        p8(rd::<u32>(l, 0))
    }
    pub fn prev(l: GPtr<u8>) -> GPtr<u8> {
        p8(rd::<u32>(l, 4))
    }
    /// `rwLinkListInitialize`: head.next = head.prev = head.
    pub fn init(head: GPtr<u8>) {
        wr::<u32>(head, 0, head.addr() as u32);
        wr::<u32>(head, 4, head.addr() as u32);
    }
    /// `rwLinkListAddLLLink`: insert `l` after `head` (link.next = head.next; link.prev = head;
    /// head.next.prev = link; head.next = link), in the store order of RW's macro.
    pub fn add_head(head: GPtr<u8>, l: GPtr<u8>) {
        let n = rd::<u32>(head, 0);
        wr::<u32>(l, 0, n);
        wr::<u32>(l, 4, head.addr() as u32);
        wr::<u32>(p8(n), 4, l.addr() as u32);
        wr::<u32>(head, 0, l.addr() as u32);
    }
    /// `rwLinkListRemoveLLLink`: prev.next = next; next.prev = prev.
    pub fn remove(l: GPtr<u8>) {
        let n = rd::<u32>(l, 0);
        let p = rd::<u32>(l, 4);
        wr::<u32>(p8(p), 0, n);
        wr::<u32>(p8(n), 4, p);
    }
}
