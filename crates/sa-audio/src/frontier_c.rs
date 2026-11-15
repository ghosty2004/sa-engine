//! Audio functions ported code still reached through the original (FRONTIER-2 lane C). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::at;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::port4_init::get_dvd_game_path;
use sa_core::bindings::{file_close, open_file};

orig! {
    /// KERNEL32 `lstrcpyA` (IAT 0x8580D4)
    fn lstrcpy_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x8580D4 => unported;
    /// KERNEL32 `lstrcatA` (IAT 0x8580D8)
    fn lstrcat_a(d: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = stdcall cell 0x8580D8 => unported;
}

/// `CAEMP3TrackLoader::IsCurrentAudioStreamAvailable()` (0x4E0BD0, thiscall): the DVD game path (`getDvdGamePath`,
/// kept at +0x18); none → +0x14 = 0. Else the path is built on the stack (`lstrcpyA(path, dvd)`, `lstrcatA` of
/// "AUDIO\STREAMS\" (0x85F1BC) and of the current stream name +0x10) and opened (`CFileMgr::OpenFile(path, "rb")`); a
/// handle above 0 → +0x14 = 1 and the file closed, else +0x14 = 0. AL = +0x14.
#[hook(addr = 0x4E0BD0, conv = thiscall, name = "CAEMP3TrackLoader::IsCurrentAudioStreamAvailable",
       expect = "81 EC 04 01 00 00 56", unverified = "frontier2-c4", route = "split", hazards = "uninit-locals",
       note = "opens a file of the install (OS state)")]
pub fn is_current_audio_stream_available(this: GPtr<u8>) -> u8 {
    let dvd = get_dvd_game_path();
    at::<u32>(this, 0x18).write(dvd);
    if dvd == 0 {
        at::<u8>(this, 0x14).write(0);
        return 0;
    }
    let mut path = [0u8; 0x104];
    let p = GPtr::from_raw(path.as_mut_ptr());
    lstrcpy_a(p, GPtr::from_addr(dvd as usize));
    lstrcat_a(p, GPtr::from_addr(0x85F1BC));
    lstrcat_a(p, at::<GPtr<u8>>(this, 0x10).read());
    let f = open_file(p, GPtr::from_addr(0x85A53C));
    if (f.addr() as i32) <= 0 {
        at::<u8>(this, 0x14).write(0);
        return 0;
    }
    at::<u8>(this, 0x14).write(1);
    file_close(f);
    at::<u8>(this, 0x14).read()
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_CURRENT_AUDIO_STREAM_AVAILABLE];
