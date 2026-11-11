# CTaskComplexFallAndGetUp (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x6786F0 CTaskComplexFallAndGetUp::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 208 (0xD0) (0x6786F0).

## 0x67CB50 CTaskComplexFallAndGetUp::~CTaskComplexFallAndGetUp  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x678790 FUN_00678790 (unnamed complete dtor: stores vtbl 0x870454) (0x67CB53); if `flags & 1` (0x67CB58) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x67CB60); returns this (0x67CB68).

## 0x67C200 CTaskComplexFallAndGetUp::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x18) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplex::CTaskComplex() 0x61A3B0; p+0x10 = this+0x10; p+0x14 = this+0x14; vtbl = 0x870454 (CTaskComplexFallAndGetUp); p+0xC = this+0xC; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
