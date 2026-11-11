//! INPUT: static initialisers / `atexit` destructors (PORT-ANIM-INPUT; `docs/re/specs/INPUT/_static-init.md`).
//! See `docs/arch/anim-input-port.md`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::pad::{CONTROLS_MANAGER, PADS, PAD_SIZE};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CControllerConfigManager::CControllerConfigManager()` (0x531EE0)
    pub fn ccm_constructor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x531EE0 => unported;
}

/// `CPad::CPad` (0x541D80), `CPad::~CPad` (0x53ED60).
const PAD_CTOR: u32 = 0x541D80;
const PAD_DTOR: u32 = 0x53ED60;

/// `__init_CPad__Pads` (0x84E1F0): constructs `CPad::Pads[2]` and registers [`dtor_pads`].
#[hook(addr = 0x84E1F0, conv = cdecl, name = "__init_CPad__Pads", expect = "68 60 ED 53 00")]
pub fn init_pads() {
    eh_vector_ctor(GPtr::from_addr(PADS), PAD_SIZE, 2, PAD_CTOR, PAD_DTOR);
    atexit(0x856460);
}

/// `__dtor_CPad__Pads` (0x856460): destroys `CPad::Pads[2]` (empty element destructor).
#[hook(addr = 0x856460, conv = cdecl, name = "__dtor_CPad__Pads", expect = "68 60 ED 53 00")]
pub fn dtor_pads() {
    eh_vector_dtor(GPtr::from_addr(PADS), PAD_SIZE, 2, PAD_DTOR);
}

/// `__init_ControlsManager` (0x84D920): `ControlsManager.CControllerConfigManager()` (tail call; no
/// `atexit`). The tail call leaves the constructor's `this` in EAX.
#[hook(addr = 0x84D920, conv = cdecl, name = "__init_ControlsManager", expect = "B9 98 01 B7 00")]
pub fn init_controls_manager() -> GPtr<u8> {
    ccm_constructor(GPtr::from_addr(CONTROLS_MANAGER))
}

pub const HOOKS: &[HookReg] = &[HOOK_INIT_PADS, HOOK_DTOR_PADS, HOOK_INIT_CONTROLS_MANAGER];
