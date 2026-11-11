# CTaskComplexArrestPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x68B9F0 CTaskComplexArrestPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 1101 (0x44D) (0x68B9F0).

## 0x68CF60 CTaskComplexArrestPed::~CTaskComplexArrestPed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x68BA00 CTaskComplexArrestPed::Destructor (0x68CF63); if `flags & 1` (0x68CF68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x68CF70); returns this (0x68CF78).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x68CD70 CTaskComplexArrestPed::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x24)` 0x61A5A0 (cdecl, task pool, 0x68CD8B); if p == 0 return 0 (0x68CDA1); else `CTaskComplexArrestPed::Constructor` 0x68B990 on p with args (this+0x10) (0x68CDA9) and return its EAX (the ctor returns p).
