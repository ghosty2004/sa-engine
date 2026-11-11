# CTaskComplexFollowPedFootsteps (AI) - RE-SPEC-AI-2 group J2

## 0x694EC0 CTaskComplexFollowPedFootsteps::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A8` (0x694EC0); `ret` -> returns 936 (TASK_COMPLEX_FOLLOW_PED_FOOTSTEPS). Vtable: slot 4 (+0x10) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).

## 0x694ED0 CTaskComplexFollowPedFootsteps::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`: forwards to `this->m_pSubTask` (+0x8, no null check, 0x694ED0) virtual slot +0x18 (= MakeAbortable) via tail-jump 0x694ED5 with the same stack args, so returns `m_pSubTask->MakeAbortable(ped, priority, event)`. Vtable: slot 6 (+0x18) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).

## 0x696260 CTaskComplexFollowPedFootsteps::~CTaskComplexFollowPedFootsteps  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00695dc0 0x695DC0 (0x696263); if `flags & 1` (0x696268) calls CTask::operator delete 0x61A5B0 (this) (0x696270, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870CC0 (CTaskComplexFollowPedFootsteps).

## 0x695890 CTaskComplexFollowPedFootsteps::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x20) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexFollowPedFootsteps::CTaskComplexFollowPedFootsteps(this+0xC) 0x694E20; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x694EE0 CTaskComplexFollowPedFootsteps::CreateNextSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CreateSubTask = vslot +0x2C (0x695E40). Fields: +0xC target ped, +0x18 CPointRoute* (count at +0).
- Behaviour: if `+0xC == NULL` -> `CreateSubTask(0x516, ped)`. Else by `t = m_pSubTask->GetTaskType()`:
  - `0x1F4` (TASK_SIMPLE_HIT_HEAD) -> `CreateSubTask(0xCB TASK_SIMPLE_STAND_STILL)`.
  - `0xC8` (TASK_NONE) -> `CreateSubTask(0x516)`.
  - `0xCB` or `0x384` (TASK_SIMPLE_GO_TO_POINT): if `+0x18 != NULL && +0x18->count != 0` -> `CreateSubTask(0x384)`; else `CreateSubTask(0xCB)`.
  - `0x38B` (TASK_COMPLEX_SEEK_ENTITY): `d = target position - ped position`; `q = (dz*dz + dx*dx) + dy*dy`; `fcomp [0x870C20]=1.4f; test ah, 0x41; jne`: `q > 1.4` -> `CreateSubTask(0x38B)`; else -> `CreateSubTask(0xCB)`.
  - anything else -> `CreateSubTask(0x516)`.

## 0x695000 CTaskComplexFollowPedFootsteps::CreateFirstSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Field +0xC = ped to follow. Uses vslot +0x2C (= CreateSubTask 0x695E40, thiscall (taskType, ped)).
- Behaviour: `if +0xC == NULL` -> return `this->CreateSubTask(0x516, ped)`. Else `d = target pos - ped pos` (matrix pos or placement for both), components in order x, y, z; `q = (dx*dx + dz*dz) + dy*dy` (x87 order, 0x69504C-0x69505A: after loading dx, dy, dz the stack is (dz, dy, dx); `fld st2; fmul st3` = dx*dx; `fld st1; fmul st2` = dz*dz; `faddp`; `fld st2; fmul st3` = dy*dy; `faddp`); `fcomp [0x870C20]=1.4f; test ah, 0x41; jne`: if `q > 1.4` -> `CreateSubTask(0x38B TASK_COMPLEX_SEEK_ENTITY, ped)`; else (less, equal, NaN) -> `CreateSubTask(0xCB TASK_SIMPLE_STAND_STILL, ped)`. Note the squared distance is compared with 1.4 (not 1.4^2).

## 0x695E40 CTaskComplexFollowPedFootsteps::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame. Callers: CreateNextSubTask 0x694EE0 (6), CreateFirstSubTask 0x695000 (3), ControlSubTask 0x695090, and virtual uses from CTaskComplexGangLeader::ScanForStuff 0x65E200 / CTaskComplexWanderStandard 0x66AF60 (slot +0x2C). Fields: +0xC target ped, +0x18 CPointRoute* (footsteps), +0x1C (int16) move state.
- Behaviour: if `+0xC == NULL || +0x18 == NULL` -> 0. Then (each allocation failure -> 0):
  - `0x1F4` (TASK_SIMPLE_HIT_HEAD): `new(0x10) CTaskSimpleHitHead()` 0x653060.
  - `0xCB` (TASK_SIMPLE_STAND_STILL): `new(0x20) CTaskSimpleStandStill(10000, false, false, 8.0f)` 0x62F310.
  - `0x384` (TASK_SIMPLE_GO_TO_POINT): `L = route->pts[0]` (route+4, 3 dwords); `0x5F13A0(route, 0)` (thiscall: removes point 0 by shifting the remaining points down and decrementing the count - CPointRoute::RemovePoint; done before the allocation); `new(0x30) CTaskSimpleGoToPoint((int32)(int16)+0x1C, &L, [0x870C28]=0.01f, false, false)` 0x667CD0.
  - `0x38B` (TASK_COMPLEX_SEEK_ENTITY): `r = target->bInVehicle (+0x46C & 0x100) ? [0x85BACC]=1.0f * [0x858B90]=4.0f : 1.0f` (product stored as f32); `new(0x4C) CTaskComplexSeekEntity<Standard>(target, 50000, 1000, r, [0x859E30]=2.0f, [0x859E34]=2.0f, 1, 1)` 0x46AC10.
  - `0x516` or anything else -> 0.
