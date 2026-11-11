//! `CWeaponModelInfo` (vtable 0x85BD78; `m_weaponInfo` at +0x24, size 0x28).
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/WeaponModelInfo.cpp` is a reading aid. `SetClump`
//! hides the "gunflash" frame's first atomic (component alpha 0, object flags byte = 0) and sets its
//! geometry's first material's ambient (+0x0C) to 16.0, as in the original.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::atomic::scalar_delete;
use super::base::{f, MiPtr, VTBL_BASE_MODEL_INFO};
use crate::shared::mi;

sa_core::globals! {
    list GLOBALS;
}

/// "gunflash" (0x85A728)
pub const STR_GUNFLASH: u32 = 0x85A728;
/// `CClumpModelInfo::SetAtomicRendererCB` (0x4C5280), passed as a callback
pub const SET_ATOMIC_RENDERER_CB: u32 = 0x4C5280;
/// `CVisibilityPlugins::RenderWeaponCB` (0x733670), passed as callback data
pub const RENDER_WEAPON_CB: u32 = 0x733670;

orig! {
    /// `CClumpModelInfo::Init` (0x4C4E40)
    pub fn clump_init(this: MiPtr) = thiscall 0x4C4E40 => super::clump::init;
    /// `CClumpModelInfo::SetClump` (0x4C4F70)
    pub fn clump_set_clump(this: MiPtr, clump: GPtr<u8>) = thiscall 0x4C4F70 => super::clump::set_clump;
    /// `CClumpModelInfo::GetFrameFromName` (0x4C5400)
    pub fn get_frame_from_name(clump: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C5400 => super::clump::get_frame_from_name;
    /// `RpClumpForAllAtomics` (0x749B70)
    pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RwHelper::GetFirstObject` (0x7348C0)
    pub fn get_first_object(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `CVehicle::SetComponentAtomicAlpha` (0x6D2960)
    pub fn set_component_atomic_alpha(atomic: GPtr<u8>, alpha: i32) = cdecl 0x6D2960 => unported;
    /// `CWeaponModelInfo::~CWeaponModelInfo` (0x4C5790)
    pub fn weapon_destructor_call(this: MiPtr) = thiscall 0x4C5790 => destructor;
}

/// `CWeaponModelInfo::GetModelType` (0x4C5780): AL = 4 (unhookable).
#[sa_core::native(addr = 0x4C5780, conv = thiscall, name = "CWeaponModelInfo::GetModelType")]
pub fn get_model_type(_this: MiPtr) -> u8 {
    4
}

/// `CWeaponModelInfo::~CWeaponModelInfo` (0x4C5790, `FUN_004c5790`).
#[hook(addr = 0x4C5790, conv = thiscall, name = "CWeaponModelInfo::~CWeaponModelInfo", expect = "C7 01 9C BB 85 00", this = "CWeaponModelInfo")]
pub fn destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CWeaponModelInfo`'s scalar deleting destructor (0x4C62D0).
#[hook(addr = 0x4C62D0, conv = thiscall, name = "CWeaponModelInfo::`scalar deleting destructor'", expect = "56 8B F1 E8 B8 F4 FF FF", this = "CWeaponModelInfo")]
pub fn scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, weapon_destructor_call)
}

/// `CWeaponModelInfo::Init` (0x4C98F0): `CClumpModelInfo::Init`, `m_weaponInfo = 0`.
#[hook(addr = 0x4C98F0, conv = thiscall, name = "CWeaponModelInfo::Init", expect = "56 8B F1 E8 48 B5 FF FF", this = "CWeaponModelInfo")]
pub fn init(this: MiPtr) {
    clump_init(this);
    f::<u32>(this, 0x24).write(0);
}

/// `CWeaponModelInfo::SetClump(RpClump*)` (0x4C9910, `ret 4`).
#[hook(addr = 0x4C9910, conv = thiscall, name = "CWeaponModelInfo::SetClump", expect = "56 8B 74 24 08", this = "CWeaponModelInfo")]
pub fn set_clump(this: MiPtr, clump: GPtr<u8>) {
    clump_set_clump(this, clump);
    rp_clump_for_all_atomics(clump, SET_ATOMIC_RENDERER_CB, RENDER_WEAPON_CB);
    let frame = get_frame_from_name(clump, GPtr::from_addr(STR_GUNFLASH as usize));
    if frame.is_null() {
        return;
    }
    let atomic = get_first_object(frame);
    if atomic.is_null() {
        return;
    }
    set_component_atomic_alpha(atomic, 0);
    let geometry = atomic.byte_add(0x18).cast::<GPtr<u8>>().read();
    atomic.byte_add(2).write(0);
    let materials = geometry.byte_add(0x20).cast::<GPtr<GPtr<u8>>>().read();
    materials.read().byte_add(0x0C).cast::<u32>().write(0x4180_0000);
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_DESTRUCTOR,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_INIT,
    HOOK_SET_CLUMP,
];
