# CTaskSimpleFall (AI) - RE-SPEC-AI-2 group J3

## 0x678300 CTaskSimpleFall::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xCF` (0x678300); `ret` -> returns 207 (TASK_SIMPLE_FALL). Vtable: slot 4 (+0x10) of vtable 0x870430 (CTaskSimpleFall).

## 0x678310 CTaskSimpleFall::~CTaskSimpleFall  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`, SEH frame: vtbl = 0x870430 (0x67832D); if `+0x14` (anim association) != 0:
  `assoc->SetFinishCallback(CDefaultAnimCallback::DefaultAnimCB 0x4D1390, nullptr)` 0x4CEBE0 (0x678342-0x678349); then CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x67CA20 CTaskSimpleFall::~CTaskSimpleFall  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleFall::~CTaskSimpleFall 0x678310 (0x67CA23); if `flags & 1` (0x67CA28) calls CTask::operator delete 0x61A5B0 (this) (0x67CA30, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870430 (CTaskSimpleFall).

## 0x67C180 CTaskSimpleFall::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x20) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else reads a = this+0x10, d = this+0x18, b = this+0xC first; p->CTaskSimple::CTaskSimple() 0x61A390; p+0x18 = d; p+0x1C = d (this+0x18 again, NOT this+0x1C); p+0x10 = a; vtbl = 0x870430 (CTaskSimpleFall); p+8 = 0 (u8); p+0xC = b; p+0x14 = 0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
