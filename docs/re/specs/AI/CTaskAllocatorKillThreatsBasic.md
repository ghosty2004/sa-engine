# CTaskAllocatorKillThreatsBasic (AI) - RE-SPEC-FINAL (helper B)

## 0x69C850 CTaskAllocatorKillThreatsBasic::ComputeClosestPeds  (catalogue: specified/REA, port level 1)
- `static void __cdecl (CPedGroup* attackers [esp+4], CPedGroup* targets [esp+8], CPed** out [esp+0xC] (8 entries))`, plain `ret`; 0x108 bytes of locals (an 8x8 float matrix). `A = &attackers->m_groupMembership (+8)`, `B = &targets->m_groupMembership (+8)`; GetMember = CPedGroupMembership::GetMember 0x5F69B0.
- Behaviour:
  1. `out[0..7] = 0`; `dist[i][j] = FLT_MAX (0x7F7FFFFF)` for all 64 entries.
  2. For i = 0..7: `a = A.GetMember(i)`; if `a && CPed::IsAlive(a) && !CPed::IsPlayer(a)`: for j = 0..7: `b = B.GetMember(j)`; if `b && IsAlive(b)`: `d = aPos - bPos` (matrix pos or placement); `dist[i][j] = (dz*dz + dx*dx) + dy*dy` (f32).
  3. Greedy matching, 8 rounds: `best = [0x863A3C] (FLT_MAX)`, `bi = bj = -1`; scan i = 0..7, j = 0..7 in order (row-major); for each entry: `fcom best, dist[i][j]; test ah, 0x41; jne skip` -> take it only when `best > dist[i][j]` (strictly; ties keep the earlier entry): `best = dist[i][j]; bi = i; bj = j`. If `bi >= 0 && bj >= 0`: set row bi and column bj of dist to FLT_MAX; `out[bi] = B.GetMember(bj)`.
  4. Fallback target: `f = CPedGroupMembership::GetLeader(B)` 0x5F69A0; if `f == NULL || !IsAlive(f)`: `f` = first `B.GetMember(k)` alive for k = 0..6; if none -> return. For i = 0..7: if `A.GetMember(i) != NULL && out[i] == 0` -> `out[i] = f`.

## 0x69DA40 CTaskAllocatorKillThreatsBasic::~CTaskAllocatorKillThreatsBasic  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor (vtable slot 0), `__thiscall void* (this, u8 flags [esp+4])`, `ret 4`; returns EAX = this. Called virtually from CPedGroupIntelligence (Flush 0x5F7350, SetPrimaryTaskAllocator 0x5F7410, SetEventResponseTaskAllocator 0x5F7440, ProcessIgnorePlayerGroup 0x5F87A0, Process 0x5FC4A0).
- Behaviour: complete dtor 0x69C780 (0x69DA43); `if flags & 1`: inlined CTaskAllocator::operator delete = pool free on `[0xB744BC ms_pTaskAllocatorPool]`: `idx = (int32)(this - pool->m_pObjects (+0)) >> 5` (object size 0x20, `sar`); `pool->m_byteMap (+4)[idx] |= 0x80`; `if idx < pool->m_nFirstFree (+0xC)` (signed) -> `m_nFirstFree = idx`.
