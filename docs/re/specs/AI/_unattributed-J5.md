# AI unattributed functions, group J5 - RE-SPEC-AI-2

Unnamed functions of the J5 assignment. Each heading gives `sub_XXXXXX (proposed: Class::Method)`; the first bullet gives the evidence (vtable slot from catalog/data-tables.csv vtables, callers, `this` layout). Task vtable slots: 0 scalar deleting dtor, 1 Clone, 2 GetSubTask, 3 IsSimple, 4 GetTaskType, 5 StopTimer, 6 MakeAbortable, then simple 7 ProcessPed, 8 SetPedPosition / complex 7 SetSubTask, 8 CreateNextSubTask, 9 CreateFirstSubTask, 10 ControlSubTask. Event vtable slots: 0 sdd, 1 GetEventType, 2 GetEventPriority, 3 GetLifeTime, 4 Clone, 5 AffectsPed, 6 AffectsPedGroup, 7 IsCriminalEvent, 8 ReportCriminalEvent, 9 HasEditableResponse, 10 GetSourceEntity, 11 TakesPriorityOver, 12 GetLocalSoundLevel, 13 DoInformVehicleOccupants, 14 IsValid, 15 CanBeInterruptedBySameEvent, 16 CloneEditable (CEventEditableResponse).

## 0x676D10 vt_870314::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00675c90 0x675C90 (0x676D13); if `flags & 1` (0x676D18) calls CTask::operator delete 0x61A5B0 (this) (0x676D20, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870314.

## 0x677F80 sub_677f80 (proposed: CTaskSimpleGetUp::~CTaskSimpleGetUp)  (catalogue: unknown/REA, port level 1)
- Name proposal: CTaskSimpleGetUp::~CTaskSimpleGetUp (non-deleting). Evidence: writes vtable 0x8703E0 (gr RH_ScopedVirtualClass(CTaskSimpleGetUp,
  0x8703E0, 9), TaskSimpleGetUp.cpp:7) at 0x677F9D; called from the scalar deleting dtor 0x67C750 (slot 0) of the same class; field +0xC is
  CTaskSimpleGetUp::m_Anim.
- `__thiscall void ()`, plain `ret`, SEH frame. Steps: this->vtbl = 0x8703E0; if m_Anim (+0xC) != 0: `m_Anim->SetFinishCallback(0x4D1390, nullptr)`
  (CAnimBlendAssociation::SetFinishCallback 0x4CEBE0; 0x4D1390 is the default/no-op finish callback, catalogue unnamed) (0x677FA3-0x677FB9);
  then the base dtor 0x61A3A0 (CTaskSimple::~CTaskSimple) (0x677FC8). m_Anim is not cleared.
- gta-reversed: TaskSimpleGetUp.cpp:28 matches the bytes (gr has no address comment for it). Open questions: none.

## 0x678A00 vt_870480::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD1` (0x678A00); `ret` -> returns 209 (TASK_COMPLEX_FALL_AND_STAY_DOWN). Vtable: slot 4 (+0x10) of vtable 0x870480.

## 0x678BC0 vt_870480::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x678BC0); ped unused. Vtable: slot 10 (+0x28) of vtable 0x870480.

## 0x67BA70 sub_67ba70 (proposed: CTaskComplexStuckInAir::~CTaskComplexStuckInAir)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexStuckInAir::~CTaskComplexStuckInAir. Evidence: stores vtable 0x870608 (CTaskComplexStuckInAir) into this; callers (1): 0x67fa60 CTaskComplexStuckInAir::~CTaskComplexStuckInAir.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870608` (CTaskComplexStuckInAir) (0x67BA70); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x67BA76). No other member cleanup.

## 0x67CBC0 vt_870480::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00678a90 0x678A90 (0x67CBC3); if `flags & 1` (0x67CBC8) calls CTask::operator delete 0x61A5B0 (this) (0x67CBD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870480.

## 0x6833C0 vt_8706f8::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4BD` (0x6833C0); `ret` -> returns 1213 (TASK_COMPLEX_WAIT_FOR_PED). Vtable: slot 4 (+0x10) of vtable 0x8706F8.

## 0x683430 vt_8706f8::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x683430; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x8706F8.

## 0x684750 sub_684750 (proposed: CTaskComplexPartnerShove::~CTaskComplexPartnerShove)  (catalogue: unknown/REA, port level 3)
- Name proposal: CTaskComplexPartnerShove::~CTaskComplexPartnerShove. Evidence: stores vtable 0x870800 (CTaskComplexPartnerShove) into this; callers (1): 0x684fe0 CTaskComplexPartnerShove::~CTaskComplexPartnerShove.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870800` (CTaskComplexPartnerShove) (0x684750); tail-jump to base dtor CTaskComplexPartner::~CTaskComplexPartner 0x683A40 (0x684756). No other member cleanup.

## 0x684780 vt_8706f8::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006833d0 0x6833D0 (0x684783); if `flags & 1` (0x684788) calls CTask::operator delete 0x61A5B0 (this) (0x684790, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x8706F8.

## 0x691A40 vt_870b98::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x136` (0x691A40); `ret` -> returns 310 (TASK_COMPLEX_GO_PICKUP_ENTITY). Vtable: slot 4 (+0x10) of vtable 0x870B98.

## 0x6926E0 vt_870bec::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x137` (0x6926E0); `ret` -> returns 311 (TASK_SIMPLE_DUCK_WHILE_SHOTS_WHIZZING). Vtable: slot 4 (+0x10) of vtable 0x870BEC.

## 0x6935E0 sub_6935e0 (proposed: CTaskSimplePutDownEntity::~CTaskSimplePutDownEntity (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimplePutDownEntity::~CTaskSimplePutDownEntity (non-deleting dtor). Evidence: only caller 0x6935c0 is slot 0 (scalar deleting dtor) of vtable 0x870B74 (CTaskSimplePutDownEntity); callers (1): 0x6935c0 CTaskSimplePutDownEntity::~CTaskSimplePutDownEntity.
- Tail-jump thunk (0x6935E0): `jmp FUN_00691630 0x691630` with all registers and stack args unchanged (same convention as the target).

## 0x6935F0 vt_870b98::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00691a50 0x691A50 (0x6935F3); if `flags & 1` (0x6935F8) calls CTask::operator delete 0x61A5B0 (this) (0x693600, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870B98.

## 0x693B80 vt_870bec::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006926f0 0x6926F0 (0x693B83); if `flags & 1` (0x693B88) calls CTask::operator delete 0x61A5B0 (this) (0x693B90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870BEC.

## 0x69DB40 sub_69db40 (proposed: CTaskAllocatorPlayerCommandAttack::~CTaskAllocatorPlayerCommandAttack)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskAllocatorPlayerCommandAttack::~CTaskAllocatorPlayerCommandAttack. Evidence: stores vtable 0x870E78 (CTaskAllocatorPlayerCommandAttack) into this; callers (1): 0x69db00 CTaskAllocatorPlayerCommandAttack::~CTaskAllocatorPlayerCommandAttack.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870E78` (CTaskAllocatorPlayerCommandAttack) (0x69DB40); tail-jump to base dtor CTaskAllocatorAttack::Destructor 0x69C2D0 (0x69DB46). No other member cleanup.
