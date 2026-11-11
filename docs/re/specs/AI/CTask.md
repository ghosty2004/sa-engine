# CTask (AI) - RE-SPEC-3 group I4

## 0x61A360 CTask::IsGoToTask  (catalogue: specified/REA, port level 0)
- `__cdecl bool (CTask* task [esp+4])` (static), plain `ret`, AL = 0/1. Body split by the no-CD build: 0x61A360-0x61A36C, the
  compare at 0x4011AF-0x4011B4 (a .text detour island), back at 0x61A371.
- Callers: 22 (CEventPotentialWalkIntoVehicle::AffectsPed 0x4AE420, CEventPotentialWalkIntoPed::AffectsPed 0x4AE800,
  CEventVehicleCollision::AffectsPed 0x4B2EE0, CEventPotentialWalkIntoFire::AffectsPed 0x4B6890, CEventHandler::... ).
- Behaviour: `t = task->GetTaskType()` (vtbl+0x10, 0x61A369); if `t == 0x384` (900) return 1 (0x4011AF, 0x61A371); else
  `t = task->GetTaskType()` AGAIN (second virtual call, 0x61A377); return `t == 0x39E` (926) (0x61A37A). No null check of task.
- gta-reversed: Task.cpp:26 `GetTaskType() == TASK_SIMPLE_GO_TO_POINT || GetTaskType() == TASK_SIMPLE_GO_TO_POINT_FINE`
  (two calls, same order): matches. 900 = TASK_SIMPLE_GO_TO_POINT, 926 = TASK_SIMPLE_GO_TO_POINT_FINE (gr eTaskType).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x61A660 CTask::~CTask  (catalogue: specified/REA, port level 0)
- Scalar deleting destructor of CTask (vtable 0x86D48C slot 0): `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`. `this->vtbl = 0x86D48C` (0x61A668); if flags & 1:
  task-pool free inlined, pool = CPools::ms_pTaskPool [0xB744A8]: `idx = (this - m_pObjects) >> 7` (element 0x80, arithmetic `sar`, no rounding fix-up, 0x61A67B-0x61A682),
  `m_byteMap[idx] |= 0x80`, `if (idx < m_nFirstFree) m_nFirstFree = idx` (signed, 0x61A68B-0x61A691). Returns this (EAX = ECX on entry).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x61A350 sub_61a350 (proposed: CTask::~CTask (non-deleting))  (catalogue: unknown/REA, port level 0)
- Name proposal: CTask base destructor. Evidence: only stores the CTask vtable 0x86D48C into this (0x61A350); referenced as a data value next to the CTask vtable.
- `__thiscall void (this)`: `this->vtbl = 0x86D48C`; `ret`.
