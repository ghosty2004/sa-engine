//! Helpers shared by the PORT-RW-1 modules: raw field access, x87 shorthands, `RwGlobals` (`[0xC97B24]`)
//! and its memory function table, engine-plugin module bases, and the RW error calls.

use sa_core::mem::{GPtr, GameData};
use sa_core::orig;

/// `RwEngineInstance` (the `RwGlobals*`, `G` in the specs).
pub const RW_ENGINE_INSTANCE: usize = 0xC97B24;

pub use sa_core::field::at;

pub use sa_core::field::rd;

pub use sa_core::field::wr;

pub use sa_core::field::g;

pub use sa_core::field::p8;

pub use sa_math::fpu::x;

pub use sa_math::fpu::f32_of;

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

pub use sa_core::bindings::rw_error1;
pub use sa_core::bindings::rw_error_set;
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
    /// `_rwerror(code, a)` (0x8088D0, varargs; two arguments)
    pub fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
}

/// Byte copy `n` bytes (`rep movs` dword part, then the byte tail; no overlap in the callers).
pub fn copy_bytes(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let dw = n >> 2;
    for k in 0..dw as isize {
        wr::<u32>(dst, k * 4, rd::<u32>(src, k * 4));
    }
    for k in (dw * 4) as isize..n as isize {
        wr::<u8>(dst, k, rd::<u8>(src, k));
    }
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
    error1_code(code);
}

/// [`error1`], returning `_rwerror(code)` (the value the original leaves in its stack-built RwError,
/// for [`sa_core::abi::stack_footprint`]).
pub fn error1_code(code: u32) -> u32 {
    let mut e = RwError {
        plugin_id: 1,
        error_code: 0,
    };
    let c = rw_error1(code);
    e.error_code = c as i32;
    rw_error_set(GPtr::from_raw(&mut e).cast());
    c
}

/// `RwErrorSet({1, _rwerror(code, a)})`.
pub fn error2(code: u32, a: u32) {
    error2_plugin(1, code, a);
}

/// `RwErrorSet({plugin, _rwerror(code, a)})`: a plugin's error as the originals build it on their stack.
pub fn error2_plugin(plugin_id: i32, code: u32, a: u32) {
    let mut e = RwError {
        plugin_id,
        error_code: 0,
    };
    e.error_code = rw_error2(code, a) as i32;
    rw_error_set(GPtr::from_raw(&mut e).cast());
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
