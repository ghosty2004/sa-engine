# _unattributed-FA (AI) - RE-SPEC-FINAL group A

## 0x46AD70 seek-entity task member dtor (sub_46ad70)  (catalogue: unknown/REA, port level 2)
Catalogue name: <none>, proposed: `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::~CTaskComplexSeekEntity` (non-deleting body;
evidence: called by CTaskComplexSeekEntityStandard::Destructor 0x46AD50 and as an SEH unwind action; writes the seek task vtable 0x859DF8 and the
embedded position-calculator vtable at +0x40).
- Convention: `__thiscall void (this)`, plain `ret`. SEH frame (handler 0x837E63).
- Behaviour: `vtable = 0x859DF8`; if `this->+0xC` (seek entity, registered reference) != 0: `CEntity::CleanUpOldReference(&this->+0xC)` 0x571A00
  (the pointer itself is not zeroed here); `this->+0x40 = 0x859DC4` (member calculator vtable, its dtor is trivial); then the base dtor
  CTaskComplex::~CTaskComplex 0x61A3D0 on `this` (vtable 0x86D4CC, deletes the sub-task at +8 through its virtual deleting dtor with 1, +8 = 0,
  vtable 0x86D48C).
- gta-reversed: the template dtor exists in gr headers (not hooked at this address). Open questions: none.

## 0x48E4F0 CTaskComplexWanderStandard  (catalogue: specified/REA, port level 2)
Proposed full name: `CTaskComplexWanderStandard::CTaskComplexWanderStandard(eMoveState moveState, uint8 dir, bool wanderSensibly)`.
- Convention: `__thiscall CTaskComplexWanderStandard* (this, int32 moveState [esp+4], uint8 dir [esp+8], bool sensibly [esp+0xC])`, `ret 0xC`,
  returns this. 0x48E4F0 jumps to the HOODLUM body 0x156B180.
- Behaviour: `CTaskComplexWander::CTaskComplexWander(this, moveState, dir, sensibly, [0x86FCB0]=0.5f)` 0x66F450 (the three args are forwarded as
  full dwords; the float is pushed as a dword copy), then `vtable = 0x85A200`, `+0x28 = 0` (m_TaskTimer.m_nStartTime), `+0x2C = 0` (m_nInterval),
  `+0x30 = 0` (bStarted), `+0x31 = 0` (bStopped), `+0x34 = 0` (m_nMinNextScanTime).
- gta-reversed: TaskComplexWanderStandard.cpp:6 - same (default fTargetRadius 0.5f). Open questions: none.

## 0x4AECD0 vt_85b070 ctor (from Clone)  (catalogue: unknown/REA, port level 2)
Catalogue name: <none>, proposed: `CEventChatPartner::CEventChatPartner(bool leader, CPed* partner)` (evidence: vtable 0x85B070, GetEventType
0x4AED30 returns 0x13 = 19 EVENT_CHAT_PARTNER; callers vt_85b070::Clone 0x4B7210 and CTaskComplexWanderStandard::LookForChatPartners 0x66FDA0).
- Convention: `__thiscall CEventChatPartner* (this, bool leader [esp+4], CPed* partner [esp+8])`, `ret 8`, returns this. SEH frame (unwind
  ~CEvent 0x4AC010).
- Behaviour (store order): +4 dword = 0, +8 byte = 0 (CEvent base); +0xC byte = leader (low byte of arg 1); vtable = 0x85B070; +0x10 = partner;
  if partner != 0 `partner->RegisterReference(&this->+0x10)` 0x571B70. Not a CEventEditableResponse (no +0xE/+0x10 task-id fields).
- gta-reversed: absent. Open questions: none.

## 0x4AEDF0 vt_85b0b0 ctor (from CloneEditable)  (catalogue: unknown/REA, port level 2)
Catalogue name: <none>, proposed: `CEventSexyPed::CEventSexyPed(CPed* ped)` (evidence: vtable 0x85B0B0, GetEventType 0x4AEE60 returns 0x14 = 20
EVENT_SEXY_PED; callers vt_85b0b0::CloneEditable 0x4B7280, CSexyPedScanner 0x603BF0, CPopulation::AddPed 0x612710).
- Convention: `__thiscall CEventSexyPed* (this, CPed* ped [esp+4])`, `ret 4`, returns this. SEH frame (unwind ~CEventEditableResponse 0x4AC480).
- Behaviour (store order): CEvent base +4 = 0 (m_nTimeActive), +8 byte = 0; CEventEditableResponse part: +0xC byte = 1 (m_bAddToEventGroup),
  +0xE i16 = 0xC8 (200, m_TaskId = TASK_NONE), +0x10 i16 = -1 (0xFFFF, m_FacialExpressionType); vtable = 0x85B0B0; +0x14 = ped; if ped != 0
  `ped->RegisterReference(&this->+0x14)` 0x571B70. Size 0x18 (dtor 0x4AEEA0 cleans +0x14 and restores base vtable 0x85AB38).
- gta-reversed: absent. Open questions: none.

## 0x6389F0 CTaskComplexSequence  (catalogue: specified/REA, port level 2)
Proposed full name: `CTaskComplexSequence::~CTaskComplexSequence` (non-deleting body; vtable 0x86E200 = gr CTaskComplexSequence).
- Convention: `__thiscall void (this)`, plain `ret`. SEH frame (handler 0x8411D8).
- Behaviour: `vtable = 0x86E200`; SEH state 0; `CTaskComplexSequence::Flush(this)` 0x632C10; SEH -1; `CTaskComplex::~CTaskComplex(this)`
  0x61A3D0 (deletes the sub-task at +8 via its virtual deleting dtor(1), +8 = 0, vtable 0x86D48C).
- gta-reversed: TaskComplexSequence dtor calls Flush - matches. Open questions: none.
