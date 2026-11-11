# CTaskComplexJump (AI) - RE-SPEC-AI-2 group J2

## 0x67A050 CTaskComplexJump::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD3` (0x67A050); `ret` -> returns 211 (TASK_COMPLEX_JUMP). Vtable: slot 4 (+0x10) of vtable 0x870570 (CTaskComplexJump).

## 0x67A100 CTaskComplexJump::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x67A100); ped unused. Vtable: slot 10 (+0x28) of vtable 0x859DCC; slot 10 (+0x28) of vtable 0x870570 (CTaskComplexJump).

## 0x67D960 CTaskComplexJump::~CTaskComplexJump  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0067a060 0x67A060 (0x67D963); if `flags & 1` (0x67D968) calls CTask::operator delete 0x61A5B0 (this) (0x67D970, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870570 (CTaskComplexJump).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x67A060 CTaskComplexJump::~CTaskComplexJump  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870570` (CTaskComplexJump) (0x67A060); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x67A066). No other member cleanup.
