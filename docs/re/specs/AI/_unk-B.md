# AI: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x5DEA00 CEventSoundQuiet::~CEventSoundQuiet (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: stores vtable 0x86C2B0 (data-tables: vtable-gr:CEventSoundQuiet); called by its scalar deleting destructor 0x5DEAB0 and inline-destroyed in CPed::PlayFootSteps, CCollisionEventScanner etc. (20).
- Convention: `__thiscall void (this = ECX)`; SEH frame handler 0x83D808.
- Behaviour: `vtbl = 0x86C2B0; if (this->m_entity /* +0x14 */) CEntity::CleanUpOldReference(m_entity, &this->m_entity)` (0x571A00, thiscall on the entity with the slot address pushed); then `CEventEditableResponse::~CEventEditableResponse(this)` (0x4AC480). m_entity is not zeroed.
- gta-reversed: matches gr `~CEventSoundQuiet` (CleanUpOldReference). Open questions: none.

## 0x5F6450 CPedGroupDefaultTaskAllocatorFollowAnyMeans::CPedGroupDefaultTaskAllocatorFollowAnyMeans (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_FollowAnyMeansAllocator (static object ctor); stores vtable 0x86C75C whose slot 1 returns 0 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C75C`. Open questions: none.

## 0x5F6480 CPedGroupDefaultTaskAllocatorFollowLimited::CPedGroupDefaultTaskAllocatorFollowLimited (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_FollowLimitedAllocator (static object ctor); stores vtable 0x86C764 whose slot 1 returns 1 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C764`. Open questions: none.

## 0x5F64B0 CPedGroupDefaultTaskAllocatorStandStill::CPedGroupDefaultTaskAllocatorStandStill (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_StandStillAllocator (static object ctor); stores vtable 0x86C76C whose slot 1 returns 2 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C76C`. Open questions: none.

## 0x5F64E0 CPedGroupDefaultTaskAllocatorChat::CPedGroupDefaultTaskAllocatorChat (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_ChatAllocator (static object ctor); stores vtable 0x86C774 whose slot 1 returns 3 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C774`. Open questions: none.

## 0x5F6510 CPedGroupDefaultTaskAllocatorRandom::CPedGroupDefaultTaskAllocatorRandom (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_RandomAllocator (static object ctor); stores vtable 0x86C77C whose slot 1 returns 5 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C77C`. Open questions: none.

## 0x5F6540 CPedGroupDefaultTaskAllocatorSitInLeaderCar::CPedGroupDefaultTaskAllocatorSitInLeaderCar (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CPedGroupDefaultTaskAllocators__ms_SitInLeaderCarAllocator (static object ctor); stores vtable 0x86C784 whose slot 1 returns 4 (allocator type).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `*(void**)this = 0x86C784`. Open questions: none.

## 0x600180 CAttractorScanner::sub_600180 (proposed: pick closest found attractor)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller the attractor scan 0x6060A0; reads three parallel 10-element arrays of the scanner (pointers A at +0x18..+0x3C, pointers B at +0x40..+0x64, f32 distances D at +0x68..+0x8C) and returns the best (A, B) pair.
- Convention: `__thiscall void (Scanner* this = ECX, void** outB [esp+4], void** outA [esp+8])`, `ret 8`.
- Constants: `3.4028235e+38f [0x863A3C]` (FLT_MAX).
- Behaviour: `*outB = 0; *outA = 0;` `if (A[4]) { *outB = B[4]; *outA = A[4]; return; }` (slot 4 has absolute priority). Else `best = FLT_MAX;` slot 0: taken if `D[0] < FLT_MAX` (strictly; equal or NaN skip) and `B[0] != 0` -> `best = D[0]; *outB = B[0]; *outA = A[0]`. Slots i = 1..9 in order: taken if `best > D[i]` (fcom best vs D[i]; skipped when best < D[i], equal, or unordered) and `B[i] != 0` -> `best = D[i]; *outB = B[i]; *outA = A[i]`. Ties keep the lower index.
- Note: the condition tests B[i] (not A[i]) for non-null, and slot 4 is re-examined in the loop (A[4] is null there).
- Open questions: which array is the 2D effect and which the entity (0x6060A0 not yet specified).

## 0x6034B0 CAttractorScanner::sub_6034B0 (proposed: scan entity list for attractor 2D effects)  (catalogue: unknown/REA, port level 11)
- Name evidence: called twice by the attractor scan 0x6060A0 (CAttractorScanner::ScanForAttractorsInRange, see its row); walks a CPtrListDoubleLink of entities and hands every ped-attractor 2D effect to 0x5FFFD0.
- Convention: `__thiscall void (CAttractorScanner* this = ECX, CPtrListDoubleLink* list [esp+4], X arg [esp+8])`, `ret 8`.
- Behaviour: `allowFlag (BL) = 0; for (node = list->head; node; node = next) { e = node->item; next = node->next;` `if ((e->m_nType (byte +0x36) & 7) == 4 /*object*/) { of = dword e+0x140; allowFlag = (of >> 24) & 1; ef = dword e+0x1C; if (!(ef & 4) && !(ef & 0x40000)) continue; if (of & 0x40) continue; }` `mi = CModelInfo::ms_modelInfoPtrs[e->m_nModelIndex (+0x22)]; for (i = 0; i < mi->m_n2dfxCount (u8 +0xD); i++) { fx = mi->Get2dEffect(i) (0x4C4C70); if (fx->m_type (byte +0xC) == 3 && (!(byte fx+0x37 & 1) || allowFlag)) this->sub_5FFFD0(fx, e, arg); } }`
- Edge (original quirk): `allowFlag` is set only for objects and is NOT reset for other entity types, so a non-object entity inherits the value left by the last object processed in the same call. Open questions: names of entity flag bits 2 / 18 and object flag bits 6 / 24 (0x6034E4..0x603500).

## 0x603A30 CPedAcquaintanceScanner::sub_603A30 (proposed: "may scan" predicate)  (catalogue: unknown/REA, port level 5)
- Name evidence: only caller 0x607D80 (the acquaintance scan), which runs 0x607A90 only when this returns true.
- Convention: `__thiscall bool (Scanner* this = ECX, CPed* ped [esp+4])`, `ret 4`, result AL.
- Behaviour: `if (!ped->IsAlive()) return false` (0x5E0170). `if (ped->m_nCreatedBy (byte +0x484) != 2 || this->byte +0xC) ok = true; else { ok = (ped dword +0x46C & 0x100) && this->byte +0xD; if (CPedScriptedTaskRecord::GetStatus(ped) (0x608750) != 0 && this->byte +0xE) ok = true; if (!ok) return false; }`.
  Then `ev = ped->m_pIntelligence (+0x47C)->m_eventHandlerHistory (+0x38).GetCurrentEvent()` (0x4B8CA0); `if (ev && ev->GetEventType() /*vtbl+4*/ == 0x24 /*EVENT_ACQUAINTANCE_PED_HATE*/) { intel = ped->m_pIntelligence; if (intel->dword +0xC4 == 0) return false; if (!ev->ComputeResponseTaskOfType(ped, 0x6A4 /*TASK_SIMPLE_INFORM_RESPECTED_FRIENDS*/)) return false; if (!intel->FindRespectedFriendInInformRange()) return false; }` `return ok;`
- Open questions: field names for this+0xC/+0xD/+0xE (bools) and ped +0x46C bit 8.

## 0x6040C0 CDecision::CDecision  (catalogue: specified/REA, port level 1)
- gr Decision.cpp:17. Convention: `__thiscall CDecision* (this = ECX)`, returns this. Callers: CDecisionMaker ctor 0x4650A0, 0x4650F0 (5 sites), CDecisionMakerTypesFileLoader::LoadDefaultDecisionMaker.
- Behaviour: `SetDefault()` (0x600530, thiscall; relies on SetDefault preserving ECX: EAX = ECX after the call at 0x6040C5); return this.
- Open question: the `mov eax, ecx` after the call (0x6040C5) assumes CDecision::SetDefault returns with ECX = this; a port should just return this.

## 0x6074E0 CPedIntelligence::operator delete  (catalogue: specified/REA, port level 0)
- gr PedIntelligence.cpp:993. `__cdecl void (void* p [esp+4])`, plain `ret`. Callers: 0x5DEB90, CPed::Constructor (unwind), CPed::Destructor.
- Behaviour (inline CPool::Delete on CPools::ms_pPedIntelligencePool 0xB744C0, element size 0x294): `idx = (p - objs) / 0x294` (signed magic imul 0x634C0635, sar edx 8, add sign bit); `map[idx] |= 0x80`; `if (idx < firstFree) firstFree = idx`. Open questions: none.

## 0x607D80 CPedAcquaintanceScanner::ScanAcquaintance (proposed)  (catalogue: unknown/REA, port level 12)
- Name evidence: only caller CEventScanner::ScanForEvents 0x607E30; owns a CTaskTimer at this+0 and calls the acquaintance helpers 0x603A30 / 0x607A90 (see their class rows). Exact gr name not confirmed.
- Convention: `__thiscall void (this = ECX, CPed* ped [esp+4], A arg1 [esp+8], B arg2 [esp+0xC])`, `ret 0xC`.
- Fields: +0 u32 m_nStartTime, +4 u32 m_nInterval, +8 bool m_bStarted, +9 bool m_bStopped (CTaskTimer). Globals: interval dword 0x8D2358 (initial 500), CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Behaviour: `if (!m_bStarted) { m_nInterval = [0x8D2358]; m_nStartTime = now; m_bStarted = 1; }` (the following "still not started" exit is dead); `if (m_bStopped) { m_nStartTime = now; m_bStopped = 0; }`; `if (now < m_nStartTime + m_nInterval) return;` (unsigned); `m_nStartTime = now; m_nInterval = [0x8D2358]; m_bStarted = 1;` `if (this->sub_603A30(ped)) { a = 0; b = -1; this->sub_607A90(ped, -1, arg1, arg2, &a, &b); }` (a lives in a local, b reuses the ped argument slot).
- Open questions: the meaning of arg1/arg2 (passed through unchanged, 0x607DF6/0x607DED) and of the outputs a/b (discarded).

## 0x61A390 CTaskSimple::CTaskSimple (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: called first by 211 simple-task constructors / Clone functions (CTaskSimpleUninterruptable, CTaskSimpleLeaveGroup, ...); stores vtable 0x86D4A8 (slots: IsSimple 0x43E300 returns true).
- Convention: `__thiscall CTaskSimple* (this = ECX)`, returns this. Behaviour: `+4 m_pParentTask = 0; +0 vtbl = 0x86D4A8` (that order). The CTask base ctor is inlined (no CTask vtable store).
- gta-reversed: gr `CTaskSimple()` + `CTask()` (m_pParentTask = nullptr). Open questions: none.

## 0x61A3A0 CTaskSimple::~CTaskSimple (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: tail target of 234 simple-task destructors; only stores the CTask vtable 0x86D48C (inlined empty CTask dtor). Byte-identical to 0x61A350 (unreferenced copy).
- Convention: `__thiscall void (this = ECX)`. Behaviour: `vtbl = 0x86D48C`. Open questions: none.

## 0x61A3B0 CTaskComplex::CTaskComplex  (catalogue: specified/REA, port level 0)
- 228 callers (complex task ctors/Clone). Convention: `__thiscall CTaskComplex* (this = ECX)`, returns this.
- Behaviour: `+4 m_pParentTask = 0; +0 vtbl = 0x86D4CC (CTaskComplex vtable); +8 m_pSubTask = 0`. gta-reversed: gr `CTaskComplex()` equivalent. Open questions: none.

## 0x61A3D0 CTaskComplex::~CTaskComplex (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: tail target / EH unwind of 317 complex-task destructors and constructors; installs CTaskComplex vtable 0x86D4CC then CTask vtable 0x86D48C.
- Convention: `__thiscall void (this = ECX)`; SEH frame handler 0x83F0E8 (unwind calls 0x61A350 = CTask dtor body).
- Behaviour: `vtbl = 0x86D4CC; if (m_pSubTask /* +8 */) m_pSubTask->vtbl[0](1)` (virtual scalar deleting dtor with flag 1 = `delete m_pSubTask`); `m_pSubTask = 0` (unconditional, 0x61A40C); `vtbl = 0x86D48C`.
- gta-reversed: gr `~CTaskComplex() { delete m_pSubTask; }`; the explicit `m_pSubTask = 0` store is extra in the original. Open questions: none.

## 0x61D5F0 SetUpSphereColModel (static col1, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: writes only the static CColModel `col1` 0xC17824 and its static CCollisionData 0xC17854 / sphere storage 0xC17884; callers 0x6240B0 and CTaskGangHassleVehicle::ControlSubTask 0x6637C0. No symbol; name is descriptive.
- Convention: `__stdcall void (float radius [esp+4])`, `ret 4` (EAX = raw radius bits on return, not meaningful). ECX on the CColSphere::Set call = sphere pointer read from 0xC1785C.
- Globals (col1 layout = CColModel): box.min 0xC17824..0xC1782C, box.max 0xC17830..0xC17838, sphere.center 0xC1783C..0xC17844, sphere.radius 0xC17848, m_pColData 0xC17850; colData.m_nNumSpheres (u16) 0xC17854, colData.m_pSpheres 0xC1785C.
- Behaviour: `if (col1.m_pColData == 0) { col1.m_pColData = 0xC17854; colData.m_pSpheres = 0xC17884; colData.m_nNumSpheres = 1; }` (one-time init); `colData.m_pSpheres[0].Set(radius, CVector(0,0,0), 0 /*material*/, 0 /*piece*/, 0xFF /*lighting*/)` (CColSphere::Set 0x40FD10); `col1.sphere.radius = radius; col1.sphere.center = (0,0,0); col1.box.min = (-radius, -radius, -radius); col1.box.max = (radius, radius, radius)` (store order as listed; -radius via fchs).
- Open questions: original name and what reads col1 afterwards (callers pass it to collision tests).

## 0x61DAE0 CTaskSimpleFight::FinishMeleeAnimCB (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: animation finish callback registered by CTaskSimpleFight::ProcessPed 0x629920 and 0x623B10 (4 refs); clears the task's anim pointers at the gr CTaskSimpleFight offsets (+0x1C m_pAnim, +0x20 m_pIdleAnim) and sets +8 m_bIsFinished; gr TaskSimpleFight.h:87 declares `FinishMeleeAnimCB`.
- Convention: `__cdecl void (CAnimBlendAssociation* assoc [esp+4], void* data = CTaskSimpleFight* [esp+8])`.
- Behaviour: `if (task->m_pAnim == assoc) task->m_pAnim = 0; else if (task->m_pIdleAnim == assoc) task->m_pIdleAnim = 0;` `if (assoc->m_nAnimId (int16 +0x2C) == 0x2F) task->m_bIsFinished = 1;` `if (task->m_pIdleAnim == 0) { c = task->m_nLastCommand (byte +0x27); if (c == 0x10 || c == 0x11 || c == 0xF || c == 1) task->m_bIsFinished = 1; }`.
- Open questions: symbolic names of anim id 0x2F and fight commands 0xF/0x10/0x11/1 (eFightCommand in gr) for the port.

## 0x62F360 CTaskSimpleStandStill::~CTaskSimpleStandStill (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: stores 0x86DD2C (vtable-gr:CTaskSimpleStandStill) then `jmp 0x61A3A0` (CTaskSimple dtor). 37 callers (inline destruction in CreateNextSubTask of seek tasks, ...).
- Convention: `__thiscall void (this = ECX)`. Open questions: none.

## 0x63A040 CPointRoute::operator= (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: callers CTaskComplexDrivePointRoute::Constructor 0x6433E0, CTaskComplexWalkRoundCar::ComputeRouteRoundBigCar 0x656BB0, CTaskComplexWalkRoundBuildingAttempt::ComputeRoute 0x657BA0, CTaskComplexFollowPointRoute (4); copies a count followed by an array of 12-byte points = gr CPointRoute {int32 m_NumPoints; CVector m_Points[8]} assignment.
- Convention: `__thiscall void (CPointRoute* this = ECX, const CPointRoute* src [esp+4])`, `ret 4` (EAX not meaningful).
- Behaviour: `this->n = src->n`; `for (i = 0; i < this->n; i++) this->pts[i] = src->pts[i]` (3 dword copies; the bound is re-read from this each iteration; skipped if n <= 0 signed). No clamp to the array size.
- Open questions: none.

## 0x63B8C0 CTaskComplexLeaveCar::CTaskComplexLeaveCar  (catalogue: specified/REA, port level 2)
- gr TaskComplexLeaveCar.cpp:16. Convention: `__thiscall T* (this = ECX, CVehicle* veh [esp+4], int32 targetDoor [esp+8], int32 delayTime [esp+0xC], bool sensibleLeaveCar [esp+0x10], bool forceGetOut [esp+0x14])`, `ret 0x14`, returns this; SEH frame handler 0x841618 (unwind CTaskComplex dtor 0x61A3D0). 49 callers.
- Behaviour (store order): `CTaskComplex::CTaskComplex(this)`; `+0x10 = targetDoor; +0x14 = delayTime; byte +0x18 = sensibleLeaveCar`; `vtbl = 0x86E828`; `+0xC = veh`; `byte +0x19 = forceGetOut`; `byte +0x1A = 0`; `+0x1C = 0`; `byte +0x20 = 0; byte +0x21 = 0`; `+0x24 = 15`; `+0x28 = 4.0f` (0x40800000); `+0x2C = 1.0f`; `byte +0x30 = 0`; `if (veh) veh->RegisterReference(&this->m_pTargetVehicle (+0xC))`.
- gta-reversed: matches gr field order. Open questions: none.

## 0x640330 CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver (proposed)  (catalogue: unknown/REA, port level 3)
- Name evidence: stores 0x86EAAC (vtable whose GetTaskType returns 701 TASK_COMPLEX_ENTER_CAR_AS_DRIVER) then `jmp CTaskComplexEnterCar::Destructor 0x63DFA0`; only caller the deleting dtor 0x643C60.
- Convention: `__thiscall void (this = ECX)`. Open questions: none.

## 0x646FE0 CTaskSimpleCarSetPedInAsPassenger::CTaskSimpleCarSetPedInAsPassenger  (catalogue: specified/REA, port level 2)
- gr TaskSimpleCarSetPedInAsPassenger.cpp:6 (`OG constructor was at 0x646FE0`). Callers: scripts (1800-1899, 400-499), AI (5), VEH (9 total).
- Convention: `__thiscall CTaskSimpleCarSetPedInAsPassenger* (this = ECX, CVehicle* veh [esp+4], int32 targetDoor [esp+8], CTaskUtilityLineUpPedWithCar* lineUp [esp+0xC])`, `ret 0xC`, returns this. SEH frame; the first instruction (`push 0xFFFFFFFF`-style frame setup) is relocated through .HOODLUM stub 0x401BFA (computes and pushes the SEH handler address via xor of two obfuscated dwords, plain code), resuming at 0x646FE7.
- Behaviour (store order): `CTaskSimple::CTaskSimple(this)` (0x61A390); `+0x14 = targetDoor`; `vtbl = 0x86EE04` (GetTaskType 811); `byte +8 = 0` (m_bIsFinished); `+0xC = 0` (m_pAnim); `+0x10 = veh` (m_pTargetVehicle); `+0x18 = lineUp`; bytes `+0x1C, +0x1D, +0x1E = 0` (m_bWarpingInToCar, m_bNumGettingInSet, m_nNumGettingInToClear); `if (veh) veh->RegisterReference(&this->m_pTargetVehicle)` (0x571B70, ECX = veh).
- gta-reversed: same field set. Open questions: none.

## 0x6470E0 CTaskSimpleCarSetPedInAsDriver::CTaskSimpleCarSetPedInAsDriver  (catalogue: specified/REA, port level 2)
- gr TaskSimpleCarSetPedInAsDriver.cpp:5. Convention: `__thiscall T* (this = ECX, CVehicle* veh [esp+4], CTaskUtilityLineUpPedWithCar* lineUp [esp+8])`, `ret 8`, returns this; SEH frame (unwind 0x61A3A0).
- Behaviour: `CTaskSimple::CTaskSimple(this)`; `vtbl = 0x86EE28` (GetTaskType 812); `byte +8 = 0`; `+0xC = 0`; `+0x10 = veh`; `+0x14 = lineUp`; bytes `+0x18, +0x19, +0x1A = 0`; `if (veh) veh->RegisterReference(&m_pTargetVehicle)` (0x571B70; EDX = &this->+0x10 pushed, ECX still = veh from the arg load at 0x647103).
- gta-reversed: matches. Open questions: none.

## 0x647170 CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: stores 0x86EE28 (vtable with GetTaskType 812 TASK_SIMPLE_CAR_SET_PED_IN_AS_DRIVER; the ctor 0x6470E0 installs the same); 9 callers (scripts 200-299/800-899 inline task destruction, ...).
- Convention: `__thiscall void (this = ECX)`; SEH frame handler 0x842708.
- Behaviour: `vtbl = 0x86EE28; if (m_pTargetVehicle /* +0x10 */) CEntity::CleanUpOldReference(m_pTargetVehicle, &m_pTargetVehicle)` (0x571A00); `CTaskSimple::~CTaskSimple(this)` (0x61A3A0).
- gta-reversed: gr dtor `CleanUpOldReference` equivalent. Open questions: none.

## 0x647710 CTaskSimpleCarForcePedOut::CTaskSimpleCarForcePedOut (proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: installs vtable 0x86EE94 whose GetTaskType returns 815 = TASK_SIMPLE_CAR_FORCE_PED_OUT; callers CTaskComplexLeaveCar::CreateSubTask 0x641530 and the class's Clone 0x649EE0.
- Convention: `__thiscall T* (this = ECX, CVehicle* veh [esp+4], int32 door [esp+8])`, `ret 8`, returns this; SEH frame (unwind 0x61A3A0).
- Behaviour: `CTaskSimple::CTaskSimple(this)`; `vtbl = 0x86EE94`; `+8 = veh` (m_Vehicle); `+0xC = door`; `if (veh) veh->RegisterReference(&this->+8)` (0x571B70).
- gta-reversed: gr CTaskSimpleCarForcePedOut (TaskSimpleCarForcePedOut.h) same members. Open questions: none.

## 0x647790 CTaskSimpleCarForcePedOut::~CTaskSimpleCarForcePedOut (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: vtable 0x86EE94 (TASK 815); only caller the deleting dtor vt_86ee94::Destructor 0x64BF50.
- Convention: `__thiscall void (this = ECX)`; SEH frame. Behaviour: `vtbl = 0x86EE94; if (+8 veh) CEntity::CleanUpOldReference(veh, &this->+8)` (0x571A00); `CTaskSimple::~CTaskSimple(this)` (0x61A3A0). Same shape as 0x647170 with the entity at +8. Open questions: none.

## 0x6478B0 CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall T* (this = ECX, CVehicle* veh [esp+4], int32 targetDoor [esp+8], bool switchOffEngine [esp+0xC])`, `ret 0xC`, returns this. SEH frame setup relocated through .HOODLUM stub 0x401D02 (rol-decoded handler address, plain code), resume 0x6478B7. 19 callers (CCarAI::AddPoliceCarOccupants, scripts 2500-2599, AI 16).
- Behaviour: `CTaskSimple::CTaskSimple(this)`; `+0xC = targetDoor`; `vtbl = 0x86EEB8` (GetTaskType 816); `+8 = veh`; `byte +0x10 = switchOffEngine`; bytes `+0x11..+0x15 = 0` (5 single-byte stores: m_bWarpingOutOfCar, m_bFallingOutOfCar, m_bKnockedOffBike, m_nDoorFlagsToClear, m_nNumGettingInToClear in gr order); `if (veh) veh->RegisterReference(&this->+8)` (0x571B70).
- gta-reversed: gr CTaskSimpleCarSetPedOut ctor equivalent. Open questions: none.

## 0x647950 CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: vtable 0x86EEB8 (TASK 816, installed by ctor 0x6478B0); 25 callers (inline destruction in CCarAI::AddPoliceCarOccupants, scripts, AI).
- Convention: `__thiscall void (this = ECX)`; SEH frame. Behaviour: `vtbl = 0x86EEB8; if (+8 veh) CEntity::CleanUpOldReference(veh, &this->+8)`; `CTaskSimple::~CTaskSimple(this)`. Open questions: none.

## 0x667D50 CTaskSimpleGoTo::~CTaskSimpleGoTo (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: stores 0x86FD1C (vtable-gr:CTaskSimpleGoTo) then `jmp 0x61A3A0` (CTaskSimple dtor); callers CTaskSimpleGoToPoint::~CTaskSimpleGoToPoint 0x66D6F0 and CPedGroup::PlayerGaveCommand_Gather (inline destruction).
- Convention: `__thiscall void (this = ECX)`. Open questions: none.

## 0x66D440 CPatrolRoute::CPatrolRoute (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: callers __init_g_c18db8 0x852870 (static instance) and 0x674930 (CTaskComplexFollowPatrolRoute ctor, constructing its member). Layout: count + 8 records of 0x28 bytes each holding a fixed string (element ctor 0x46A380 / dtor 0x46AE30, the same pair gr uses for its anim-name strings). Exact gr type not confirmed.
- Convention: `__thiscall T* (this = ECX)`, returns this.
- Behaviour: `dword +0 = 0`; `eh_vector_constructor_iterator(this+4, 0x28, 8, ctor 0x46AE30, dtor 0x46A380)`; then 16 byte stores of 0 at +4, +0x1C, +0x2C, +0x44, +0x54, +0x6C, +0x7C, +0x94, +0xA4, +0xBC, +0xCC, +0xE4, +0xF4, +0x10C, +0x11C, +0x134 (first byte of two 0x18/0x10 sub-fields per record: e.g. two strings per record).
- Open questions: element type of the 8 records and of 0x46AE30 / 0x46A380 (0x66D443/0x66D448).

## 0x694850 CTaskComplexFollowLeaderInFormation::Offsets::Offsets (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_CTaskComplexFollowLeaderInFormation__ms_offsets 0x852F10; gr TaskComplexFollowLeaderInFormation.h:19 `struct Offsets { CVector2D Offsets[8]; CVector2D MovingOffsets[8]; float Scale; }` (ms_offsets at 0xC196E8).
- Convention: `__thiscall Offsets* (this = ECX)`, returns this.
- Constants: `0.25f [0x858C84]`, `-2.0f [0x858B18]`; immediates -1.0f/1.0f.
- Behaviour: `Scale (+0x80) = 1.0f`; with `s` = Scale reloaded from memory each time and `e = s + 0.25f`:
  - Offsets: [0] = (-1.0f, 1.0f) (immediates, not scaled), [1] = (1.0f, 1.0f) (immediates), [2] = (0, s), [3] = (-s, 0), [4] = (s, 0), [5] = (-s, -s), [6] = (0, -s), [7] = (s, -s).
  - MovingOffsets: [0] = (-s, 0), [1] = (s, 0), [2] = (0, -e), [3] = (-e, -e), [4] = (e, -e), [5] = (0, e * -2.0f), [6] = (-s, e * -2.0f), [7] = (s, e * -2.0f).
  - Some copies go through an f32 stack temp (e.g. 0x6948A3, 0x69492F); values are identical at f32 precision.
- Open questions: none.
