//! ANIM: `CAnimatedBuilding` (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US bytes; gta-reversed
//! `Entity/AnimatedBuilding.cpp`, `docs/re/specs/ANIM/CAnimatedBuilding.md`. See
//! `docs/arch/anim-input-port.md`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

use crate::common::{rd, wr};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CEntity::PreRender()` (0x535FA0, called directly, not through the vtable)
    pub fn entity_pre_render(e: GPtr<u8>) = thiscall 0x535FA0 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    pub fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CEntity::~CEntity()` (0x535E90)
    pub fn entity_destructor(e: GPtr<u8>) = thiscall 0x535E90 => unported;
    /// `CBuilding::operator delete` (0x4040A0)
    pub fn building_delete(p: GPtr<u8>) = cdecl 0x4040A0 => unported;
    /// `CAnimatedBuilding::~CAnimatedBuilding()` (0x537A70)
    pub fn call_destructor(e: GPtr<u8>) = thiscall 0x537A70 => destructor;
}

/// `CAnimatedBuilding::PreRender()` (0x403F00): with an RW object: `CEntity::PreRender`,
/// `UpdateRwFrame`, then tail-jump `UpdateRpHAnim`.
#[hook(addr = 0x403F00, conv = thiscall, name = "CAnimatedBuilding::PreRender", expect = "56 8B F1 8B 46 18", this = "CAnimatedBuilding")]
pub fn pre_render(this: GPtr<u8>) {
    if rd::<u32>(this, 0x18) == 0 {
        return;
    }
    entity_pre_render(this);
    entity_update_rw_frame(this);
    entity_update_rp_hanim(this);
}

/// `CAnimatedBuilding::ProcessControl()` (0x537A30): `ret`. Not hookable (too short).
#[sa_core::native(addr = 0x537A30, conv = thiscall, name = "CAnimatedBuilding::ProcessControl")]
pub fn process_control(_this: GPtr<u8>) {}

/// `CAnimatedBuilding::ProcessCollision()` (0x537A40): `m_nFlags (+0x1C) |= 0x20`
/// (`bUsesCollision`... bit 5). Not hookable: 4-byte `or` then `ret` (5 bytes with the `ret`).
#[sa_core::native(addr = 0x537A40, conv = thiscall, name = "CAnimatedBuilding::ProcessCollision")]
pub fn process_collision(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x20);
}

/// `CAnimatedBuilding::~CAnimatedBuilding` (0x537A70): `jmp CEntity::~CEntity`.
#[hook(addr = 0x537A70, conv = thiscall, name = "CAnimatedBuilding::~CAnimatedBuilding", expect = "E9 1B E4 FF FF 90", this = "CAnimatedBuilding")]
pub fn destructor(this: GPtr<u8>) {
    entity_destructor(this);
}

/// `CAnimatedBuilding::scalar deleting destructor(flags)` (0x537A50): dtor, `CBuilding::operator
/// delete` when `flags & 1`; returns this.
#[hook(addr = 0x537A50, conv = thiscall, name = "CAnimatedBuilding::`scalar deleting destructor'", expect = "56 8B F1 E8 18 00 00 00", this = "CAnimatedBuilding")]
pub fn scalar_deleting_destructor(this: GPtr<u8>, flags: u8) -> GPtr<u8> {
    call_destructor(this);
    if flags & 1 != 0 {
        building_delete(this);
    }
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_PRE_RENDER, HOOK_DESTRUCTOR, HOOK_SCALAR_DELETING_DESTRUCTOR];
