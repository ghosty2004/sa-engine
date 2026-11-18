//! RtPNG's stream write callback, which the statically linked libpng 1.0.12 calls with the PNG bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::rw_stream_write;
use crate::common::rd;

orig! {
    /// libpng `png_error(png_ptr, message)` (0x7D34F0)
    fn png_error(png: GPtr<u8>, msg: GPtr<u8>) = cdecl 0x7D34F0 => unported;
}

/// `_rtPNGWriteData(png_ptr, data, length)` (0x7CF980, cdecl): `RwStreamWrite(io_ptr, data, length)` when the
/// png struct's `io_ptr` (+0x54, set by `png_set_write_fn`) is set (a tail jump: its result stays in EAX), else
/// `png_error(png_ptr, NULL)`, which does not return.
#[hook(addr = 0x7CF980, conv = cdecl, name = "_rtPNGWriteData", expect = "8B 4C 24 04 8B 41 54",
       unverified = "pure1f-1", route = "L2", hazards = "heap-writes", note = "writes the PNG file")]
pub fn rt_png_write_data(png: GPtr<u8>, data: GPtr<u8>, len: u32) -> u32 {
    let io = rd::<u32>(png, 0x54);
    if io == 0 {
        png_error(png, GPtr::NULL);
        return 0;
    }
    rw_stream_write(GPtr::from_addr(io as usize), data, len).addr() as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RT_PNG_WRITE_DATA];
