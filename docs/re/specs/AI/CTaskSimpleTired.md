# CTaskSimpleTired (AI) - RE-SPEC-AI-2 group J4

## 0x630F50 CTaskSimpleTired::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xDB` (0x630F50); `ret` -> returns 219 (TASK_SIMPLE_TIRED). Vtable: slot 4 (+0x10) of vtable 0x86DED0 (CTaskSimpleTired).

## 0x637950 CTaskSimpleTired::~CTaskSimpleTired  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleTired::Destructor 0x630F60 (0x637953); if `flags & 1` (0x637958) calls CTask::operator delete 0x61A5B0 (this) (0x637960, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DED0 (CTaskSimpleTired).
