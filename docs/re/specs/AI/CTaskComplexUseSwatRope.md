# CTaskComplexUseSwatRope (AI) - RE-SPEC-AI-2 group J2

## 0x659440 CTaskComplexUseSwatRope::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x6C` (0x659440); `ret` -> returns 108 (TASK_COMPLEX_USE_SWAT_ROPE). Vtable: slot 4 (+0x10) of vtable 0x86F59C (CTaskComplexUseSwatRope).

## 0x659450 CTaskComplexUseSwatRope::IsInterruptable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x659450; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 11 (+0x2C) of vtable 0x86F59C (CTaskComplexUseSwatRope).

## 0x659460 CTaskComplexUseSwatRope::AbortUrgently  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x659460; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 12 (+0x30) of vtable 0x86F59C (CTaskComplexUseSwatRope).

## 0x65A3C0 CTaskComplexUseSwatRope::~CTaskComplexUseSwatRope  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006594d0 0x6594D0 (0x65A3C3); if `flags & 1` (0x65A3C8) calls CTask::operator delete 0x61A5B0 (this) (0x65A3D0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F59C (CTaskComplexUseSwatRope).
