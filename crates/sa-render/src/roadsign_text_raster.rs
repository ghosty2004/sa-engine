//! Road sign text rasters (batch `mvpm-render-6`): `RoadsignGenerateTextRaster`. Ported from the 1.0 US bytes
//! (gta-reversed `CustomRoadsignMgr.cpp` as a reading aid; the bytes win: the charset cell offset is the constant
//! `(row * 64 + col) * 32` bytes, not derived from the charset stride). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `RwRasterLock(RwRaster*, uint8 level, int32 mode)` (0x7FB2D0)
    fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> u32 = cdecl 0x7FB2D0 => unported;
    /// `RwRasterUnlock(RwRaster*)` (0x7FAEC0)
    fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => unported;
    /// `RoadsignGetLineAndRow(char c (AL), int32* col, int32* row)` (0x6FE260, register convention, caller pops)
    fn roadsign_get_line_and_row(c: u8, col: GPtr<i32>, row: GPtr<i32>) = reg("args = al stack stack") 0x6FE260 => unported;
}

/// `CCustomRoadsignMgr::pCharsetLockedRaster` (0xC3EF88)
const CHARSET_PIXELS: usize = 0xC3EF88;

/// `RoadsignGenerateTextRaster(char* text, int32 numLetters, RwRaster* charset, int32 unused, RwRaster* sign)` (0x6FEB70,
/// cdecl), AL.
///
/// AL = 0 when the sign raster does not lock (`RwRasterLock(sign, 0, 5)`; it is then left as it is), the locked charset
/// pixels (0xC3EF88) are missing, or either stride (+0x18) is 0. Each letter: `RoadsignGetLineAndRow(c, &col, &row)`
/// (both start at 1), then 16 rows of 8 pixels (dwords) from the charset at byte `(row * 64 + col) * 32` (rows advance
/// by the charset stride) to the sign at letter * 0x20 (rows advance by the sign stride). `RwRasterUnlock(sign)`, AL = 1.
#[hook(addr = 0x6feb70, conv = cdecl, name = "CCustomRoadsignMgr::RoadsignGenerateTextRaster", expect = "83 EC 14 56 8B 74 24 2C",
       unverified = "mvpm-render-6", route = "L2", hazards = "ax-al-return,heap-writes", recs = "driving",
       note = "writes the locked sign raster; RoadsignGetLineAndRow is a register-convention callee (AL + two stack pointers)")]
pub fn roadsign_generate_text_raster(text: GPtr<u8>, num: i32, charset: GPtr<u8>, _unused: i32, sign: GPtr<u8>) -> u8 {
    let lock = rw_raster_lock(sign, 0, 5);
    if lock == 0 {
        return 0;
    }
    let pixels = GPtr::<u32>::from_addr(CHARSET_PIXELS).read();
    if pixels == 0 {
        return 0;
    }
    let cstride = charset.byte_add(0x18).cast::<u32>().read();
    if cstride == 0 {
        return 0;
    }
    let sstride = sign.byte_add(0x18).cast::<u32>().read();
    if sstride == 0 {
        return 0;
    }
    let mut dst_letter = lock;
    for i in 0..num.max(0) {
        let mut col: i32 = 1;
        let mut row: i32 = 1;
        let c = text.byte_add(i as isize).read();
        roadsign_get_line_and_row(c, GPtr::from_raw(&mut col), GPtr::from_raw(&mut row));
        let mut src = GPtr::<u32>::from_addr(CHARSET_PIXELS).read().wrapping_add((((row << 6).wrapping_add(col)) << 5) as u32);
        let mut dst = dst_letter;
        for _ in 0..16 {
            for k in 0..8u32 {
                GPtr::<u32>::from_addr(dst.wrapping_add(4 * k) as usize)
                    .write(GPtr::<u32>::from_addr(src.wrapping_add(4 * k) as usize).read());
            }
            src = src.wrapping_add(cstride);
            dst = dst.wrapping_add(sstride);
        }
        dst_letter = dst_letter.wrapping_add(0x20);
    }
    rw_raster_unlock(sign);
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_ROADSIGN_GENERATE_TEXT_RASTER];
