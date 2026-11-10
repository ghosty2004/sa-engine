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
