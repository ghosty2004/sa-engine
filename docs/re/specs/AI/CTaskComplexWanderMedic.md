# CTaskComplexWanderMedic (AI) - RE-SPEC-AI-2 group J3

## 0x658810 CTaskComplexWanderMedic::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2` (0x658810); `ret` -> returns 2. Vtable: slot 11 (+0x2C) of vtable 0x86F48C.

## 0x658820 CTaskComplexWanderMedic::ScanForStuff  (catalogue: specified/REA, port level 0)
- Empty body: `ret 4` (0x658820). Vtable: slot 12 (+0x30) of vtable 0x86F48C.

## 0x658830 CTaskComplexWanderMedic::~CTaskComplexWanderMedic  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00658850 0x658850 (0x658833); if `flags & 1` (0x658838) calls CTask::operator delete 0x61A5B0 (this) (0x658840, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F48C.

## 0x658770 CTaskComplexWanderMedic::CTaskComplexWanderMedic  (catalogue: specified/REA, port level 2)
- `__thiscall this (int32 moveState [esp+4], uint8 dir [esp+8], bool wanderRandomly [esp+0xC])`, `ret 0xC`.
- Entry `jmp 0x406A25`: obfuscated load (`clc; mov eax,[0x156082C]; sbb eax,[0x65BD2D]` = 0x86FCB0; `mov eax,[eax]`) -> EAX = the
  dword at 0x86FCB0 (0.5f); `jmp 0x658775`. Then CTaskComplexWander::CTaskComplexWander(moveState, dir, wanderRandomly, 0.5f (that
  dword)) 0x66F450; vtbl = 0x86F48C; returns this.

## 0x6587A0 CTaskComplexWanderMedic::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x28) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexWander::CTaskComplexWander(this+0xC, (u8)this+0x10, this+0x24 & 1, [0x86FCB0]=0.5f raw) 0x66F450; vtbl = 0x86F48C (CTaskComplexWanderMedic); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x658850 CTaskComplexWanderMedic::~CTaskComplexWanderMedic  (catalogue: unknown/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86F48C` (0x658850); tail-jump to base dtor thunk_FUN_0040387c 0x669D20 (0x658856). No other member cleanup.
