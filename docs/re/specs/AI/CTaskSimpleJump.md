# CTaskSimpleJump (AI) - RE-SPEC-AI-2 group J3

## 0x679AE0 CTaskSimpleJump::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD2` (0x679AE0); `ret` -> returns 210 (TASK_SIMPLE_JUMP). Vtable: slot 4 (+0x10) of vtable 0x87054C (CTaskSimpleJump).

## 0x67D570 CTaskSimpleJump::~CTaskSimpleJump  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleJump::~CTaskSimpleJump 0x679AF0 (0x67D573); if `flags & 1` (0x67D578) calls CTask::operator delete 0x61A5B0 (this) (0x67D580, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x87054C (CTaskSimpleJump).

## 0x679AF0 CTaskSimpleJump::~CTaskSimpleJump  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x87054C; if +0x28 (m_pAnim) != NULL: SetFinishCallback(0x4D1390, NULL)
  0x4CEBE0; if +0x1C (m_pClimbEntity) != NULL: CleanUpOldReference(&this->+0x1C) 0x571A00; CTaskSimple::~CTaskSimple 0x61A3A0.
