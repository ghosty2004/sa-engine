# CTaskComplexGangJoinRespond (AI) - RE-SPEC-AI-2 group J2

## 0x661710 CTaskComplexGangJoinRespond::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4C3` (0x661710); `ret` -> returns 1219 (TASK_COMPLEX_GANG_JOIN_RESPOND). Vtable: slot 4 (+0x10) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x661790 CTaskComplexGangJoinRespond::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x661790; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x664A80 CTaskComplexGangJoinRespond::~CTaskComplexGangJoinRespond  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGangJoinRespond::Destructor 0x661720 (0x664A83); if `flags & 1` (0x664A88) calls CTask::operator delete 0x61A5B0 (this) (0x664A90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x662290 CTaskComplexGangJoinRespond::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x14) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplex::CTaskComplex() 0x61A3B0; p+0xC = this+0xC (u8); vtbl = 0x86FB5C (CTaskComplexGangJoinRespond); p+0xD = 0 (u8); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
