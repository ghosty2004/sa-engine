//! `RoadsignGetLineAndRow` (batch `mvpn-render-1`): the road-sign font cell of a character. Ported from the 1.0 US
//! bytes (the original is a 94-entry jump table; the port is the same mapping as a table). Register convention
//! (HARNESS-5): the character in AL, stack `(col, row)`, caller pops. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `(column, row)` of characters 0x21..=0x7E in the road-sign font raster (from the jump table at 0x6FE9F4).
const CELLS: [(i32, i32); 94] = [
    (0, 0), (0, 26), (3, 21), (0, 22), (2, 21), (2, 0), (0, 26), (0, 1), (1, 1), (1, 22), (2, 1), (0, 26), (0, 2),
    (1, 2), (0, 26), (3, 2), (0, 3), (1, 3), (2, 3), (3, 3), (0, 4), (1, 4), (2, 4), (3, 4), (0, 5), (2, 5), (1, 5),
    (2, 20), (0, 26), (3, 20), (3, 5), (2, 22), (0, 6), (1, 6), (2, 6), (3, 6), (0, 7), (1, 7), (2, 7), (3, 7), (0, 8),
    (1, 8), (2, 8), (3, 8), (0, 9), (1, 9), (2, 9), (3, 9), (0, 10), (1, 10), (2, 10), (3, 10), (0, 11), (1, 11),
    (2, 11), (3, 11), (0, 12), (1, 12), (2, 12), (0, 26), (0, 13), (0, 21), (0, 26), (0, 26), (1, 13), (2, 13), (3, 13),
    (0, 14), (1, 14), (2, 14), (3, 14), (0, 15), (1, 15), (2, 15), (3, 15), (0, 16), (1, 16), (2, 16), (3, 16), (0, 17),
    (1, 17), (2, 17), (3, 17), (0, 18), (1, 18), (2, 18), (3, 18), (0, 19), (1, 19), (2, 19), (0, 23), (3, 22), (2, 23),
    (1, 21),
];

/// `RoadsignGetLineAndRow(char c (AL), int32* col, int32* row)` (0x6FE260, caller pops): `*col`, `*row` = the font
/// cell of `c` (unsigned byte); characters outside 0x21..=0x7E and the unmapped ones (`"`, `'`, `,`, `/`, `=`, `\`,
/// `_`, `` ` ``) give the blank cell (0, 26); `!` is (0, 0). Column first, then row.
#[hook(addr = 0x6fe260, conv = reg("args = al stack stack"), name = "RoadsignGetLineAndRow", expect = "0F B6 C0 83 C0 DF",
       route = "L1-pure", hazards = "none", recs = "grove-walk",
       note = "register convention AL + stack (HARNESS-5); pure: writes only its two out pointers")]
pub fn roadsign_get_line_and_row(c: u8, col: GPtr<i32>, row: GPtr<i32>) {
    let (x, y) = match c {
        0x21..=0x7E => CELLS[(c - 0x21) as usize],
        _ => (0, 26),
    };
    col.write(x);
    row.write(y);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ROADSIGN_GET_LINE_AND_ROW];
