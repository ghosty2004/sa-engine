# CTaskComplexDragPedFromCar (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x6404C0 CTaskComplexDragPedFromCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 703 (0x2BF) (0x6404C0).

## 0x643CE0 CTaskComplexDragPedFromCar::`scalar deleting destructor'  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6404D0 CTaskComplexDragPedFromCar::~CTaskComplexDragPedFromCar (0x643CE3); if `flags & 1` (0x643CE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x643CF0); returns this (0x643CF8).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x643950 CTaskComplexDragPedFromCar::Clone  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x54)` 0x61A5A0 (cdecl, task pool, 0x64396B); if p == 0 return 0 (0x643981); else `CTaskComplexDragPedFromCar::CTaskComplexDragPedFromCar` 0x640430 on p with args (this+0x50, this+0x20) (0x64398D) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x6404D0 CTaskComplexDragPedFromCar::~CTaskComplexDragPedFromCar  (catalogue: specified/REA, port level 3)
- Complete dtor, SEH: vtbl = 0x86EB6C (0x6404ED); if +0x50 (dragged ped) `CEntity::CleanUpOldReference(p, &+0x50)` 0x571A00; then `CTaskComplexEnterCar::~CTaskComplexEnterCar`
  0x63DFA0 (base, 0x640515).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x640430 CTaskComplexDragPedFromCar::CTaskComplexDragPedFromCar  (catalogue: specified/REA, port level 3)
- `__thiscall (this, CPed* draggedPed [esp+4], int32 arg2 [esp+8])`, `ret 8`, SEH: `CTaskComplexEnterCar::Constructor(this, nullptr, 0, 0, 1, 0)` 0x63A220 (pushes 0, 1, 0, 0,
  0 at 0x640447-0x640451); vtbl = 0x86EB6C; +0x50 = draggedPed; if non-null `RegisterReference(draggedPed, &+0x50)` 0x571B70; +0x20 = arg2 (both paths, 0x640481 /
  0x64049C); returns this.
