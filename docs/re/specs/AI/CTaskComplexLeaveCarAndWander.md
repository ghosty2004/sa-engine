# CTaskComplexLeaveCarAndWander (AI) - RE-SPEC-AI-2 group J2

## 0x63BE40 CTaskComplexLeaveCarAndWander::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2C3` (0x63BE40); `ret` -> returns 707 (TASK_COMPLEX_LEAVE_CAR_AND_WANDER). Vtable: slot 4 (+0x10) of vtable 0x86E880 (CTaskComplexLeaveCarAndWander).

## 0x63BEB0 CTaskComplexLeaveCarAndWander::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x63BEB0); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86E880 (CTaskComplexLeaveCarAndWander).

## 0x63DAD0 CTaskComplexLeaveCarAndWander::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x1C) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexLeaveCarAndWander::CTaskComplexLeaveCarAndWander(this+0xC, this+0x10, this+0x14, (u8)this+0x18) 0x63BDD0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x6422C0 CTaskComplexLeaveCarAndWander::CreateNextSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`.
- t = m_pSubTask (+8)->GetTaskType() (vtable +0x10). If t is neither 0xC8 (TASK_NONE) nor 0x2C0 (TASK_COMPLEX_LEAVE_CAR) return NULL.
- Else r = this->CreateSubTask(0x516 TASK_FINISHED) 0x63BEC0. Then: d = ped->m_pIntelligence (+0x47C)->+0x14 (default primary task);
  if d == NULL or d->GetTaskType() != 0x390 (TASK_COMPLEX_WANDER): w = CTaskComplexWander::GetWanderTaskByPedType(ped) 0x673D00
  (cdecl); CTaskManager::SetTask(&intelligence->+4, w, 4 (TASK_PRIMARY_DEFAULT), 0) 0x681AF0. Return r.

## 0x642340 CTaskComplexLeaveCarAndWander::CreateFirstSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, tail call. Replaces its own stack argument: [esp+4] = (ped+0x46C & 0x100) (in
  vehicle) ? 0x2C0 (704, TASK_COMPLEX_LEAVE_CAR) : 0xC8 (200, TASK_NONE), then `jmp CreateSubTask 0x63BEC0` - i.e. returns
  this->CreateSubTask(taskType) (the ped argument is overwritten; CreateSubTask takes the task id in that slot, `ret 4`).

## 0x63BEC0 CTaskComplexLeaveCarAndWander::CreateSubTask  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`, SEH frame (also reached by tail jump from
  CreateFirstSubTask 0x642340).
- taskType 0xC8 (200): p = new(8) 0x61A5A0; if p: CTaskSimple::CTaskSimple 0x61A390, vtbl = 0x859CFC (the CTaskSimpleLeaveGroup
  vtable, same as ctor 0x463540) - return p (NULL on failure).
- 0x2C0 (704): new(0x34) CTaskComplexLeaveCar(+0xC (vehicle), +0x10, +0x14, (u8)+0x18, 0) 0x63B8C0 (NULL on failure).
- 0x516 (FINISHED) and every other value: NULL.
- gta-reversed: not hooked at this address.
