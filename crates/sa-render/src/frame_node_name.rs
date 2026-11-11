//! `GetFrameNodeName` (STD-6). Ported from the 1.0 US bytes (0x72FB30).

use sa_core::abi::NativeReg;
use sa_core::mem::GPtr;

use crate::rwplugins::NODE_NAME_OFFSET;

/// `GetFrameNodeName(frame)` (0x72FB30, a protected entry: `jmp 0x405AC5`, whose chunk loads the
/// node-name plugin offset `[0xC87C5C]` through a rotated `.HOODLUM` pointer): `frame + offset` for a
/// positive offset, else NULL.
#[sa_core::native(addr = 0x72FB30, conv = cdecl, name = "GetFrameNodeName")]
pub fn get_frame_node_name(frame: GPtr<u8>) -> u32 {
    let off = NODE_NAME_OFFSET.get();
    if off > 0 { (frame.addr() as u32).wrapping_add(off as u32) } else { 0 }
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_GET_FRAME_NODE_NAME];
