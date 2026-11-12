//! `CColBox` copy assignment (stub 0x40EEC0, body 0x156D230 in `.HOODLUM`; batch `boot-coll-1`), reached while
//! loading through `CCollisionData::Copy` (0x40F120, one call per box). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::world::common::{rd, wr};

/// `CColBox& CColBox::operator=(const CColBox& other)` (stub 0x40EEC0 `jmp 0x156D230`, thiscall, `ret 4`;
/// catalogue `thunk_FUN_0156d230`, unnamed). Copies the box's `min` (+0..+0xB) and `max` (+0xC..+0x17) as dwords,
/// then the surface: the word +0x18 (material, piece) and the byte +0x1A (lighting); the byte +0x1B (padding) is not
/// copied. Every field is written right after it is read, in this order (an overlapping source behaves the same).
/// Returns `this` (EAX = the original ECX).
#[hook(addr = 0x40EEC0, conv = thiscall, name = "thunk_FUN_0156d230", expect = "E9 6B E3 15 01", this = "CColBox",
       route = "L2", hazards = "none", recs = "new-game",
       note = "only caller CCollisionData::Copy (0x40F120) while loading collision; the source and target are heap col data")]
pub fn col_box_assign(this: GPtr<u8>, other: GPtr<u8>) -> GPtr<u8> {
    for off in [0x0isize, 0x4, 0x8, 0xC, 0x10, 0x14] {
        wr::<u32>(this, off, rd::<u32>(other, off));
    }
    wr::<u16>(this, 0x18, rd::<u16>(other, 0x18));
    wr::<u8>(this, 0x1A, rd::<u8>(other, 0x1A));
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COL_BOX_ASSIGN];
