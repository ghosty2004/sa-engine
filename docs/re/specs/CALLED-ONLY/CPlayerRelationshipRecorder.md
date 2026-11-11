# CPlayerRelationshipRecorder (CALLED-ONLY) — RE-SPEC-REST

Layout (gr `PlayerRelationshipRecorder.h`): 16 entries {CPed* ped (+0), u8 value (+4)}, stride 8.

## 0x61A1D0 CPlayerRelationshipRecorder::RecordRelationshipWithPlayer(const CPed* ped)  (catalogue: specified/REA, port level 1)
- `__thiscall` (ECX = this → ESI), ped [esp+4] (EBP), `ret 4`. Caller: CPedIntelligence::Process 0x608260. Callees:
  CTaskManager::GetActiveTask 0x681720 (thiscall on ped->intelligence(+0x47C)+4), CTaskCategories::IsKillPedTask 0x6985E0 and
  IsFollowPedTask 0x698610 (cdecl (task, bool* a2, bool* a3); both outputs pre-cleared to 0 by the caller).
- Behaviour:
  1. Find the first i in 0..15 with `entry[i].ped == ped`; if found: `entry[i].ped = NULL; entry[i].value = 0` (0x61A1EE).
  2. `task = GetActiveTask()`; NULL → return.
  3. `IsKillPedTask(task, &k2, &k3)`; if k3 → step 5 with value 3.
  4. Else `IsFollowPedTask(task, &f2, &f3)`; if f3 → step 5 with value 3. Else `IsKillPedTask(task, &k2', &k3')` again (same
     task, 0x61A27A) and if k3' → step 5 with value 7; else return. (The second IsKillPedTask cannot return true when the first
     did not, so value 7 is unreachable unless the categoriser is non-deterministic.)
  5. ORIGINAL BUG: `if (entry[0].ped != NULL) { entry[0].value = v; entry[0].ped = ped; }` (0x61A253 / 0x61A28A: `cmp [esi], 0; je
     return`) — only slot 0 is considered, and only overwritten when it is OCCUPIED; an empty recorder never records anything.
     Ports keep this.
- gta-reversed: `plugin::CallMethod` only.
