# CTaskComplexWanderStandard (AI) - RE-SPEC-3 group I1

Vtable 0x85A200, size 0x38. Layout: CTaskComplexWander (gr TaskComplexWander.h, 0x28): +0xC m_nMoveState i32, +0x10 m_nDir u8,
+0x14 m_fTargetRadius f32, +0x18 m_LastNode, +0x1C m_NextNode (CNodeAddress), +0x20 m_nLastUpdateDirFrameCount, +0x24 flags
(bit0 m_bWanderSensibly); then +0x28 CTaskTimer m_TaskTimer {+0x28 start u32, +0x2C interval i32, +0x30 started u8, +0x31 stopped u8},
+0x34 m_nMinNextScanTime u32. Ped fields: +0xBA m_nContactSurface (u8), +0x47C intelligence (+4 task manager, +0x68 event group,
+0xE0 vehicle-scanner entity array [16], +0x130 ped-scanner entity array [16]), +0x598 m_nPedType, +0x14 matrix (forward at +0x10).
Surface helper: `SurfaceInfos_c::IsPavement(&g_surfaceInfos 0xB79538, surface)` 0x55E7F0. Helper 0x5FE9C0 (catalogue unknown,
proposed `CPedType::IsGangType(int32 pedType)`, cdecl): returns 1 for 7 <= type <= 16 (the 1 is produced by an obfuscated jump to
0x401DFA), else 0.

## 0x48E530 CTaskComplexWanderStandard::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (const this)`, plain ret, SEH. `p = CTask::operator new(0x38)`; if null return 0. Else
  `CTaskComplexWander::CTaskComplexWander(p, m_nMoveState, (u8)m_nDir, (bool)(flags & 1), 0.5f [0x86FCB0])` 0x66F450; vtbl = 0x85A200;
  +0x28 = 0, +0x2C = 0, +0x30 = 0, +0x31 = 0, +0x34 = 0 (0x48E588-0x48E59B); return p. (The inlined ctor 0x48E4F0 body.)
- gta-reversed: header inline `new CTaskComplexWanderStandard(m_nMoveState, m_nDir, m_bWanderSensibly)` - matches.

## 0x48E5D0 CTaskComplexWanderStandard::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0 (WANDER_TYPE_STANDARD). Slot 11 of the wander vtables.

## 0x48E5E0 CTaskComplexWanderStandard::`scalar deleting destructor'  (catalogue: specified/REA, port level 4)
- Slot 0 of 0x85A200: call 0x48E600 (complete dtor); `if (flags & 1) CTask::operator delete(this)` 0x61A5B0; return this; `ret 4`.

## 0x66AED0 CTaskComplexWanderStandard::WillChat  (catalogue: specified/REA, port level 3)
- `__thiscall bool (this (unused), CPed* a [esp+4], CPed* b [esp+8])`, `ret 8`.
- Returns 0 if (in this order): a.pedType == 20 (CRIMINAL), b.pedType == 20, a.pedType == 6 (COP), b.pedType == 6, `a->IsPlayer()`,
  `b->IsPlayer()` 0x5DF8F0, `IsGangType(a.pedType)`, `IsGangType(b.pedType)` 0x5FE9C0. Otherwise returns
  `CPedIntelligence::AreFriends(a, b)` 0x601D10 (cdecl) normalised to 0/1 (0x66AED6-0x66AF43). gr: plugin::Call.

## 0x66AF60 CTaskComplexWanderStandard::SetNextMinScanTime  (catalogue: specified/REA, port level 37)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Entry obfuscated (0x66AF65 `nop; jmp 0x401390` loads `eax = ped->+0x47C`
  and jumps back) - no effect.
- `a = ped->m_pIntelligence->m_TaskMgr.GetActiveTask()` 0x681720; if a and `a->GetTaskType() == this->GetTaskType()` (a's first) and
  `a->GetWanderType() == this->GetWanderType()` (vtbl+0x2C, a's first): `GetActiveTask()` again, and its `+0x34 = CTimer::m_snTimeInMilliseconds
  [0xB7CB84] + 100000` (0x66AFB5-0x66AFC1). Note: writes the OTHER ped's active wander task (this is called for chat partners too).
- gr: plugin::Call.

## 0x66AFD0 CTaskComplexWanderStandard::LookForSexyCars  (catalogue: specified/REA, port level 38)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, SEH (`sub esp,0x14`; stack CEventSexyVehicle).
- P = ped position (matrix ? +0x30 : +4). For i = 0..15 over `ped->intel->+0xE0[i]` (null entries are skipped, not a break):
  1. skip if `v == ped->m_pVehicle (+0x58C)`; skip unless `v->m_pHandlingData->+0xD8 (monetary value, u32) > 40000` (unsigned `jbe`);
     skip unless `v->m_fHealth (+0x4C0) > 500.0f [0x858B58]` (strict; NaN skips).
  2. `d = vPos - P` (x, y, z); `distSq = (dz*dz + dy*dy) + dx*dx`; skip unless `distSq < 5.0f*5.0f` (constant 0x86FCD8 = 5.0f squared
     on the x87 stack, fcompp; equal/NaN skip) (0x66B06A-0x66B09E).
  3. `dot = (dz*f.z + dy*f.y) + dx*f.x` with f = ped matrix forward (+0x10; the matrix pointer is NOT null-checked here); skip unless
     `dot > 0.0f [0x858B50]` (strict) (0x66B0A4-0x66B0C3).
  4. skip unless `CWorld::GetIsLineOfSightClear(P, vPos, true, false, false, true, false, false, false)` 0x56A490.
  5. `CEventSexyVehicle ev(v)` 0x4AF010; `ped->intel->m_eventGroup.Add(&ev, false)` 0x4AB420; `this->SetNextMinScanTime(ped)`;
     `ev.~CEventSexyVehicle()` 0x4AF0A0; return 1.
  Return 0 after the loop.
- gr: plugin::Call.

## 0x66FDA0 CTaskComplexWanderStandard::LookForChatPartners  (catalogue: specified/REA, port level 38)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, SEH (`sub esp,0x38`; two stack CEventChatPartner objects). The ped arg slot is
  reused to hold `&pedPos` (0x66FE66).
- Behaviour:
  1. Return 0 unless `IsPavement(ped->+0xBA)`; return 0 if `m_nMoveState > 4` (signed).
  2. `pl = CWorld::Players[CWorld::PlayerInFocus (u8 0xB7CD74)].m_pPed` (array 0xB7CD98, stride 0x190): if pl bInVehicle and
     `pl->m_pVehicle`: `s = (vx*vx + vy*vy) + vz*vz` of the vehicle's m_vecMoveSpeed (+0x44); if `s > 0.04f [0x863244]` return 0
     (0x66FDEA-0x66FE3C). (pl is not null-checked.)
  3. Return 0 if `CStreaming::IsVeryBusy()` 0x4076A0 or `CGameLogic::LaRiotsActiveHere()` 0x441C10.
  4. For i = 0..15 over `ped->intel->+0x130[i]` = e (null skipped); skip e unless all of, in order:
     IsPavement(e->+0xBA); `e->intel->taskMgr.GetActiveTask()` non-null and its type (fetched again) == `this->GetTaskType()`;
     `ped->intel->FindTaskByType(0x4B4 PARTNER_CHAT)` == 0 and the same for e; `ped->intel->m_eventGroup.GetEventOfType(0x13 CHAT_PARTNER)`
     0x4AB650 == 0 and the same for e; FindTaskByType(0x4BF BE_IN_COUPLE) == 0 for ped and for e; `this->WillChat(ped, e)`;
     `d = CVector::Difference(ePos, pedPos)` 0x40FE00 (thiscall out = a - b); `|d|^2 < 10.0f*10.0f` (0x406DA0, constant 0x86FCD4 = 10.0f,
     strict); `dotPed = (d.y*f.y + d.z*f.z) + d.x*f.x > 0` (ped forward, strict); `dotE = (d.y*g.y + d.z*g.z) + d.x*g.x < 0` (e forward,
     strict; e's matrix is read without a null check); `GetIsLineOfSightClear(pedPos, ePos, true, false, false, true, false, false, false)`.
  5. Found (0x67006A): `CEventChatPartner ev1(true, e)` 0x4AECD0; `ped->intel->m_eventGroup.Add(&ev1, false)`;
     `CEventChatPartner ev2(false, ped)`; `e->intel->m_eventGroup.Add(&ev2, false)`; `SetNextMinScanTime(ped)`; `SetNextMinScanTime(e)`;
     destroy ev2 then ev1 (0x4AED60); return 1.
  Return 0 after the loop.
- gr: plugin::Call.

## 0x670100 CTaskComplexWanderStandard::LookForGangMembers  (catalogue: specified/REA, port level 38)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, SEH (`sub esp,0x28`). The ped arg slot is reused to hold `ped->m_pIntelligence`.
- Behaviour:
  1. If `CPedGroups::GetPedsGroup(ped)` 0x5F7E80 != 0 return 0.
  2. `T = ComputeTargetPos(ped, &T, &m_NextNode)` (CTaskComplexWander::ComputeTargetPos 0x669F60, thiscall args (ped, CVector* out,
     CNodeAddress* node)); A = pedPos - T (kept on the x87 stack).
  3. For i = 0..15 over `intel->+0x130[i]` = e (null skipped): `B = ePos - T` stored f32; skip unless `(B.z*A.z + B.y*A.y) + B.x*A.x > 0`
     (strict, NaN skips); skip unless `(B.z*B.z + B.y*B.y) + B.x*B.x < 25.0f [0x858FE8]` (strict, from the f32 stores); `g =
     GetPedsGroup(e)`; skip unless g and `g->m_groupMembership.CountMembersExcludingLeader()` 0x5F6AA0 > 0.
  4. Found: `CEventAcquaintancePed ev(e)` 0x4AF820 then vtbl = 0x86CAC8 (CEventAcquaintancePedDislike: its GetEventType 0x5FF1C0
     returns 0x25); `ev.m_TaskId (+0xE, i16) = 0x3AC` (TASK_SMART_FLEE_ENTITY_WALKING); `ped->intel->m_eventGroup.Add(&ev, false)`;
     `SetNextMinScanTime(ped)`; vtbl = 0x86CAC8 again; `~CEventAcquaintancePed` 0x4AF890; return 1.
  Return 0 after the loop.
- gr: plugin::Call.

## 0x672600 CTaskComplexWanderStandard::ScanForStuff  (catalogue: specified/REA, port level 39)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. now = `CTimer::m_snTimeInMilliseconds` [0xB7CB84] (re-read where noted).
- If !started (+0x30): start (+0x28) = now, interval (+0x2C) = 50, started = 1. If `now < m_nMinNextScanTime` (unsigned) return.
  If !started return (dead). If stopped (+0x31): start = now, stopped = 0, re-read now. If `now >= start + interval` (unsigned,
  `cmp; sbb; inc`): interval = 50, start = now, started = 1; then `LookForGangMembers(ped) || LookForChatPartners(ped) ||
  LookForSexyCars(ped)` (short-circuit) (0x672603-0x67267E).
- gta-reversed: cpp:12 C++ (not hooked) matches the bytes (unsigned compares; gr uses uint32 fields - same).
