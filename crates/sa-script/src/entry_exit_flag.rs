//! STD-7: `CEntryExitManager::SetEntryExitFlag` (stub 0x43EF90 → 0x1564BE0), reached by script opcode 09B4.
//! Not in the catalogue (no name in gta-reversed or the plugin SDK; named here after its behaviour), so it
//! is a phase B native. Ported from the 1.0 US bytes; unverified.

use sa_core::mem::GPtr;

/// `CEntryExitManager::mp_poolEntryExits` (a `CPool<CEntryExit>`, 0x3C per slot)
const ENTRY_EXIT_POOL: usize = 0x96A7D8;

/// `SetEntryExitFlag(int32 index, uint16 flag, bool set)` (cdecl): the pool slot `index` (null when its
/// flag byte is negative, then written through as the original does); `set` (the low byte) ORs `flag`
/// into the word +0x30, else ANDs it with `~flag`.
#[sa_core::native(addr = 0x43EF90, conv = cdecl, name = "CEntryExitManager::SetEntryExitFlag")]
pub fn set_entry_exit_flag(index: i32, flag: u32, set: u32) {
    let pool = GPtr::<GPtr<u8>>::from_addr(ENTRY_EXIT_POOL).read();
    let map = pool.byte_add(4).cast::<GPtr<i8>>().read();
    let e = if map.byte_add(index as isize).read() < 0 {
        GPtr::<u8>::NULL
    } else {
        pool.cast::<GPtr<u8>>().read().byte_add(index as isize * 0x3C)
    };
    let w = e.byte_add(0x30).cast::<u16>();
    if set as u8 != 0 {
        w.write(w.read() | flag as u16);
    } else {
        w.write(w.read() & !(flag as u16));
    }
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_SET_ENTRY_EXIT_FLAG];
