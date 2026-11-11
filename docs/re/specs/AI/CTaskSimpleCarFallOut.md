# CTaskSimpleCarFallOut (AI) - RE-SPEC-AI-2 group J5

## 0x64CC40 CTaskSimpleCarFallOut::~CTaskSimpleCarFallOut  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleCarFallOut::Destructor 0x648DE0 (0x64CC43); if `flags & 1` (0x64CC48) calls CTask::operator delete 0x61A5B0 (this) (0x64CC50, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EFD0 (CTaskSimpleCarFallOut).
