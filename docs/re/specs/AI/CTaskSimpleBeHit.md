# CTaskSimpleBeHit (AI) - RE-SPEC-AI-2 group J5

## 0x625BE0 CTaskSimpleBeHit::~CTaskSimpleBeHit  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleBeHit::Destructor 0x620810 (0x625BE3); if `flags & 1` (0x625BE8) calls CTask::operator delete 0x61A5B0 (this) (0x625BF0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D844 (CTaskSimpleBeHit).
