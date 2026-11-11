# CTaskSimpleIKChain (AI) - RE-SPEC-AI-2 group J7

## 0x636F00 CTaskSimpleIKChain::~CTaskSimpleIKChain  (catalogue: specified/REA, port level 5)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleIKChain::Destructor 0x633A90 (0x636F03); if `flags & 1` (0x636F08) calls CTask::operator delete 0x61A5B0 (this) (0x636F10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DB48 (CTaskSimpleIKChain).
