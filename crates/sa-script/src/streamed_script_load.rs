//! `CStreamedScripts::LoadStreamedScript` (batch `mvpl-script-1`): reads one streamed script (`.scm` from
//! `script.img`) into a new buffer. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CTheScripts::CheckStreamedScriptVersion(RwStream*, const char*)` (0x464FF0; `mov al, 1; ret`)
    fn check_streamed_script_version(stream: GPtr<u8>, name: GPtr<u8>) -> u8 = cdecl 0x464FF0 => unported;
    /// `operator new(size)` (0x821195)
    fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `RwStreamRead(RwStream*, void*, uint32)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
}

/// `CStreamedScripts::LoadStreamedScript(RwStream*, int32 index)` (stub 0x470840 → 0x1565EC0, thiscall, `ret 8`).
/// `CheckStreamedScriptVersion(stream, 0x859E84)` false → AL = 0. Else the entry `e = this + index * 0x20`: `e+0 =
/// operator new(e+0x1C size)`, `RwStreamRead(stream, e+0, e+0x1C)`, AL = 1. AL only: the upper EAX bytes are those
/// of the last callee's result.
#[hook(addr = 0x470840, conv = thiscall, name = "CStreamedScripts::LoadStreamedScript", expect = "E9 7B 56 0F 01",
       route = "split", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "reads the script file through RwStreamRead (file position) and allocates the script buffer: one run per state")]
pub fn load_streamed_script(this: GPtr<u8>, stream: GPtr<u8>, index: i32) -> u8 {
    if check_streamed_script_version(stream, GPtr::from_addr(0x859E84)) == 0 {
        return 0;
    }
    let e = this.byte_add((index as u32).wrapping_shl(5) as i32 as isize);
    let size = e.byte_add(0x1C).cast::<u32>().read();
    let buf = operator_new(size);
    e.cast::<GPtr<u8>>().write(buf);
    stream_read(stream, buf, e.byte_add(0x1C).cast::<u32>().read());
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_LOAD_STREAMED_SCRIPT];
