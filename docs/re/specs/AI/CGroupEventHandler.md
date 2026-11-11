# CGroupEventHandler (AI) - RE-SPEC-REST group B

## 0x5F9530 CGroupEventHandler::ComputeResponsLeaderQuitEnteringCar  (catalogue: specified/REA, port level 3)
- `__cdecl CTaskAllocator* (const CEvent* e [esp+4] unused, CPedGroup* g [esp+8], CPed* originator [esp+0xC] unused)`, plain `ret`,
  SEH frame. Always returns 0. `I = g + 0x30` (CPedGroupIntelligence), followers `m[k] = g->+0xC + 4k`, k = 0..6 (`delay base = 500*k`,
  EBX steps by 0x1F4 up to 0xDAC).
- For each non-null m[k]:
  - If `m->+0x46C & 0x100` (in vehicle) and `v = m->+0x58C` != 0: local `CTaskComplexLeaveCar t(v, 0, 500*k, true, false)` 0x63B8C0.
  - Else: `v = FindTaskByType(m->m_pIntelligence, 0x2CE /*ENTER_CAR_AS_PASSENGER_WAIT*/)` 0x600EE0 `->+0xC`; if that task is absent or
    v == 0: `v = FindTaskByType(..., 0x2BC /*ENTER_CAR_AS_PASSENGER*/)->+0xC`; if absent or 0 -> next k. `d = _ftol((f32)(rand() &
    0xFFFF) * [0x858B14]=3.0517578e-05f * [0x85AB78]=-500.0f)` (0x821B40, truncation; d in (-500, 0]); local `CTaskComplexLeaveCar t(v, 0,
    500*k - d + 250, false, false)`.
  - In both cases a local CTaskSimple with vtable 0x859CFC (CTaskSimpleLeaveGroup: CTaskSimple ctor 0x61A390, vtable store, later vtable
    store + CTaskSimple dtor 0x61A3A0) is constructed and destroyed around the call without being used (no observable effect).
  - `I->SetTask(m, &t, I + 0xC (ped/task pair array), -1, false)` 0x5F7540; `t.~CTaskComplexLeaveCar()` 0x63B970.
- gta-reversed: plugin::CallAndReturn (GroupEventHandler.cpp:468). Open questions: none.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5F8900 CGroupEventHandler::ComputeResponseLeaderEnteredCar  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl CTaskAllocator* (const CEvent& e [esp+4], CPedGroup* pg [esp+8], CPed* originator [esp+0xC] (unused))`, plain `ret`; SEH frame (handler 0x83E12D), 0x118 bytes of locals. Always returns EAX = 0 (0x5F9089). Caller: CGroupEventHandler::ComputeEventResponseTasks 0x5FC200.
- Callees: CTaskComplexSequence ctor 0x632BD0 / AddTask 0x632D10 / dtor 0x6389F0; CTask::operator new 0x61A5A0; CTaskComplexEnterCarAsPassengerWait ctor 0x63B320 / dtor 0x63B3C0; CTaskComplexUseSequence ctor 0x635450; CTaskSimpleCarDrive ctor 0x63C340; CTaskSimple ctor/dtor 0x61A390/0x61A3A0; CTaskComplex ctor/dtor 0x61A3B0/0x61A3D0; CPedGroupIntelligence::SetTask 0x5F7540; CPed::IsPlayer 0x5DF8F0.
- Layout used: pg+0xC..+0x24 = membership followers[7] (CPed*), pg+0x28 = leader (members[7]); `I = pg+0x30` = CPedGroupIntelligence: I+0xC m_PedTaskPairs[8], I+0x1EC m_DefaultPedTaskPairs[8] (CPedTaskPair stride 0x14: +0 Ped, +4 Task, +8 Slot), I+0x29C m_TaskSeqId. e+0xE = m_TaskId (int16, sign-extended), e+0x14 = vehicle (CEventLeaderEnteredCarAsDriver::m_Vehicle). Vehicle: +0x464 m_apPassengers[0] (front passenger seat), +0x488 m_nMaxPassengers (u8), +0x590 m_nVehicleType, +0x594 m_nVehicleSubType. Ped: +0x484 m_nCreatedBy (u8), flags +0x46C bit 0x20000000 = bDontDragMeOutCar, +0x474 bit 0x2000000 = bHasGroupDriveTask.
- Behaviour:
  1. `taskId = (int32)(int16)e.m_TaskId`; `seqId = I->m_TaskSeqId`; `leader = pg+0x28`; if leader == NULL -> return 0 (0x5F894F); `veh = e+0x14`; if veh == NULL -> return 0 (0x5F895E). `maxPass = veh->m_nMaxPassengers` (u8 zero-extended, 0x5F8964).
  2. Build `list[7]` (zeroed, 0x5F8981): first every non-null follower (pg+0xC.., ascending) with `m_nCreatedBy == 2` (mission ped), then every non-null follower with `m_nCreatedBy != 2` (0x5F8990-0x5F89D8).
  3. `waitFor = leader; nAssigned = 0`; for i = 0..6 (`cmp [i], 7; jl` 0x5F8F95): `m = list[i]`; if m == NULL -> next.
     - If `nAssigned >= maxPass` (signed `jge` 0x5F8A11) -> "no seat" path (step 6).
     - `front = ComputeFrontSeatFlag(m)` (inline, done identically twice at 0x5F8A4B and 0x5F8BCC): `front = 0`; count the non-null followers (7 slots); if count == 1 and `veh->m_nVehicleType != 9` (BIKE) and `veh->m_nVehicleSubType != 2` (QUAD): `front = 1`; then `d = veh->m_apPassengers[0]` (+0x464, current front-seat passenger); if d != NULL and d != m: if d is one of the 7 followers, or d == leader, or `d->bDontDragMeOutCar` -> `front = 0` (seat taken by a group member or by a ped that may not be dragged out).
     - Sequence path (step 4) if `taskId == 0x5E9` (TASK_GROUP_ENTER_CAR_AND_PERFORM_SEQUENCE) and `0 <= seqId < 0x40` (signed) and `CTaskSequences::ms_taskSequence[seqId].m_aTasks[0]` ([0xC17900 + seqId*0x40] != 0, 0x5F8A3D); otherwise step 5.
  4. Sequence path: local `seq` (CTaskComplexSequence); `seq.AddTask(new(0x24) CTaskComplexEnterCarAsPassengerWait(veh, waitFor, front, 6))` (NULL if new fails; 0x5F8AFF/0x5F8B18); `seq.AddTask(new(0x1C) CTaskComplexUseSequence(seqId))` (0x5F8B3E/0x5F8B57); `CPedGroupIntelligence::SetTask(I, m, &seq, &I->m_PedTaskPairs, -1, false)` (0x5F8B8B); destroy seq. `nAssigned++; waitFor = m` (0x5F8BC1). (A stack CTaskSimpleNone, vtable 0x859CFC, is constructed and destroyed around the call; unused.)
  5. Normal path: stack `t = CTaskComplexEnterCarAsPassengerWait(veh, waitFor, front, 6)` (0x5F8C53). Inline SetTask into `I->m_PedTaskPairs` (0x5F8C83-0x5F8D06): find the first pair (of 8) with `Ped == m`; if none -> skip; if `pair.Task != NULL`: if `pair.Task->GetTaskType() != t.GetTaskType()` (vslot +0x10 on both, pair's first) -> `old = pair.Task; pair.Task = t.Clone() (vslot +4); pair.Slot = -1; if old: old->vslot0(1)` (delete); else (same type) leave it. If `pair.Task == NULL` -> `pair.Task = t.Clone(); pair.Slot = -1`.
     Then if `!(m->+0x474 & 0x2000000)` (bHasGroupDriveTask clear, 0x5F8D2C): set it (`or`, store 0x5F8D40); local `seq2`; `seq2.AddTask(new(0x60) CTaskSimpleCarDrive(veh, NULL, true))` (args pushed 1, 0, veh; 0x5F8D78); find m in `I->m_DefaultPedTaskPairs` (I+0x1EC); if found and its Task != NULL -> `seq2.AddTask(Task->Clone())` (0x5F8DBD/0x5F8DC5); then the same inline SetTask as above but into m_DefaultPedTaskPairs with seq2 (0x5F8DCA-0x5F8E40, a second independent search); destroy seq2. Destroy t (0x5F8E6A). `nAssigned++; waitFor = m` (0x5F8E7F).
  6. No-seat path (0x5F8E8A): stack `CTaskComplexLeaveAnyCar` (CTaskComplex ctor, vtable 0x858EB0, +0xC = 0, +0x10 = 1 (u8), +0x11 = 0 (u8)) i.e. `(delay 0, sensibleLeaveCar true, forceGetOut false)`; inline SetTask into `I->m_PedTaskPairs` exactly as in step 5 (0x5F8ED8-0x5F8F4E); destroy (vtable re-stored, CTaskComplex dtor 0x61A3D0). nAssigned and waitFor unchanged.
  7. After the loop (0x5F8FA0): if `!CPed::IsPlayer(leader)` and `!(leader->bHasGroupDriveTask)`: set the flag; local `seq3`; `seq3.AddTask(new(0x60) CTaskSimpleCarDrive(veh, NULL, true))` (0x5F9005); if leader has a pair in m_DefaultPedTaskPairs with Task != NULL -> `seq3.AddTask(Task->Clone())`; `CPedGroupIntelligence::SetTask(I, leader, &seq3, &I->m_DefaultPedTaskPairs, -1, false)` (0x5F9066); destroy seq3. Return 0.
- Notes: each follower waits for the previous assigned follower (waitFor chain), the first for the leader. The default-task sequence keeps the follower's previous default task after the drive task. Allocation failures yield NULL tasks passed to AddTask (original behaviour).
- gta-reversed: `plugin::CallAndReturn` stub (GroupEventHandler.cpp:358), `.Reversed = false`; no gr code to compare.
