//! `RwEngineGetVideoModeInfo` / `RwEngineGetCurrentVideoMode` (rwplcore), ported from the 1.0 US bytes.
//! Both are thin wrappers over `_rwDeviceSystemRequest(&RWSRCGLOBAL(dOpenDevice), request, ...)`; the
//! open device (`RwDevice`) lives at `RwGlobals + 0x10`.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::rw_globals;

/// `RwGlobals::dOpenDevice` (the `RwDevice` the engine opened).
const OPEN_DEVICE: isize = 0x10;
/// `rwDEVICESYSTEMGETMODEINFO`
const GET_MODE_INFO: i32 = 6;
/// `rwDEVICESYSTEMGETMODE`
const GET_MODE: i32 = 0xA;

orig! {
    /// `_rwDeviceSystemRequest(RwDevice*, RwInt32 requestID, void* pOut, void* pInOut, RwInt32 nIn)`
    /// (0x7F2AB0): calls `device->fpSystem(requestID, pOut, pInOut, nIn)`
    pub fn device_system_request(device: GPtr<u8>, request: i32, out: GPtr<u8>, inout: GPtr<u8>, n_in: i32) -> i32 = cdecl 0x7F2AB0 => unported;
}

/// `RwEngineGetVideoModeInfo(RwVideoMode* modeinfo, RwInt32 modeIndex)` (0x7F2CF0, cdecl):
/// `_rwDeviceSystemRequest(G + 0x10, 6, modeinfo, NULL, modeIndex)`; returns `modeinfo` when the
/// request returns non-zero, else NULL (the request's zero result).
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x7F2CF0, conv = cdecl, name = "RwEngineGetVideoModeInfo", expect = "8B 44 24 08 8B 0D 24 7B C9 00")]
pub fn engine_get_video_mode_info(info: GPtr<u8>, mode: i32) -> GPtr<u8> {
    let dev = rw_globals().byte_add(OPEN_DEVICE);
    if device_system_request(dev, GET_MODE_INFO, info, GPtr::NULL, mode) != 0 {
        info
    } else {
        GPtr::NULL
    }
}

/// `RwEngineGetCurrentVideoMode()` (0x7F2D20, cdecl): `_rwDeviceSystemRequest(G + 0x10, 0xA, &mode,
/// NULL, 0)`; returns `mode` when the request returns non-zero, else -1.
///
/// Not modelled: the local `mode` is the stack slot of `push ecx` (the caller's ECX) and is returned
/// as is if the request succeeds without writing it; here it starts at 0.
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x7F2D20, conv = cdecl, name = "RwEngineGetCurrentVideoMode", expect = "51 8B 0D 24 7B C9 00")]
pub fn engine_get_current_video_mode() -> i32 {
    let mut mode: i32 = 0;
    let dev = rw_globals().byte_add(OPEN_DEVICE);
    if device_system_request(dev, GET_MODE, GPtr::from_raw(&raw mut mode).cast(), GPtr::NULL, 0) != 0 {
        mode
    } else {
        -1
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ENGINE_GET_VIDEO_MODE_INFO, HOOK_ENGINE_GET_CURRENT_VIDEO_MODE];

/// Globals owned by this module (none).
pub const GLOBALS: &[GlobalDesc] = &[];
