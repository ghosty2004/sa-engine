//! `__init_CTheScripts__MissionCleanUp` (0x84A720, batch `boot-script-1`): the C++ static initialiser of
//! `CTheScripts::MissionCleanUp` (`CMissionCleanup`, 0xA90850). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `CTheScripts::MissionCleanUp.m_Objects` (75 entries of 8 bytes: `uint8 type` at +0, `int32 handle` at +4).
const MISSION_CLEANUP: usize = 0xA9_0850;
/// The entry count the loop runs (`mov ecx, 0x4B`).
const ENTRIES: usize = 0x4B;

/// `__init_CTheScripts__MissionCleanUp` (0x84A720, cdecl): the inlined `CMissionCleanupEntry` constructors, one
/// per entry in address order: type byte (+0) := 0, then the handle dword (+4) := 0. `CMissionCleanup::m_Count`
/// (0xA90AA8) is not written. No `atexit`. The original leaves EAX = 0xA90AAC, ECX = EDX = 0 (ignored by
/// `_initterm`).
#[hook(addr = 0x84A720, conv = cdecl, name = "__init_CTheScripts__MissionCleanUp", expect = "B8 54 08 A9 00",
       unverified = "boot-script-1", route = "split", hazards = "none", recs = "boot",
       note = "C++ static initialiser: runs once from _initterm before WinMain")]
pub fn init_mission_cleanup() {
    for i in 0..ENTRIES {
        let e = GPtr::<u8>::from_addr(MISSION_CLEANUP + 8 * i);
        e.write(0);
        e.byte_add(4).cast::<u32>().write(0);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INIT_MISSION_CLEANUP];
