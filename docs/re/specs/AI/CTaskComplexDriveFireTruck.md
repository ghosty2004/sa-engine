# CTaskComplexDriveFireTruck (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x659390 CTaskComplexDriveFireTruck::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 107 (0x6B) (0x659390).

## 0x65A220 CTaskComplexDriveFireTruck::~CTaskComplexDriveFireTruck  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6593A0 CTaskComplexDriveFireTruck::~CTaskComplexDriveFireTruck (0x65A223); if `flags & 1` (0x65A228) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x65A230); returns this (0x65A238).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x6593A0 CTaskComplexDriveFireTruck::~CTaskComplexDriveFireTruck  (catalogue: specified/REA, port level 2)
- Complete dtor, SEH: vtbl = 0x86F570; `CleanUpOldReference` 0x571A00 on +0xC (vehicle) if non-null (0x6593D6), then on +0x14 if non-null (0x6593E6); then
  `CTaskComplex::~CTaskComplex` 0x61A3D0 (0x6593F5).
