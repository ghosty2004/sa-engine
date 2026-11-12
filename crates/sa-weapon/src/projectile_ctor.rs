//! `CProjectile::CProjectile(int32 modelIndex)` (batch `samp-unk-1`; catalogue row UNK-game `CProjectile`), ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CObject::CObject()` (0x5A1D10)
    fn object_ctor(o: Ent) -> Ent = thiscall 0x5A1D10 => unported;
    /// `CObject::SetIsStatic(bool)` (0x5A0760, `ret 4`)
    fn object_set_is_static(o: Ent, s: u32) = thiscall 0x5A0760 => unported;
    /// `CEntity::SetModelIndex(uint32)` (0x532AE0, `ret 4`)
    fn entity_set_model_index(e: Ent, model: i32) = thiscall 0x532AE0 => unported;
}

/// `CProjectile::CProjectile(int32 modelIndex)` (0x5A4030, thiscall, `ret 4`; called by
/// `CProjectileInfo::AddProjectile`): `CObject`'s constructor; physical flags +0x1C |= 0x80000000; vtable
/// 0x867030; +0x8C and +0x90 = 1.0, +0x98 = 0x3F7FFF58 (0.99999), +0x9C = 0.75, +0xA0 = 0x3A51B718 (0.0008);
/// `SetIsStatic(false)`; entity flags +0x40 |= 0x800000; `CEntity::SetModelIndex(modelIndex)` (direct call);
/// object type +0x13C = 5. Returns `this`. The entry `push -1; jmp 0x4037B3` is a no-CD chunk that pushes the SEH
/// handler 0x83D418 and returns to 0x5A4037 (decoded from the exe image).
#[hook(addr = 0x5A4030, conv = thiscall, name = "CProjectile", expect = "6A FF E9 7C F7 E5 FF",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes,entry-protection-read", recs = "weapon-combat",
       note = "the .HOODLUM code at 0x1561116 reads the entry dword 0x5A4032 as a key (keycells reader)")]
pub fn projectile_ctor(this: Ent, model: i32) -> Ent {
    object_ctor(this);
    wu(this, 0x1C, ru(this, 0x1C) | 0x8000_0000);
    wu(this, 0, 0x86_7030);
    wu(this, 0x8C, 0x3F80_0000);
    wu(this, 0x90, 0x3F80_0000);
    wu(this, 0x98, 0x3F7F_FF58);
    wu(this, 0x9C, 0x3F40_0000);
    wu(this, 0xA0, 0x3A51_B718);
    object_set_is_static(this, 0);
    wu(this, 0x40, ru(this, 0x40) | 0x80_0000);
    entity_set_model_index(this, model);
    wb(this, 0x13C, 5);
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROJECTILE_CTOR];
