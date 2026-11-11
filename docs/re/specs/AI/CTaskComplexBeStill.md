# CTaskComplexBeStill (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5F66D0 CTaskComplexBeStill::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 274 (0x112) (0x5F66D0).

## 0x5F66E0 CTaskComplexBeStill::~CTaskComplexBeStill  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5F6700 CTaskComplexBeStill::~CTaskComplexBeStill (0x5F66E3); if `flags & 1` (0x5F66E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x5F66F0); returns this (0x5F66F8).

## 0x5F6700 CTaskComplexBeStill::~CTaskComplexBeStill  (catalogue: specified/REA, port level 2)
- Complete destructor, `__thiscall void ()`: `this->vtbl = 0x86C7BC` (0x5F6700); tail-jump 0x61A3D0 FUN_0061a3d0 (unnamed complete dtor: stores vtbl 0x86D4CC then 0x86D48C) (0x5F6706).

## 0x62F9F0 CTaskComplexBeStill::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 4` (one ignored stack arg): returns the dword at this+0x8 (CTaskComplex::m_pSubTask) (0x62F9F0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5F6680 CTaskComplexBeStill::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0xC)` 0x61A5A0; null -> 0; else `CTaskComplex::CTaskComplex(p)` 0x61A3B0, `p->vtbl = 0x86C7BC` (0x5F66B8); return p.

## 0x62F8D0 CTaskComplexBeStill::CreateNextSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: `t = m_pSubTask (+8)->GetTaskType()` (vtbl+0x10, no null check); if t == 0xCB or t == 0x2C5 -> return
  `this->CreateFirstSubTask(ped)` (vtbl+0x24, 0x62F8F5); else return 0.

## 0x62F910 CTaskComplexBeStill::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH (entry starts at 0x62F910 with an `fs:[0]` load; the bundle shows the body from 0x62F916).
- If ped bInVehicle (+0x46C bit 8): `new(0x60) CTaskSimpleCarDrive(ped->m_pVehicle (+0x58C), 0, 0)` 0x63C340 (0x62F936-0x62F95D) (0 if allocation failed).
- Else `new(0x20)`: `CTaskSimple::CTaskSimple` 0x61A390 (base ctor), vtbl = 0x86DD2C, +8 = 0, +0xC = 0, +0x10 = 0, +0x14 u8 = 0, +0x15 u8 = 0, +0x18 u8 = 1, +0x19 u8 = 0,
  +0x1C f32 = 8.0f (immediate 0x41000000) (0x62F993-0x62F9B6) - an inlined stand-still/pause-type simple task (vtable 0x86DD2C). Returns it (0 if allocation failed).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5F6660 CTaskComplexBeStill::CTaskComplexBeStill  (catalogue: unknown/REA, port level 1)
- `__thiscall CTaskComplexBeStill* (this)`, plain `ret`. No callers (inlined elsewhere).
- Behaviour: `CTaskComplex::CTaskComplex(this)` 0x61A3B0 (0x5F6663); `vtbl = 0x86C7BC` (0x5F6668); return this.
