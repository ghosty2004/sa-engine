# CTaskSimpleJetPack (AI) - RE-SPEC-AI-2 group J7

## 0x6801D0 CTaskSimpleJetPack::~CTaskSimpleJetPack  (catalogue: specified/REA, port level 12)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleJetPack::Destructor 0x67E600 (0x6801D3); if `flags & 1` (0x6801D8) calls CTask::operator delete 0x61A5B0 (this) (0x6801E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x8705C4 (CTaskSimpleJetPack).
