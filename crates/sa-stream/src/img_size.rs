//! STD-5: the gta3.img size query of `CStreaming::LoadCdDirectory` (stub 0x406360 → .HOODLUM
//! 0x15700D0). The body calls `GetFileSize` through an obfuscated import trampoline (0x1141980: the IAT
//! cell is `[0x401005] ^ 0x214D4C48` = 0x858074); the port calls the import cell directly.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `KERNEL32!GetFileSize(HANDLE, DWORD*)` (IAT 0x858074)
    fn get_file_size(h: u32, hi: GPtr<u32>) -> u32 = stdcall cell 0x858074 => unported;
}

/// `CdStream` handle of gta3.img (0x8E4010).
const GTA3_IMG_HANDLE: usize = 0x8E4010;

/// The gta3.img size (stub 0x406360): `GetFileSize([0x8E4010], NULL)`.
#[hook(addr = 0x406360, conv = cdecl, name = "thunk_FUN_015700d0", expect = "E9 6B 9D 16 01",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn gta3_img_size() -> u32 {
    get_file_size(GPtr::<u32>::from_addr(GTA3_IMG_HANDLE).read(), GPtr::NULL)
}

/// `CdStreamGetLastPosn()` (stub 0x406450 → .HOODLUM 0x1567540): `[0x8E4898]`, the sector after the
/// last read.
#[hook(addr = 0x406450, conv = cdecl, name = "thunk_FUN_01567540", expect = "E9 EB 10 16 01",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn cd_stream_get_last_posn() -> u32 {
    GPtr::<u32>::from_addr(0x8E4898).read()
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GTA3_IMG_SIZE, HOOK_CD_STREAM_GET_LAST_POSN];
