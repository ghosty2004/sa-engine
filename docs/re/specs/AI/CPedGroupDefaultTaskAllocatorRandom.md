# CPedGroupDefaultTaskAllocatorRandom (AI) - RE-SPEC-3 group I4

Vtable 0x86C77C (gr PedGroupDefaultTaskAllocatorRandom.h:18). CPedGroup (gr PedGroup.h, size 0x2D4): +0x4 m_bMembersEnterLeadersVehicle
(u8), +0x8 membership (+0x28 = members[7] = leader), +0x30 intelligence (default task pairs at group+0x21C, stride 0x14: +0 Ped, +4 Task;
leader pair at +0x2A8/+0x2AC).

## 0x5F6E90 CPedGroupDefaultTaskAllocatorRandom::AllocateDefaultTasks  (catalogue: specified/REA, port level 3)
- `__thiscall void (const this, CPedGroup* group [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH (handler 0x83DF86). `this` unused.
- Callers: virtual (slot of vtable 0x86C77C). Callees: CTask::operator new 0x61A5A0, CTaskComplexGangFollower::CTaskComplexGangFollower
  0x65EAA0, 0x65EE90 (catalogue unknown; proposed CTaskComplexGangFollower::SetEnterLeadersVehicle: `__thiscall void (bool b [esp+4])`,
  `ret 4`: `byte[this+0x3D] = (byte[this+0x3D] & ~4) | ((b << 2) & 4)`), CTaskComplexGangLeader::CTaskComplexGangLeader 0x65DED0.
- Globals: CTaskComplexFollowLeaderInFormation::ms_offsets 0xC196E8 (CVector2D[7], stride 8).
- Behaviour:
  1. Followers, i = 0..6 (0x5F6EB4-0x5F6F5E): pair = defaultPairs[i]; skip if pair.Ped == 0 or (ped != 0 and pair.Ped != ped).
     `t = operator new(0x4C)`; if t != 0: `off = { ms_offsets[i].x, ms_offsets[i].y, 0.0 }` (y loaded first, both through x87, z = 0 at
     0x5F6F05) and `CTaskComplexGangFollower(t, group, leader = group->members[7] (+0x28), (uint8)i, off (CVector by value), 10.0f
     (imm 0x41200000))` (0x5F6EEF-0x5F6F2F); `pair.Task = t` (or 0) (0x5F6F3A) - the previous Task is overwritten without deletion.
     Then `0x65EE90(t, group->m_bMembersEnterLeadersVehicle (+4) != 0)` (0x5F6F3D-0x5F6F52) - executed even when t == 0 (ECX = 0:
     writes byte [0x3D], an access violation; original bug, only reachable when the task pool is full).
  2. Leader (0x5F6F64-0x5F6F9C): pair 7 (+0x2A8 Ped, +0x2AC Task); same filter; `t = operator new(0x38)`; if t != 0
     `CTaskComplexGangLeader(t, group)` 0x65DED0; `pair.Task = t` (or 0).
- gta-reversed: `plugin::CallMethod<0x5F6E90>` (`.Reversed = false`): port from these bytes. Open questions: name of 0x65EE90 and the
  meaning of bit 2 of task byte +0x3D (gr CTaskComplexGangFollower layout) - behaviour is fully specified above.
