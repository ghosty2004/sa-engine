# CTaskComplexCarDriveMissionFleeScene (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B89C0 CTaskComplexCarDriveMissionFleeScene::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 726 (0x2D6) (0x4B89C0).

## 0x4B89D0 CTaskComplexCarDriveMissionFleeScene::~CTaskComplexCarDriveMissionFleeScene  (catalogue: specified/REA, port level 5)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B89F0 CTaskComplexCarDriveMissionFleeScene::~CTaskComplexCarDriveMissionFleeScene (0x4B89D3); if `flags & 1` (0x4B89D8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x4B89E0); returns this (0x4B89E8).

## 0x4B89F0 CTaskComplexCarDriveMissionFleeScene::~CTaskComplexCarDriveMissionFleeScene  (catalogue: specified/REA, port level 4)
- Complete destructor, `__thiscall void ()`: `this->vtbl = 0x85B95C` (0x4B89F0); tail-jump 0x63CCC0 CTaskComplexCarDriveMission::Destructor (0x4B89F6).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B8920 CTaskComplexCarDriveMissionFleeScene::CTaskComplexCarDriveMissionFleeScene  (catalogue: specified/REA, port level 4)
- `__thiscall (this, CVehicle* vehicle [esp+4])`, `ret 4`: `CTaskComplexCarDriveMission::Constructor(this, vehicle, 0 /*target*/, 1 /*mission*/, 2 /*driving style*/, 40.0f
  (immediate 0x42200000) /*cruise speed*/)` 0x63CC30 (0x4B8925-0x4B8933); vtbl = 0x85B95C; returns this.

## 0x4B8950 CTaskComplexCarDriveMissionFleeScene::Clone  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x2C)` 0x61A5A0; null -> 0; else `CTaskComplexCarDriveMission::Constructor(p, this+0xC vehicle, 0, 1, 2, 40.0f)`
  0x63CC30 (0x4B8984-0x4B8994), `p->vtbl = 0x85B95C`; return p.
