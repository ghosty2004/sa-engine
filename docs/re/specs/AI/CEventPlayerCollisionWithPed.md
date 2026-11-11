# CEventPlayerCollisionWithPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5FEE90 CEventPlayerCollisionWithPed::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 4 (0x4) (0x5FEE90).

## 0x5FEF20 CEventPlayerCollisionWithPed::~CEventPlayerCollisionWithPed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5FEE80 FUN_005fee80 (unnamed complete dtor: stores vtbl 0x86C9B0) (0x5FEF23); if `flags & 1` (0x5FEF28) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x5FEF30); returns this (0x5FEF38).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5FEEA0 CEventPlayerCollisionWithPed::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEvent::operator new(0x34)` 0x4B5620; null -> 0 (0x5FEED2). Else `CEventPedCollisionWithPed::Constructor` 0x4AC990 on p with
  args in order: ((u16)this+0xC zero-extended, (f32)this+0x10 (copied through `fld/fstp`), this+0x14 victim ped, &this+0x18, &this+0x24, (int16)this+0x30 sign-extended,
  (int16)this+0x32 sign-extended) (0x5FEED4-0x5FEEFA); then `p->vtbl = 0x86C9B0` (0x5FEEFF); return p.
