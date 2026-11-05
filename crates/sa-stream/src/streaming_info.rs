//! `CStreamingInfo` (0x407460–0x4075E0): one entry of `CStreaming::ms_aInfoForModel` and the intrusive
//! doubly linked lists threaded through it (indices into `CStreamingInfo::ms_pArrayBase`). PORT-STREAM
//! owner A.
//!
//! Ported from the 1.0 US bytes; gta-reversed `StreamingInfo.cpp` differs here (original kept):
//! - `Init` (0x407460) does not touch `m_Flags` (+6) (*new*: gr `*this = {}` clears it); store order
//!   load state, next, prev, next-on-cd, img id, size, offset;
//! - `InList` (0x407560) tests only `m_NextIndex != -1` and returns EAX 0/1 (gr-deviations
//!   `StreamingInfo.cpp:48`, gr also tests `m_PrevIndex`);
//! - list indices are `(ptr - ms_pArrayBase) / 0x14` with MSVC's signed magic division, truncated to
//!   16 bits; `AddToList` writes `base[this.next].prev` with the computed index of `this`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::shared::{field, CStreamingInfo, STREAMING_INFO_ARRAY_BASE};

sa_core::globals! {
    list GLOBALS;
}

const NEXT: isize = 0x0;
const PREV: isize = 0x2;

fn idx(base: GPtr<CStreamingInfo>, p: GPtr<CStreamingInfo>) -> i16 {
    p.index_from(base) as i16
}
fn at(base: GPtr<CStreamingInfo>, i: i16) -> GPtr<CStreamingInfo> {
    base.wrapping_index(i as i32)
}
fn w16(p: GPtr<CStreamingInfo>, off: isize) -> GPtr<i16> {
    field(p, off)
}

/// `CStreamingInfo::Init()` (0x407460 → 0x1567520, thiscall).
#[hook(addr = 0x407460, conv = thiscall, name = "CStreamingInfo::Init", expect = "E9 BB 00 16 01", this = "CStreamingInfo")]
pub fn init(this: GPtr<CStreamingInfo>) {
    field::<u8, _>(this, 0x10).write(0);
    w16(this, 0x0).write(-1);
    w16(this, 0x2).write(-1);
    w16(this, 0x4).write(-1);
    field::<u8, _>(this, 0x7).write(0);
    field::<u32, _>(this, 0xC).write(0);
    field::<u32, _>(this, 0x8).write(0);
}

/// `CStreamingInfo::AddToList(CStreamingInfo* after)` (0x407480 → 0x15674C0, thiscall, `ret 4`).
#[hook(addr = 0x407480, conv = thiscall, name = "CStreamingInfo::AddToList", expect = "E9 3B 00 16 01", this = "CStreamingInfo")]
pub fn add_to_list(this: GPtr<CStreamingInfo>, after: GPtr<CStreamingInfo>) {
    add_to_list_with(STREAMING_INFO_ARRAY_BASE.get(), this, after)
}

/// `AddToList` with `ms_pArrayBase` passed in (pure: L1).
pub fn add_to_list_with(base: GPtr<CStreamingInfo>, this: GPtr<CStreamingInfo>, after: GPtr<CStreamingInfo>) {
    let me = idx(base, this);
    let a = idx(base, after);
    w16(this, NEXT).write(w16(after, NEXT).read());
    w16(this, PREV).write(a);
    w16(after, NEXT).write(me);
    w16(at(base, w16(this, NEXT).read()), PREV).write(me);
}

/// `CStreamingInfo::RemoveFromList()` (0x4074E0 → 0x156CE90, thiscall).
#[hook(addr = 0x4074E0, conv = thiscall, name = "CStreamingInfo::RemoveFromList", expect = "E9 AB 59 16 01", this = "CStreamingInfo")]
pub fn remove_from_list(this: GPtr<CStreamingInfo>) {
    remove_from_list_with(STREAMING_INFO_ARRAY_BASE.get(), this)
}

/// `RemoveFromList` with `ms_pArrayBase` passed in (pure: L1). `ms_pArrayBase` is re-read before
/// each of the two stores in the original; it does not change in between.
pub fn remove_from_list_with(base: GPtr<CStreamingInfo>, this: GPtr<CStreamingInfo>) {
    let prev = w16(this, PREV).read();
    w16(at(base, w16(this, NEXT).read()), PREV).write(prev);
    let next = w16(this, NEXT).read();
    w16(at(base, w16(this, PREV).read()), NEXT).write(next);
    w16(this, NEXT).write(-1);
    w16(this, PREV).write(-1);
}

/// `CStreamingInfo::InList()` (0x407560, thiscall): `xor eax,eax; cmp word [ecx], -1; setne al`.
#[hook(addr = 0x407560, conv = thiscall, name = "CStreamingInfo::InList", expect = "33 C0 66 83 39 FF", this = "CStreamingInfo")]
pub fn in_list(this: GPtr<CStreamingInfo>) -> u32 {
    (w16(this, NEXT).read() != -1) as u32
}

/// `CStreamingInfo::GetCdPosn()` (0x407570, thiscall): `ms_files[m_ImgID].StreamHandle + m_CDOffset`
/// (wrapping 32-bit add; the handle is `FileID << 24`).
#[hook(addr = 0x407570, conv = thiscall, name = "CStreamingInfo::GetCdPosn", expect = "0F B6 41 07 8B 51 08", this = "CStreamingInfo")]
pub fn get_cd_posn(this: GPtr<CStreamingInfo>) -> u32 {
    get_cd_posn_with(crate::streaming::file_handle, this)
}

/// `GetCdPosn` with the `ms_files[img].StreamHandle` lookup passed in (pure: L1).
pub fn get_cd_posn_with(handle: impl Fn(i32) -> u32, this: GPtr<CStreamingInfo>) -> u32 {
    let i = this.read();
    handle(i.img_id as i32).wrapping_add(i.cd_offset)
}

/// `CStreamingInfo::GetCdPosnAndSize(CdStreamPos&, size_t&)` (0x4075A0 → 0x1560E50, thiscall,
/// `ret 8`, AL): false when `m_CDSize == 0`; else stores the position, then the size.
#[hook(addr = 0x4075A0, conv = thiscall, name = "CStreamingInfo::GetCdPosnAndSize", expect = "E9 AB 98 15 01", this = "CStreamingInfo")]
pub fn get_cd_posn_and_size(this: GPtr<CStreamingInfo>, pos: GPtr<u32>, size: GPtr<u32>) -> bool {
    get_cd_posn_and_size_with(crate::streaming::file_handle, this, pos, size)
}

/// `GetCdPosnAndSize` with the handle lookup passed in (pure: L1).
pub fn get_cd_posn_and_size_with(
    handle: impl Fn(i32) -> u32,
    this: GPtr<CStreamingInfo>,
    pos: GPtr<u32>,
    size: GPtr<u32>,
) -> bool {
    if this.read().cd_size == 0 {
        return false;
    }
    let i = this.read();
    pos.write(handle(i.img_id as i32).wrapping_add(i.cd_offset));
    size.write(this.read().cd_size);
    true
}

/// `CStreamingInfo::SetCdPosnAndSize(uint32, size_t)` (0x4075E0 → 0x1564070, thiscall, `ret 8`).
#[hook(addr = 0x4075E0, conv = thiscall, name = "CStreamingInfo::SetCdPosnAndSize", expect = "E9 8B CA 15 01", this = "CStreamingInfo")]
pub fn set_cd_posn_and_size(this: GPtr<CStreamingInfo>, offset: u32, size: u32) {
    field::<u32, _>(this, 0x8).write(offset);
    field::<u32, _>(this, 0xC).write(size);
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_ADD_TO_LIST,
    HOOK_REMOVE_FROM_LIST,
    HOOK_IN_LIST,
    HOOK_GET_CD_POSN,
    HOOK_GET_CD_POSN_AND_SIZE,
    HOOK_SET_CD_POSN_AND_SIZE,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn lists_link_and_unlink() {
        let arr = crate::shared::INFO_FOR_MODEL.ptr().cast::<CStreamingInfo>();
        for i in 0..4 {
            init(arr.wrapping_index(i));
        }
        let at = |i: i16| at(arr, i);
        // head (0) <-> tail (1)
        w16(at(0), NEXT).write(1);
        w16(at(1), PREV).write(0);
        add_to_list_with(arr, at(2), at(0));
        assert_eq!(w16(at(0), NEXT).read(), 2);
        assert_eq!(w16(at(2), NEXT).read(), 1);
        assert_eq!(w16(at(2), PREV).read(), 0);
        assert_eq!(w16(at(1), PREV).read(), 2);
        assert_eq!(in_list(at(2)), 1);
        remove_from_list_with(arr, at(2));
        assert_eq!(w16(at(0), NEXT).read(), 1);
        assert_eq!(w16(at(1), PREV).read(), 0);
        assert_eq!(in_list(at(2)), 0);
    }
}
