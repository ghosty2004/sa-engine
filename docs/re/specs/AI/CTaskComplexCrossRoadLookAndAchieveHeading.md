# CTaskComplexCrossRoadLookAndAchieveHeading (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x631B10 CTaskComplexCrossRoadLookAndAchieveHeading::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 227 (0xE3) (0x631B10).

## 0x631B50 CTaskComplexCrossRoadLookAndAchieveHeading::~CTaskComplexCrossRoadLookAndAchieveHeading  (catalogue: specified/REA, port level 2)
- Complete destructor, `__thiscall void ()`: `this->vtbl = 0x86DFF0` (0x631B50); tail-jump 0x61A3D0 FUN_0061a3d0 (unnamed complete dtor: stores vtbl 0x86D4CC then 0x86D48C) (0x631B56).

## 0x631CB0 CTaskComplexCrossRoadLookAndAchieveHeading::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 4` (one ignored stack arg): returns the dword at this+0x8 (CTaskComplex::m_pSubTask) (0x631CB0).

## 0x637FA0 CTaskComplexCrossRoadLookAndAchieveHeading::~CTaskComplexCrossRoadLookAndAchieveHeading  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x631B50 CTaskComplexCrossRoadLookAndAchieveHeading::~CTaskComplexCrossRoadLookAndAchieveHeading (0x637FA3); if `flags & 1` (0x637FA8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637FB0); returns this (0x637FB8).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x631AE0 CTaskComplexCrossRoadLookAndAchieveHeading::CTaskComplexCrossRoadLookAndAchieveHeading  (catalogue: specified/REA, port level 1)
- `__thiscall (this, int32 durationMs [esp+4], float heading [esp+8])`, `ret 8`: `CTaskComplex::CTaskComplex(this)` 0x61A3B0; +0xC = arg1, vtbl = 0x86DFF0, +0x10 = arg2 (raw
  dword) (0x631AF0-0x631AF9); returns this.

## 0x631B20 CTaskComplexCrossRoadLookAndAchieveHeading::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`: if priority is 1 or 2 -> return
  `m_pSubTask (+8)->MakeAbortable(ped, priority, event)` (vtbl+0x18, no null check, 0x631B33-0x631B44); else AL = 0 (0x631B2E).

## 0x631C40 CTaskComplexCrossRoadLookAndAchieveHeading::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: `t = m_pSubTask->GetTaskType()` (vtbl+0x10); t == 0x190 -> `CreateSubTask(0x516 /*finished*/, ped)` 0x631B60
  (0x631C72); t == 0x386 -> `CreateSubTask(0x190, ped)` (0x631C5C); otherwise 0.

## 0x631C90 CTaskComplexCrossRoadLookAndAchieveHeading::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `CreateSubTask(0x386, ped)` 0x631B60 (ECX passed through).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x631B60 CTaskComplexCrossRoadLookAndAchieveHeading::CreateSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH (prologue from 0x631B60). Returns 0 for any type other than the two below
  (incl. 0x516) and on allocation failure.
- 0x386: `new(0x18) CTaskSimpleAchieveHeading(this+0x10 heading (raw f32), 0.5f [0x86FC7C], 0.2f [0x86FC80])` 0x667E20 (0x631BAA-0x631BD8).
- 0x190: `new(0x20) CTaskSimpleRunAnim(ped->m_nAnimGroup (+0x4D4), 4 /*anim id*/, 4.0f (immediate 0x40800000), 0)` 0x61A8B0 (0x631BEF-0x631C1F).

## 0x636500 CTaskComplexCrossRoadLookAndAchieveHeading::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x14)` 0x61A5A0; null -> 0; else `CTaskComplex::CTaskComplex(p)` 0x61A3B0, vtbl = 0x86DFF0, p+0xC = this+0xC,
  p+0x10 = this+0x10 (both read before the base ctor, 0x636536-0x636554); return p.
