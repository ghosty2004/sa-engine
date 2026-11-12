//! `CTaskComplexChat::CTaskComplexChat` (0x682CB0, `#[native]`). Ported from the 1.0 US bytes (the
//! entry's `push <handler>` sits in the protection fragment 0x407C1D). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::orig;

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTaskComplexChat` vtable
const VTABLE: u32 = 0x8706A0;

/// `CTaskComplexChat::CTaskComplexChat(bool isChatter, CPed* partner, int32 stage, int16 speechContext)` (0x682CB0,
/// thiscall, `ret 0x10`, SEH frame): `CTaskComplex` base, +0xC = isChatter, +0x14 = stage, vtable 0x8706A0, +0x10 =
/// partner, +0x18 = context (word); `RegisterReference(&+0x10)` on the partner when set; +0x1A = 0 (byte), +0x1C = 0.
/// Returns `this`.
/// Registered for phase B only (`#[native]`): the protection reads the entry's `jmp rel32` bytes (0x15625EF reads
/// 0x682CB3), so a phase A hook would need a `keycells::READERS` entry.
#[sa_core::native(addr = 0x682CB0, conv = thiscall, name = "CTaskComplexChat::Constructor")]
pub fn task_complex_chat_ctor(
    this: GPtr<u8>,
    is_chatter: u8,
    partner: GPtr<u8>,
    stage: i32,
    context: i16,
) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u8>(this, 0xC, is_chatter);
    wr::<i32>(this, 0x14, stage);
    wr::<u32>(this, 0, VTABLE);
    wr::<GPtr<u8>>(this, 0x10, partner);
    wr::<i16>(this, 0x18, context);
    if !partner.is_null() {
        entity_register_reference(partner, this.byte_add(0x10));
    }
    wr::<u8>(this, 0x1A, 0);
    wr::<u32>(this, 0x1C, 0);
    this
}
