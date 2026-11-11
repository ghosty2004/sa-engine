# AI unattributed functions, group FB - RE-SPEC-FINAL (helper B)

Unnamed / vtable-only functions of the FB assignment. Headings give the catalogue name plus `(proposed: Class::Method)`; the first bullet gives the evidence (vtable slot, callers, `this` layout). Task vtable slots: 0 scalar deleting dtor, 1 Clone, 2 GetSubTask, 3 IsSimple, 4 GetTaskType, 5 StopTimer, 6 MakeAbortable, then simple 7 ProcessPed, 8 SetPedPosition / complex 7 SetSubTask, 8 CreateNextSubTask, 9 CreateFirstSubTask, 10 ControlSubTask. Event vtable slots: 0 sdd, 1 GetEventType, 2 GetEventPriority, 3 GetLifeTime, 4 Clone, ... (see CEvent).

## 0x48E180 sub_48e180 (proposed: CPointRoute::AddPoint)  (catalogue: unknown/REA, port level 0)
- Name proposal: CPointRoute::AddPoint (notsa::Route<CVector>::push_back with capacity 8). Evidence: layout `u32 count @+0; CVector pts[8] @+4` (stride 0xC via `lea eax,[eax+eax*2]; lea edx,[ecx+eax*4+4]` 0x1569997/0x156999A) matches gr `CPointRoute : notsa::Route<CVector>` (PedGeometryAnalyser.h:17); callers CTaskComplexWalkRoundObject::CreateNextSubTask 0x657220, CTaskComplexFollowPedFootsteps::ControlSubTask 0x695090, script 1400-1499 switch 0x48EAA0.
- `__thiscall bool (this = ECX: CPointRoute*, const CVector* pt [esp+4])`, `ret 4`. The entry 0x48E180 is a `jmp` to the .HOODLUM body 0x1569990.
- Behaviour: `n = this->count` (0x1569990); `if (int32)n >= 8` (signed `jge` 0x1569995) -> return AL = 0 (`xor al, al` 0x15699BB). Else copy pt->x, pt->y, pt->z (3 dwords, raw) to `this + 4 + n*0xC` (0x15699A3-0x15699B0); `this->count++` (`inc dword ptr [ecx]` 0x15699B3); return AL = 1. Only AL is defined (EAX upper bytes = garbage from the pt pointer / n*3 computation).

## 0x48E220 sub_48e220 (proposed: CEventLeaderEnteredCarAsDriver::~CEventLeaderEnteredCarAsDriver (non-deleting))  (catalogue: unknown/REA, port level 1)
- Name proposal: CEventLeaderEnteredCarAsDriver complete dtor. Evidence: stores vtable 0x85A198 into this (0x48E23D); caller 0x48E320 CEventLeaderEnteredCarAsDriver::~CEventLeaderEnteredCarAsDriver (scalar deleting dtor) and the EH-unwind of CRunningScript::ProcessCommands1500To1599 0x490DB0 (stack-allocated event).
- `__thiscall void (this = ECX)`, plain `ret`. SEH frame (entry `push -1; jmp 0x40166A`: .HOODLUM stub that pushes the EH handler address and continues at 0x48E227).
- Behaviour: `this->vtbl = 0x85A198` (0x48E23D); `if this->+0x14 (CVehicle* m_Vehicle) != NULL` (0x48E246) -> `CEntity::CleanUpOldReference(this->+0x14, &this->+0x14)` 0x571A00 (thiscall on the entity, arg = address of the field, 0x48E256); then `CEventEditableResponse::~CEventEditableResponse(this)` 0x4AC480 (0x48E265). No return value.

## 0x4993D0 sub_4993d0 (proposed: CTaskComplexBeCop::CTaskComplexBeCop)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexBeCop ctor. Evidence: stores vtable 0x85A428 (slot 4 GetTaskType 0x499430 returns 0x450 = TASK_COMPLEX_BE_COP) into this (0x156E976); callers vt_85a428::Clone 0x499440 and CSetPiece::Update 0x499A80.
- `__thiscall CTaskComplexBeCop* (this = ECX, int32 moveState [esp+4], uint8/int32 wanderDir [esp+8], int32 arg3 [esp+0xC])`, `ret 0xC`; returns this in EAX. Entry is a `jmp` to the .HOODLUM body 0x156E930.
- Behaviour: `CTaskComplexWander::CTaskComplexWander(this, moveState, wanderDir, 1, [0x86FCB0]=0.5f)` 0x66F450 (thiscall, 4 stack args pushed as raw dwords: the 0.5f is passed as its bit pattern, 0x156E930/0x156E93A; arg 3 = immediate 1). Then (eax = 0): `+0x2C = 0; +0x30 = 0; +0x34 = 0 (u8); +0x35 = 0 (u8); +0x38 = 0; +0x3C = 0; +0x40 = 0; +0x44 = 0; +0x48 = 0; +0x4C = 0 (u8); +0x4D = 0 (u8)` (0x156E94E-0x156E96C, in that order); `+0x28 = arg3` (0x156E973); `this->vtbl = 0x85A428` (0x156E976).
- gta-reversed: no CTaskComplexBeCop class; field meanings beyond the CTaskComplexWander base (size 0x28) are unknown from this function.

## 0x4AC130 sub_4ac130 (proposed: CInformFriendsEvent::Set)  (catalogue: unknown/REA, port level 2)
- Name proposal: CInformFriendsEvent setter (catalogue: "ped setter (RegisterReference)"). Evidence: layout +0 ped / +4 event / +8 time as gr CInformFriendsEvent; same store sequence as the inlined tail of CInformFriendsEventQueue::Add 0x4AC1E0. No callers / data refs (unreferenced).
- `__thiscall void (this, CPed* ped [esp+4], CEvent* event [esp+8], int32 delayMs [esp+0xC])`, `ret 0xC`.
- Behaviour: `m_Ped = ped` (0x4AC139); `if ped` -> `CEntity::RegisterReference(ped, &m_Ped)` 0x571B70 (0x4AC13E); `m_Event = event` (0x4AC14B); `m_Time = [0xB7CB84 CTimer::m_snTimeInMilliseconds] + delayMs` (u32 add, 0x4AC14E-0x4AC156).

## 0x4AC3B0 sub_4ac3b0 (proposed: CInformGroupEvent::Set)  (catalogue: unknown/REA, port level 2)
- Name proposal: CInformGroupEvent setter (counterpart of 0x4AC130). Evidence: layout as gr CInformGroupEvent; unreferenced.
- `__thiscall void (this, CPed* ped [esp+4], CPedGroup* group [esp+8], CEvent* event [esp+0xC], int32 delayMs [esp+0x10])`, `ret 0x10`.
- Behaviour: `m_Ped = ped` (0x4AC3B9); `if ped` -> RegisterReference(ped, &m_Ped) 0x571B70 (0x4AC3BE); `m_PedGroup = group` (0x4AC3CF); `m_Event = event` (0x4AC3D2); `m_Time = [0xB7CB84] + delayMs` (0x4AC3D5-0x4AC3DC).

## 0x4AD800 sub_4ad800 (proposed: UNCLEAR, a CEvent* "AffectsPed"-style predicate)  (catalogue: unknown/REA, port level 1)
- Name proposal: UNCLEAR. No callers, no data references (dead code). Signature shape matches an event `AffectsPed(CPed*)` vfunc, but no vtable holds it.
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`.
- Behaviour: `if CPed::IsPlayer(ped)` 0x5DF8F0 (0x4AD807) -> return AL = 0. Else `t = this->+0xC (int32)`; return AL = 1 iff t == 4 || t == 8 || t == 9 (compares 0x4AD813/0x4AD818/0x4AD81D in that order), else 0.

## 0x4AEAC0 sub_4aeac0 (proposed: CEventPedEnteredMyVehicle::CEventPedEnteredMyVehicle)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventPedEnteredMyVehicle ctor. Evidence: stores vtable 0x85AFE8 (slot 1 returns EVENT_PED_ENTERED_MY_VEHICLE) (0x4AEB0F); callers vt_85afe8::CloneEditable 0x4B7170, CEventScanner::ScanForEvents 0x607E30, CCarEnterExit::MakeUndraggedPassengerPedsLeaveCar 0x64F540.
- `__thiscall CEventPedEnteredMyVehicle* (this, CPed* ped [esp+4], CVehicle* vehicle [esp+8], int32 targetDoor [esp+0xC])`, `ret 0xC`; SEH frame (handler 0x839CC8); returns this.
- Behaviour (CEventEditableResponse base inlined): `+4 = 0` (CEvent::m_nTimeActive), `+8 = 0 (u8)` (CEvent::m_bValid), `+0xC = 1 (u8)` (m_bAddToEventGroup), `+0xE = 0xC8 (u16, 200 = TASK_NONE, m_TaskId)`, `+0x10 = 0xFFFF (u16, m_FacialExpressionType = -1)` (0x4AEAE0-0x4AEAF0); `+0x18 = vehicle` (0x4AEB08); `vtbl = 0x85AFE8`; `+0x14 = ped`; `+0x1C = targetDoor` (0x4AEB17); `CEntity::RegisterReference(+0x14 entity, &this->+0x14)` 0x571B70 (0x4AEB1A, called even if ped is NULL - no null check, ECX = ped); then `CEntity::RegisterReference(+0x18 entity, &this->+0x18)` (0x4AEB22, also unchecked). A NULL ped/vehicle would crash inside RegisterReference (callers never pass NULL).
- gta-reversed: CEventEditableResponse (Events/EventEditableResponse.h) fields: +0xC m_bAddToEventGroup (u8), +0xE m_TaskId (int16, 200 = TASK_NONE), +0x10 m_FacialExpressionType (int16, -1). gr's CEventPedEnteredMyVehicle ctor uses SafeRegisterRef (null-checked); the original is not null-checked.

## 0x4AF010 sub_4af010 (proposed: CEventSexyVehicle::CEventSexyVehicle)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventSexyVehicle ctor. Evidence: stores vtable 0x85B0F8 (slot 1 returns EVENT_SEXY_VEHICLE) (0x4AF042); callers vt_85b0f8::Clone 0x4B72F0 and CTaskComplexWanderStandard::LookForSexyCars 0x66AFD0.
- `__thiscall CEventSexyVehicle* (this, CVehicle* vehicle [esp+4])`, `ret 4`; SEH frame (handler 0x839D88); returns this.
- Behaviour: `+4 m_nTimeActive = 0; +8 m_bValid = 0 (u8)` (0x4AF02F/0x4AF032); `vtbl = 0x85B0F8`; `+0xC = vehicle` (0x4AF048); `if vehicle` -> `CEntity::RegisterReference(vehicle, &this->+0xC)` 0x571B70 (0x4AF04D). Plain CEvent base (not editable-response).

## 0x4AFB50 sub_4afb50 (proposed: CEventVehicleThreat::CEventVehicleThreat)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventVehicleThreat ctor. Evidence: stores vtable 0x85B298 (slot 1 returns EVENT_VEHICLE_THREAT) (0x4AFB92); caller vt_85b298::CloneEditable 0x4B7520.
- `__thiscall CEventVehicleThreat* (this, CVehicle* vehicle [esp+4])`, `ret 4`; SEH frame (handler 0x839F08); returns this.
- Behaviour: CEventEditableResponse base inline: `+4 = 0; +8 = 0 (u8); +0xC = 1 (u8, m_bAddToEventGroup); +0xE = 200 (u16, m_TaskId = TASK_NONE); +0x10 = 0xFFFF (u16, m_FacialExpressionType = -1)` (0x4AFB6F-0x4AFB7F); `vtbl = 0x85B298`; `+0x14 = vehicle` (0x4AFB98); `if vehicle` -> RegisterReference(vehicle, &this->+0x14) 0x571B70 (0x4AFB9D).

## 0x4B1680 sub_4b1680 (proposed: UNCLEAR event `IsValid(CPed*)` override)  (catalogue: unknown/REA, port level 1)
- Name proposal: UNCLEAR (slot-14 CEvent::IsValid shape: "valid if the ped is alive, else base validity"). No callers, no data references (dead code).
- `__thiscall bool (this: CEvent*, CPed* ped [esp+4])`, `ret 4`.
- Behaviour: `if ped != NULL` (0x4B1687) -> return `CPed::IsAlive(ped)` 0x5E0170 (EAX passed through, 0x4B16AD). Else: `if this->m_bValid (+8 u8) != 0` -> return EAX = 1 (0x4B16A4); else `lt = this->vtbl[+0xC] (GetLifeTime)()` (0x4B1696); `if (int32)m_nTimeActive (+4) <= lt` (signed `jle` 0x4B169C) -> return 1, else return EAX = 0.

## 0x4B1D50 sub_4b1d50 (proposed: CEventCarUpsideDown::~CEventCarUpsideDown (non-deleting))  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventCarUpsideDown complete dtor (catalogue "ctor/dtor UNCLEAR" resolved). Evidence: stores vtable 0x85B660 (EVENT_CAR_UPSIDE_DOWN) then the CEvent base vtable 0x85AB38 (0x4B1D6D/0x4B1D8F) - the dtor pattern; callers: scalar deleting dtor 0x4B67F0 (slot 0 of 0x85B660) and CTaskSimpleCarDrive::ProcessPed 0x644470 (stack instance).
- `__thiscall void (this)`, plain `ret`, SEH frame (handler 0x83A1D8).
- Behaviour: `vtbl = 0x85B660`; `if this->+0xC (m_vehicle)` -> **`CEntity::RegisterReference(m_vehicle, &this->+0xC)` 0x571B70** (0x4B1D86); `vtbl = 0x85AB38`.
- Original bug (keep): the destructor REGISTERS the reference again instead of cleaning it up, so the vehicle keeps a reference slot pointing at this (dead/stack) object's +0xC; when the vehicle is later destroyed it writes NULL to that stale address. gr reproduces it under `#if !FIX_BUGS` (Events/EventCarUpsideDown.cpp:22-27).

## 0x4B4A50 sub_4b4a50 (proposed: CEventKnockOffBike::CEventKnockOffBike(const CEventKnockOffBike&))  (catalogue: unknown/REA, port level 3)
- Name proposal: the real copy constructor. Evidence: initialises the CEvent base (`+4 = 0`, `+8 = 0 (u8)`, 0x4B4A6F/0x4B4A72), stores vtable 0x85B2E0 (EVENT_KNOCK_OFF_BIKE, 0x4B4A7E), then copies with CEventKnockOffBike::From 0x4AFDD0. No callers / data refs (dead code; the copy is done by 0x4B4AA0 instead).
- `__thiscall CEventKnockOffBike* (this, const CEventKnockOffBike* other [esp+4])`, `ret 4`; SEH frame (handler 0x83A338); returns this.
- Behaviour: base init, vtbl, then `CEventKnockOffBike::From(this, other)` 0x4AFDD0 (thiscall, 0x4B4A84). Note: the CEventEditableResponse fields +0xC..+0x11 are not initialised here (From presumably sets them; not verified in this function).

## 0x5BA020 sub_5BA020 (proposed: CDecisionMaker::operator= (void-returning copy))  (catalogue: unknown/REA, port level 2)
- Name proposal: CDecisionMaker copy-assign helper. Evidence: 41 x CDecision::From with stride 0x3C over this and the argument (CDecisionMaker = CDecision[41], gr DecisionMaker.h). No callers / data refs.
- `__thiscall void (this: CDecisionMaker*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set.
- Behaviour: for i = 0..40 ascending (`ebx = 0x29`, 0x5BA02B): `CDecision::From(&this->m_Decisions[i], &src->m_Decisions[i])` 0x6006B0 (0x5BA036).

## 0x5BD4E0 sub_5BD4E0 (proposed: CDecisionMaker::operator=(const CDecisionMaker&))  (catalogue: unknown/REA, port level 2)
- Name proposal: as 0x5BA020 but returns this (`mov eax, ebp` 0x5BD506). No callers / data refs.
- `__thiscall CDecisionMaker* (this, const CDecisionMaker* src [esp+4])`, `ret 4`.
- Behaviour: for i = 0..40 ascending: `CDecision::From(this + i*0x3C, src + i*0x3C)` 0x6006B0 (0x5BD4F9); return this.

## 0x5BD510 sub_5bd510 (proposed: CDecisionMakerTypes::SetDefaultRandomPedDecisionMaker)  (catalogue: unknown/REA, port level 2)
- Name proposal: setter copying a CDecisionMaker into `CDecisionMakerTypes::m_DefaultRandomPedDecisionMaker` (+0xC1B4; gr layout: +0 m_NoOfDecisionMakers, +4 m_DecisionMakers[20] x 0x99C, +0xC034 m_EventIndices[96], then the five default makers at +0xC1B4 + k*0x99C, here k = 0). No callers / data refs (inlined elsewhere; dead out-of-line copy).
- `__thiscall void (this = ECX: CDecisionMakerTypes*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set (garbage).
- Behaviour: `dst = this + 0xC1B4` (`lea esi` 0x5BD517); 41 iterations (`ebx = 0x29`): `CDecision::From(dst + i*0x3C, src + i*0x3C)` 0x6006B0 (thiscall, 0x5BD52A); i ascending.

## 0x5BD540 sub_5bd540 (proposed: CDecisionMakerTypes::SetDefaultMissionPedDecisionMaker)  (catalogue: unknown/REA, port level 2)
- Name proposal: setter copying a CDecisionMaker into `CDecisionMakerTypes::m_DefaultMissionPedDecisionMaker` (+0xCB50; gr layout: +0 m_NoOfDecisionMakers, +4 m_DecisionMakers[20] x 0x99C, +0xC034 m_EventIndices[96], then the five default makers at +0xC1B4 + k*0x99C, here k = 1). No callers / data refs (inlined elsewhere; dead out-of-line copy).
- `__thiscall void (this = ECX: CDecisionMakerTypes*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set (garbage).
- Behaviour: `dst = this + 0xCB50` (`lea esi` 0x5BD547); 41 iterations (`ebx = 0x29`): `CDecision::From(dst + i*0x3C, src + i*0x3C)` 0x6006B0 (thiscall, 0x5BD55A); i ascending.

## 0x5BD570 sub_5bd570 (proposed: CDecisionMakerTypes::SetDefaultPlayerPedDecisionMaker)  (catalogue: unknown/REA, port level 2)
- Name proposal: setter copying a CDecisionMaker into `CDecisionMakerTypes::m_DefaultPlayerPedDecisionMaker` (+0xD4EC; gr layout: +0 m_NoOfDecisionMakers, +4 m_DecisionMakers[20] x 0x99C, +0xC034 m_EventIndices[96], then the five default makers at +0xC1B4 + k*0x99C, here k = 2). No callers / data refs (inlined elsewhere; dead out-of-line copy).
- `__thiscall void (this = ECX: CDecisionMakerTypes*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set (garbage).
- Behaviour: `dst = this + 0xD4EC` (`lea esi` 0x5BD577); 41 iterations (`ebx = 0x29`): `CDecision::From(dst + i*0x3C, src + i*0x3C)` 0x6006B0 (thiscall, 0x5BD58A); i ascending.

## 0x5BD5A0 sub_5bd5a0 (proposed: CDecisionMakerTypes::SetDefaultRandomPedGroupDecisionMaker)  (catalogue: unknown/REA, port level 2)
- Name proposal: setter copying a CDecisionMaker into `CDecisionMakerTypes::m_DefaultRandomPedGroupDecisionMaker` (+0xDE88; gr layout: +0 m_NoOfDecisionMakers, +4 m_DecisionMakers[20] x 0x99C, +0xC034 m_EventIndices[96], then the five default makers at +0xC1B4 + k*0x99C, here k = 3). No callers / data refs (inlined elsewhere; dead out-of-line copy).
- `__thiscall void (this = ECX: CDecisionMakerTypes*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set (garbage).
- Behaviour: `dst = this + 0xDE88` (`lea esi` 0x5BD5A7); 41 iterations (`ebx = 0x29`): `CDecision::From(dst + i*0x3C, src + i*0x3C)` 0x6006B0 (thiscall, 0x5BD5BA); i ascending.

## 0x5BD5D0 sub_5bd5d0 (proposed: CDecisionMakerTypes::SetDefaultMissionPedGroupDecisionMaker)  (catalogue: unknown/REA, port level 2)
- Name proposal: setter copying a CDecisionMaker into `CDecisionMakerTypes::m_DefaultMissionPedGroupDecisionMaker` (+0xE824; gr layout: +0 m_NoOfDecisionMakers, +4 m_DecisionMakers[20] x 0x99C, +0xC034 m_EventIndices[96], then the five default makers at +0xC1B4 + k*0x99C, here k = 4). No callers / data refs (inlined elsewhere; dead out-of-line copy).
- `__thiscall void (this = ECX: CDecisionMakerTypes*, const CDecisionMaker* src [esp+4])`, `ret 4`; EAX not set (garbage).
- Behaviour: `dst = this + 0xE824` (`lea esi` 0x5BD5D7); 41 iterations (`ebx = 0x29`): `CDecision::From(dst + i*0x3C, src + i*0x3C)` 0x6006B0 (thiscall, 0x5BD5EA); i ascending.

## 0x5FB990 CPedGroupMembership (proposed: CPedGroupMembership::RemoveAllMembers)  (catalogue: specified/REA, port level 2)
- Name proposal: `CPedGroupMembership::RemoveAllMembers` (catalogue has no method name). Evidence: removes all 8 member slots (followers and leader) via RemoveMember(int) 0x5F80D0; gr's RemoveAllFollowers 0x5FB190 is a different function (7 slots). One caller recorded (address not resolved in the bundle).
- `__thiscall void (this: CPedGroupMembership*)`, plain `ret`.
- Behaviour: for i = 0..7 ascending (`cmp esi, 8; jl` 0x5FB9B1): `if this->m_members[i] (+4 + 4*i) != NULL` -> `CPedGroupMembership::RemoveMember(this, i)` 0x5F80D0 (thiscall, 0x5FB9A8). The slot pointer is re-read each iteration after the previous removal.

## 0x61A5E0 vt_86d4a8::Destructor (proposed: CTaskSimple scalar deleting destructor)  (catalogue: specified/REA, port level 1)
- Name proposal: `CTaskSimple::~CTaskSimple` (scalar deleting, vtable slot 0). Evidence: vtable 0x86D4A8 is the CTaskSimple vtable (CTaskSimple::CTaskSimple 0x61A390 stores it; slots 1/4/6/7 are `_purecall` 0x82263A).
- `__thiscall void* (this, u8 flags [esp+4])`, `ret 4`; returns EAX = this (`mov eax, ecx` 0x61A5E1; the callee does not touch EAX).
- Behaviour: `CTaskSimple::~CTaskSimple` 0x61A3A0 (stores CTask vtable 0x86D48C) (0x61A5E3). `if flags & 1` (0x61A5E8): inlined `CTask::operator delete` -> task pool free: `pool = [0xB744A8 CPools::ms_pTaskPool]`; `idx = (int32)(this - pool->m_pObjects (+0)) >> 7` (`sar 7`, object size 0x80); `pool->m_byteMap (+4)[idx] |= 0x80` (mark free); `if idx < pool->m_nFirstFree (+0xC)` (signed `jge` 0x61A60E) -> `m_nFirstFree = idx`.

## 0x61A6C0 CTaskSimpleAnim (proposed: CTaskSimpleAnim::CTaskSimpleAnim(bool holdLastFrame))  (catalogue: specified/REA, port level 1)
- Name proposal: CTaskSimpleAnim ctor. Evidence: stores vtable 0x86D504; callers are the derived ctors CTaskSimpleRunNamedAnim 0x6674B0 and CTaskSimpleSlideToCoord 0x66C3E0.
- `__thiscall CTaskSimpleAnim* (this, bool holdLastFrame [esp+4])`, `ret 4`.
- Behaviour: `CTaskSimple::CTaskSimple(this)` 0x61A390; `+0xC = (+0xC & 0xF8) | ((holdLastFrame & 1) << 2)` (bits 0,1 cleared, bit 2 = holdLastFrame, bits 3-7 keep the previous (uninitialised) byte; 0x61A6C8-0x61A6D9); `vtbl = 0x86D504`; `m_pAnim (+8) = 0`; return this.

## 0x61AC90 vt_86d594::ctor-helper (proposed: CTaskSimpleAnimLoopedMiddle::CTaskSimpleAnimLoopedMiddle(AssocGroupId, AnimationId, ...))  (catalogue: unknown/REA, port level 1)
- Name proposal: class with vtable 0x86D594 (12 slots; GetTaskType 0x61AD00 returns the u16 at +0x6C, which both ctors set to 0x1A8 = TASK_SIMPLE_ANIM_LOOPED_MIDDLE; no gr class). This is the anim-id ctor (the other, 0x61AD10, takes anim/block names). Caller: Clone 0x61B890.
- `__thiscall T* (this, int32 animGroup [esp+4], int32 animId [esp+8], float blendDelta [esp+0xC], u32 a4 [esp+0x10], u32 a5 [esp+0x14], u32 a6 [esp+0x18], bool holdLastFrame [esp+0x1C])`, `ret 0x1C`; size 0x70.
- Behaviour: CTaskSimple ctor 0x61A390; `+0xC = (+0xC & 0xF8) | ((holdLastFrame & 1) << 2)`; `+0x10 = animGroup; +0x14 = animId; +8 m_pAnim = 0; +0x48 = blendDelta (raw); vtbl = 0x86D594; +0x60 = 0; +0x64 = 0; +0x68 = 0 (u8); +0x69 = 0 (u8); +0x4C = a4; +0x44 (hierarchy) = 0; +0x18 (anim flags) = 0; +0x6C = 0x1A8 (u16); +0x50 = a5; +0x5C = a6` (store order 0x61ACAE-0x61ACF7); return this. The name buffers +0x1C/+0x34 are not initialised.

## 0x61AD00 vt_86d594::GetTaskType (proposed: CTaskSimpleAnimLoopedMiddle::GetTaskType)  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns `(int32)(u16)this->+0x6C` (`movzx eax, word [ecx+0x6C]`, 0x61AD00; zero-extended), plain `ret`. Slot 4 of vtable 0x86D594. Both ctors store 0x1A8 (TASK_SIMPLE_ANIM_LOOPED_MIDDLE).

## 0x61AD10 vt_86d594::sub_61AD10 (proposed: CTaskSimpleAnimLoopedMiddle::CTaskSimpleAnimLoopedMiddle(const char* animName, const char* blockName, ...))  (catalogue: unknown/REA, port level 2)
- Name proposal: named-anim ctor of the 0x86D594 class (see 0x61AC90). Caller: Clone 0x61B890.
- `__thiscall T* (this, const char* animName [esp+4], const char* blockName [esp+8], int32 animFlags [esp+0xC], float blendDelta [esp+0x10], u32 a5 [esp+0x14], u32 a6 [esp+0x18], u32 a7 [esp+0x1C], bool holdLastFrame [esp+0x20])`, `ret 0x20`; SEH frame (handler 0x83F148).
- Behaviour: CTaskSimple ctor 0x61A390; holdLastFrame into bit 2 of +0xC as in 0x61AC90; `+8 = 0; +0x18 = animFlags; +0x48 = blendDelta; vtbl = 0x86D594; +0x60 = 0; +0x64 = 0; +0x68 = 0 (u8); +0x69 = 0 (u8); +0x50 = a6; +0x5C = a7; +0x6C = 0x1A8 (u16); +0x4C = a5`; strcpy(this+0x1C, animName) (inline byte loop incl. terminator, 0x61AD98); strcpy(this+0x34, blockName) (0x61ADB0) - no length limit (buffers 24 / 16 bytes); `+0x44 = CAnimManager::GetAnimation(this+0x1C, CAnimManager::GetAnimationBlock(this+0x34) 0x4D3940) 0x4D42F0` (cdecl, 0x61ADC2); `+0x10 = 0` (animGroup); `+0x14 = 0xBF` (animId 191); return this.

## 0x61ADF0 vt_86d594::sub_61ADF0 (proposed: CTaskSimpleAnimLoopedMiddle::StartAnim)  (catalogue: unknown/REA, port level 9)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Caller: ProcessPed 0x61BCB0.
- Behaviour: `if this->+0x44 (hierarchy) != NULL` -> `assoc = CAnimManager::BlendAnimation(ped->m_pRwClump (+0x18), hierarchy, flags = +0x18, blendDelta = +0x48)` 0x4D4410 (cdecl, 0x61AE0B); else `assoc = CAnimManager::BlendAnimation(ped->m_pRwClump, animGroup +0x10, animId +0x14, blendDelta +0x48)` 0x4D4610 (0x61AE26). `this->m_pAnim (+8) = assoc` (0x61AE36); `assoc->SetFinishCallback(CTaskSimpleAnim::FinishRunAnimCB 0x61A8A0, this)` 0x4CEBE0 (0x61AE39). No NULL check on assoc.

## 0x61B890 vt_86d594::Clone (proposed: CTaskSimpleAnimLoopedMiddle::Clone)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame (handler 0x83F286). Slot 1 of vtable 0x86D594.
- Behaviour: `p = CTask::operator new(0x70)` 0x61A5A0 (the size push happens before the branch). Two paths on `this->+0x44 (hierarchy)` (0x61B8AC):
  - hierarchy != NULL: if p == NULL return 0; else `0x61AD10(p, animName = this+0x1C, blockName = this+0x34, animFlags = this->+0x18, blendDelta = this->+0x48, this->+0x4C, this->+0x50, this->+0x5C, holdLastFrame = (this->+0xC & 4) >> 2)` (named ctor, 0x61B8F8); return its EAX.
  - hierarchy == NULL: if p == NULL return 0; else `0x61AC90(p, this->+0x10 (group), this->+0x14 (animId), this->+0x48, this->+0x4C, this->+0x50, this->+0x5C, (this->+0xC & 4) >> 2)` (id ctor, 0x61B94B); return its EAX.
- Note: the named path passes this->+0x18 (anim flags) as the 3rd arg; the id path does not copy +0x18 (the id ctor zeroes it).

## 0x61BCA0 vt_86d594::~dtor-thunk (proposed: CTaskSimpleAnimLoopedMiddle::~CTaskSimpleAnimLoopedMiddle)  (catalogue: unknown/REA, port level 2)
- Name proposal: non-deleting dtor of the 0x86D594 class (see 0x61AC90): nothing of its own to destroy, so it tail-jumps to the CTaskSimpleAnim dtor.
- Tail-jump thunk (0x61BCA0): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x61BCB0 vt_86d594::ProcessPed (proposed: CTaskSimpleAnimLoopedMiddle::ProcessPed)  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; returns AL. Slot 7 of vtable 0x86D594 (class of 0x61AC90/0x61AD10). Fields: +8 m_pAnim, +0xC flags (bit0 finished), +0x14 animId, +0x4C loopStartFrac, +0x50 loopEndFrac, +0x54 loopStart (s), +0x58 loopEnd (s), +0x5C durationMs, +0x60/+0x64/+0x68 CTaskTimer {start, interval, started}.
- Behaviour:
  1. `if +0xC & 1` -> return 1 (0x61BCB3).
  2. If `m_pAnim == NULL` (0x61BCC9), or `0x4D68B0(ped->m_pRwClump (+0x18), (u32)(this->+0x14 == 0)) == NULL` (cdecl, 0x61BCDA; NOTE the second argument is the boolean `animId == 0` (`sete al`), not the anim id - original quirk, keep) -> (re)start (0x61BD34): `StartAnim(ped)` 0x61ADF0; `h = m_pAnim->m_BlendHier (+0x14)`; `+0x54 = h->+0x10 (total length) * +0x4C` (fld/fmul/fstp, 0x61BD42-0x61BD48); `+0x58 = h->+0x10 * +0x50` (0x61BD4E-0x61BD57); timer: `+0x60 = [0xB7CB84 CTimer::m_snTimeInMilliseconds]; +0x64 = +0x5C; +0x68 = 1` (u8); return 0. No NULL check on m_pAnim after StartAnim.
  3. Else: `if +0x68 != 0 && CTaskTimer::IsOutOfTime(this+0x60)` 0x420E30 -> return 0 (looping stops; the anim then plays to its end). `a = m_pAnim`; if NULL return 0.
  4. `if !(a->m_CurrentTime (+0x20) > +0x58)` -> return 0 (`fld [a+0x20]; fcomp [this+0x58]; test ah, 0x41; jne`: less, equal or unordered return, 0x61BD00-0x61BD0B).
  5. `t = a->m_CurrentTime - a->m_TimeStep (+0x28)` (`fld [a+0x28]; fsubr [a+0x20]`); `fcomp [this+0x58]; test ah, 0x41; jp` (0x61BD1B): continue only when exactly one of C0/C3 is set, i.e. `t <= loopEnd` (less or equal); greater or unordered -> return 0. So the loop end was crossed this frame.
  6. `a->SetCurrentTime(+0x54 loopStart)` 0x4CEA80 (thiscall, float pushed as raw dword, 0x61BD21); `a->m_Flags (+0x2E) |= 1` (0x61BD29); return 0.

## 0x61C650 sub_61c650 (proposed: CTaskSimpleFight::IsBlockingStrike)  (catalogue: unknown/REA, port level 0)
- Name proposal: block test on the VICTIM's CTaskSimpleFight (layout matches gr CTaskSimpleFight: +0x1C m_pAnim, +0x24 m_nComboSet (int8), +0x27 m_nLastCommand; command 2 = block). Caller: CTaskSimpleFight::FightHitPed 0x61CBA0, which calls it on the victim's fighting task with (victim, attacker, attacker's m_nComboSet, attacker's m_nCurrentMove). Exact gr name unknown.
- `__thiscall bool (this: CTaskSimpleFight* (victim's), CPed* owner [esp+4], CPed* attacker [esp+8], uint8 attackerCombo [esp+0xC], uint8 attackerMove [esp+0x10])`, `ret 0x10`.
- Behaviour:
  1. Return 0 unless all hold: `m_nLastCommand (+0x27) == 2`; `m_pAnim (+0x1C) != NULL`; `!(m_pAnim->m_Flags (+0x2E) & 1)`; and `m_pAnim->m_BlendHier (+0x14)->m_fTotalLength (+0x10) > m_pAnim->m_CurrentTime (+0x20)` (`fld cur; fld total; fcompp; test ah, 0x41; jne`: less, equal or NaN return 0; 0x61C671-0x61C681).
  2. `P = owner position` (matrix pos or placement), `Q = attacker position`; `d = Q - P` (dx, dy, dz); `dot = (dz*F.z + dy*F.y) + dx*F.x` with F = owner->m_matrix->forward (+0x10/+0x14/+0x18; the matrix pointer is dereferenced without NULL check); `behind = (dot < [0x858C24]=0.3f)` (`fcomp; test ah, 5; jp`: only C0 set -> behind = 1; greater, equal, NaN -> 0).
  3. Combo filter (on the attacker's combo/move args): if `attackerCombo` in {8, 0xA, 0xB, 0xC}: require the owner's own `8 <= (int8)m_nComboSet <= 0xC && m_nComboSet != 9`, else return 0. Else if `attackerCombo == 9`: require `8 <= m_nComboSet <= 0xC`, else return 0. Else if `attackerCombo == 7` and `attackerMove == 1` -> return 0.
  4. If `behind` (attacker not in front of the owner) -> return 0. Else `m_pAnim->m_Flags |= 1` and return 1 (strike blocked).

## 0x61CBA0 sub_61cba0 (proposed: CTaskSimpleFight::FightHitPed)  (catalogue: unknown/REA, port level 16)
- Name proposal: CTaskSimpleFight::FightHitPed (gr declares `FightHitPed(CPed* creator, CPed* victim, CVector&, CVector&, int16)` without an address; argument count/shape and behaviour match). Caller: 0x6240B0 (strike code).
- `__thiscall CPed* (this: CTaskSimpleFight* (attacker's), CPed* attacker [esp+4], CPed* victim [esp+8], const CVector* hitPos [esp+0xC], const CVector* arg4 [esp+0x10] (unused), int16 pieceType [esp+0x14])`, `ret 0x14`; SEH frame (local CEventSoundQuiet). Returns victim when a damage event was generated, else 0. The argument slots of attacker, victim and pieceType are reused as locals (shot side, damage, "event generated" flag).
- Fields: +0x14 m_nContinueStrike (int8), +0x24 m_nComboSet (int8), +0x25 m_nCurrentMove (int8). `C = &CTaskSimpleFight::m_aComboData[(int8)m_nComboSet - 4]` (0xC170D0 + idx*0x88); `C->+0x5C[move]` and `C->+0x70[move]` are audio event ids; `T[move]` = dword table at 0x8D2E3C (move 0..2); "Audio(id, vol, M)" = `CAEPedAudioEntity::AddAudioEvent(&attacker->+0x138, id, vol, 1.0f, victim, 0, 0, M)` 0x4E2BB0.
- Behaviour:
  1. If `CPed::IsPlayer(victim)` and `CTaskManager::GetActiveTask(&victim->m_pIntelligence->m_TaskMgr)->GetTaskType() == 0xCD` -> return 0.
  2. Block: if `f = CPedIntelligence::GetTaskFighting(victim->m_pIntelligence)` 0x600F30 != NULL and `0x61C650(f, victim, attacker, (u8)m_nComboSet, (u8)m_nCurrentMove)` (IsBlockingStrike) -> `Audio(C->+0x70[move], -9.0f, 0)`; if `m_nComboSet == 5 && 0 <= move <= 2` -> `Audio(C->+0x70[move], -9.0f, T[move])`; return 0.
  3. `CWeaponInfo::GetWeaponInfo(attacker weapon type (+0x5A0 + slot*0x1C, slot = (int8)+0x718), 1)` 0x743C60 - result unused. `dmg = _ftol2(0x61C740(this, attacker))` (float strike damage -> int, truncation). `m_nContinueStrike = -1`.
  4. `side = CPedGeometryAnalyser::ComputePedShotSide(victim, &attackerPos)` 0x5F13F0; `ok = CWeapon::GenerateDamageEvent(victim, attacker, weaponType, dmg, (int32)pieceType, side)` 0x73A530 (`setne`). If `weaponType == 9` (chainsaw) -> `CAEPedWeaponAudioEntity::AddAudioEvent(&attacker->+0x394, 0x9B)` 0x4E69F0.
  5. Hit sound (vol 0.0f): if `side != 0` -> `Audio(C->+0x70[move], 0, 0)`, then if `combo == 5 && 0 <= move <= 2` -> `Audio(C->+0x70[move], 0, T[move])`. If `side == 0`: if `combo == 5 && 0 <= move <= 2` -> `Audio(C->+0x5C[move], 0, 0)` then `Audio(C->+0x5C[move], 0, T[move])`; else if `combo == 7 && move == 1` -> `Audio(C->+0x60, 0, [0x8D2E40])`, then `v = (float)(u32)T[move]` (fild; `+ [0x858C54]=4294967296.0f` when the int is negative) `* [0x86D6A8]=2.8f`, `Audio(C->+0x5C[move], 0, _ftol2(v))`; else `Audio(C->+0x5C[move], 0, 0)`.
  6. If `CPed::IsPlayer(attacker)` -> `CPed::Say(attacker, 0x59, 0, 1.0f, 0, 0, 0)`.
  7. Local `CEventSoundQuiet e(attacker, 55.0f, -1, &(0,0,0))` 0x5E05B0; `CEventGroup::Add(GetEventGlobalGroup() 0x4ABA50, &e, false)` 0x4AB420.
  8. Blood chance: `thr = 100` if combo in {8..0xC}; `thr = -1` if `combo == 4 && move == 4`; else `thr = _ftol2(100.0f - victim->m_fHealth (+0x540))` (fld health; fld 100; fsub st(1)). `r = _ftol2((float)(_rand() & 0xFFFF) * [0x858B14]=3.0517578e-05f * [0x858628]=100.0f)`; if `r < thr` (signed): `pos = *hitPos`; `dir = attackerPos - victimPos` (z, y, x computed in that order, f32 stores); `CVector::Normalise(&dir)`; if `!CPed::IsAlive(victim)` -> `dir = (0, 0, 2.0f)`; `n = 8`; if combo in {8..0xC}: `n = 16` and if `CPed::IsAlive(victim)` -> `dir *= [0x858CE8]=1.5f` (per component); `Fx_c::AddBlood(&g_fx 0xA9AE00, &pos, &dir, n, victim->+0x12C (lighting))` 0x49EB00.
  9. Destroy e (0x5DEA00); return `ok ? victim : 0`.

## 0x61D400 sub_61d400 (proposed: CTaskSimpleFight::FightHitObj)  (catalogue: unknown/REA, port level 18)
- Name proposal: `CTaskSimpleFight::FightHitObj(CPed* attacker, CObject* obj, CVector& pos1, CVector& pos2, int16, int8 surface)` (gr declares it without address; sibling of FightHitCar 0x61D0B0). Caller: 0x6240B0.
- `__thiscall void (this: CTaskSimpleFight*, CPed* attacker [esp+4], CObject* obj [esp+8], CVector* pos1 [esp+0xC], CVector* pos2 [esp+0x10], int16 unused [esp+0x14], int8 surface [esp+0x18])`, `ret 0x18`. Local `D = *pos2` (copied first); the pos2 slot is then reused for the float damage.
- Behaviour:
  1. `dmg = 0x61C740(this, attacker)` (float).
  2. Push the object: only if `obj->+0x144 (u8) < 0xC8` and `!(obj->+0x40 & 4)` and `obj->+0x160 (object info)->+0x18 < [0x86D6B0]=99.9f` (only C0 continues):
     a. If `(obj->m_nFlags (+0x1C) & 4)` or `(+0x1C & 0x40000)`: if `info->+0x14 <= 0.0` (`fcomp [0x858B50]; test ah, 0x41; jp` skips on greater/unordered) -> `obj->vfunc[+0x10](false)` (SetIsStatic) and `CPhysical::AddToMovingList(obj)` 0x542800.
     b. Re-read the flags; if neither bit is set: `k = ((int8)obj->+0x40 < 0) ? [0x858EF4]=-0.1f : [0x858F40]=-0.5f`; `p = pos1 - objPos` (x, y, z, each fsub/fstp f32); `F = (D.x*k, D.y*k, D.z*k)` (each fmul/fstp f32); `CPhysical::ApplyForce(obj, F, p, true)` 0x542B50 (both CVectors by value).
  3. `CObject::ObjectDamage(obj, dmg * [0x85862C]=10.0f, pos1, &D, attacker, attacker weapon type)` 0x5A0D90.
  4. If the attacker's weapon is 9 (chainsaw) -> `CAEPedWeaponAudioEntity::AddAudioEvent(&attacker->+0x394, 0x9B)`.
  5. `snd = dword [0xC16F0C + ((int8)this->+0x24 * 0x22 + (int8)this->+0x25) * 4]`; `CAEPedAudioEntity::AddAudioEvent(&attacker->+0x138, snd, 0.0f, 1.0f, obj, (u8)surface, 0, 0)`.
  6. `Fx_c::AddPunchImpact(&g_fx, pos1, &D, 4)` 0x49F670.

## 0x61FCC0 vt_86d7d8::MakeAbortable (proposed: CTaskSimpleBeKickedOnGround::MakeAbortable)  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x86D7D8.
- Behaviour: `if priority == 1 || priority == 2` (ABORT_PRIORITY_URGENT / IMMEDIATE, 0x61FCC4/0x61FCCC): `if m_pAnim` -> `m_pAnim->SetFinishCallback(0x4D1390 (empty default CB), NULL)` 0x4CEBE0 and `m_pAnim = 0`; return 1. Else (leisure): `if m_pAnim` -> `m_pAnim->m_Flags (+0x2E) |= 4` (FREEZE/unlock bit, 0x61FCD8) and `m_pAnim->m_BlendDelta (+0x1C) = -4.0f` (0x61FCDF); return 0.

## 0x61FE00 vt_86d7d8::ProcessPed (proposed: CTaskSimpleBeKickedOnGround::ProcessPed)  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Slot 7 of vtable 0x86D7D8.
- Behaviour: `if m_bIsFinished (+8)` -> return 1 (0x61FE07). `if m_pAnim (+0xC) == NULL` -> `StartAnim(this, ped)` 0x61FD20 (thiscall, 0x61FE18). Return 0.

## 0x61FEE0 vt_86d7fc::MakeAbortable (proposed: CTaskSimpleBeHitWhileMoving::MakeAbortable)  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped, int32 priority [esp+8], const CEvent* event)`, `ret 0xC`. Slot 6 of vtable 0x86D7FC. Identical to 0x61FCC0 with m_pAnim at +0x14:
- `if priority == 1 || priority == 2`: `if m_pAnim` -> SetFinishCallback(0x4D1390, NULL) 0x4CEBE0 and `m_pAnim = 0`; return 1. Else: `if m_pAnim` -> `m_Flags (+0x2E) |= 4`, `m_BlendDelta (+0x1C) = -4.0f` (0x61FEFF); return 0.

## 0x620290 vt_86d7fc::ProcessPed (proposed: CTaskSimpleBeHitWhileMoving::ProcessPed)  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Slot 7 of vtable 0x86D7FC.
- Behaviour: `if +8 (finished)` -> return 1. `if m_pAnim (+0x14) == NULL` -> `0x61FF40(this, ped)` (StartAnim helper, thiscall, 0x6202A8). Return 0.

## 0x620BF0 vt_86d868::MakeAbortable (proposed: CTaskComplexReactToAttack::MakeAbortable)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x86D868.
- Behaviour: `if +0xD (u8) != 0` -> return 1 (0x620BFA). `if priority != 1 && priority != 2` -> return 0. Else `r = m_pSubTask (+8)->MakeAbortable(ped, priority, event)` (vslot +0x18, no NULL check, 0x620C25); `+0xD = r` (AL, 0x620C29); return r.

## 0x620E00 vt_86d868::ControlSubTask (proposed: CTaskComplexReactToAttack::ControlSubTask)  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 10 of vtable 0x86D868.
- Behaviour: `if +0xC (u8) != 0` -> return `this->CreateFirstSubTask(ped)` (vslot +0x24, 0x620E13). Else call `m_pSubTask->GetTaskType()` (vslot +0x10, result discarded, 0x620E1F) and return `m_pSubTask` (+8, re-read).

## 0x6217D0 CTaskSimpleGangDriveBy (proposed: CTaskSimpleGangDriveBy::CTaskSimpleGangDriveBy)  (catalogue: specified/REA, port level 2)
- `__thiscall T* (this, CEntity* target [esp+4], const CVector* targetPos [esp+8], float abortRange [esp+0xC], int8 frequencyPercentage [esp+0x10], int8 drivebyStyle [esp+0x14], bool seatRHS [esp+0x18])`, `ret 0x18`; SEH frame. 10 callers (CCarAI::UpdateCarAI 0x41DA30, CGangWars::CreateAttackWave 0x444810, script 1800-1899 0x46D050, CEventHandler::ComputePedThreatResponse 0x4C19A0, CCam::Process_Cam_TwoPlayer_InCarAndShooting 0x519810, Clone 0x6236D0, ...).
- Behaviour (field names from gr TaskSimpleGangDriveBy.h): CTaskSimple ctor 0x61A390; `+0xA m_bSeatRHS = seatRHS; +0x12 m_nDrivebyStyle = drivebyStyle; +0x20 m_fAbortRange = abortRange (raw); +0x34 m_pTargetEntity = target; +0x13 m_nFrequencyPercentage = frequencyPercentage; vtbl = 0x86D944`; `+8, +9, +0xB, +0xC, +0xD, +0xE = 0` (u8); `+0x38/+0x3C/+0x40 m_vecCoords = 0`; `+0x11 m_nBurstShots = -1; +0x14 m_nFakeShootDirn = -1; +0x16 m_nAttackTimer = -1 (u16)`; `+0x10 m_nLastCommand = 0; +0xF m_nNextCommand = 1; +0x18 m_nLOSCheckTime = 0; +0x1C m_nLOSBlocked = 1; +0x2C m_pAnimAssoc = 0; +0x24 m_nRequiredAnimID = 0xBF; +0x28 m_nRequiredAnimGroup = 0; +0x30 m_pWeaponInfo = 0`; `if target` -> RegisterReference(target, &+0x34) 0x571B70; `if targetPos` -> `m_vecCoords = *targetPos` (3 dwords); return this.

## 0x622680 sub_622680 (proposed: CTaskSimpleStealthKill::~CTaskSimpleStealthKill (non-deleting))  (catalogue: unknown/REA, port level 1)
- Name proposal: CTaskSimpleStealthKill complete dtor. Evidence: stores vtable 0x86DA74 (slot 4 returns TASK_SIMPLE_STEALTH_KILL); caller 0x6296B0 = slot 0 (scalar deleting dtor) of 0x86DA74.
- `__thiscall void (this)`, plain `ret`; SEH frame (.HOODLUM stub).
- Behaviour: `vtbl = 0x86DA74`; `if +0xC (target)` -> CleanUpOldReference(+0xC entity, &this->+0xC) 0x571A00 (0x6226B6); `if +0x18 (anim)` -> `anim->SetDeleteCallback(0x4D1390 (empty default CB), NULL)` 0x4CEBC0 (0x6226C9); CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x623120 vt_86d7d8::Clone (proposed: CTaskSimpleBeKickedOnGround::Clone)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86D7D8.
- Behaviour: `p = CTask::operator new(0x10)` 0x61A5A0; if NULL return 0 (0x62317D). Else CTaskSimple ctor 0x61A390 on p; `p->vtbl = 0x86D7D8; p->+8 = 0 (u8); p->+0xC = 0`; return p. Nothing is copied from this.

## 0x623190 vt_86d7fc::Clone (proposed: CTaskSimpleBeHitWhileMoving::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1 of vtable 0x86D7FC. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x18)`; if NULL return 0. Else read `b = this->+0x10, a = this->+0xC`; CTaskSimple ctor 0x61A390; `p+0x10 = b; p+0xC = a; p->vtbl = 0x86D7FC; p+8 = 0 (u8); p+0x14 = 0`; return p.

## 0x623300 vt_86d868::Clone (proposed: CTaskComplexReactToAttack::Clone)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86D868.
- Behaviour: `p = CTask::operator new(0x24)`; if NULL return 0. Else `CTaskComplexReactToAttack::CTaskComplexReactToAttack(p, this+0x10, this+0x14, this+0x18, this+0x1C, this+0x20)` 0x620B00 (0x623349); return p.

## 0x623750 vt_86da1c::Clone (proposed: CTaskComplexKillAllThreats::Clone)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86DA1C.
- Behaviour: `p = CTask::operator new(0x18)`; if NULL return 0. Else read `c = this+0x14, b = this+0x10, a = this+0xC`; CTaskComplex ctor 0x61A3B0 on p; `p+0x10 = b; p+0x14 = c; p->vtbl = 0x86DA1C; p+0xC = a`; return p.

## 0x625C20 vt_86d868::CreateNextSubTask (proposed: CTaskComplexReactToAttack::CreateNextSubTask)  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 8 of vtable 0x86D868.
- Behaviour: `if +0xD (aborted flag)` -> return 0. `t = m_pSubTask (+8)->GetTaskType()` (vslot +0x10, no NULL check); if `t == 0xC8 (TASK_NONE) || t == 0xD0 (TASK_COMPLEX_FALL_AND_GET_UP) || t == 0x3F0 (TASK_SIMPLE_BE_HIT)` (cumulative `sub` chain 0x625C3C-0x625C4D) -> return `CreateSubTask(this, 0x516)` 0x620C30 (thiscall, task id 1302 = TASK_FINISHED); else return 0.

## 0x625C70 vt_86d868::CreateFirstSubTask (proposed: CTaskComplexReactToAttack::CreateFirstSubTask)  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 9 of vtable 0x86D868. Fields: +0xC (u8), +0x10 weapon type, +0x14 attacker, +0x20 hit piece. CreateSubTask = 0x620C30.
- Behaviour:
  1. `+0xC = 0`.
  2. If `CPed::IsPlayer(ped) && (int32)+0x10 > 9`: `pad = CPad::GetPad(0)` 0x53FB70; if `pad->+0x10E (u16 DisablePlayerControls) == 0` and (`CPad::GetTarget(pad)` 0x540670 or any of the u16 pad words +0x0, +0x2, +0x10, +0x12, +0x14, +0x16 is non-zero (NewState LeftStickX/Y and the D-pad words)) -> return `CreateSubTask(0xC8 TASK_NONE)` (player is actively controlling: no reaction).
  3. If `+0x20 == 3`: if `ped->+0x470 & 0x4000000` (gr bitfield order: bCanBeShotInVehicle - verify) and (`+0x10 == 0x1B` or `+0x10 == 0x19` or (`+0x10 == 0x18` and !IsPlayer(ped))): knock-back: `A = attacker position, P = ped position` (matrix pos or placement); local `v = (A.x - P.x, A.y - P.y)` (each fsub then fstp f32); `CVector2D::Normalise(&v)` 0x44E480; `ped->+0x46C &= ~1` (bIsStanding cleared); `CPhysical::ApplyMoveForce(ped, CVector(v.x * [0x858FC8]=-5.0f, v.y * -5.0f, 5.0f (0x40A00000)))` 0x5429F0 (vector passed by value); return `CreateSubTask(0xD0 TASK_COMPLEX_FALL_AND_GET_UP)`.
  4. (Hit piece 3 without knock-back) if `(int32)+0x10 < 9` and `ped->m_fHealth (+0x540) < [0x858CA4]=30.0f` (`fcomp; test ah, 5; jnp` -> only C0 set) -> `CreateSubTask(0xD0)`.
  5. Otherwise (including hit piece != 3) -> `CreateSubTask(0x3F0 TASK_SIMPLE_BE_HIT)`.

## 0x6293C0 vt_86da1c::CreateNextSubTask (proposed: CTaskComplexKillAllThreats::CreateNextSubTask)  (catalogue: specified/REA, port level 13)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 8 of vtable 0x86DA1C.
- Behaviour: `t = m_pSubTask->GetTaskType()` (vslot +0x10). If `t == 0xCB` (TASK_SIMPLE_STAND_STILL) -> return 0. If `t != 0x3E8` (TASK_COMPLEX_KILL_PED_ON_FOOT) -> return 0. Else `threat = 0x6221D0(this, ped)` (find-threat helper); if NULL return 0; `p = CTask::operator new(0x38)`; if NULL return 0; return `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(p, threat, -1, this->+0xC, this->+0x10, this->+0x14, 1)` 0x620E30 (args pushed 1, +0x14, +0x10, +0xC, -1, threat).

## 0x629480 vt_86da1c::CreateFirstSubTask (proposed: CTaskComplexKillAllThreats::CreateFirstSubTask)  (catalogue: specified/REA, port level 13)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 9 of vtable 0x86DA1C.
- Behaviour: `threat = 0x6221D0(this, ped)` (find-threat helper). If threat != NULL: `p = CTask::operator new(0x38)`; if NULL return 0; return `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(p, threat, -1, this->+0xC, this->+0x10, this->+0x14, 1)` 0x620E30. Else: `p = CTask::operator new(0x20)`; if NULL return 0; return `CTaskSimpleStandStill::CTaskSimpleStandStill(p, 0, false, false, 8.0f)` 0x62F310 (pushes 0x41000000, 0, 0, 0).

## 0x62F2D0 sub_62f2d0 (proposed: IsExtraSunnyWeather(int16 weatherType))  (catalogue: unknown/REA, port level 0)
- Name proposal: static helper "weather type is extra-sunny". Evidence: returns true for 0, 2, 0xD, 6, 0xB, 0x11 = WEATHER_EXTRASUNNY_LA, EXTRASUNNY_SMOG_LA, EXTRASUNNY_VEGAS, EXTRASUNNY_COUNTRYSIDE, EXTRASUNNY_SF, EXTRASUNNY_DESERT (gr eWeatherType); caller CTaskComplexSunbathe::CanSunbathe 0x632140.
- `__cdecl int32 (int16 weatherType [esp+4])`, plain `ret`; only the low 16 bits are compared (`mov ax, word ptr [esp+4]`).
- Behaviour: compares in the order 0, 2, 0xD, 6, 0xB, 0x11; any match -> EAX = 1 (0x62F2FB), else EAX = 0 (`xor eax, eax`, full register).

## 0x62F520 vt_85b8dc::ProcessPed (proposed: CTaskSimpleOnEscalator::ProcessPed)  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Slot 7 of vtable 0x85B8DC (slot 4 returns TASK_SIMPLE_ON_ESCALATOR; derived from CTaskSimpleStandStill).
- Behaviour: `CTaskSimpleStandStill::ProcessPed(this, ped)` 0x62F370 (result ignored, 0x62F526). `e = ped->+0x584` (entity the ped stands on); if e == NULL -> return 1 (0x62F57D). `mi = (int32)(int16)e->m_nModelIndex (+0x22)`; if `mi != (u16)[0x8CD674 MI_ESCALATORSTEP] && mi != (u16)[0x8CD678 MI_ESCALATORSTEP8]` -> return 1. Else `a = CGeneral::GetRadianAngleBetweenPoints(e->+0x44, e->+0x48, 0.0f, 0.0f)` 0x53CBE0 (cdecl, args x1 = matrix forward.x? raw dwords at e+0x44/e+0x48 (entity CMatrix* fields? see note), x2 = 0, y2 = 0; returns ST0); store to temp f32 (`fstp [esp+0x18]`), `ped->+0x55C = CGeneral::LimitRadianAngle(temp)` 0x53CB50 (fstp, 0x62F571); return 0.
- Note: e+0x44/e+0x48 are read as raw dwords passed as floats (for a CEntity with matrix these are the forward vector x/y of the placeable's embedded CSimpleTransform? - verify the entity layout in the port: CPlaceable +0x14 m_matrix pointer; +0x44/+0x48 are inside the entity body, not the matrix). The 4 args are pushed in the order 0, 0, [e+0x48], [e+0x44], so the call is `GetRadianAngleBetweenPoints([e+0x44], [e+0x48], 0, 0)`.

## 0x632B80 vt_86e1dc::ProcessPed (proposed: CTaskSimpleWaitForPizza::ProcessPed)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped unused)`, `ret 4`. Slot 7 of vtable 0x86E1DC. Inline CTaskTimer {+8 start, +0xC interval, +0x10 started, +0x11 reset}.
- Behaviour: `if !started`: `start = now ([0xB7CB84])`, `interval = 0x7D0 (2000 ms)`, `started = 1` (0x632B89-0x632B99). (The following `cmp started, 0; je` 0x632B9D is dead: started is now 1.) `if reset (+0x11)` -> `start = now; reset = 0`. Return AL = `(u32)now >= (u32)(start + interval)` (`cmp now, start+interval; sbb al, al; inc al`: 1 when not below, 0 when below; 0x632BBB-0x632BC3).

## 0x634560 vt_86e3cc::Clone (proposed: CTaskSimpleTriggerLookAt::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x86E3CC. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `time = m_time (+0xC); bone = m_nOffsetBoneTag (+0x10)`; `if bone > -1 (signed) && m_pEntity == NULL` -> `bone = -1; time = 100` (0x634582-0x634591; the entity was lost, look at the stored point briefly). `p = CTask::operator new(0x30)`; if NULL return 0; return `CTaskSimpleTriggerLookAt(p, m_pEntity, time, bone, m_vOffsetPos (copied by value), (u32)(u8)m_bUseTorso, m_Speed, m_nBlendTime, (int32)(int8)m_nPriority)` 0x634440.

## 0x634620 vt_86e3cc::ProcessPed (proposed: CTaskSimpleTriggerLookAt::ProcessPed)  (catalogue: specified/REA, port level 7)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; always returns AL = 1. Slot 7 of vtable 0x86E3CC. Fields (gr TaskSimpleTriggerLookAt.h): +8 m_pEntity, +0xC m_time, +0x10 m_nOffsetBoneTag, +0x14 m_vOffsetPos (RwV3d), +0x20 m_bUseTorso, +0x24 m_Speed, +0x28 m_nBlendTime, +0x2C m_bEntityExist, +0x2D m_nPriority (int8).
- Behaviour: `if m_bEntityExist && m_pEntity == NULL` -> skip (0x634625-0x63462C). Else `g_ikChainMan.LookAt("TaskTriggerLookAt" [0x86E3F0], ped, m_pEntity, m_time, m_nOffsetBoneTag, &m_vOffsetPos, (u32)(u8)m_bUseTorso, m_Speed (raw dword), m_nBlendTime, (int32)(int8)m_nPriority, 0)` 0x618970 (thiscall, ECX = 0xC15448 g_ikChainMan, 11 stack args; 0x634662). Return 1.

## 0x636860 sub_636860 (proposed: CTaskComplexUseEffectSprinting::CTaskComplexUseEffectSprinting)  (catalogue: unknown/REA, port level 3)
- Name proposal: sibling of CTaskComplexUseEffectRunning (0x636780) with move state 7 (SPRINT); vtable 0x86E5C0. Caller: CTaskComplexUseClosestFreeScriptedAttractor::CreateFirstSubTask 0x639530.
- `__thiscall T* (this, a1 [esp+4], a2 [esp+8])`, `ret 8`: `CTaskComplexUseEffect::CTaskComplexUseEffect(this, a1, a2)` 0x6321F0; `vtbl = 0x86E5C0; +0x1C = 7`; return this.

## 0x636B40 vt_86e1dc::Clone (proposed: CTaskSimpleWaitForPizza::Clone)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86E1DC.
- Behaviour: `p = CTask::operator new(0x14)`; if NULL return 0. Else CTaskSimple ctor 0x61A390; `p->vtbl = 0x86E1DC; p+8 = 0; p+0xC = 0; p+0x10 = 0 (u8); p+0x11 = 0 (u8)`; return p.

## 0x6396C0 sub_6396c0 (proposed: CTaskComplexUseSequence::~CTaskComplexUseSequence (non-deleting))  (catalogue: unknown/REA, port level 2)
- Name proposal: complete dtor. Evidence: stores vtable 0x86E518 (TASK_COMPLEX_USE_SEQUENCE); caller 0x639F90 (scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame.
- Behaviour: `vtbl = 0x86E518`; `id = this->+0xC (m_nSequenceIndex)`; `if id != -1` (0x6396E8): `s = &CTaskSequences::ms_taskSequence[id]` (0xC178F0 + id*0x40); `if --s->+0x3C (m_nReferenceCount) == 0` (`dec; jne` 0x6396F9) and `s->+0x38 (m_bFlushTasks, u8) != 0`: `s->+0x38 = 0`, `CTaskComplexSequence::Flush(s)` 0x632C10 (0x639708). Then CTaskComplex::~CTaskComplex 0x61A3D0. No bounds check on id.

## 0x6402F0 CTaskComplexEnterCarAsDriver (proposed: CTaskComplexEnterCarAsDriver::CTaskComplexEnterCarAsDriver(CVehicle*))  (catalogue: specified/REA, port level 3)
- `__thiscall T* (this, CVehicle* veh [esp+4])`, `ret 4`. 13 callers (CCarAI::UpdateCarAI 0x41DA30, CPlayerInfo::Process 0x56F8D0, many CreateSubTask).
- Behaviour: `CTaskComplexEnterCar::CTaskComplexEnterCar(this, veh, asDriver = 1, quitAfterOpeningDoor = 0, quitAfterDraggingPedOut = 0, carryOnAfterFallingOff = 0)` 0x63A220 (pushes 0,0,0,1,veh); `vtbl = 0x86EAAC`; return this.

## 0x640340 CTaskComplexEnterCarAsPassenger (proposed: CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32 targetSeat, bool carryOnAfterFallingOff))  (catalogue: specified/REA, port level 3)
- `__thiscall T* (this, CVehicle* veh [esp+4], int32 targetSeat [esp+8], bool carryOn [esp+0xC])`, `ret 0xC`. 15 callers.
- Behaviour: `CTaskComplexEnterCar(this, veh, 0, 0, 0, carryOn)` 0x63A220; `vtbl = 0x86EADC; +0x1C (m_nTargetSeat) = targetSeat`; return this.

## 0x640390 sub_640390 (proposed: CTaskComplexOpenDriverDoor::CTaskComplexOpenDriverDoor(CVehicle*))  (catalogue: unknown/REA, port level 3)
- Evidence: vtable 0x86EB0C (TASK_COMPLEX_CAR_OPEN_DRIVER_DOOR). Caller: CTaskComplexArrestPed::CreateSubTask 0x68CF80.
- `__thiscall T* (this, CVehicle* veh [esp+4])`, `ret 4`: `CTaskComplexEnterCar(this, veh, 1, 1, 0, 0)` 0x63A220; `vtbl = 0x86EB0C; +0x1C = 0xA` (target door 10); return this.

## 0x6403E0 sub_6403e0 (proposed: CTaskComplexOpenPassengerDoor::CTaskComplexOpenPassengerDoor(CVehicle*, int32 seat))  (catalogue: unknown/REA, port level 3)
- Evidence: vtable 0x86EB3C (TASK_COMPLEX_CAR_OPEN_PASSENGER_DOOR). Caller: CTaskComplexArrestPed::CreateSubTask 0x68CF80.
- `__thiscall T* (this, CVehicle* veh [esp+4], int32 seat [esp+8])`, `ret 8`: `CTaskComplexEnterCar(this, veh, 0, 1, 0, 0)` 0x63A220; `vtbl = 0x86EB3C; +0x1C = seat`; return this.

## 0x642700 sub_642700 (proposed: CTaskSimpleCarDrive::StartCloseDoorAnim)  (catalogue: unknown/REA, port level 8)
- Name proposal: UNCLEAR beyond "start the anim 0x182 and arm 0x63C840". Caller: CTaskSimpleCarDrive::ProcessPed 0x644470.
- `__thiscall void (this: CTaskSimpleCarDrive*, CPed* ped [esp+4])`, `ret 4`.
- Behaviour: `veh = ped->m_pVehicle (+0x58C)`; `grp = CVehicleAnimGroupData::m_vehicleAnimGroups[veh->m_pHandlingData (+0x384)->+0xDE].GetGroup(0x182)` 0x6E3B00 (0x64272C); `a = CAnimManager::AddAnimation(ped->m_pRwClump (+0x18), grp, 0x182)` 0x4D3AA0 (cdecl, 0x64273B); `this->+0xC = a`; `a->SetFinishCallback(0x63C840, this)` 0x4CEBE0. No NULL checks.

## 0x6497A0 SayJacked  (catalogue: specified/REA, port level 6)
- `__cdecl bool (CPed* jacked [esp+4], CVehicle* vehicle [esp+8], uint32 timeDelay [esp+0xC])`, plain `ret`; returns AL. Callers: CTaskSimpleBikeJacked::ProcessPed 0x64C970, CTaskSimpleCarSlowDragPedOut::ProcessPed 0x64E060 (2 sites each).
- Behaviour: `t = CAEVehicleAudioEntity::GetVehicleTypeForAudio(&vehicle->m_vehicleAudio (+0x138))` 0x4F4F00 (int16 in AX); `ctx = (t == 0 (CAR)) ? 0x76 (CTX_GLOBAL_JACKED_CAR) : 0x77 (CTX_GLOBAL_JACKED_GENERIC)` (`test ax, ax; jne` 0x6497AF/0x6497BD - every non-zero type, including BIKE and GENERIC, takes the generic line); `r = CPed::Say(jacked, ctx, timeDelay, 1.0f, 0, 0, 0)` 0x5EFFE0 (thiscall; 1.0f pushed as 0x3F800000); return `(int16)r != -1` (`cmp ax, 0xFFFF; setne al`).
- gta-reversed (Ped.cpp:4052-4062): switch with CAR -> JACKED_CAR, BIKE/GENERIC -> JACKED_GENERIC, `default: NOTSA_UNREACHABLE()` (gr-deviations row Ped.cpp:4060). Original bytes: no default trap, any other value says JACKED_GENERIC (0x6497D7). Verdict: CONFIRMED structural difference, only reachable for audio types outside 0..2 (eAEVehicleAudioType has 3 values). Class: harmless (port keeps the original two-way test).

## 0x6497F0 SayJacking  (catalogue: specified/REA, port level 6)
- `__cdecl bool (CPed* jacker [esp+4], CPed* jacked [esp+8], CVehicle* vehicle [esp+0xC], uint32 timeDelay [esp+0x10])`, plain `ret`; returns AL. Callers: as SayJacked.
- Behaviour: `a = &vehicle->m_vehicleAudio (+0x138)`. `if GetVehicleTypeForAudio(a) == 1` (BIKE, 0x649802) -> `ctx = 0x79` (JACKING_BIKE). Else call GetVehicleTypeForAudio(a) AGAIN (0x64982E); `if == 0` (CAR): `fem = jacked->m_pedSpeech (+0x294)->vfunc[+0x1C]()` (IsPedFemaleForAudio, thiscall on jacked+0x294, 0x649848); `ctx = fem ? 0x7A (JACKING_CAR_FEM) : 0x7B (JACKING_CAR_MALE)`. Any other value -> `ctx = 0x7C` (JACKING_GENERIC, 0x64988C). `return CPed::Say(jacker, ctx, timeDelay, 1.0f, 0, 0, 0) != -1` (16-bit compare).
- gta-reversed (Ped.cpp:4065-4078): same mapping but `default: NOTSA_UNREACHABLE()` (row Ped.cpp:4076) and a single type query. Original: no trap (other values -> JACKING_GENERIC), type queried twice (no observable effect if the getter is pure). Verdict: CONFIRMED structural difference, harmless.

## 0x649EE0 vt_86ee94::Clone (proposed: CTaskSimpleCarForcePedOut::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x86EE94 (TASK_SIMPLE_CAR_FORCE_PED_OUT). `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x10)`; if NULL return 0. Else `0x647710(p, this->+8, this->+0xC)` (the CTaskSimpleCarForcePedOut ctor (vehicle, door), thiscall) (0x649F1D); return its EAX.

## 0x655290 sub_655290 (proposed: CTaskComplexWalkRoundObject::CreateSubTask)  (catalogue: unknown/REA, port level 4)
- Name proposal: CreateSubTask of CTaskComplexWalkRoundObject (caller CTaskComplexWalkRoundObject::ControlSubTask 0x6575F0; calls CreateRouteTask 0x655140 on this).
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame.
- Behaviour: `if taskType == 0xCB` (TASK_SIMPLE_STAND_STILL) -> `p = CTask::operator new(0x20)`; NULL -> return 0; else return `CTaskSimpleStandStill(p, 500 (0x1F4), false, false, 8.0f)` 0x62F310. `if taskType == 0x389` (TASK_COMPLEX_FOLLOW_POINT_ROUTE) -> return `CTaskComplexWalkRoundObject::CreateRouteTask(this, ped)` 0x655140. Anything else (including 0x516) -> return 0.

## 0x6563D0 sub_6563d0 (proposed: CTaskSimpleKillPedWithCar::ShakePlayerPad)  (catalogue: unknown/REA, port level 1)
- Name proposal: pad-shake helper of CTaskSimpleKillPedWithCar (caller ProcessPed 0x6564A0; this+8 = the vehicle).
- `__thiscall void (this)`, plain `ret`.
- Behaviour: `veh = this->+8`; `if CWorld::FindPlayerVehicle(-1, false) != veh` 0x56E0D0 -> return. `inv = [0x858624]=1.0f / veh->m_fMass (+0x8C)` (fdiv, stored to an f32 local). `s = sqrt(vx*vx + vy*vy + vz*vz)` with v = veh->m_vecMoveSpeed (+0x44/+0x48/+0x4C), x87 order `((x*x + y*y) + z*z)`, fsqrt; `f = s * inv * [0x86C688]=200000.0f + [0x858A14]=80.0f`. `fcomp [0x859F80]=250.0f; test ah, 0x41; jne`: if `f > 250` -> `f = 250.0f`; otherwise (less, equal, NaN) the same expression is recomputed and used. `freq = (u8)_ftol2(f)` 0x821B40; `CPad::StartShake(CPad::GetPad(0) 0x53FB70, (int16)(40000 / freq) (`idiv`, signed, freq zero-extended), freq, 0)` 0x53F920. The u8 freq is passed in a dword whose upper 3 bytes are leftovers of the `inv` local (callee reads only the byte). freq is never 0 for finite non-negative values (>= 80); a NaN/negative mass would make `idiv` fault (original behaviour).

## 0x6586D0 vt_86f45c::Clone (proposed: CTaskSimpleAbseil::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1 of vtable 0x86F45C. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x20)`; if NULL return 0. Else same construction as 0x6586A0 on p (CTaskSimpleRunAnim Constructor2 0x61A900 with (0, 0x98, 4.0f, 0x1A7, "Abseil", 0); `vtbl = 0x86F45C`); return p. Nothing copied from this.

## 0x6588A0 sub_6588a0 (proposed: CTaskSimpleGiveCPR::~CTaskSimpleGiveCPR (non-deleting))  (catalogue: unknown/REA, port level 1)
- Evidence: stores vtable 0x86F4C8 (TASK_SIMPLE_GIVE_CPR); caller 0x659E10 CTaskSimpleGiveCPR::~CTaskSimpleGiveCPR (scalar deleting).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x86F4C8`; `if +0x10 (m_pAnim)` -> `m_pAnim->SetFinishCallback(0x4D1390 (empty), NULL)` 0x4CEBE0 (0x6588D9); CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x6592A0 sub_6592A0 (proposed: CTaskComplexPresentIDToCop::ComputeHeadingToCop)  (catalogue: unknown/REA, port level 1)
- Name proposal: helper of CTaskComplexPresentIDToCop (this +0xC = cop). Caller: CTaskComplexPresentIDToCop::CreateSubTask 0x65A110.
- `__thiscall float (this, CPed* ped [esp+4])`, `ret 4`; returns ST0.
- Behaviour: `P = ped->m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &ped->m_placement.pos (+4)` (0x6592A4-0x6592B3/0x6592D2); `C = cop (this->+0xC)` position chosen the same way (the matrix test is evaluated twice, 0x6592B9/0x6592C6). `a = CGeneral::GetRadianAngleBetweenPoints(C.x, C.y, P.x, P.y)` 0x53CBE0 (cdecl, 4 floats pushed as raw dwords in the order P.y, P.x, C.y, C.x); `fstp` to a temp f32, `return CGeneral::LimitRadianAngle(temp)` 0x53CB50 (ST0). No NULL check on the cop.

## 0x6594D0 sub_6594d0 (proposed: CTaskComplexUseSwatRope::~CTaskComplexUseSwatRope (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: stores vtable 0x86F59C (TASK_COMPLEX_USE_SWAT_ROPE); caller 0x65A3C0 (scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x86F59C`; `if (+0x14 & 1) && +0x18 (heli) != NULL` -> CleanUpOldReference(+0x18 entity, &this->+0x18) 0x571A00 (0x65950C); CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x6598C0 vt_86f5fc::CreateNextSubTask (proposed: CTaskComplexExtinguishFireOnFoot::CreateNextSubTask)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 8 of vtable 0x86F5FC.
- Behaviour: `t = m_pSubTask->GetTaskType()`. `t == 0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL) -> `p = CTask::operator new(0x3C)`; NULL -> 0; else return `CTaskSimpleGunControl::CTaskSimpleGunControl(p, NULL, &this->+0xC (fire position), NULL, 4, 1, -1)` 0x61F3F0 (pushes -1, 1, 4, 0, this+0xC, 0). `t == 0x3FC` (TASK_SIMPLE_GUN_CTRL) -> return `this->CreateFirstSubTask(ped)` (vslot +0x24). Else return 0.

## 0x659B60 vt_86f544::Clone (proposed: CTaskComplexPresentIDToCop::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x86F544. `p = CTask::operator new(0x10)`; if NULL return 0; else `CTaskComplexPresentIDToCop::CTaskComplexPresentIDToCop(p, this->+0xC)` 0x6591D0; return p. SEH frame, plain `ret`.

## 0x659D70 vt_86f5fc::Clone (proposed: CTaskComplexExtinguishFireOnFoot::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1 of vtable 0x86F5FC. `p = CTask::operator new(0x18)`; if NULL return 0; else CTaskComplex ctor, `p->vtbl = 0x86F5FC`, `p+0xC..+0x14 = this+0xC..+0x14` (3 dwords); return p. SEH frame, plain `ret`.

## 0x659E00 vt_86f45c::~dtor-thunk (proposed: CTaskSimpleAbseil::~CTaskSimpleAbseil)  (catalogue: unknown/REA, port level 2)
- Name proposal: non-deleting dtor of CTaskSimpleAbseil (vtable 0x86F45C, see 0x6586A0); tail-jumps to the CTaskSimpleAnim dtor (CTaskSimpleRunAnim adds nothing to destroy).
- Tail-jump thunk (0x659E00): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x65A720 vt_86f5fc::CreateFirstSubTask (proposed: CTaskComplexExtinguishFireOnFoot::CreateFirstSubTask)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 9 of vtable 0x86F5FC.
- Behaviour: `f = 0x659990(ped)`; NULL -> return 0. `this->+0xC..+0x14 = f->m_vecPosition` (3 dwords); `p = CTask::operator new(0x28)`; NULL -> 0; return `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(p, 6 (move state), &f->m_vecPosition, 2.0f (immediate 0x40000000), [0x86FC88]=2.0f, false, false)` 0x668120.

## 0x65AFE0 vt_86f544::CreateNextSubTask (proposed: CTaskComplexPresentIDToCop::CreateNextSubTask)  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 8 of vtable 0x86F544.
- Behaviour: `t = m_pSubTask->GetTaskType()` (vslot +0x10). If `t == 0x19D` (TASK_SIMPLE_HANDS_UP) -> return `CreateSubTask(this, 0x516 TASK_FINISHED, ped)` 0x65A110. If `t == 0x386` (TASK_SIMPLE_ACHIEVE_HEADING) -> `CPed::Say(ped, 0x45, 0, 1.0f, 0, 0, 0)` 0x5EFFE0 (result ignored), then return `CreateSubTask(this, 0x19D, ped)`. Else return 0.

## 0x65B050 vt_86f544::CreateFirstSubTask (proposed: CTaskComplexPresentIDToCop::CreateFirstSubTask)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: return `CreateSubTask(this, 0x386 TASK_SIMPLE_ACHIEVE_HEADING, ped)` 0x65A110 (ECX unchanged).

## 0x65B070 vt_86f544::ControlSubTask (proposed: CTaskComplexPresentIDToCop::ControlSubTask)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: `if this->+0xC (cop) == NULL` -> return `CreateSubTask(this, 0x516 TASK_FINISHED, ped)` 0x65A110; else return `m_pSubTask` (+8).

## 0x661A60 CTaskComplexProstituteSolicit (proposed: CTaskComplexProstituteSolicit::CTaskComplexProstituteSolicit(CPed* client))  (catalogue: specified/REA, port level 2)
- `__thiscall T* (this, CPed* client [esp+4])`, `ret 4`; SEH frame. Callers: CEventHandler::ComputePedFriendResponse 0x4B9DD0, Clone 0x6622F0.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0x1C = 0; +0x20 = 0; +0x24 = 0; +0x2A = 0 (u16)`; `vtbl = 0x86FB88`; `+0xC = client`; `+0x28 = 0x352 (u16, 850)`; `+0x2C (u16) = (+0x2C & 0xE2C0) | 0x2C0` (bits 6, 7, 9 set; bits 13-15 keep their (uninitialised) value; all others cleared) (0x661A9B-0x661ABD); `CEntity::RegisterReference(client, &this->+0xC)` 0x571B70 - not null-checked.

## 0x667460 sub_667460 (proposed: string IsEmpty helper (char* this))  (catalogue: unknown/REA, port level 0)
- `__thiscall bool (const char* this)`, plain `ret`. No callers.
- Behaviour: compares 1 byte of `this` with the byte at 0x858B54 (= 0, an empty string) via `repe cmpsb` with ECX = 1; returns AL = 1 iff `this[0] == 0` (0x66747D/0x66747F), else 0.

## 0x667900 sub_667900 (proposed: CNodeRoute::Reverse)  (catalogue: unknown/REA, port level 0)
- Name proposal: in-place reversal of a `{u32 count @+0; u32 entries[] @+4}` route (CNodeRoute = notsa::Route<CNodeAddress>). Caller: CTaskComplexFollowNodeRoute::ComputePathNodes 0x66EFA0.
- `__thiscall void (this)`, plain `ret`.
- Behaviour: `j = count - 1`; if `(int32)j <= 0` return. `i = 0`; loop: swap `e[i]` and `e[j]` (dwords); `i++; j--`; while `i < j` (signed, 0x667923).

## 0x669D30 sub_669d30 (proposed: CTaskComplexWander::SetDirection)  (catalogue: unknown/REA, port level 0)
- Name proposal: setter on a CTaskComplexWander-layout object (+0x10 m_nDir u8, +0x24 flags bit 1 = "new direction"); caller CTaskComplexSmartFleePoint::ControlSubTask 0x65C1E0 (calls it on its wander subtask).
- `__thiscall void (this, int32 dir [esp+4])`, `ret 4`.
- Behaviour: `if (u32)(u8)this->+0x10 != (u32)dir` (full 32-bit arg compared with the zero-extended byte, 0x669D38) -> `this->+0x10 = (u8)dir; this->+0x24 |= 2`.

## 0x669D50 sub_669d50 (proposed: CTaskComplexWander::CopyRouteFrom)  (catalogue: unknown/REA, port level 0)
- Name proposal: copies the node pair and direction from another wander task. Evidence: CTaskComplexWander layout (+0x10 m_nDir u8, +0x18/+0x1C two CNodeAddress {u16 area, u16 node}, +0x24 flags); caller CTaskComplexSmartFleePoint::SetDefaultTaskWanderDir 0x65BE00.
- `__thiscall void (this, const CTaskComplexWander* other [esp+4])`, `ret 4`.
- Behaviour: compare the u16 words +0x18, +0x1A, +0x1C, +0x1E of this and other in that order; if all four are equal -> return. Otherwise: `this+0x18 = other+0x18` (dword), `this+0x1C = other+0x1C` (dword), `this+0x10 = other+0x10` (u8), `this+0x24 |= 4` (0x669D7C-0x669D8E).

## 0x66AD40 vt_86ff00::MakeAbortable (proposed: CTaskComplexAvoidEntity::MakeAbortable)  (catalogue: specified/REA, port level 2)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority, const CEvent* event)`, `ret 0xC`. Slot 6 of vtable 0x86FF00.
- Behaviour: same inline body as 0x66AD10 (if +0x50 & 1 and g_ikChainMan.IsLooking(ped) -> AbortLookAt(ped, 250)); return AL = 1 always (the subtask is not asked).

## 0x66AD70 vt_86ff00::CreateNextSubTask (proposed: CTaskComplexAvoidEntity::CreateNextSubTask)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 8. Same IK abort block as 0x66AD10; returns EAX = 0 (task finishes).

## 0x66ADE0 vt_86ff00::ControlSubTask (proposed: CTaskComplexAvoidEntity::ControlSubTask)  (catalogue: specified/REA, port level 8)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 10 of vtable 0x86FF00.
- Behaviour:
  1. If `+0xC (entity) == NULL`: if `+0x50 & 1` and `g_ikChainMan.IsLooking(ped)` -> `AbortLookAt(ped, 250)`; return 0.
  2. `SetUpIK(this, ped)` 0x66AB40. If `ped->m_pIntelligence (+0x47C)->+0x274 (int32, gr m_AnotherStaticCounter) > [0x86C938]=30` (signed `jle` continues) -> same IK abort as 1 and return 0.
  3. `d = pedPos - entityPos` (dx, dy, dz); `q = (dz*dz + dx*dx) + dy*dy`; `t = [0x86C8FC]=7.5f * 7.5f * [0x858B90]=4.0f` (= 225.0, computed at run time in this order); `fcompp` compares t (ST0) with q: if `t < q` (only C0; `test ah, 5; jp` falls through) -> `QuitIK(this, ped)` 0x66AD10 and return 0. Otherwise (q <= 225 or NaN) return `m_pSubTask` (+8).

## 0x66BA70 sub_66BA70 (proposed: CPatrolRoute scalar deleting destructor)  (catalogue: unknown/REA, port level 1)
- Name proposal: CPatrolRoute scalar deleting dtor: 0x6677A0 = CPatrolRoute::~CPatrolRoute (destroys the 8 x 0x28 entries), 0x41B820 = CPatrolRoute::operator delete (pool free). No callers.
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006677a0 0x6677A0 (0x66BA73); if `flags & 1` (0x66BA78) calls thunk_FUN_0156a220 0x41B820 (this) (0x66BA80, cdecl, caller pops); returns this in EAX.

## 0x66BA90 vt_870114::MakeAbortable (proposed: CTaskComplexFollowPatrolRoute::MakeAbortable)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x870114. Fields: +0x1C CPatrolRoute*, +0x20 flags, +0x24 CVector.
- Behaviour: `if priority == 0` -> `*(u32*)this->+0x1C = 0` (the route's first dword, its point count, is zeroed - route cleared; no NULL check). `this->+0x24..+0x2C = ped position` (matrix pos or placement, 3 dwords). `r = m_pSubTask->MakeAbortable(ped, priority, event)` (vslot +0x18); `+0x20 = (+0x20 & ~2) | ((r & 1) << 1)` (xor/and/xor merge, 0x66BAD7-0x66BAE8); return `(+0x20 >> 1) & 1` (= r & 1) in EAX.

## 0x66BB00 vt_870114::ControlSubTask (proposed: CTaskComplexFollowPatrolRoute::ControlSubTask)  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 10 of vtable 0x870114 (TASK_COMPLEX_FOLLOW_PATROL_ROUTE).
- Behaviour: `if this->+0x20 & 3` (route-changed flags): `this->+0x12 = 0 (u16)` (0x66BB0E) and tail-jump to `this->CreateFirstSubTask(ped)` (vslot +0x24, `jmp [eax+0x24]` with the caller's stack args, 0x66BB14). Else return `m_pSubTask` (+8).

## 0x66C280 sub_66C280 (proposed: UNCLEAR::CreateFirstSubTask - go to the midpoint of a 2D segment)  (catalogue: unknown/REA, port level 2)
- Name proposal: UNCLEAR; no callers and no data references (dead code). Shape: complex-task slot 9 (`ret 4`, ped unused).
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame.
- Behaviour: `s = this->+0xC`; local `P.x = (s+0x10 + s+8) * [0x858B8C]=0.5f` (fld [s+0x10]; fadd [s+8]; fmul; fstp f32); `P.y = (s+0x14 + s+0xC) * 0.5f`; `P.z = s+0x18` (raw dword). `p = CTask::operator new(0x30)`; NULL -> 0; return `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(p, 6, &P, 0.5f, false, false)` 0x667CD0.

## 0x66CB30 vt_87004c::MakeAbortable (proposed: CTaskComplexSeekEntity<...>::MakeAbortable)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x87004C (TASK_COMPLEX_SEEK_ENTITY).
- Behaviour: `if priority == 0` (ABORT_PRIORITY_LEISURE): `+0x10 = -1`; second timer `+0x34 = now ([0xB7CB84]); +0x38 = -1; +0x3C = 1 (u8)` (0x66CB3B-0x66CB4D). Then `r = m_pSubTask->MakeAbortable(ped, priority, event)` (vslot +0x18, 0x66CB62); `if r` -> first timer `+0x28 = now; +0x2C = 0; +0x30 = 1 (u8)`. Return r.

## 0x66D0C0 vt_86ff00::Clone (proposed: CTaskComplexAvoidEntity::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x86FF00. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x54)`; if NULL return 0. Else `0x66AA20(p, this->+0x10, this->+0xC, &this->+0x20)` (CTaskComplexAvoidEntity ctor (moveState?, entity, const CVector* pos), thiscall, 0x66D101); return its EAX.

## 0x66D4B0 sub_66D4B0 (proposed: CPatrolRoute::operator=(const CPatrolRoute&))  (catalogue: unknown/REA, port level 0)
- Name proposal: copy of a CPatrolRoute (`u32 count @+0; 8 x {char animName[24]; char animBlock[16]} @+4 stride 0x28; CVector pos[8] @+0x144`). Caller: CTaskComplexFollowPatrolRoute::CTaskComplexFollowPatrolRoute 0x674930.
- `__thiscall void (this, const CPatrolRoute* src [esp+4])`, `ret 4`. The loop counter init `i = 0` lives in a .HOODLUM detour (0x66D4C0 -> 0x401764 -> 0x66D4C5).
- Behaviour: `this->count = src->count` (0x66D4BB); if `(int32)count <= 0` return. For i = 0 .. this->count-1 (count re-read from this each iteration, signed compare 0x66D539): `this->pos[i] = src->pos[i]` (3 dwords at +0x144 + i*0xC); strcpy(this+4+i*0x28, src+4+i*0x28) (unbounded byte loop incl. terminator); strcpy(this+0x1C+i*0x28, src+0x1C+i*0x28).

## 0x66D550 sub_66D550 (proposed: CPatrolRoute::Reverse)  (catalogue: unknown/REA, port level 0)
- Name proposal: in-place reversal of a CPatrolRoute (`u32 count @+0; 8 x {char animName[24]; char animBlock[16]} @+4 stride 0x28; CVector pos[8] @+0x144`). Caller: CTaskComplexFollowPatrolRoute::GetNextSubTaskType 0x670A70 (route modes 1/2).
- `__thiscall void (this: CPatrolRoute*)`, plain `ret`; 0x44 bytes of locals (one CVector temp, two string temps of 24 and 16 bytes).
- Behaviour: `j = count - 1`; if `(int32)j <= 0` return. `i = 0`; loop while `i < j` (signed; `j` decremented and `i` incremented per iteration, 0x66D683-0x66D6A1): swap `pos[i]` and `pos[j]` (3 dwords); swap the two strings of entry i and entry j via unbounded byte-copy loops (strcpy semantics): temp <- i, i <- j, j <- temp, separately for `animName` (+0) and `animBlock` (+0x18).

## 0x670D80 vt_870114::CreateNextSubTask (proposed: CTaskComplexFollowPatrolRoute::CreateNextSubTask)  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. Slot 8 of vtable 0x870114.
- Behaviour (GetTaskType of m_pSubTask is called anew for each test): `t == 0x38A` (TASK_COMPLEX_FOLLOW_NODE_ROUTE) -> return `CreateSubTask(this, GetNextSubTaskType(this))` (0x670A70 then 0x670B00, thiscall 1 arg) without advancing. `t == 0xC8` (TASK_NONE) -> return `CreateSubTask(this, 0x516)`. `t == 0x191` (TASK_SIMPLE_NAMED_ANIM) -> `+0x12 += 1` (u16) then return `CreateSubTask(this, GetNextSubTaskType(this))`. Otherwise (arrived at a point): if the anim name of the current point `route+4+i*0x28` is empty (first byte compared with the empty string 0x858B54) -> `+0x12 += 1` and continue as for 0x191; else return `CreateSubTask(this, 0x191)` (play the point's named anim).

## 0x671330 vt_87004c::Clone (proposed: CTaskComplexSeekEntity<CEntitySeekPosCalculatorFixedPos>::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x87004C. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x58)`; if p != NULL: `EAX = CTaskComplexSeekEntity(p, +0xC entity, +0x10, +0x14, +0x18, +0x1C, +0x24, (u32)(u8)+0x54 & 1 (bPlayTiredAnim), 1)` 0x66CA40 - NOTE the last argument (bFaceEntityWhenDone) is the constant 1, not copied; else EAX = 0. Then OUTSIDE the NULL check (0x671395): `EAX+0x44..+0x4C = this+0x44..+0x4C` (calculator offset vector); `EAX+0x50 = this+0x50` (move state); `EAX+0x54 bit 3 = this+0x54 bit 3` (xor/and/xor); `EAX+0x20 = this+0x20` (fld/fstp f32); `EAX+0x50 = this+0x50` (again). Original bug (keep): on allocation failure these writes target addresses 0x20..0x54.
- gta-reversed: copy ctor passes the source's bFaceEntityWhenDone; the bytes pass true. Port follows the bytes.

## 0x672530 vt_86ff00::CreateFirstSubTask (proposed: CTaskComplexAvoidEntity::CreateFirstSubTask)  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 9 of vtable 0x86FF00.
- Behaviour: `this->+0x14..+0x1C = ped position` (matrix pos or placement); `ok = ComputeAvoidRoute(this, ped)` 0x66FD30 (AL). If !ok: `if (+0x50 & 1) && g_ikChainMan.IsLooking(ped)` -> `g_ikChainMan.AbortLookAt(ped, 250)`; return 0. Else `p = CTask::operator new(0x30)`; NULL -> 0; return `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(p, this->+0x10 (move state), &this->+0x2C (first route point), [0x86FC78]=0.5f, false, false)` 0x667CD0.

## 0x672E80 vt_87004c::CreateNextSubTask (proposed: CTaskComplexSeekEntity<CEntitySeekPosCalculatorFixedPos>::CreateNextSubTask)  (catalogue: specified/REA, port level 10)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame (a local CTaskSimpleStandStill). Slot 8 of vtable 0x87004C. "Create(t)" = `CreateSubTask(this, t, ped)` 0x673070; "First" = `this->CreateFirstSubTask(ped)` (vslot +0x24). Fields as in 0x66CA40/0x6733E0.
- Behaviour: if `m_entityToSeek (+0xC) == NULL` -> `Create(0x516)`. Else by `t = m_pSubTask->GetTaskType()`:
  - `0x2CF` (CAR_DRIVE_TIMED) -> First.
  - `0xF4` (SEQUENCE) or `0x398` (TURN_TO_FACE_ENTITY) -> `Create(0x516)`.
  - `0xCB` (STAND_STILL): `ped->+0xFC (m_pAttachedTo) != NULL` -> `Create(0xCB)`; else First.
  - `0xDB` (TIRED): local `CTaskSimpleStandStill(-1, false, false, 8.0f)`, `ProcessPed(local, ped)` 0x62F370, destroyed 0x62F360; then `Create(0x516)`.
  - `0x2C0` (LEAVE_CAR): `ped->bInVehicle (+0x46C & 0x100)` -> `Create(0x516)`; else `Create(0x2CF)`.
  - `0x387` or `0x38A`: `calc->ComputeEntitySeekPos(ped, entity, &L)` (vslot 0 of +0x40); `q = (P.x - L.x)^2 + (P.y - L.y)^2` (2D, x term first); `m = m_maxEntityDist2D (+0x18)`; `fcompp` q vs m*m with `test ah, 0x41; jne`: if `q <= m*m` (or NaN) -> reached: `+0x54 |= 4`; return `Create((+0x54 & 2) ? 0x398 : 0x516)` (bit 1 tested on the value before the OR). Else `r = +0x20`: if `r == 0.0` -> `Create(0x38A)`; else if `q > r*r` -> `Create(0x38A)`; else (q <= r*r or NaN) -> `Create(0x387)`.
  - anything else -> 0.

## 0x6733E0 vt_87004c::CreateFirstSubTask (proposed: CTaskComplexSeekEntity<CEntitySeekPosCalculatorFixedPos>::CreateFirstSubTask)  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 9 of vtable 0x87004C. CreateSubTask = 0x673070 (thiscall (type, ped)). Field names as in 0x66CA40 (+0x10 m_seekInterval, +0x14 m_scanInterval, +0x18 m_maxEntityDist2D, +0x20 float second radius, +0x28/+0x34 timers, +0x40 calculator, +0x54 flags).
- Behaviour:
  1. Scan timer: `+0x28 = now ([0xB7CB84]); +0x2C = m_scanInterval; +0x30 = 1`. If `m_seekInterval != -1` -> seek timer `+0x34 = now; +0x38 = m_seekInterval; +0x3C = 1`.
  2. If `m_entityToSeek (+0xC) == NULL` -> return `CreateSubTask(0xDB, ped)` (TASK_SIMPLE_TIRED per gr enum numbering).
  3. If `ped->bInVehicle && ped->m_pVehicle` -> return `CreateSubTask(0x2C0 TASK_COMPLEX_LEAVE_CAR, ped)`.
  4. `calc->ComputeEntitySeekPos(ped, entity, &L)` (vslot 0 of the calculator at +0x40); `P = ped position`; `q = (P.x - L.x)^2 + (P.y - L.y)^2` (x87: dx, dy; `dx*dx` then `+ dy*dy`, 2D only).
  5. `m = m_maxEntityDist2D`; if `q < m*m` (`fcompp` q vs m*m; `test ah, 5; jp` falls through only for C0) -> `+0x54 |= 4` (m_bAchievedSeekEntity); `CPed::SetMoveState(ped, 1)`; return `CreateSubTask(0xF4 TASK_COMPLEX_SEQUENCE, ped)`.
  6. Else `r = +0x20`: if `r == 0.0` (`test ah, 0x44; jnp` -> exactly C3) -> `CreateSubTask(0x38A TASK_COMPLEX_FOLLOW_NODE_ROUTE, ped)`. Otherwise (non-zero or NaN): if `q > r*r` (`test ah, 0x41; jne` not taken) -> `CreateSubTask(0x38A)`; else (q <= r*r or NaN) -> `CreateSubTask(0x387 TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL)`.

## 0x673EF0 sub_673EF0 (proposed: CTaskComplexWander::GetWanderTaskByPedType(ePedType))  (catalogue: unknown/REA, port level 4)
- Name proposal: static factory returning a new wander task for a ped type (unreferenced in our corpus; gr's GetWanderTaskByPedType takes a CPed* - this variant takes the type value).
- `__cdecl CTask* (int32 pedType [esp+4])`, plain `ret`; SEH frame. Each branch allocates with `CTask::operator new` and returns 0 if it fails. "dir" below = `CGeneral::GetRandomNumberInRange(0, 8)` 0x407180 (original int version).
- Behaviour: `k = pedType - 6`; if `(u32)k > 0x10` -> default; else index byte table 0x6740C4[k] into jump table 0x6740B0:
  - pedType 6 (COP): `new(0x50) CTaskComplexWanderCop(4, dir)` 0x460C80.
  - pedType 18 (MEDIC): `new(0x28) CTaskComplexWanderMedic(4, dir, true)` 0x658770.
  - pedType 20 (CRIMINAL): `new(0x38) CTaskComplexWanderCriminal(4, dir, true)` 0x48E610.
  - pedType 22 (PROSTITUTE): `new(0x3C) CTaskComplexWanderProstitute(4, dir, true)` 0x672690.
  - default (every other value): `new(0x38)`; `d = _ftol2((float)(_rand() & 0xFFFF) * [0x858B14]=3.0517578e-05f * [0x859000]=8.0f)` (rand drawn only after a successful allocation); return `CTaskComplexWanderStandard(p, 4, (u8)d, true)` 0x48E4F0.

## 0x674BB0 vt_870114::Clone (proposed: CTaskComplexFollowPatrolRoute::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1 of vtable 0x870114. `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x30)`; if NULL return 0. Else `CTaskComplexFollowPatrolRoute::CTaskComplexFollowPatrolRoute(p, (int32)(int16)this->+0x10, this->+0x1C (route), (int32)(int16)this->+0xC, this->+0x14, this->+0x18)` 0x674930 (0x674BFB); return its EAX.

## 0x674EA0 sub_674ea0 (proposed: CEventInteriorUseInfo::CEventInteriorUseInfo)  (catalogue: unknown/REA, port level 1)
- Evidence: CEvent ctor + vtable 0x8701A8 (slot 1 returns EVENT_INTERIOR_USE_INFO); caller CTaskInteriorBeInShop::ScanForStuff 0x6754D0 (2 sites).
- `__thiscall T* (this, a1 [esp+4], a2 [esp+8], a3 [esp+0xC], uint8 a4 [esp+0x10])`, `ret 0x10`: `CEvent::CEvent(this)` 0x4ABFC0; `+0xC = a1; +0x18 = (u8)a4; vtbl = 0x8701A8; +0x10 = a2; +0x14 = a3`; return this.

## 0x674F00 vt_8701a8::Clone (proposed: CEventInteriorUseInfo::Clone)  (catalogue: specified/REA, port level 2)
- Slot 4 of vtable 0x8701A8 (event Clone). `__thiscall CEvent* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CEvent::operator new(0x1C)` 0x4B5620 (event pool); if NULL return 0. Else read `+0x10, +0x18 (u8), +0x14, +0xC` of this; `CEvent::CEvent(p)` 0x4ABFC0; `p+0x14 = this+0x14; p+0x18 = this+0x18 (u8); p->vtbl = 0x8701A8; p+0xC = this+0xC; p+0x10 = this+0x10`; return p. (No reference registration: the fields are not entities.)

## 0x675CF0 vt_870314::Clone (proposed: CTaskInteriorSitOnChair::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1 of vtable 0x870314. `p = CTask::operator new(0x2C)`; if NULL return 0; else `CTaskInteriorSitOnChair::CTaskInteriorSitOnChair(p, this->+8, this->+0xC, (u32)(u8)this->+0x10)` 0x675C30; return its EAX. SEH frame, plain `ret`.

## 0x675D60 vt_870314::MakeAbortable (proposed: CTaskInteriorSitOnChair::MakeAbortable)  (catalogue: specified/REA, port level 2)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event)`, `ret 0xC`. Slot 6 of vtable 0x870314.
- Behaviour: `if priority == 2` (ABORT_PRIORITY_IMMEDIATE): `if g_ikChainMan.IsLooking(ped)` 0x6181A0 -> `g_ikChainMan.AbortLookAt(ped, 250)` 0x618280; `if m_pAnim (+0x14)`: `m_pAnim->m_BlendDelta (+0x1C) = -1000.0f` (immediate 0xC47A0000, 0x675D98), `m_pAnim->SetDeleteCallback(0x4D1390, NULL)` 0x4CEBC0, `m_pAnim = 0`; return 1. Else: `+0x2A = 1 (u8, abort requested)`; return 0.

## 0x678AA0 vt_870480::MakeAbortable (proposed: CTaskComplexFallAndStayDown::MakeAbortable)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x870480.
- Behaviour: `s = m_pSubTask` (+8). `priority == 2` -> tail-jump `s->MakeAbortable(ped, 2, event)` (vslot +0x18, the priority slot is rewritten with 2 = same value). `priority == 1` -> if `s->GetTaskType() == 0xCF` (TASK_SIMPLE_FALL) return 0; else return `s->MakeAbortable(ped, 1, event)`. Any other priority -> if `s->GetTaskType() != 0xCF` call `s->MakeAbortable(ped, priority, event)` (result ignored); return 0.

## 0x678B20 vt_870480::CreateFirstSubTask (proposed: CTaskComplexFallAndStayDown::CreateFirstSubTask)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame. Slot 9.
- Behaviour: `p = CTask::operator new(0x20)`; NULL -> 0. Else inline CTaskSimpleFall ctor: read `b = this+0x10 (time), a = this+0xC (animId)`; CTaskSimple ctor 0x61A390; `p+0x10 = b; p+0xC = a; p+0x18 = 0x5F5E0FF; p+0x1C = 0x5F5E0FF` (99999999); `p->vtbl = 0x870430` (CTaskSimpleFall); `p+8 = 0 (u8); p+0x14 = 0`; return p.

## 0x67BA40 CTaskComplexStuckInAir (proposed: CTaskComplexStuckInAir::CTaskComplexStuckInAir)  (catalogue: specified/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`. Caller: CEventHandler::ComputeEventResponseTask 0x4C3870.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x870608` (TASK_COMPLEX_STUCK_IN_AIR); return this.

## 0x67C270 vt_870480::Clone (proposed: CTaskComplexFallAndStayDown::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1 of vtable 0x870480. `p = CTask::operator new(0x14)`; if NULL return 0; else read `b = this+0x10, a = this+0xC`; CTaskComplex ctor; `p+0x10 = b; p->vtbl = 0x870480; p+0xC = a`; return p. SEH frame, plain `ret`.

## 0x67CBE0 vt_870480::CreateNextSubTask (proposed: CTaskComplexFallAndStayDown::CreateNextSubTask)  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. Slot 8 of vtable 0x870480.
- Behaviour: `if m_pSubTask->GetTaskType() == 0xCF` (TASK_SIMPLE_FALL) -> return `0x678BD0(this, 0x516 TASK_FINISHED)` (CreateSubTask, thiscall, 1 arg); else return 0.

## 0x683440 vt_8706f8::CreateNextSubTask (proposed: CTaskComplexWaitForPed::CreateNextSubTask)  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`: returns 0 (`xor eax, eax`, 0x683440). Slot 8 of vtable 0x8706F8.

## 0x683450 vt_8706f8::CreateFirstSubTask (proposed: CTaskComplexWaitForPed::CreateFirstSubTask)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame. Slot 9 of vtable 0x8706F8.
- Behaviour: `if this->+0xC (ped to wait for) == NULL` -> return 0 (0x68346B). Timer: `+0x1C = now ([0xB7CB84]); +0x20 = this->+0x14 (wait time); +0x24 = 1 (u8)`; `+0x28 = 0` (0x683482). `p = CTask::operator new(0x20)`; if NULL return 0; else return `CTaskSimpleStandStill::CTaskSimpleStandStill(p, 0xF423F (999999 ms), false, false, 8.0f)` 0x62F310 (pushes 8.0f as 0x41000000, 0, 0, 999999).

## 0x6834E0 vt_8706f8::ControlSubTask (proposed: CTaskComplexWaitForPed::ControlSubTask)  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame (local CEventDeadPed). Slot 10 of vtable 0x8706F8. Fields (ctor 0x683340): +0xC waited ped, +0x10 radius, +0x18 (u8) face-the-ped flag, +0x1C/+0x20/+0x24 CTaskTimer, +0x28 frame counter.
- Behaviour:
  1. If `+0xC == NULL` -> return 0. If `!CPed::IsAlive(+0xC)` 0x5E0170: local `CEventDeadPed(+0xC, false, now)` 0x4ADEA0 with its `+0x16 (u16) = 0x258`; `CEventGroup::Add(&ped->m_pIntelligence->+0x68, &e, false)`; `~CEventDeadPed` 0x4ADF70; return 0.
  2. If `timer.started (+0x24)` and `CTaskTimer::IsOutOfTime(this+0x1C)` -> return 0.
  3. `t = CPedIntelligence::FindTaskByType(waited->m_pIntelligence, 0x4B8)` 0x600EE0: if t -> `W = (*ped->m_matrix) * t->+0x14 (CVector)` (CMatrix::operator* 0x59C890, ped matrix without NULL check), `d = W - waitedPos`; else `d = pedPos - waitedPos` (positions: matrix pos or placement). `q = (dz*dz + dy*dy) + dx*dx`; `r = +0x10`; `fcompp` q vs r*r: if `q < r*r` (only C0) -> return 0 (close enough).
  4. If `+0x18 != 0 && +0x28 == 0`: `VectorSub(&D, waitedPos, pedPos)` 0x40FE60; `ped->+0x55C = CGeneral::LimitRadianAngle(GetRadianAngleBetweenPoints(D.x, D.y, 0, 0))` (f32 stores).
  5. `+0x28 += 1`; if `(int32)+0x28 > 10` -> `+0x28 = 0`. Return `m_pSubTask` (+8).

## 0x683950 vt_8706f8::Clone (proposed: CTaskComplexWaitForPed::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1. `p = CTask::operator new(0x2C)`; if NULL return 0; else `CTaskComplexWaitForPed::CTaskComplexWaitForPed(p, this->+0xC, this->+0x10, this->+0x14, (u32)(u8)this->+0x18)` 0x683340; return its EAX. SEH frame, plain `ret`.

## 0x6912C0 sub_6912c0 (proposed: CTaskSimpleAffectSecondaryBehaviour::~CTaskSimpleAffectSecondaryBehaviour (non-deleting))  (catalogue: unknown/REA, port level 1)
- Evidence: stores vtable 0x870B08 (TASK_SIMPLE_AFFECT_SECONDARY_BEHAVIOUR); caller 0x692FB0 (catalogued "CTaskSimple::~CTaskSimple", actually slot 0 of 0x870B08).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x870B08`; `if this->+0x10 (owned CTask*)` -> virtual slot 0 with flags 1 (delete, 0x6912F6); CTaskSimple::~CTaskSimple 0x61A3A0. Pointer not nulled.

## 0x691AE0 vt_870b98::CreateNextSubTask (proposed: CTaskComplexGoPickUpEntity::CreateNextSubTask)  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Slot 8 of vtable 0x870B98. Fields: +0xC entity, +0x10 CVector hold offset, +0x1C CVector, +0x28 time stamp, +0x2C anim id.
- Behaviour: if `+0xC == NULL` -> 0. By `t = m_pSubTask->GetTaskType()`:
  - `0x387` (GO_TO_POINT_AND_STAND_STILL): `dy = E.y - P.y` (f32), `dx = E.x - P.x` (f32) with E = entity position, P = ped position; `p = new(0x18)`; NULL -> 0; return `CTaskSimpleAchieveHeading(p, atan2(-dx, dy) (fchs/fpatan, f32), 1.0f, 0.001f)` 0x667E20.
  - `0x386` (ACHIEVE_HEADING): `blend = [0x870A80]=0.6f`. If `+0x2C == 0x51`: `h = E.z + entity->GetColModel()->bbox.min.z (+8)` (CEntity::GetColModel 0x535300; fld/fadd); if `h > P.z` (`fcom; test ah, 0x41; jne` not taken) -> `+0x2C = 0x53; blend = [0x870A88]=0.2f`; else if `P.z < h + [0x866CAC]=0.55f` (only C0) -> `+0x2C = 0x52; blend = [0x870A84]=0.26666668f`. `p = new(0x4C)`; `EAX = p ? CTaskSimplePickUpEntity(p, entity, &+0x10, 6, 1, 0x13D, +0x2C, blend) : 0` 0x6917B0; then OUTSIDE the NULL check `EAX->+0x40..+0x48 = this->+0x1C..+0x24` (original NULL-write bug); return EAX.
  - `0x134`: `+0x28 = now`; if `CTaskManager::GetTaskSecondary(&ped->m_pIntelligence->m_TaskMgr, 4) == NULL` -> `CTaskManager::SetTaskSecondary(taskMgr, new(0x3C) CTaskSimpleHoldEntity(entity, &+0x10, 6, 1, 0x13E, 0x51, false) (NULL if allocation fails), 4)` 0x6913A0 / 0x681B60. Return 0.
  - anything else -> 0.

## 0x691D50 vt_870b98::ControlSubTask (proposed: CTaskComplexGoPickUpEntity::ControlSubTask)  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Slot 10 of vtable 0x870B98. Fields: +0x28 time stamp, +0x2C anim group, +0x30 (u8) anim block referenced. "Abort" = `m_pSubTask->MakeAbortable(ped, 1, NULL)` (vslot +0x18) then return 0.
- Behaviour:
  1. Anim block: if `+0x2C != 0 && +0x30 == 0`: `blk = ms_aAnimAssocGroups[+0x2C].m_pAnimBlock` or, if NULL, `CAnimManager::GetAnimationBlock(GetAnimBlockName(+0x2C))`; `idx = (blk - 0xB5D4A0) >> 5`; if `blk->m_bLoaded (+0x10) != 0` -> `CAnimManager::AddAnimBlockRef(idx)`, `+0x30 = 1`; else `CStreaming::RequestModel(idx + 0x63E7, 8)`.
  2. `s = m_pSubTask`; if NULL return 0. By `s->GetTaskType()`:
     - `0x134`: if `s->+0x38 == 0 && s->+0x34 (u8) == 0 && (u32)now > +0x28 + [0x8D2FFC]` (dword global) -> Abort. Else return s.
     - `0x387`: if `(u32)now > +0x28 + [0x8D2FF8]` (dword global) -> Abort. If `ped->m_pPlayerData (+0x480) == NULL` or `pad = CPlayerPed::GetPadFromPlayer(ped)` 0x609560 is NULL -> return s. `ud = (float)(int16)CPad::GetPedWalkUpDown(pad)`, `lr = (float)(int16)GetPedWalkLeftRight(pad)`; `mag = fsqrt(ud*ud + lr*lr) * [0x858B88]=0.0078125f`. `a = atan2(-(T.x - P.x), T.y - P.y)` (T = s->+0x10 target point, P = ped position; f32). `b = GetRadianAngleBetweenPoints(0, 0, -lr, ud) - [0xB6F178]` (camera heading in TheCamera; f32). If `a + pi < b` -> `b -= 2pi`; else if `a - pi > b` -> `b += 2pi` (constants [0x858CB8]=pi, [0x858CBC]=2pi). If `CPad::JumpJustDown(pad)` or `CPad::SprintJustDown(pad)` -> Abort. If `mag > [0x858F34]=0.75f` and `b - a > [0x859AB0]=pi/4` -> Abort (only this direction is tested; the code then takes fabs of the integer 0/1 flag - no effect). Otherwise return s.
     - anything else -> return s.

## 0x692700 vt_870bec::MakeAbortable (proposed: CTaskSimpleDuckWhileShotsWhizzing::MakeAbortable)  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`. Slot 6 of vtable 0x870BEC.
- Behaviour: `priority == 2` -> return `CTaskSimpleDuck::MakeAbortable(this, ped, 2, event)` 0x692100. Else if `event != NULL && event->GetEventType() == 0x31` (EVENT_SHOT_FIRED_WHIZZED_BY, vslot +4): if `+0x10 (duck anim) == NULL` -> return 0; else `+0x10->SetFinishCallback(0x4D1390 (empty), NULL)` 0x4CEBE0, `+0x10 = 0`; `if +0x14 (second anim)` -> same and `+0x14 = 0`; `ped->+0x46C &= ~0x4000000` (bIsDucking cleared); return 1. Otherwise return `CTaskSimpleDuck::MakeAbortable(this, ped, priority, event)`.

## 0x692C80 vt_870b98::Clone (proposed: CTaskComplexGoPickUpEntity::Clone)  (catalogue: specified/REA, port level 3)
- Slot 1. `p = CTask::operator new(0x34)`; if NULL return 0; else `CTaskComplexGoPickUpEntity::CTaskComplexGoPickUpEntity(p, this->+0xC, this->+0x2C)` 0x6919C0; return its EAX. SEH frame, plain `ret`.

## 0x692D60 vt_870bec::Clone (proposed: CTaskSimpleDuckWhileShotsWhizzing::Clone)  (catalogue: specified/REA, port level 2)
- Slot 1. `p = CTask::operator new(0x28)`; if NULL return 0; else `0x692680(p, (u32)(u16)this->+0xC)`; return its EAX. SEH frame, plain `ret`.

## 0x694640 vt_870bec::ProcessPed (proposed: CTaskSimpleDuckWhileShotsWhizzing::ProcessPed)  (catalogue: specified/REA, port level 11)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Slot 7 of vtable 0x870BEC.
- Behaviour: `if this->+0x10 (duck anim) == NULL`: `a = CAnimManager::BlendAnimation(ped->m_pRwClump (+0x18), 0, 0x8D, 4.0f)` 0x4D4610 (group 0, anim 141, blend 4.0f pushed as 0x40800000); `this->+0x10 = a`; `a->SetDeleteCallback(CTaskSimpleDuck::DeleteDuckAnimCB 0x692550, this)` 0x4CEBC0. Then return `CTaskSimpleDuck::ProcessPed(this, ped)` 0x694390 (AL passed through).

## 0x6955D0 sub_6955d0 (proposed: CTaskComplexSeekEntityAnyMeans::SetRadius-like float setter with timer restart)  (catalogue: unknown/REA, port level 0)
- Caller: CTaskComplexFollowLeaderInFormation::ControlSubTask 0x696940 (applied to its subtask).
- `__thiscall void (this, float v [esp+4])`, `ret 4`.
- Behaviour: `fld v; fcomp [this+0x18]; fnstsw; test ah, 0x44; jnp skip` (0x6955D0-0x6955DC): skips only when exactly C3 is set (v == this+0x18); for less, greater and unordered (NaN: C3 and C2 both set, even parity) it updates: `this->+0x18 = v` (raw dword); timer `+0x28 = now ([0xB7CB84]); +0x2C = 0; +0x30 = 1 (u8)`.

## 0x695600 sub_695600 (proposed: CTaskComplexSeekEntityAnyMeans<CEntitySeekPosCalculatorXYOffset>::CTaskComplexSeekEntityAnyMeans)  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x870CF0 (TASK_COMPLEX_SEEK_ENTITY_ANY_MEANS) with an embedded calculator at +0x24 whose vtable is 0x86F8F8 (CEntitySeekPosCalculatorXYOffset); callers CTaskComplexSeekEntityAnyMeansXYOffset::Clone 0x696030 and CTaskComplexFollowLeaderAnyMeans::CreateSubTask 0x696680.
- `__thiscall T* (this, CEntity* entity [esp+4], int32 a2 [esp+8], int32 a3 [esp+0xC])`, `ret 0xC`; SEH frame.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0xC = entity; +0x10 = a2; vtbl = 0x870CF0; +0x14 = a3; +0x18 = 0; +0x1C = 0; +0x20 = 0 (u8); +0x21 = 0 (u8)` (timer); `+0x24 = 0x86F8F8` (calculator vtable); `+0x28 = 0; +0x2C = 0; +0x30 = 0` (offset vector); `if entity` -> RegisterReference(entity, &this->+0xC) 0x571B70.

## 0x6956D0 sub_6956d0 (proposed: CTaskComplexSeekEntityAnyMeans<XYOffset>::~CTaskComplexSeekEntityAnyMeans (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x870CF0; caller 0x6956B0 (CTaskComplexSeekEntityAnyMeansXYOffset scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x870CF0`; `if +0xC` -> CleanUpOldReference(+0xC entity, &this->+0xC) 0x571A00; `+0x24 = 0x859DC4` (calculator member reset to the base vtable); CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x695DC0 sub_695dc0 (proposed: CTaskComplexFollowPedFootsteps::~CTaskComplexFollowPedFootsteps (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x870CC0 (TASK_COMPLEX_FOLLOW_PED_FOOTSTEPS); caller 0x696260 (scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x870CC0`; `if +0xC (ped)` -> CleanUpOldReference(+0xC, &this->+0xC) 0x571A00; `if +0x18 (CPointRoute*)` -> `CPointRoute::operator delete(+0x18)` 0x41B7D0 (cdecl, pool free; no dtor call) and `+0x18 = 0`; CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x6986E0 sub_6986e0 (proposed: CPointList::CPointList)  (catalogue: unknown/REA, port level 1)
- `__thiscall CPointList* (this)`, plain `ret`. No callers. `CPointList::Empty(this)` 0x699F00; return this.
