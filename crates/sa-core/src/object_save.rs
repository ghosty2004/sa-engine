//! FRONTIER-2 lane B (batch `frontier2-b4`): `CObjectSaveStructure` (the save block of one mission object):
//! `Construct` / `Extract`. Ported from the 1.0 US bytes; gta-reversed `ObjectSaveStructure.cpp` as a reading aid.

use crate::abi::HookReg;
use crate::bindings::{compress_from_full_matrix, decompress_into_full_matrix};
use crate::field::{rd, wr};
use crate::mem::GPtr;
use crate::hook;


/// The object flag bits (`CObject` +0x40) and the save structure's flag byte bits (+0x2F), in the original's order.
const FLAG_BITS: [(u32, u8); 7] =
    [(4, 1), (0x2000, 2), (0x4_0000, 4), (0x8_0000, 8), (0x10_0000, 0x10), (0x20_0000, 0x20), (0x80_0000, 0x40)];

/// `CObjectSaveStructure::Construct(CObject*)` (0x5D2160, thiscall, `ret 4`): the object's matrix (+0x14) compressed
/// into +0; +0x18 = byte +0x13D, +0x1A = word +0x13E, +0x1C = +0x150, +0x24 = 0, +0x20 = +0x1C, +0x28 = +0x140, +0x2C =
/// byte +0x13C, +0x2D = byte +0x144, +0x2E = byte +0x145; the flag byte +0x2F from the object flags +0x40 (bit 4 sets
/// it to 1, the others OR in 2..0x40).
#[hook(addr = 0x5D2160, conv = thiscall, name = "CObjectSaveStructure::Construct", expect = "56 57 8B 7C 24 0C",
       unverified = "frontier2-b4", route = "L2", hazards = "none")]
pub fn object_save_construct(this: GPtr<u8>, obj: GPtr<u8>) {
    compress_from_full_matrix(this, rd(obj, 0x14));
    wr::<u8>(this, 0x18, rd(obj, 0x13D));
    wr::<u16>(this, 0x1A, rd(obj, 0x13E));
    wr::<u32>(this, 0x1C, rd(obj, 0x150));
    wr::<u8>(this, 0x24, 0);
    wr::<u32>(this, 0x20, rd(obj, 0x1C));
    wr::<u32>(this, 0x28, rd(obj, 0x140));
    wr::<u8>(this, 0x2C, rd(obj, 0x13C));
    wr::<u8>(this, 0x2D, rd(obj, 0x144));
    wr::<u8>(this, 0x2F, 0);
    wr::<u8>(this, 0x2E, rd(obj, 0x145));
    if rd::<u8>(obj, 0x40) & 4 != 0 {
        wr::<u8>(this, 0x2F, 1);
    }
    for (bit, b) in &FLAG_BITS[1..] {
        if rd::<u32>(obj, 0x40) & bit != 0 {
            wr::<u8>(this, 0x2F, rd::<u8>(this, 0x2F) | b);
        }
    }
}

/// `CObjectSaveStructure::Extract(CObject*)` (0x5D2220, thiscall, `ret 4`): the matrix decompressed into the
/// object's (+0x14) and the fields copied back as `Construct` wrote them; each flag bit of the object (+0x40) set or
/// cleared from the flag byte +0x2F.
#[hook(addr = 0x5D2220, conv = thiscall, name = "CObjectSaveStructure::Extract", expect = "56 8B 74 24 08",
       unverified = "frontier2-b4", route = "L2", hazards = "none")]
pub fn object_save_extract(this: GPtr<u8>, obj: GPtr<u8>) {
    decompress_into_full_matrix(this, rd(obj, 0x14));
    wr::<u8>(obj, 0x13D, rd(this, 0x18));
    wr::<u16>(obj, 0x13E, rd(this, 0x1A));
    wr::<u32>(obj, 0x150, rd(this, 0x1C));
    wr::<u32>(obj, 0x1C, rd(this, 0x20));
    wr::<u32>(obj, 0x140, rd(this, 0x28));
    wr::<u8>(obj, 0x13C, rd(this, 0x2C));
    wr::<u8>(obj, 0x144, rd(this, 0x2D));
    wr::<u8>(obj, 0x145, rd(this, 0x2E));
    for (bit, b) in FLAG_BITS {
        let f: u32 = rd(obj, 0x40);
        wr(obj, 0x40, if rd::<u8>(this, 0x2F) & b != 0 { f | bit } else { f & !bit });
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_SAVE_CONSTRUCT, HOOK_OBJECT_SAVE_EXTRACT];
