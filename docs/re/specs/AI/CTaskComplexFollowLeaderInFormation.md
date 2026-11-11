# CTaskComplexFollowLeaderInFormation (AI) - RE-SPEC-AI-2 group J2

Vtable 0x870C3C, size 0x28. Layout: +0xC CPedGroup* (not ref-counted), +0x10 leader CPed* (registered ref), +0x14 CVector offset, +0x20 i32 (= 4 at
construction, move state), +0x24 f32/i32 (4th ctor arg, copied raw).

## 0x6949A0 CTaskComplexFollowLeaderInFormation::Constructor  (catalogue: specified/REA, port level 2)
- `__thiscall (this, CPedGroup* group [esp+4], CPed* leader [+8], const CVector* offset [+0xC], f32 a4 [+0x10])`, `ret 0x10`, SEH frame; returns this.
  Entry is obfuscated (`jmp 0x4015C3`: pushes `[0x1560658] - [0x5CFF64]` with EDI/flags preserved, i.e. an obfuscated `push <SEH handler>`) - no effect.
- CTaskComplex::CTaskComplex 0x61A3B0; +0xC = group; vtbl = 0x870C3C; +0x10 = leader; +0x14 = *offset (3 dwords); +0x20 = 4; +0x24 = a4 (raw dword)
  (0x6949C4-0x694A06); if leader `leader->RegisterReference(&+0x10)` 0x571B70.

## 0x694A30 CTaskComplexFollowLeaderInFormation::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x391` (0x694A30); `ret` -> returns 913 (TASK_COMPLEX_FOLLOW_LEADER_IN_FORMATION). Vtable: slot 4 (+0x10) of vtable 0x870C3C (CTaskComplexFollowLeaderInFormation).

## 0x695740 CTaskComplexFollowLeaderInFormation::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, plain `ret`, SEH: `p = new(0x28)`; if p: `Constructor(+0xC, +0x10, &+0x14, +0x24)` 0x6949A0; returns p or 0 (+0x20 is not
  copied; the ctor resets it to 4).

## 0x695BC0 CTaskComplexFollowLeaderInFormation::~CTaskComplexFollowLeaderInFormation  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexFollowLeaderInFormation::Destructor 0x694A40 (0x695BC3); if `flags & 1` (0x695BC8) calls CTask::operator delete 0x61A5B0 (this) (0x695BD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870C3C (CTaskComplexFollowLeaderInFormation).

## 0x6968E0 CTaskComplexFollowLeaderInFormation::CreateFirstSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Calls 0x6962A0 (thiscall `CTask* (int32 taskType, CPed* ped)`, catalogue
  `thunk_FUN_00401913`; proposed name `CTaskComplexFollowLeaderInFormation::CreateSubTask`, evidence: the only callee, takes a task id
  and the ped, result returned unchanged) with:
  - ped in vehicle (ped+0x46C & 0x100) and ped+0x58C (vehicle) != NULL: if the leader (this+0x10) is in a vehicle (leader+0x46C & 0x100)
    and leader+0x58C == ped's vehicle -> 0x2C5 (709, TASK_SIMPLE_CAR_DRIVE); else 0x2C0 (704, TASK_COMPLEX_LEAVE_CAR);
  - otherwise -> 0x38B (907, TASK_COMPLEX_SEEK_ENTITY).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x695B20 CTaskComplexFollowLeaderInFormation::sub_695B20 (proposed: CTaskComplexFollowLeaderInFormation::CTaskComplexFollowLeaderInFormation)  (catalogue: unknown/REA, port level 2)
- Evidence: CTaskComplex ctor + vtable 0x870C3C (TASK_COMPLEX_FOLLOW_LEADER_IN_FORMATION). No callers (inlined elsewhere).
- `__thiscall T* (this, CPedGroup* group [esp+4], CPed* leader [esp+8], a3 [esp+0xC], a4 [esp+0x10], float a5 [esp+0x14])`, `ret 0x14`; SEH frame.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0xC = group; +0x24 = a5; vtbl = 0x870C3C; +0x10 = leader; +0x20 = 4`; `o = 0x611F20(&CTaskComplexFollowLeaderInFormation::ms_offsets 0xC196E8, a3, a4)` (thiscall, returns a pointer to a 2D offset); `+0x14 = o[0]; +0x18 = o[1]` (2 dwords); `+0x1C = 0`; `if leader` -> RegisterReference(leader, &+0x10) 0x571B70; return this.

## 0x696820 CTaskComplexFollowLeaderInFormation::CreateNextSubTask  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CreateSubTask = 0x6962A0 (thiscall (taskType, ped)).
- Behaviour: `st = leader (+0x10)->m_nPedState (+0x534)` (read first; no NULL check on the leader). `t = m_pSubTask->GetTaskType()`:
  - `t == 0x2C0` (TASK_COMPLEX_LEAVE_CAR) or `t == 0x2C5` (TASK_SIMPLE_CAR_DRIVE) or `t == 0xCB` (TASK_SIMPLE_STAND_STILL) -> `CreateSubTask(0x38B TASK_COMPLEX_SEEK_ENTITY, ped)`.
  - `t == 0x38B` -> if `st == 1 || st == 0` -> `CreateSubTask(0xCB, ped)`; else `CreateSubTask(0x38B, ped)`.
  - `t == 0xCA` (TASK_SIMPLE_PAUSE) -> `CreateSubTask(0xCA, ped)`.
  - `t == 0x2BC` (TASK_COMPLEX_ENTER_CAR_AS_PASSENGER) -> return `this->CreateFirstSubTask(ped)` (vslot +0x24).
  - anything else -> 0.
