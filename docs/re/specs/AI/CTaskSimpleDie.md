# CTaskSimpleDie (AI) - RE-SPEC-AI-2 group J3

## 0x62FA50 CTaskSimpleDie::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD4` (0x62FA50); `ret` -> returns 212 (TASK_SIMPLE_DIE). Vtable: slot 4 (+0x10) of vtable 0x86DDBC (CTaskSimpleDie).

## 0x62FB40 CTaskSimpleDie::~CTaskSimpleDie  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`, SEH frame: vtbl = 0x86DDBC (0x62FB5D); if `+0x24` (m_animAssociation) != 0:
  `assoc->SetFinishCallback(CDefaultAnimCallback::DefaultAnimCB 0x4D1390, nullptr)` 0x4CEBE0 (0x62FB72-0x62FB79); then CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x637500 CTaskSimpleDie::~CTaskSimpleDie  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleDie::~CTaskSimpleDie 0x62FB40 (0x637503); if `flags & 1` (0x637508) calls CTask::operator delete 0x61A5B0 (this) (0x637510, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DDBC (CTaskSimpleDie).
