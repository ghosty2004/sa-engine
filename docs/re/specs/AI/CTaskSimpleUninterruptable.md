# CTaskSimpleUninterruptable (AI) - RE-SPEC-AI-2 group J3

## 0x43E330 CTaskSimpleUninterruptable::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = CTask::operator new(8)` 0x61A5A0; if p: CTaskSimple::CTaskSimple 0x61A390, vtbl = 0x859580; returns p (0 if allocation failed).

## 0x43E380 CTaskSimpleUninterruptable::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xC9` (0x43E380); `ret` -> returns 201 (TASK_SIMPLE_UNINTERRUPTABLE). Vtable: slot 4 (+0x10) of vtable 0x859580 (CTaskSimpleUninterruptable).

## 0x43E390 CTaskSimpleUninterruptable::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed*, int32 priority [esp+8], const CEvent*)`, `ret 0xC`: returns AL = (priority == 2) (`sete`, 0x43E395); upper EAX bits undefined.

## 0x43E3A0 CTaskSimpleUninterruptable::ProcessPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x43E3A0; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 7 (+0x1C) of vtable 0x859580 (CTaskSimpleUninterruptable).

## 0x43E3B0 CTaskSimpleUninterruptable::~CTaskSimpleUninterruptable  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0043e3d0 0x43E3D0 (0x43E3B3); if `flags & 1` (0x43E3B8) calls CTask::operator delete 0x61A5B0 (this) (0x43E3C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x859580 (CTaskSimpleUninterruptable).
