# CTaskComplex and complex-task deleting destructors (AI) - RE-SPEC-3 group I1

CTask vtable slots (gr Task.h / TaskComplex.h): 0 dtor, 1 Clone, 2 GetSubTask, 3 IsSimple, 4 GetTaskType, 5 StopTimer,
6 MakeAbortable(ped, priority, event), 7 SetSubTask, 8 CreateNextSubTask, 9 CreateFirstSubTask, 10 ControlSubTask.
Layout: +0 vtbl, +4 m_Parent (CTaskComplex*), +8 m_pSubTask (CTask*). CTaskComplex vtable 0x86D4CC; its complete dtor is
0x61A3D0 (SEH; `vtbl = 0x86D4CC`; `if (m_pSubTask) m_pSubTask->vtbl[0](1)`; then CTask vtbl).
`CTask::operator delete` 0x61A5B0 (cdecl): inlined CPool::Delete on `CPools::ms_pTaskPool` [0xB744A8] with stride 0x80:
`idx = (obj - objects) >> 7` (arithmetic `sar`, not a divide), `byteMap[idx] |= 0x80`, `if (idx < firstFree) firstFree = idx`.

## 0x421190 CTaskComplex::GetSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* ()`: `return m_pSubTask` (+8). Slot 2, shared by all complex tasks. gr matches.

## 0x4211A0 CTaskComplex::IsSimple  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`: `xor al,al` -> false. Slot 3. gr matches.

## 0x4211B0 CTaskComplex::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)` (3 stack args): tail-jumps to
  `m_pSubTask->MakeAbortable(...)` (vtbl+0x18) with ECX replaced and the caller's stack args reused; the callee's `ret 0xC` returns.
  No null check of m_pSubTask (crashes if null). Slot 6 of many complex tasks that do not override it.
- gta-reversed: not a CTaskComplex method in gr (each task overrides; several gr tasks write `return m_pSubTask->MakeAbortable(...)`)
  - behaviour identical.

## 0x61A430 CTaskComplex::SetSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CTask* sub [esp+4])`, `ret 4`. Slot 7.
- Behaviour: `old = m_pSubTask`; if `old == sub` return without doing anything (0x61A43B-0x61A43D; note: the parent pointer of
  `sub` is NOT (re)set in that case); if old != 0 `old->vtbl[0](1)` (deleting dtor, 0x61A443-0x61A447); `m_pSubTask = sub`
  (0x61A44B); if sub != 0 `sub->m_Parent (+4) = this` (0x61A450).
- gta-reversed: TaskComplex.h:28 inline - same logic (checks equality first). Matches.

## Deleting destructors (catalogue name `CTaskComplex::~CTaskComplex`; NAME CORRECTION: each is the
## `scalar deleting destructor' of the class whose vtable slot 0 holds it)
All eleven share one shape: `__thiscall this (uint32 flags [esp+4])`, `ret 4`: call the class's complete-object dtor (thiscall),
then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0 (cdecl, `add esp,4`), return this (EAX). Port: implement as the
class's Drop + pool free. Port level per catalogue (3 unless noted).

| address | vtable (slot 0) | proposed name | complete dtor called |
|---|---|---|---|
| 0x61C0D0 | 0x86D5DC | CTaskComplexPlayHandSignalAnim::`scalar deleting destructor' (level 12) | 0x61BDF0 |
| 0x629060 | 0x86D9F0 | CTaskComplexRoadRage::`scalar deleting destructor' | 0x622110 |
| 0x637D90 | 0x86DF60 | CTaskComplexSitDownThenIdleThenStandUp::`scalar deleting destructor' | 0x6314A0 |
| 0x6385B0 | 0x86E134 | CTaskComplexUseAttractor::`scalar deleting destructor' | 0x632720 |
| 0x6434F0 | 0x86EA14 | CTaskComplexEnterAnyCarAsDriver::`scalar deleting destructor' | 0x63D100 (`vtbl = 0x86EA14`; jmp 0x61A3D0 = ~CTaskComplex; proposed ~CTaskComplexEnterAnyCarAsDriver) |
| 0x656E50 | 0x86F334 | CTaskComplexWalkRoundBuildingAttempt::`scalar deleting destructor' | 0x654950 |
| 0x657B60 | 0x86F308 | CTaskComplexWalkRoundCar::`scalar deleting destructor' | 0x656B00 |
| 0x65D260 | 0x86F744 | CTaskComplexSmartFleePoint::`scalar deleting destructor' | 0x65BDB0 |
| 0x662350 | 0x86F8FC | CTaskComplexGangLeader::`scalar deleting destructor' | 0x65DF30 |
| 0x6644C0 | 0x86FA8C | CTaskComplexSignalAtPed::`scalar deleting destructor' | 0x660AB0 |
| 0x684760 | 0x8706CC | CTaskComplexWalkAlongsidePed::`scalar deleting destructor' | 0x6832C0 |
| 0x68DAB0 | 0x870A00 | CTaskComplexKillCriminal::`scalar deleting destructor' (level 4) | 0x68BF30 |

Instruction evidence (same offsets in each): call dtor at +3, `test byte [esp+8],1` at +8, `call 0x61A5B0` at +0x10, `ret 4` at +0x1B.
gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x421250 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x421270 CTaskComplexLeaveAnyCar::Destructor (0x421253); if `flags & 1` (0x421258) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x421260); returns this (0x421268). Proposed name: `CTaskComplexLeaveAnyCar::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexLeaveAnyCar::Destructor).

## 0x626AA0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x621090 CTaskComplexKillPedOnFootMelee::Destructor (0x626AA3); if `flags & 1` (0x626AA8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x626AB0); returns this (0x626AB8). Proposed name: `CTaskComplexKillPedOnFootMelee::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexKillPedOnFootMelee::Destructor).

## 0x629550 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6222B0 CTaskComplexKillPedGroupOnFoot::Destructor (0x629553); if `flags & 1` (0x629558) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x629560); returns this (0x629568). Proposed name: `CTaskComplexKillPedGroupOnFoot::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexKillPedGroupOnFoot::Destructor).

## 0x629900 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x622C20 CTaskComplexSeekCoverUntilTargetDead::Destructor (0x629903); if `flags & 1` (0x629908) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x629910); returns this (0x629918). Proposed name: `CTaskComplexSeekCoverUntilTargetDead::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexSeekCoverUntilTargetDead::Destructor).

## 0x638180 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x632050 CTaskComplexSunbathe::Destructor (0x638183); if `flags & 1` (0x638188) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x638190); returns this (0x638198). Proposed name: `CTaskComplexSunbathe::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexSunbathe::Destructor).

## 0x638590 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x632630 CTaskComplexWaitAtAttractor::Destructor (0x638593); if `flags & 1` (0x638598) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6385A0); returns this (0x6385A8). Proposed name: `CTaskComplexWaitAtAttractor::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexWaitAtAttractor::Destructor).

## 0x638900 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x632830 CTaskComplexWaitForDryWeather::Destructor (0x638903); if `flags & 1` (0x638908) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x638910); returns this (0x638918). Proposed name: `CTaskComplexWaitForDryWeather::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexWaitForDryWeather::Destructor).

## 0x638920 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x632930 CTaskComplexWaitForBus::Destructor (0x638923); if `flags & 1` (0x638928) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x638930); returns this (0x638938). Proposed name: `CTaskComplexWaitForBus::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexWaitForBus::Destructor).

## 0x6390A0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x633360 CTaskComplexUseScriptedAttractor::Destructor (0x6390A3); if `flags & 1` (0x6390A8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6390B0); returns this (0x6390B8). Proposed name: `CTaskComplexUseScriptedAttractor::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexUseScriptedAttractor::Destructor).

## 0x639190 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6333C0 CTaskComplexOnFire::Destructor (0x639193); if `flags & 1` (0x639198) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6391A0); returns this (0x6391A8). Proposed name: `CTaskComplexOnFire::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexOnFire::Destructor).

## 0x639510 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x634720 CTaskComplexUseClosestFreeScriptedAttractor::Destructor (0x639513); if `flags & 1` (0x639518) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x639520); returns this (0x639528). Proposed name: `CTaskComplexUseClosestFreeScriptedAttractor::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexUseClosestFreeScriptedAttractor::Destructor).

## 0x6396A0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x635100 CTaskComplexInWater::Destructor (0x6396A3); if `flags & 1` (0x6396A8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6396B0); returns this (0x6396B8). Proposed name: `CTaskComplexInWater::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexInWater::Destructor).

## 0x6397C0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x635900 CTaskComplexUseScriptedBrain::Destructor (0x6397C3); if `flags & 1` (0x6397C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6397D0); returns this (0x6397D8). Proposed name: `CTaskComplexUseScriptedBrain::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexUseScriptedBrain::Destructor).

## 0x640590 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63AE00 CTaskComplexEnterCarAsDriverTimed::Destructor (0x640593); if `flags & 1` (0x640598) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6405A0); returns this (0x6405A8). Proposed name: `CTaskComplexEnterCarAsDriverTimed::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexEnterCarAsDriverTimed::Destructor).

## 0x640710 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63B3C0 CTaskComplexEnterCarAsPassengerWait::Destructor (0x640713); if `flags & 1` (0x640718) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x640720); returns this (0x640728). Proposed name: `CTaskComplexEnterCarAsPassengerWait::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexEnterCarAsPassengerWait::Destructor).

## 0x640BC0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63B4A0 CTaskComplexLeaveCarAsPassengerWait::Destructor (0x640BC3); if `flags & 1` (0x640BC8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x640BD0); returns this (0x640BD8). Proposed name: `CTaskComplexLeaveCarAsPassengerWait::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexLeaveCarAsPassengerWait::Destructor).

## 0x640E40 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63B650 CTaskComplexEnterBoatAsDriver::Destructor (0x640E43); if `flags & 1` (0x640E48) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x640E50); returns this (0x640E58). Proposed name: `CTaskComplexEnterBoatAsDriver::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexEnterBoatAsDriver::Destructor).

## 0x640F10 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63B850 CTaskComplexStealCar::Destructor (0x640F13); if `flags & 1` (0x640F18) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x640F20); returns this (0x640F28). Proposed name: `CTaskComplexStealCar::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexStealCar::Destructor).

## 0x6422A0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63BE50 CTaskComplexLeaveCarAndWander::Destructor (0x6422A3); if `flags & 1` (0x6422A8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6422B0); returns this (0x6422B8). Proposed name: `CTaskComplexLeaveCarAndWander::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexLeaveCarAndWander::Destructor).

## 0x642370 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63C030 CTaskComplexLeaveCarAndFlee::Destructor (0x642373); if `flags & 1` (0x642378) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x642380); returns this (0x642388). Proposed name: `CTaskComplexLeaveCarAndFlee::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexLeaveCarAndFlee::Destructor).

## 0x6424F0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63C2C0 CTaskComplexScreamInCarThenLeave::Destructor (0x6424F3); if `flags & 1` (0x6424F8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x642500); returns this (0x642508). Proposed name: `CTaskComplexScreamInCarThenLeave::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexScreamInCarThenLeave::Destructor).

## 0x6436E0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63D1A0 CTaskComplexShuffleSeats::Destructor (0x6436E3); if `flags & 1` (0x6436E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6436F0); returns this (0x6436F8). Proposed name: `CTaskComplexShuffleSeats::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexShuffleSeats::Destructor).

## 0x645350 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x643490 CTaskComplexDrivePointRoute::Destructor (0x645353); if `flags & 1` (0x645358) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x645360); returns this (0x645368). Proposed name: `CTaskComplexDrivePointRoute::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexDrivePointRoute::Destructor).

## 0x6560C0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x653A30 CTaskComplexHitPedWithCar::Destructor (0x6560C3); if `flags & 1` (0x6560C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6560D0); returns this (0x6560D8). Proposed name: `CTaskComplexHitPedWithCar::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexHitPedWithCar::Destructor).

## 0x657750 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x655370 CTaskComplexMoveBackAndJump::Destructor (0x657753); if `flags & 1` (0x657758) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x657760); returns this (0x657768). Proposed name: `CTaskComplexMoveBackAndJump::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexMoveBackAndJump::Destructor).

## 0x6579F0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x655780 CTaskComplexWalkRoundFire::Destructor (0x6579F3); if `flags & 1` (0x6579F8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x657A00); returns this (0x657A08). Proposed name: `CTaskComplexWalkRoundFire::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexWalkRoundFire::Destructor).

## 0x65D0E0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65B410 CTaskComplexFleePoint::Destructor (0x65D0E3); if `flags & 1` (0x65D0E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x65D0F0); returns this (0x65D0F8). Proposed name: `CTaskComplexFleePoint::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexFleePoint::Destructor).

## 0x65D240 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65B9D0 CTaskComplexFleeEntity::Destructor (0x65D243); if `flags & 1` (0x65D248) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x65D250); returns this (0x65D258). Proposed name: `CTaskComplexFleeEntity::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexFleeEntity::Destructor).

## 0x663620 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65F460 CTaskComplexTrackEntity::Destructor (0x663623); if `flags & 1` (0x663628) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x663630); returns this (0x663638). Proposed name: `CTaskComplexTrackEntity::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexTrackEntity::Destructor).

## 0x664480 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x660310 CTaskComplexStareAtPed::Destructor (0x664483); if `flags & 1` (0x664488) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x664490); returns this (0x664498). Proposed name: `CTaskComplexStareAtPed::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexStareAtPed::Destructor).

## 0x664A30 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x661230 CTaskComplexPassObject::Destructor (0x664A33); if `flags & 1` (0x664A38) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x664A40); returns this (0x664A48). Proposed name: `CTaskComplexPassObject::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexPassObject::Destructor).

## 0x6712E0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x66C7A0 CTaskComplexInvestigateDisturbance::Destructor (0x6712E3); if `flags & 1` (0x6712E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6712F0); returns this (0x6712F8). Proposed name: `CTaskComplexInvestigateDisturbance::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexInvestigateDisturbance::Destructor).

## 0x6761E0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x675000 CTaskInteriorBeInHouse::Destructor (0x6761E3); if `flags & 1` (0x6761E8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6761F0); returns this (0x6761F8). Proposed name: `CTaskInteriorBeInHouse::`scalar deleting destructor'` (the complete dtor it calls is CTaskInteriorBeInHouse::Destructor).

## 0x6764A0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6756A0 CTaskInteriorShopKeeper::Destructor (0x6764A3); if `flags & 1` (0x6764A8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6764B0); returns this (0x6764B8). Proposed name: `CTaskInteriorShopKeeper::`scalar deleting destructor'` (the complete dtor it calls is CTaskInteriorShopKeeper::Destructor).

## 0x676860 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x675A90 CTaskInteriorUseInfo::Destructor (0x676863); if `flags & 1` (0x676868) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x676870); returns this (0x676878). Proposed name: `CTaskInteriorUseInfo::`scalar deleting destructor'` (the complete dtor it calls is CTaskInteriorUseInfo::Destructor).

## 0x695BE0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x694B30 CTaskComplexFollowLeaderAnyMeans::Destructor (0x695BE3); if `flags & 1` (0x695BE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x695BF0); returns this (0x695BF8). Proposed name: `CTaskComplexFollowLeaderAnyMeans::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexFollowLeaderAnyMeans::Destructor).

## 0x695C00 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x694C10 CTaskComplexSeekEntityAiming::Destructor (0x695C03); if `flags & 1` (0x695C08) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x695C10); returns this (0x695C18). Proposed name: `CTaskComplexSeekEntityAiming::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexSeekEntityAiming::Destructor).

## 0x697820 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x696E80 CTaskComplexInvestigateDeadPed::Destructor (0x697823); if `flags & 1` (0x697828) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x697830); returns this (0x697838). Proposed name: `CTaskComplexInvestigateDeadPed::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexInvestigateDeadPed::Destructor).

## 0x697B50 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x697230 CTaskComplexReactToGunAimedAt::Destructor (0x697B53); if `flags & 1` (0x697B58) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x697B60); returns this (0x697B68). Proposed name: `CTaskComplexReactToGunAimedAt::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexReactToGunAimedAt::Destructor).

## 0x697EF0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x697330 CTaskComplexWaitForBackup::Destructor (0x697EF3); if `flags & 1` (0x697EF8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x697F00); returns this (0x697F08). Proposed name: `CTaskComplexWaitForBackup::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexWaitForBackup::Destructor).

## 0x6980F0 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6975F0 CTaskComplexExtinguishFires::Destructor (0x6980F3); if `flags & 1` (0x6980F8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x698100); returns this (0x698108). Proposed name: `CTaskComplexExtinguishFires::`scalar deleting destructor'` (the complete dtor it calls is CTaskComplexExtinguishFires::Destructor).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x61A620 CTaskComplex::~CTaskComplex  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor of CTaskComplex: calls 0x61A3D0 (complete dtor `CTaskComplex::~CTaskComplex`, unnamed in the catalogue) (0x61A623); if flags & 1: task-pool free
  inlined ([0xB744A8], `>> 7`, byteMap |= 0x80, firstFree = min, 0x61A631-0x61A652); returns this; `ret 4`.

## 0x634F30 CTaskComplex::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed*, int32 priority, const CEvent*)`: tail-jumps to `m_pSubTask (+8)->MakeAbortable(...)` (vtbl+0x18) with the SAME stack arguments and ECX =
  subtask (0x634F30-0x634F35). No null check of the subtask.
