# CTaskSimple and simple-task deleting destructors (AI) - RE-SPEC-3 group I1

CTaskSimple slots: 0-6 as CTask, 7 ProcessPed(ped), 8 SetPedPosition(ped). Layout: +0 vtbl, +4 m_Parent; size 8.

## 0x43E300 CTaskSimple::GetSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* ()`: `xor eax,eax` -> nullptr. Slot 2 of every simple task (shared body). gr matches.

## 0x43E310 CTaskSimple::IsSimple  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`: `mov al,1` -> true. Slot 3. gr matches.

## 0x43E320 CTaskSimple::SetPedPosition  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* [esp+4])`: `xor al,al; ret 4` -> false. Slot 8. gr matches.

## Deleting destructors (catalogue name `CTaskSimple::~CTaskSimple`; NAME CORRECTION)
Same shape as the complex ones (CTaskComplex.md): call the complete dtor, `if (flags & 1) CTask::operator delete(this)`
0x61A5B0, return this, `ret 4`. Port level 2.

| address | vtable (slot 0) | proposed name | complete dtor called |
|---|---|---|---|
| 0x48E4C0 | 0x85A1DC | CTaskSimpleSay::`scalar deleting destructor' | 0x48E4E0 |
| 0x48E880 | 0x85A278 | CTaskSimplePause::`scalar deleting destructor' | 0x48E8A0 (unnamed in catalogue; proposed ~CTaskSimplePause) |
| 0x637A40 | 0x86DF18 | CTaskSimpleSitIdle::`scalar deleting destructor' | 0x6311B0 |
| 0x64B000 | 0x86ED2C | CTaskSimpleCarCloseDoorFromOutside::`scalar deleting destructor' | 0x646570 |
| 0x64BDC0 | 0x86EE4C | CTaskSimpleCarGetOut::`scalar deleting destructor' | 0x647260 |
| 0x6644A0 | 0x86FA68 | CTaskSimpleDoHandSignal::`scalar deleting destructor' | 0x6608B0 |

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x46AEF0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x46AF10 CTaskSimpleSetCharDecisionMaker::Destructor (0x46AEF3); if `flags & 1` (0x46AEF8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x46AF00); returns this (0x46AF08). Proposed name: `CTaskSimpleSetCharDecisionMaker::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleSetCharDecisionMaker::Destructor).

## 0x624B30 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61DCA0 CTaskSimpleFightingControl::Destructor (0x624B33); if `flags & 1` (0x624B38) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x624B40); returns this (0x624B48). Proposed name: `CTaskSimpleFightingControl::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleFightingControl::Destructor).

## 0x625250 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61F4D0 CTaskSimpleGunControl::Destructor (0x625253); if `flags & 1` (0x625258) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x625260); returns this (0x625268). Proposed name: `CTaskSimpleGunControl::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleGunControl::Destructor).

## 0x625B60 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x61F950 CTaskSimpleThrowControl::Destructor (0x625B63); if `flags & 1` (0x625B68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x625B70); returns this (0x625B78). Proposed name: `CTaskSimpleThrowControl::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleThrowControl::Destructor).

## 0x625BC0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x620370 CTaskSimpleChoking::Destructor (0x625BC3); if `flags & 1` (0x625BC8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x625BD0); returns this (0x625BD8). Proposed name: `CTaskSimpleChoking::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleChoking::Destructor).

## 0x6296B0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x622680 FUN_00622680 (unnamed complete dtor: stores vtbl 0x86DA74) (0x6296B3); if `flags & 1` (0x6296B8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6296C0); returns this (0x6296C8).

## 0x6379D0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x631070 CTaskSimpleSitDown::Destructor (0x6379D3); if `flags & 1` (0x6379D8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6379E0); returns this (0x6379E8). Proposed name: `CTaskSimpleSitDown::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleSitDown::Destructor).

## 0x637D40 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x631320 CTaskSimpleStandUp::Destructor (0x637D43); if `flags & 1` (0x637D48) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637D50); returns this (0x637D58). Proposed name: `CTaskSimpleStandUp::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleStandUp::Destructor).

## 0x637FC0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x631D00 CTaskSimpleTurn180::Destructor (0x637FC3); if `flags & 1` (0x637FC8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637FD0); returns this (0x637FD8). Proposed name: `CTaskSimpleTurn180::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleTurn180::Destructor).

## 0x6389B0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x632A90 CTaskSimpleWaitForBus::Destructor (0x6389B3); if `flags & 1` (0x6389B8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6389C0); returns this (0x6389C8). Proposed name: `CTaskSimpleWaitForBus::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleWaitForBus::Destructor).

## 0x639310 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6335B0 CTaskSimplePlayerOnFire::Destructor (0x639313); if `flags & 1` (0x639318) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x639320); returns this (0x639328). Proposed name: `CTaskSimplePlayerOnFire::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimplePlayerOnFire::Destructor).

## 0x6394F0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6346A0 CTaskSimpleClearLookAt::Destructor (0x6394F3); if `flags & 1` (0x6394F8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x639500); returns this (0x639508). Proposed name: `CTaskSimpleClearLookAt::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleClearLookAt::Destructor).

## 0x64ABE0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x645BE0 CTaskSimpleCarAlign::Destructor (0x64ABE3); if `flags & 1` (0x64ABE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64ABF0); returns this (0x64ABF8). Proposed name: `CTaskSimpleCarAlign::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarAlign::Destructor).

## 0x64ACE0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x645EE0 CTaskSimpleCarOpenDoorFromOutside::Destructor (0x64ACE3); if `flags & 1` (0x64ACE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64ACF0); returns this (0x64ACF8). Proposed name: `CTaskSimpleCarOpenDoorFromOutside::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarOpenDoorFromOutside::Destructor).

## 0x64ADE0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6460F0 CTaskSimpleCarOpenLockedDoorFromOutside::Destructor (0x64ADE3); if `flags & 1` (0x64ADE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64ADF0); returns this (0x64ADF8). Proposed name: `CTaskSimpleCarOpenLockedDoorFromOutside::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarOpenLockedDoorFromOutside::Destructor).

## 0x64AE70 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646230 CTaskSimplePickUpBike::Destructor (0x64AE73); if `flags & 1` (0x64AE78) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64AE80); returns this (0x64AE88). Proposed name: `CTaskSimplePickUpBike::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimplePickUpBike::Destructor).

## 0x64AF30 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646380 CTaskSimpleCarCloseDoorFromInside::Destructor (0x64AF33); if `flags & 1` (0x64AF38) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64AF40); returns this (0x64AF48). Proposed name: `CTaskSimpleCarCloseDoorFromInside::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarCloseDoorFromInside::Destructor).

## 0x64B2B0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646710 CTaskSimpleCarGetIn::Destructor (0x64B2B3); if `flags & 1` (0x64B2B8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64B2C0); returns this (0x64B2C8). Proposed name: `CTaskSimpleCarGetIn::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarGetIn::Destructor).

## 0x64B3C0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646890 CTaskSimpleCarShuffle::Destructor (0x64B3C3); if `flags & 1` (0x64B3C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64B3D0); returns this (0x64B3D8). Proposed name: `CTaskSimpleCarShuffle::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarShuffle::Destructor).

## 0x64B460 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646C00 CTaskSimpleCarWaitForDoorNotToBeInUse::Destructor (0x64B463); if `flags & 1` (0x64B468) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64B470); returns this (0x64B478). Proposed name: `CTaskSimpleCarWaitForDoorNotToBeInUse::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarWaitForDoorNotToBeInUse::Destructor).

## 0x64B480 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x646E50 CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse::Destructor (0x64B483); if `flags & 1` (0x64B488) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64B490); returns this (0x64B498). Proposed name: `CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarGoToPointNearDoorUntilDoorNotInUse::Destructor).

## 0x64BEE0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x647490 CTaskSimpleCarJumpOut::Destructor (0x64BEE3); if `flags & 1` (0x64BEE8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64BEF0); returns this (0x64BEF8). Proposed name: `CTaskSimpleCarJumpOut::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarJumpOut::Destructor).

## 0x64C060 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6482C0 CTaskSimpleCarSlowBeDraggedOut::Destructor (0x64C063); if `flags & 1` (0x64C068) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64C070); returns this (0x64C078). Proposed name: `CTaskSimpleCarSlowBeDraggedOut::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarSlowBeDraggedOut::Destructor).

## 0x64C200 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x648430 CTaskSimpleCarSetPedSlowDraggedOut::Destructor (0x64C203); if `flags & 1` (0x64C208) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64C210); returns this (0x64C218). Proposed name: `CTaskSimpleCarSetPedSlowDraggedOut::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleCarSetPedSlowDraggedOut::Destructor).

## 0x64C950 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x648C40 CTaskSimpleBikeJacked::Destructor (0x64C953); if `flags & 1` (0x64C958) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64C960); returns this (0x64C968). Proposed name: `CTaskSimpleBikeJacked::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleBikeJacked::Destructor).

## 0x64CD60 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x648FC0 CTaskSimpleSetPedAsAutoDriver::Destructor (0x64CD63); if `flags & 1` (0x64CD68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64CD70); returns this (0x64CD78). Proposed name: `CTaskSimpleSetPedAsAutoDriver::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleSetPedAsAutoDriver::Destructor).

## 0x64D200 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6496A0 CTaskSimpleWaitUntilPedIsInCar::Destructor (0x64D203); if `flags & 1` (0x64D208) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64D210); returns this (0x64D218). Proposed name: `CTaskSimpleWaitUntilPedIsInCar::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleWaitUntilPedIsInCar::Destructor).

## 0x6772C0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x675E90 CTaskInteriorLieInBed::Destructor (0x6772C3); if `flags & 1` (0x6772C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6772D0); returns this (0x6772D8). Proposed name: `CTaskInteriorLieInBed::`scalar deleting destructor'` (the complete dtor it calls is CTaskInteriorLieInBed::Destructor).

## 0x677760 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x676080 CTaskInteriorSitAtDesk::Destructor (0x677763); if `flags & 1` (0x677768) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x677770); returns this (0x677778). Proposed name: `CTaskInteriorSitAtDesk::`scalar deleting destructor'` (the complete dtor it calls is CTaskInteriorSitAtDesk::Destructor).

## 0x692DD0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x690BB0 CTaskSimpleShakeFist::Destructor (0x692DD3); if `flags & 1` (0x692DD8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x692DE0); returns this (0x692DE8). Proposed name: `CTaskSimpleShakeFist::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleShakeFist::Destructor).

## 0x692FB0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6912C0 FUN_006912c0 (unnamed complete dtor: stores vtbl 0x870B08) (0x692FB3); if `flags & 1` (0x692FB8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x692FC0); returns this (0x692FC8).

## 0x6939D0 CTaskSimple::~CTaskSimple  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x692030 CTaskSimpleDuck::Destructor (0x6939D3); if `flags & 1` (0x6939D8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6939E0); returns this (0x6939E8). Proposed name: `CTaskSimpleDuck::`scalar deleting destructor'` (the complete dtor it calls is CTaskSimpleDuck::Destructor).
