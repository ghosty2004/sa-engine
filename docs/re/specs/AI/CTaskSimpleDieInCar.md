# CTaskSimpleDieInCar (AI) - RE-SPEC-AI-2 group J3

## 0x62FC60 CTaskSimpleDieInCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD6` (0x62FC60); `ret` -> returns 214 (TASK_SIMPLE_DIE_IN_CAR). Vtable: slot 4 (+0x10) of vtable 0x86DDE0.

## 0x6375B0 CTaskSimpleDieInCar::~CTaskSimpleDieInCar  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0062fc70 0x62FC70 (0x6375B3); if `flags & 1` (0x6375B8) calls CTask::operator delete 0x61A5B0 (this) (0x6375C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DDE0.

## 0x6398F0 CTaskSimpleDieInCar::ProcessPed  (catalogue: specified/REA, port level 11)
- Tail-jump thunk (0x6398F0): `jmp CTaskSimpleDie::ProcessPed 0x6397E0` with all registers and stack args unchanged (same convention as the target). Vtable: slot 7 (+0x1C) of vtable 0x86DDE0.

## 0x62FC20 CTaskSimpleDieInCar::CTaskSimpleDieInCar  (catalogue: specified/REA, port level 1)
- `__thiscall this (AssocGroupId animGroup [esp+4], AnimationId animId [esp+8])`, `ret 8`, returns this.
- CTaskSimple::CTaskSimple() 0x61A390; +0xC = animId; +8 = animGroup; +0x10 = 0; +0x14 = 0; +0x1C = 0; +0x24 = 0; +0x18 = 4.0f
  (imm 0x40800000, blend delta); byte +0x20 &= 0xFC (bits 0-1 cleared, rest left as found); vtbl = 0x86DDE0. (Inline CTaskSimpleDie
  init, same field set as Clone 0x635EF0.)

## 0x635EF0 CTaskSimpleDieInCar::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x28) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else reads a = this+0xC, g = this+8 first; p->CTaskSimple::CTaskSimple() 0x61A390; p+0xC = a; p+8 = g; p+0x20 (u8) &= 0xFC (rest uninitialised); p+0x10 = 0; p+0x14 = 0; p+0x1C = 0; p+0x24 = 0; p+0x18 = 4.0f; vtbl = 0x86DDE0 (CTaskSimpleDieInCar); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x62FC70 CTaskSimpleDieInCar::~CTaskSimpleDieInCar  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DDE0` (0x62FC70); tail-jump to base dtor CTaskSimpleDie::~CTaskSimpleDie 0x62FB40 (0x62FC76). No other member cleanup.
