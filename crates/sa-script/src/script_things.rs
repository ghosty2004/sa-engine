//! `CTheScripts::GetNewUniqueScriptThingIndex` (batch `mvpi-script-1`): the unique-id counters of the
//! script-owned things. Ported from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0): creates the singleton on first use
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
}

/// The int16 id counter of each thing type (0..=8): its address for index 0 and the record stride.
const COUNTERS: [(u32, u32); 9] = [
    (0xA9126A, 0x18), // 0
    (0xA44112, 8),    // 1
    (0xA94D6C, 0x7C), // 2
    (0xA44072, 8),    // 3
    (0xA43F6A, 4),    // 4: also sets the in-use byte two bytes before
    (0xB71F82, 0x28), // 5
    (0xC3A120, 2),    // 6
    (0xC0AFF4, 2),    // 7: GetInstance around every access
    (0xC098D0, 2),    // 8
];

/// `CTheScripts::GetNewUniqueScriptThingIndex(int32 index, uint8 type)` (0x483720, cdecl): for type
/// 0..=8 (the exe's jump table) the int16 counter of `index` (address `base + index * stride`, 32-bit
/// wrapping) goes up by one, or restarts at 1 when it was 0xFFFE or more (then re-read); returns
/// `counter << 16 | index`. Type 4 also sets the byte at `0xA43F68 + index * 4` to 1 after the counter
/// is re-read. Type 7 calls `CDecisionMakerTypes::GetInstance()` before the compare, before the update
/// and before the re-read (results unused). Other types return -1.
#[hook(addr = 0x483720, conv = cdecl, name = "CTheScripts::GetNewUniqueScriptThingIndex", expect = "0F B6 4C 24 08",
       unverified = "mvpi-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_new_unique_script_thing_index(index: i32, ty: u32) -> i32 {
    let ty = ty as u8;
    if ty > 8 {
        return -1;
    }
    let (base, stride) = COUNTERS[ty as usize];
    let cell = GPtr::<u16>::from_addr(base.wrapping_add((index as u32).wrapping_mul(stride)) as usize);
    let dm = ty == 7;
    if dm {
        decision_maker_types_get_instance();
    }
    let c = cell.read();
    if dm {
        decision_maker_types_get_instance();
    }
    if c < 0xFFFE {
        cell.write(c + 1);
    } else {
        cell.write(1);
    }
    if dm {
        decision_maker_types_get_instance();
    }
    let v = u32::from(cell.read());
    if ty == 4 {
        GPtr::<u8>::from_addr(0xA43F68u32.wrapping_add((index as u32).wrapping_mul(4)) as usize).write(1);
    }
    ((v << 16) | index as u32) as i32
}

pub const HOOKS: &[HookReg] = &[HOOK_GET_NEW_UNIQUE_SCRIPT_THING_INDEX];
