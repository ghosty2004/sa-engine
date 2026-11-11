# CTaskComplexDieInCar (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x62FCB0 CTaskComplexDieInCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 215 (0xD7) (0x62FCB0).

## 0x6375D0 CTaskComplexDieInCar::~CTaskComplexDieInCar  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x62FCF0 FUN_0062fcf0 (unnamed complete dtor: stores vtbl 0x86DE04) (0x6375D3); if `flags & 1` (0x6375D8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6375E0); returns this (0x6375E8).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x62FC80 CTaskComplexDieInCar::CTaskComplexDieInCar  (catalogue: specified/REA, port level 1)
- `__thiscall (this, int32 weaponType [esp+4])`, `ret 4`: `CTaskComplex::CTaskComplex(this)` 0x61A3B0; +0xC = weaponType; vtbl = 0x86DE04; +0x10 = 0, +0x14 = 0, +0x18 u8 = 0,
  +0x19 u8 = 0 (0x62FC8C-0x62FCA0); returns this.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x635F90 CTaskComplexDieInCar::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x1C)` 0x61A5A0; null -> 0; else inline ctor: `CTaskComplex::CTaskComplex(p)` 0x61A3B0, p+0xC = this+0xC
  (weapon type, read before the base ctor), vtbl = 0x86DE04, p+0x10 = 0, p+0x14 = 0, p+0x18 u8 = 0, p+0x19 u8 = 0 (0x635FC5-0x635FE2); return p.
