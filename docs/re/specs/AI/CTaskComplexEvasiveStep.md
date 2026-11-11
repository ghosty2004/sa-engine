# CTaskComplexEvasiveStep (AI) - RE-SPEC-3 group I1

Vtable 0x86F218, task type 0x1F6 (502). Layout (gr, size 0x1C): CTaskComplex (0xC), +0xC m_Pos CVector, +0x18 m_Entity CEntity*
(registered ref). Task types used: 0x1F5 TASK_SIMPLE_EVASIVE_STEP (501), 0x386 TASK_SIMPLE_ACHIEVE_HEADING (902), 0x516 TASK_FINISHED.

## 0x6532D0 CTaskComplexEvasiveStep::CTaskComplexEvasiveStep  (catalogue: specified/REA, port level 2)
- `__thiscall this (CEntity* entity [esp+4], const CVector* pos [esp+8])`, `ret 8`. SEH prologue obfuscated (0x4074EF:
  `stc; sbb` of two dwords pushed as the handler; no game-visible effect).
- Behaviour: `CTaskComplex()` 0x61A3B0; vtbl = 0x86F218; m_Pos = *pos (3 dword copies, 0x6532FC-0x65330C); m_Entity = entity;
  if entity `CEntity::RegisterReference(entity, &m_Entity)` 0x571B70. gr cpp:7 matches.

## 0x653340 CTaskComplexEvasiveStep::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x1F6.

## 0x653350 CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep  (catalogue: specified/REA, port level 2)
- Complete dtor: vtbl = 0x86F218; if m_Entity `CleanUpOldReference(m_Entity, &m_Entity)` 0x571A00; `~CTaskComplex` 0x61A3D0. gr matches.

## 0x6533E0 CTaskComplexEvasiveStep::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`, SEH frame.
- Behaviour (compare order 0x1F5, 0x386, 0x516):
  - 0x1F5: `p = CTask::operator new(0x14)` 0x61A5A0; if p `CTaskSimpleEvasiveStep::CTaskSimpleEvasiveStep(p, m_Entity)` 0x653160;
    return p or 0 (0x6534AF-0x6534D3).
  - 0x386: `p = operator new(0x18)`; if null return 0. Else `a = CGeneral::GetRadianAngleBetweenPoints(-m_Pos.x, -m_Pos.y, 0, 0)`
    0x53CBE0 (cdecl 4 floats; the negations are `fchs` of the loaded f32, stored f32) - computed AFTER the allocation; a is stored
    to f32; `CTaskSimpleAchieveHeading::CTaskSimpleAchieveHeading(p, a, 2.0f, 0.2f [0x86FC80])` 0x667E20; return p (0x65342F-0x653482).
  - 0x516 and any other value: return 0.
- gta-reversed: cpp:19 is plugin::Call followed by dead C++ that matches the bytes (allocation-before-angle order aside: in gr
  `new` also allocates before evaluating ctor args under MSVC).

## 0x6534F0 CTaskComplexEvasiveStep::CreateNextSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. `t = m_pSubTask->GetTaskType()` (vtbl+0x10, no null check); 0x1F5 ->
  `CreateSubTask(0x516)` (= nullptr); 0x386 -> `CreateSubTask(0x1F5)`; else return 0 (0x6534F3-0x65352E). gr cpp:37 matches.

## 0x653540 CTaskComplexEvasiveStep::CreateFirstSubTask  (catalogue: specified/REA, port level 4)
- Overwrites its own stack arg (ped) with 0x386 and tail-jumps to CreateSubTask 0x6533E0 (whose `ret 4` returns):
  `return CreateSubTask(TASK_SIMPLE_ACHIEVE_HEADING)`. gr cpp:49 matches.

## 0x653550 CTaskComplexEvasiveStep::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed*)`: `return m_pSubTask` (+8), `ret 4`. Shared shape. gr matches.

## 0x655EE0 CTaskComplexEvasiveStep::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- Slot 0 of 0x86F218: call 0x653350; `if (flags & 1) CTask::operator delete(this)` 0x61A5B0; return this; `ret 4`.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x655870 CTaskComplexEvasiveStep::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x1C)` 0x61A5A0 (cdecl, task pool, 0x65588B); if p == 0 return 0 (0x6558A1); else `CTaskComplexEvasiveStep::CTaskComplexEvasiveStep` 0x6532D0 on p with args (this+0x18, &this+0xC) (0x6558AD) and return its EAX (the ctor returns p).
