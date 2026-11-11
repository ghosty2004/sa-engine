# CTaskComplexCarDriveMissionKillPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B8AA0 CTaskComplexCarDriveMissionKillPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 729 (0x2D9) (0x4B8AA0).

## 0x4B8AB0 CTaskComplexCarDriveMissionKillPed::~CTaskComplexCarDriveMissionKillPed  (catalogue: specified/REA, port level 5)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B8AD0 CTaskComplexCarDriveMissionKillPed::~CTaskComplexCarDriveMissionKillPed (0x4B8AB3); if `flags & 1` (0x4B8AB8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x4B8AC0); returns this (0x4B8AC8).

## 0x4B8AD0 CTaskComplexCarDriveMissionKillPed::~CTaskComplexCarDriveMissionKillPed  (catalogue: specified/REA, port level 4)
- Complete destructor, `__thiscall void ()`: `this->vtbl = 0x85B994` (0x4B8AD0); tail-jump 0x63CCC0 CTaskComplexCarDriveMission::Destructor (0x4B8AD6).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B8A00 CTaskComplexCarDriveMissionKillPed::CTaskComplexCarDriveMissionKillPed  (catalogue: specified/REA, port level 4)
- `__thiscall (this, CVehicle* vehicle [esp+4], CEntity* target [esp+8])`, `ret 8`: `CTaskComplexCarDriveMission::Constructor(this, vehicle, target, 0x36 /*mission*/, 3
  /*driving style*/, 40.0f (immediate 0x42200000))` 0x63CC30 (0x4B8A05-0x4B8A18); vtbl = 0x85B994; returns this.

## 0x4B8A30 CTaskComplexCarDriveMissionKillPed::Clone  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x2C)` 0x61A5A0; null -> 0; else `CTaskComplexCarDriveMission::Constructor(p, this+0xC vehicle, this+0x24 target,
  0x36, 3, 40.0f)` 0x63CC30 (0x4B8A64-0x4B8A77), `p->vtbl = 0x85B994`; return p.
