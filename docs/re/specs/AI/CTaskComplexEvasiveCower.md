# CTaskComplexEvasiveCower (AI) - RE-SPEC-AI-2 group J2

Vtable 0x86F3BC, size 0x1C (Clone allocates 0x1C). Layout: +0xC CVector (direction of the threat; used negated for the heading), +0x18 CEntity*
(registered ref).

## 0x655460 CTaskComplexEvasiveCower::CTaskComplexEvasiveCower  (catalogue: specified/REA, port level 1)
- `__thiscall (this, CEntity* entity [esp+4], const CVector* dir [esp+8])`, `ret 8`, SEH frame, returns this.
- CTaskComplex::CTaskComplex 0x61A3B0; vtbl = 0x86F3BC; +0xC = *dir (3 dwords); +0x18 = entity; if entity `entity->RegisterReference(&+0x18)` 0x571B70.

## 0x6554D0 CTaskComplexEvasiveCower::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x200` (0x6554D0); `ret` -> returns 512 (TASK_COMPLEX_EVASIVE_COWER). Vtable: slot 4 (+0x10) of vtable 0x86F3BC.

## 0x6554E0 CTaskComplexEvasiveCower::~CTaskComplexEvasiveCower  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`, SEH: vtbl = 0x86F3BC; if +0x18 `CleanUpOldReference(&+0x18)` 0x571A00; CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x655570 CTaskComplexEvasiveCower::CreateSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`, SEH frame. Entry is obfuscated (`jmp 0x403CC5`: computes `[0x15607AC] - [0x58FD50] - 1`
  into the pushed slot with ECX/flags preserved, i.e. an obfuscated `push <SEH handler>`, then returns to 0x655577) - no behavioural effect.
- 0x19C (TASK_SIMPLE_COWER): `p = new(0x20)`; if p: `CTaskSimpleRunAnim(0 /*group*/, 0x8F /*anim*/, 4.0f, 0x19C, "Cower" (0x85A0F8), 0)` 0x61A900
  then vtbl = 0x85A0D0 (CTaskSimpleCower) (0x65565F-0x655682). Returns p (0 if allocation failed).
- 0x386 (TASK_SIMPLE_ACHIEVE_HEADING): `p = new(0x18)`; if p: `h = CGeneral::GetRadianAngleBetweenPoints(-(+0xC), -(+0x10), 0, 0)` 0x53CBE0
  (x1 = -dir.x, y1 = -dir.y negated with `fchs`, 0x6555DC-0x6555F4); `CTaskSimpleAchieveHeading(h, 2.0f, 0.2 [0x86FC80])` 0x667E20. Returns p or 0.
- Any other type (incl. 0x516 TASK_FINISHED): returns 0 (0x6555AB).

## 0x6556A0 CTaskComplexEvasiveCower::CreateNextSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: `t = m_pSubTask->GetTaskType()` (vtbl +0x10, no null check). t == 0x19C -> return
  `CreateSubTask(0x516)` (0x6556CD; returns 0); t == 0x386 -> return `CreateSubTask(0x19C)` (0x6556BC); else return 0.

## 0x6556F0 CTaskComplexEvasiveCower::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4` (via the target): overwrites the ped argument slot with 0x386 (0x6556F0) and tail-jumps to
  CreateSubTask 0x655570, i.e. returns `CreateSubTask(TASK_SIMPLE_ACHIEVE_HEADING)`.

## 0x655700 CTaskComplexEvasiveCower::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x655700); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86F3BC.

## 0x655CD0 CTaskComplexEvasiveCower::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = new(0x1C)`; if p: `CTaskComplexEvasiveCower(+0x18, &+0xC)` 0x655460; returns p or 0.

## 0x6577D0 CTaskComplexEvasiveCower::~CTaskComplexEvasiveCower  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexEvasiveCower::~CTaskComplexEvasiveCower 0x6554E0 (0x6577D3); if `flags & 1` (0x6577D8) calls CTask::operator delete 0x61A5B0 (this) (0x6577E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F3BC.
