# CTaskCategories (AI) - RE-SPEC-3 group I3

Task types: 0x3FB 1019 TASK_SIMPLE_FIGHT_CTRL .. 0x3FD 1021 TASK_SIMPLE_THROW_CTRL. GetTaskType = CTask vtbl+0x10 (slot 4).

## 0x6985E0 CTaskCategories::IsKillPedTask  (catalogue: specified/REA, port level 0)
- `__cdecl void (const CTask* task [esp+4], bool* a2 [esp+8], bool* a3 [esp+0xC])`, plain `ret`. Caller: CPlayerRelationshipRecorder::
  RecordRelationshipWithPlayer 0x61A1D0 (2 sites).
- Behaviour: `*a2 = 0; *a3 = 0` (0x6985ED-0x6985F0); `t = task->GetTaskType()` (0x6985F5; no null check); if `1019 <= t <= 1021` (signed,
  0x6985F8-0x698604) `*a2 = 1`.
- gta-reversed: TaskCategories.cpp:35 - matches.

## 0x698610 CTaskCategories::IsFollowPedTask  (catalogue: specified/REA, port level 0)
- `__cdecl void (const CTask* task, bool* a2 [esp+8], bool* a3 [esp+0xC])`: `*a2 = 0; *a3 = 0` (0x698618-0x69861B); task unused. gr matches.

## 0x698680 CTaskCategories::ComputeTaskCategory  (catalogue: specified/REA, port level 0)
- `__cdecl int8 (const CTask* task [esp+4])`, plain `ret`, result in AL. Caller: CEventHandler::ComputeBuildingCollisionResponse 0x4BF2B0.
- Behaviour: `c = (task != 0)` (0x698686-0x69868C); then `task->GetTaskType()` is called THREE times (0x698692, 0x6986A9, 0x6986C0; no null
  check: a null task crashes at 0x69868E) with the same range test 1019..1021 (signed): first sets c = 4, second c = 2, third returns 4 if in range
  (0x6986CF-0x6986D1), else returns c. Net result: in range -> 4, otherwise (task != 0) i.e. 1.
- gta-reversed: TaskCategories.cpp:4 - same three calls and result (gr also derefs a null task). Matches.
