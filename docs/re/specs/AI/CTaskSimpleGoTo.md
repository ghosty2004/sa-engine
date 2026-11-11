# CTaskSimpleGoTo (AI) - RE-SPEC-AI-2 group J5

## 0x667A00 CTaskSimpleGoTo::~CTaskSimpleGoTo  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86FD1C` (CTaskSimpleGoTo) (0x667A00); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x667A06). No other member cleanup.

## 0x66D6D0 CTaskSimpleGoTo::~CTaskSimpleGoTo  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleGoTo::~CTaskSimpleGoTo 0x667A00 (0x66D6D3); if `flags & 1` (0x66D6D8) calls CTask::operator delete 0x61A5B0 (this) (0x66D6E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FD1C (CTaskSimpleGoTo).
