# CTaskSimpleGetUp (AI) - RE-SPEC-AI-2 group J3

## 0x677F70 CTaskSimpleGetUp::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xCD` (0x677F70); `ret` -> returns 205 (TASK_SIMPLE_GET_UP). Vtable: slot 4 (+0x10) of vtable 0x8703E0 (CTaskSimpleGetUp).

## 0x67C750 CTaskSimpleGetUp::~CTaskSimpleGetUp  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00677f80 0x677F80 (0x67C753); if `flags & 1` (0x67C758) calls CTask::operator delete 0x61A5B0 (this) (0x67C760, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x8703E0 (CTaskSimpleGetUp).

## 0x67C0C0 CTaskSimpleGetUp::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x10) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimple::CTaskSimple() 0x61A390; p+8 = 0 (u8); p+9 = 0 (u8); p+0xC = 0; vtbl = 0x8703E0 (CTaskSimpleGetUp); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
