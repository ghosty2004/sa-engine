//! `CColPoint::operator=` (batch `std8-world-1`), ported from the 1.0 US bytes (`.HOODLUM` body 0x15637E0).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `CColPoint::operator=(const CColPoint& o)` (stub 0x40FC80 → 0x15637E0, thiscall, `ret 4`, returns `this`): the
/// point (+0..+0xB) and the normal (+0x10..+0x1B) as dwords, the three surface bytes at +0x20 and the three at
/// +0x23 (a word and a byte each), the depth (+0x28); the padding at +0xC, +0x1C and +0x26 is not copied.
#[hook(addr = 0x40FC80, conv = thiscall, name = "thunk_FUN_015637e0", expect = "E9 5B 3B 15 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn col_point_assign(this: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> {
    for off in [0, 4, 8, 0x10, 0x14, 0x18] {
        this.byte_add(off).cast::<u32>().write(o.byte_add(off).cast::<u32>().read());
    }
    for off in [0x20, 0x23] {
        // `mov di, [src]; mov [dst], di; mov dl, [src+2]; mov [dst+2], dl` (+0x23 is unaligned: bytes)
        for i in 0..3 {
            this.byte_add(off + i).write(o.byte_add(off + i).read());
        }
    }
    this.byte_add(0x28).cast::<u32>().write(o.byte_add(0x28).cast::<u32>().read());
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_COL_POINT_ASSIGN];
