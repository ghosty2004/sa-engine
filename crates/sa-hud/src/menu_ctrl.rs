//! `CMenuManager` controller screens, `RedefineScreenUserInput`, `SaveStatsToFile`; `DIReleaseMouse` and
//! two HUD-catalogued thunks (PORT-HUD-3). Ported from the 1.0 US bytes; gta-reversed as reading aid (the
//! bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `RsGlobal.ps` (the `psGlobalType*`); `diMouse` (`IDirectInputDevice8*`) at +0x1C
const RS_GLOBAL_PS: usize = 0xC17054;

orig! {
    /// `RwEngineGetNumVideoModes()` (0x7F2CC0)
    fn rw_engine_get_num_video_modes() -> i32 = cdecl 0x7F2CC0 => unported;
    /// A COM method `(this)` through its vtable slot (stdcall)
    fn com_call0(f: u32, this: GPtr<u8>) -> u32 = stdcall dyn => unported;
}

/// 0x4410E0 (catalogue: unnamed, `thunk_FUN_015667a0`; a `.HOODLUM` stub): `CPad::IsDPadRightPressed()`
/// (thiscall): `NewState.DPadRight (+0x16) != 0 && OldState.DPadRight (+0x46) == 0`, EAX = 1 / 0. The same
/// shape as `CPad::IsDPadUpPressed` (0x53EE60, +0x10 / +0x40). Called by `CMenuManager::UserInput`
/// (0x5802A2).
#[hook(addr = 0x4410E0, conv = thiscall, name = "thunk_FUN_015667a0", expect = "E9 BB 56 12 01")]
pub fn pad_is_dpad_right_pressed(pad: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(pad, 0x16) != 0 && rd::<u16>(pad, 0x46) == 0)
}

/// 0x745A70 (catalogue: unnamed, `thunk_FUN_007f2cc0`): `jmp RwEngineGetNumVideoModes` (0x7F2CC0). Called by
/// `CMenuManager::ProcessPCMenuOptions` (0x57CFDF, 0x57D047), which compare the result with the selected
/// video mode. Hooked inline (the `jmp` relocated; the expect includes the following `nop` so the
/// installer does not take the 5-byte `jmp` for a `.HOODLUM` stub).
#[hook(addr = 0x745A70, conv = cdecl, name = "thunk_FUN_007f2cc0", expect = "E9 4B D2 0A 00 90")]
pub fn rw_engine_get_num_video_modes_thunk() -> i32 {
    rw_engine_get_num_video_modes()
}

/// `DIReleaseMouse()` (0x746F70): with a DirectInput mouse device (`RsGlobal.ps->diMouse`), `Unacquire`
/// (vtable +0x20) and `Release` (+0x08) it and clear the pointer; EAX = `Release`'s result, or 0 without a
/// device (gta-reversed declares the return as `IDirectInputDevice8*`). Each step rereads the globals, as
/// the bytes do.
#[hook(addr = 0x746F70, conv = cdecl, name = "DIReleaseMouse", expect = "A1 54 70 C1 00")]
pub fn di_release_mouse() -> u32 {
    let mouse = || rd::<GPtr<u8>>(g::<GPtr<u8>>(RS_GLOBAL_PS).read(), 0x1C);
    let m = mouse();
    if m.is_null() {
        return 0;
    }
    com_call0(rd::<u32>(rd::<GPtr<u8>>(m, 0), 0x20), m);
    let m = mouse();
    let r = com_call0(rd::<u32>(rd::<GPtr<u8>>(m, 0), 0x08), m);
    wr::<u32>(g::<GPtr<u8>>(RS_GLOBAL_PS).read(), 0x1C, 0);
    r
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PAD_IS_DPAD_RIGHT_PRESSED, HOOK_RW_ENGINE_GET_NUM_VIDEO_MODES_THUNK, HOOK_DI_RELEASE_MOUSE];
