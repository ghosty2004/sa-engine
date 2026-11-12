//! `CWorld::ClearScanCodes()` (0x563470, `#[native]`): reached when the 16-bit scan code wraps (every 65,535 world
//! scans; the car AI's ped-danger scan reaches it within a minute of Grove Street traffic). The entry `jmp 0x408258`
//! (a protection chunk loading ECX = 0xB7D0B8) is read as data at 0x4013A2, so the port is phase B only. Ported from
//! the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::native;

use super::common::{g, rd, wr};

/// `CWorld::ms_aSectors` (120 x 120 sectors of two entity lists, 8 bytes) and `ms_aRepeatSectors` (16 x 16 of three
/// lists, 12 bytes).
const SECTORS: usize = 0xB7D0B8;
const REPEAT_SECTORS: usize = 0xB992B8;
const REPEAT_END: usize = 0xB99EB8;

/// Every entity of the `CPtrList` at `list`: scan code (+0x2C, u16) = 0.
fn clear_list(list: usize) {
    let mut n = g::<GPtr<u8>>(list).read();
    while !n.is_null() {
        wr::<u16>(rd::<GPtr<u8>>(n, 0), 0x2C, 0);
        n = rd::<GPtr<u8>>(n, 4);
    }
}

/// `CWorld::ClearScanCodes()` (0x563470, cdecl): the scan code of every entity in every sector's two lists, then in
/// every repeat sector's three lists, cleared.
#[native(addr = 0x563470, conv = cdecl, name = "CWorld::ClearScanCodes")]
pub fn clear_scan_codes() {
    let mut s = SECTORS;
    while s < REPEAT_SECTORS {
        clear_list(s);
        clear_list(s + 4);
        s += 8;
    }
    let mut s = REPEAT_SECTORS;
    while s < REPEAT_END {
        clear_list(s);
        clear_list(s + 4);
        clear_list(s + 8);
        s += 12;
    }
}
