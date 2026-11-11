# AI unattributed functions, group J4 - RE-SPEC-AI-2

Unnamed functions of the J4 assignment. Each heading gives `sub_XXXXXX (proposed: Class::Method)`; the first bullet gives the evidence (vtable slot from catalog/data-tables.csv vtables, callers, `this` layout). Task vtable slots: 0 scalar deleting dtor, 1 Clone, 2 GetSubTask, 3 IsSimple, 4 GetTaskType, 5 StopTimer, 6 MakeAbortable, then simple 7 ProcessPed, 8 SetPedPosition / complex 7 SetSubTask, 8 CreateNextSubTask, 9 CreateFirstSubTask, 10 ControlSubTask. Event vtable slots: 0 sdd, 1 GetEventType, 2 GetEventPriority, 3 GetLifeTime, 4 Clone, 5 AffectsPed, 6 AffectsPedGroup, 7 IsCriminalEvent, 8 ReportCriminalEvent, 9 HasEditableResponse, 10 GetSourceEntity, 11 TakesPriorityOver, 12 GetLocalSoundLevel, 13 DoInformVehicleOccupants, 14 IsValid, 15 CanBeInterruptedBySameEvent, 16 CloneEditable (CEventEditableResponse).

## 0x43E2B0 sub_43e2b0 (proposed: CEventLeaderEntryExit::~CEventLeaderEntryExit)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventLeaderEntryExit::~CEventLeaderEntryExit. Evidence: stores vtable 0x859540 (CEventLeaderEntryExit) into this; callers (1): 0x43e290 CEventLeaderEntryExit::~CEventLeaderEntryExit.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x859540` (CEventLeaderEntryExit) (0x43E2B0); tail-jump to base dtor FUN_004b2210 0x4B2210 (0x43E2B6). No other member cleanup.

## 0x43E3D0 sub_43e3d0 (proposed: CTaskSimpleUninterruptable::~CTaskSimpleUninterruptable)  (catalogue: unknown/REA, port level 1)
- Name proposal: CTaskSimpleUninterruptable::~CTaskSimpleUninterruptable. Evidence: stores vtable 0x859580 (CTaskSimpleUninterruptable) into this; callers (1): 0x43e3b0 CTaskSimpleUninterruptable::~CTaskSimpleUninterruptable.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x859580` (CTaskSimpleUninterruptable) (0x43E3D0); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x43E3D6). No other member cleanup.

## 0x4636E0 sub_4636e0 (proposed: CTaskSimpleFinishBrain::~CTaskSimpleFinishBrain (non-deleting dtor))  (catalogue: unknown/REA, port level 1)
- Name proposal: CTaskSimpleFinishBrain::~CTaskSimpleFinishBrain (non-deleting dtor). Evidence: stores vtable 0x859CFC (not in data-tables) into this; callers (1): 0x4636c0 CTaskSimpleFinishBrain::~CTaskSimpleFinishBrain.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x859CFC` (0x4636E0); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x4636E6). No other member cleanup.

## 0x499430 vt_85a428::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x450` (0x499430); `ret` -> returns 1104 (TASK_COMPLEX_BE_COP). Vtable: slot 4 (+0x10) of vtable 0x85A428.

## 0x4994C0 vt_85a428::Destructor  (catalogue: specified/REA, port level 6)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_004994e0 0x4994E0 (0x4994C3); if `flags & 1` (0x4994C8) calls CTask::operator delete 0x61A5B0 (this) (0x4994D0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x85A428.

## 0x4AE220 vt_85ae88::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AE220; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85AE88.

## 0x4AEB40 vt_85afe8::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x11` (0x4AEB40); `ret` -> returns 17 (EVENT_PED_ENTERED_MY_VEHICLE). Vtable: slot 1 (+0x4) of vtable 0x85AFE8.

## 0x4AEB50 vt_85afe8::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AEB50; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85AFE8.

## 0x4AEB80 vt_85afe8::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x27` (0x4AEB80); `ret` -> returns 39. Vtable: slot 2 (+0x8) of vtable 0x85AFE8.

## 0x4AEC70 vt_85b030::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x12` (0x4AEC70); `ret` -> returns 18 (EVENT_REVIVE). Vtable: slot 1 (+0x4) of vtable 0x85B030.

## 0x4AEC80 vt_85b030::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AEC80; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B030.

## 0x4AEC90 vt_85b030::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4A` (0x4AEC90); `ret` -> returns 74. Vtable: slot 2 (+0x8) of vtable 0x85B030.

## 0x4AED40 vt_85b070::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AED40; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B070.

## 0x4AEE70 vt_85b0b0::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AEE70; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B0B0.

## 0x4AF080 vt_85b0f8::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AF080; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B0F8.

## 0x4AFBC0 vt_85b298::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1E` (0x4AFBC0); `ret` -> returns 30 (EVENT_VEHICLE_THREAT). Vtable: slot 1 (+0x4) of vtable 0x85B298.

## 0x4AFBD0 vt_85b298::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4AFBD0; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B298.

## 0x4AFBE0 vt_85b298::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x13` (0x4AFBE0); `ret` -> returns 19. Vtable: slot 2 (+0x8) of vtable 0x85B298.

## 0x4B0E80 vt_86cbe8::AffectsPed (proposed: CEventHealthReallyLow::AffectsPed)  (catalogue: specified/REA, port level 1)
- Name evidence: slot 5 of vtable 0x86CBE8 whose GetEventType 0x5FF530 returns 0x34 (EVENT_HEALTH_REALLY_LOW).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = `ped->IsAlive()` 0x5E0170 != 0 (`setne`, 0x4B0E8B).

## 0x4B1390 vt_85b4c8::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x42` (0x4B1390); `ret` -> returns 66 (EVENT_IN_WATER). Vtable: slot 1 (+0x4) of vtable 0x85B4C8.

## 0x4B13A0 vt_85b4c8::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x4B13A0; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x85B4C8.

## 0x4B13B0 vt_85b4c8::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3E` (0x4B13B0); `ret` -> returns 62. Vtable: slot 2 (+0x8) of vtable 0x85B4C8.

## 0x4B13D0 vt_85b4c8::AffectsPed (proposed: CEventInWater::AffectsPed)  (catalogue: specified/REA, port level 1)
- Name evidence: slot 5 of vtable 0x85B4C8 whose GetEventType 0x4B1390 returns 0x42 (EVENT_IN_WATER).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: `t = ped->m_pIntelligence(+0x47C)->m_TaskMgr(+4).GetActiveTask()` 0x681720 (0x4B13DF);
  if `!ped->IsPlayer()` and t and `t->GetTaskType()` (vtbl +0x10) == 0x10C (TASK_COMPLEX_IN_WATER) return 0 (0x4B1404); else return
  `ped->IsAlive()` 0x5E0170 != 0 (0x4B1414).

## 0x4B1420 vt_85b4c8::TakesPriorityOver (proposed: CEventInWater::TakesPriorityOver)  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, const CEvent& other [esp+4])`, `ret 4`. `other.GetEventType()` (vtbl +4) is re-called for each test.
- Returns 1 if the type is 8 (EVENT_KNOCK_OFF_BIKE, 0x4B142F) or 9 (EVENT_DAMAGE, 0x4B143B), or if it is 0x5D (EVENT_STUCK_IN_AIR) and
  `this->+0xC > 1.0 [0x858624]` (strict; `fld +0xC; fcomp; test ah,0x41; je return1`, NaN -> false; 0x4B144C-0x4B145A).
- Otherwise returns `this->GetEventPriority() >= other.GetEventPriority()` (signed `setge`; other's priority (vtbl +8) is queried first at 0x4B1460,
  then this's at 0x4B1469).

## 0x4B6610 vt_85b4c8::Destructor (proposed: CEventInWater scalar deleting destructor)  (catalogue: specified/REA, port level 1)
- Slot 0 of vtable 0x85B4C8. `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls 0x4B13C0 (inlined CEvent dtor: vtbl = 0x85AB38);
  if `flags & 1`: inline `CEventPool::Delete` on ms_pEventPool 0xB744AC: `idx = (this - pool->m_pObjects) / 0x44` (magic 0x78787879, `sar 5`, sign fix,
  0x4B6626-0x4B6639), `pool->m_byteMap[idx] |= 0x80` (free bit), and if `idx < pool->+0xC` (first free) then `pool->+0xC = idx` (0x4B6648-0x4B6650).
  Returns this.

## 0x4B7810 vt_85b4c8::Clone (proposed: CEventInWater::Clone)  (catalogue: specified/REA, port level 1)
- `__thiscall CEvent* ()`, plain `ret`: `p = ms_pEventPool(0xB744AC)->New()` 0x4B5570; if p: `p->+4 = 0`, `p->+0xC = this->+0xC` (via fld/fstp, f32),
  `p->+8 = 0`, vtbl = 0x85B4C8 (0x4B7824-0x4B7830); returns p (0 if the pool is full).

## 0x4B83C0 sub_4b83c0 (proposed: CEventLeaderExitedCarAsDriver::~CEventLeaderExitedCarAsDriver)  (catalogue: unknown/REA, port level 1)
- Name proposal: CEventLeaderExitedCarAsDriver::~CEventLeaderExitedCarAsDriver. Evidence: stores vtable 0x85B898 (CEventLeaderExitedCarAsDriver) into this; callers (1): 0x4b83a0 CEventLeaderExitedCarAsDriver::~CEventLeaderExitedCarAsDriver.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x85B898` (CEventLeaderExitedCarAsDriver) (0x4B83C0); tail-jump to base dtor CEventEditableResponse::~CEventEditableResponse 0x4AC480 (0x4B83C6). No other member cleanup.

## 0x4B8720 vt_85b8dc::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x51C` (0x4B8720); `ret` -> returns 1308 (TASK_SIMPLE_ON_ESCALATOR). Vtable: slot 4 (+0x10) of vtable 0x85B8DC.

## 0x4B8730 vt_85b8dc::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_004b8750 0x4B8750 (0x4B8733); if `flags & 1` (0x4B8738) calls CTask::operator delete 0x61A5B0 (this) (0x4B8740, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x85B8DC.

## 0x5EE880 sub_5ee880 (proposed: CPedShelterAttractor::~CPedShelterAttractor)  (catalogue: unknown/REA, port level 3)
- Name proposal: CPedShelterAttractor::~CPedShelterAttractor. Evidence: stores vtable 0x86C5B4 (CPedShelterAttractor) into this; callers (1): 0x5ee830 CPedAttractor::~CPedAttractor.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86C5B4` (CPedShelterAttractor) (0x5EE880); tail-jump to base dtor CPedAttractor::Destructor 0x5EC410 (0x5EE886). No other member cleanup.

## 0x5F6270 vt_86c6d0::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x5F6270; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x86C6D0.

## 0x5F6920 sub_5f6920 (proposed: CTaskAllocatorKillThreatsBasicRandomGroup::~CTaskAllocatorKillThreatsBasicRandomGroup)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskAllocatorKillThreatsBasicRandomGroup::~CTaskAllocatorKillThreatsBasicRandomGroup. Evidence: stores vtable 0x86C80C (CTaskAllocatorKillThreatsBasicRandomGroup) into this; callers (1): 0x5f6900 CTaskAllocatorKillThreatsBasicRandomGroup::~CTaskAllocatorKillThreatsBasicRandomGroup.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86C80C` (CTaskAllocatorKillThreatsBasicRandomGroup) (0x5F6920); tail-jump to base dtor CTaskAllocatorKillThreatsBasic::Destructor 0x69C780 (0x5F6926). No other member cleanup.

## 0x5FF470 vt_86cba0::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x35` (0x5FF470); `ret` -> returns 53 (EVENT_HEALTH_LOW). Vtable: slot 1 (+0x4) of vtable 0x86CBA0.

## 0x5FF480 vt_86cba0::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x5FF480; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x86CBA0.

## 0x5FF4E0 vt_86cba0::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor thunk_FUN_00401a42 0x5FF460 (0x5FF4E3); if `flags & 1` (0x5FF4E8) calls CEvent::operator delete 0x4B5630 (this) (0x5FF4F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86CBA0.

## 0x5FF530 vt_86cbe8::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x34` (0x5FF530); `ret` -> returns 52 (EVENT_HEALTH_REALLY_LOW). Vtable: slot 1 (+0x4) of vtable 0x86CBE8.

## 0x5FF540 vt_86cbe8::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x5FF540; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x86CBE8.

## 0x5FF5A0 vt_86cbe8::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_005ff520 0x5FF520 (0x5FF5A3); if `flags & 1` (0x5FF5A8) calls CEvent::operator delete 0x4B5630 (this) (0x5FF5B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86CBE8.

## 0x5FF5F0 vt_86cc30::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x32` (0x5FF5F0); `ret` -> returns 50 (EVENT_LOW_ANGER_AT_PLAYER). Vtable: slot 1 (+0x4) of vtable 0x86CC30.

## 0x5FF600 vt_86cc30::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x5FF600; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x86CC30.

## 0x5FF660 vt_86cc30::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_005ff5e0 0x5FF5E0 (0x5FF663); if `flags & 1` (0x5FF668) calls CEvent::operator delete 0x4B5630 (this) (0x5FF670, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86CC30.

## 0x5FF6B0 vt_86cc78::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x33` (0x5FF6B0); `ret` -> returns 51 (EVENT_HIGH_ANGER_AT_PLAYER). Vtable: slot 1 (+0x4) of vtable 0x86CC78.

## 0x5FF6C0 vt_86cc78::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x5FF6C0; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x86CC78.

## 0x5FF720 vt_86cc78::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_005ff6a0 0x5FF6A0 (0x5FF723); if `flags & 1` (0x5FF728) calls CEvent::operator delete 0x4B5630 (this) (0x5FF730, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86CC78.

## 0x5FF8E0 vt_86cbe8::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x45` (0x5FF8E0); `ret` -> returns 69. Vtable: slot 2 (+0x8) of vtable 0x86CBE8.

## 0x5FF8F0 vt_86cc78::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x22` (0x5FF8F0); `ret` -> returns 34. Vtable: slot 2 (+0x8) of vtable 0x86CC78.

## 0x5FF900 vt_86cba0::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x44` (0x5FF900); `ret` -> returns 68. Vtable: slot 2 (+0x8) of vtable 0x86CBA0.

## 0x5FF910 vt_86cc30::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x21` (0x5FF910); `ret` -> returns 33. Vtable: slot 2 (+0x8) of vtable 0x86CC30.

## 0x608F00 sub_608f00 (proposed: CWeaponInfo::GetAnimLoopStart(bool))  (catalogue: unknown/REA, port level 0)
- Name evidence: `this` offsets +0x40 / +0x4C = gr CWeaponInfo m_fAnimLoopStart / m_fAnimLoop2Start (WeaponInfo.h:91-95, inline at :126); callers
  CTaskSimpleUseGun::StartAnim 0x624F30, CTaskSimpleUseGun::ProcessPed 0x62A380.
- `__thiscall float (this, bool set2 [esp+4])`, `ret 4`, result in ST0: `set2 ? this->+0x4C : this->+0x40` (0x608F04-0x608F0E).

## 0x608F20 sub_608f20 (proposed: CWeaponInfo::GetAnimLoopEnd(bool))  (catalogue: unknown/REA, port level 0)
- Name evidence: +0x44 / +0x50 = m_fAnimLoopEnd / m_fAnimLoop2End (gr WeaponInfo.h:92-96, :127); caller CTaskSimpleUseGun::ProcessPed 0x62A380.
- `__thiscall float (this, bool set2 [esp+4])`, `ret 4`, ST0: `set2 ? this->+0x50 : this->+0x44`.

## 0x609300 sub_609300 (proposed: CEventPlayerCommandToGroupGather::~CEventPlayerCommandToGroupGather)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventPlayerCommandToGroupGather::~CEventPlayerCommandToGroupGather. Evidence: stores vtable 0x86D120 (CEventPlayerCommandToGroupGather) into this; callers (1): 0x6092e0 CEventPlayerCommandToGroupGather::~CEventPlayerCommandToGroupGather.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86D120` (CEventPlayerCommandToGroupGather) (0x609300); tail-jump to base dtor FUN_004b2470 0x4B2470 (0x609306). No other member cleanup.

## 0x61BC80 vt_86d594::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor thunk_FUN_0061a6f0 0x61BCA0 (0x61BC83); if `flags & 1` (0x61BC88) calls CTask::operator delete 0x61A5B0 (this) (0x61BC90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D594.

## 0x61C150 sub_61c150 (proposed: CWeaponInfo::GetAnimLoopFire(bool))  (catalogue: unknown/REA, port level 0)
- Name evidence: +0x48 / +0x54 = m_fAnimLoopFire / m_fAnimLoop2Fire (gr WeaponInfo.h:93-97); caller CTaskSimpleUseGun::ProcessPed 0x62A380.
- `__thiscall float (this, bool set2 [esp+4])`, `ret 4`, ST0: `set2 ? this->+0x54 : this->+0x48`.

## 0x61C7F0 sub_61c7f0 (proposed: CTaskSimpleFight::GetAvailableComboSet(CPed*, int8 command))  (catalogue: unknown/REA, port level 2)
- Name evidence: only caller CTaskSimpleFight::ProcessPed 0x629920 (4 sites; result stored into +0x24 m_nComboSet); `this` = CTaskSimpleFight
  (+0xA anims referenced, +0xC anim group, +0x26 next command); gr declares `GetAvailableComboSet(CPed*, int8)` (TaskSimpleFight.h:92).
- `__thiscall int8 (this, CPed* ped [esp+4], int8 cmd [esp+8])`, `ret 8`; returns AL (combo set, 4 = default unarmed).
- cmd < 0 (signed, 0x61C7FA): if `+0xC != 0x21 && +0xA == 0`: block = ms_aAnimAssocGroups(0xB4EA34)[+0xC].m_pAnimBlock (stride 0x14) or, when null,
  `GetAnimationBlock(GetAnimBlockName(+0xC))` 0x4D3A30/0x4D3940; if `block->+0x10` (loaded): `AddAnimBlockRef((block - ms_aAnimBlocks 0xB5D4A0) >> 5)`
  0x4D3FB0 and `+0xA = 1`. Return 0 (0x61C847).
- cmd >= 0: unless cmd is 0, 2 or in 11..14, return 0 (0x61C84D-0x61C85E). c = `GetWeaponInfo(ped->m_aWeapons[activeSlot].type, 1)->+0x6E` (combo).
  If cmd == 12: c = `ped->+0x72D` (fighting style) and go to the group step. Else if c == 4 and (cmd == 2 or cmd == 0): c = ped->+0x72D. Then if cmd == 0
  and `!(m_aComboData[c-4].+0x85 & 4)` return 4 (0x61C8C4-0x61C8CB -> 0x61C99A).
- Group step (0x61C8D1): g = m_aComboData[c-4].+0x0 (0xC170D0, stride 0x88). If g == 0x21 return c. If g != +0xC: if +0xA, RemoveAnimBlockRef(old
  block index) 0x4D3FD0 and `+0xA = 0`; `+0xC = g`. Else if +0xA already set return c. Resolve the block as above; if loaded: AddAnimBlockRef,
  `+0xA = 1`, return c (AL reloaded from the saved byte, 0x61C969). Else `CStreaming::RequestModel(blockIndex + 0x63E7, 8)` 0x4087E0; if
  `11 <= (i8)+0x26 <= 14` then `+0x26 = 11`; return 4.

## 0x61C9B0 sub_61c9b0 (proposed: CTaskSimpleFight::SetPlayerMoveAnim(CPlayerPed*))  (catalogue: unknown/REA, port level 8)
- Name evidence: only caller CTaskSimpleFight::ProcessPed 0x629920 (player branch, `3 <= +0x26 <= 6`, 0x629C23); `this` = CTaskSimpleFight; uses
  ped->m_pPlayerData (+0x480) +0xC/+0x10 (2D fight movement) and the four fight strafe anims 0x29..0x2C; gr declares `SetPlayerMoveAnim(CPlayerPed*)`.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Constants: 0.1 [0x858B1C], 1.0 [0x858624], 0.0 [0x858B50]; -8.0f immediate.
- `a29, a2A, a2B, a2C = RpAnimBlendClumpGetAssociation(ped->m_pRwClump, 0x29 / 0x2A / 0x2B / 0x2C)` 0x4D68B0 (in this order, 0x61C9C5-0x61C9EC).
- Stop path: if `+0x26 == 0 && +0x24 == 0` (0x61C9FD-0x61CA06), or `sqrt(m.x*m.x + m.y*m.y) < 0.1` is NOT false (`fsqrt` of (x*x)+(y*y) with x = pd+0xC,
  y = pd+0x10, accumulation `x*x` first then `+ y*y`; `fcomp 0.1; test ah,5; jp move` -> stop when length < 0.1 or equal? exact: jump to the move path
  when ah&5 is 0 or 5, i.e. length >= 0.1 or NaN): each non-null aXX gets `->m_fBlendDelta (+0x1C) = -8.0f`; `+0x24 = 0; +0x27 = 0; +0x26 = 0`; return.
- Move path (0x61CA6C): `k = 1.0 / (|y| + |x|)` (fabs y, fabs x, `faddp`, `fdivr`); `fx = k*x` stored f32, `fy = k*y` stored f32.
  - If `fx > 0.0` (strict): if a2A: `a2A->m_fBlendAmount (+0x18) = 0`; if a2C == 0: `a2C = CAnimManager::AddAnimation(clump, 0, 0x2C)` 0x4D3AA0 (group arg is the
    zero register); `a2C->+0x18 = fx`. Else if NOT (`fx < 0.0` false, `test ah,5; jp`): i.e. fx < 0.0: if a2C: `a2C->+0x18 = 0`; if a2A == 0 add anim 0x2A;
    `a2A->+0x18 = -fx` (fchs). (fx == 0 or NaN: neither.)
  - Then for fy: if `fy < 0.0` (0x61CB06-0x61CB15 not-jp path): a2B->+0x18 = 0 if present; a29 created (0x29) if null; `a29->+0x18 = -fy`. Else if
    `fy > 0.0`: a29->+0x18 = 0 if present; a2B created (0x2B) if null; `a2B->+0x18 = fy`.
  - Finally `+0x24 = 1; +0x27 = +0x26; +0x26 = 0` (0x61CB7A-0x61CB8B).
- Open question: the stop/move compare at 0x61CA24-0x61CA2F: `fcomp 0.1` with `test ah,5; jp` -> jumps (move path) when C0 = C2 (length >= 0.1, or NaN);
  stop path when length < 0.1.

## 0x61FC40 vt_86d7d8::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3EF` (0x61FC40); `ret` -> returns 1007 (TASK_SIMPLE_BE_KICKED_ON_GROUND). Vtable: slot 4 (+0x10) of vtable 0x86D7D8.

## 0x61FE60 vt_86d7fc::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3F1` (0x61FE60); `ret` -> returns 1009 (TASK_SIMPLE_BE_HIT_WHILE_MOVING). Vtable: slot 4 (+0x10) of vtable 0x86D7FC.

## 0x620B80 vt_86d868::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3EE` (0x620B80); `ret` -> returns 1006 (TASK_COMPLEX_REACT_TO_ATTACK). Vtable: slot 4 (+0x10) of vtable 0x86D868.

## 0x6221B0 vt_86da1c::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3F6` (0x6221B0); `ret` -> returns 1014 (TASK_KILL_ALL_THREATS). Vtable: slot 4 (+0x10) of vtable 0x86DA1C.

## 0x622230 vt_86da1c::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x622230); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86DA1C.

## 0x625B80 vt_86d7d8::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0061fc50 0x61FC50 (0x625B83); if `flags & 1` (0x625B88) calls CTask::operator delete 0x61A5B0 (this) (0x625B90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D7D8.

## 0x625BA0 vt_86d7fc::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0061fe70 0x61FE70 (0x625BA3); if `flags & 1` (0x625BA8) calls CTask::operator delete 0x61A5B0 (this) (0x625BB0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D7FC.

## 0x625C00 vt_86d868::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00620b90 0x620B90 (0x625C03); if `flags & 1` (0x625C08) calls CTask::operator delete 0x61A5B0 (this) (0x625C10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D868.

## 0x6293A0 vt_86da1c::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006221c0 0x6221C0 (0x6293A3); if `flags & 1` (0x6293A8) calls CTask::operator delete 0x61A5B0 (this) (0x6293B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DA1C.

## 0x62F270 sub_62f270 (proposed: CTaskComplexLeaveCarAndDie::~CTaskComplexLeaveCarAndDie)  (catalogue: unknown/REA, port level 3)
- Name proposal: CTaskComplexLeaveCarAndDie::~CTaskComplexLeaveCarAndDie. Evidence: stores vtable 0x86DD00 (CTaskComplexLeaveCarAndDie) into this; callers (1): 0x62f250 CTaskComplexLeaveCar::~CTaskComplexLeaveCar.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DD00` (CTaskComplexLeaveCarAndDie) (0x62F270); tail-jump to base dtor CTaskComplexLeaveCar::~CTaskComplexLeaveCar 0x63B970 (0x62F276). No other member cleanup.

## 0x62FCF0 sub_62fcf0 (proposed: CTaskComplexDieInCar::~CTaskComplexDieInCar)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexDieInCar::~CTaskComplexDieInCar. Evidence: stores vtable 0x86DE04 (CTaskComplexDieInCar) into this; callers (1): 0x6375d0 CTaskComplexDieInCar::~CTaskComplexDieInCar.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DE04` (CTaskComplexDieInCar) (0x62FCF0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x62FCF6). No other member cleanup.

## 0x62FF10 sub_62ff10 (proposed: CTaskSimpleDrown::~CTaskSimpleDrown)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleDrown::~CTaskSimpleDrown. Evidence: stores vtable 0x86DE30 (CTaskSimpleDrown) into this; callers (1): 0x6378d0 CTaskSimpleDrown::~CTaskSimpleDrown.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DE30` (CTaskSimpleDrown) (0x62FF10); tail-jump to base dtor CTaskSimpleDie::~CTaskSimpleDie 0x62FB40 (0x62FF16). No other member cleanup.

## 0x6300C0 sub_6300c0 (proposed: CTaskComplexDie::~CTaskComplexDie)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexDie::~CTaskComplexDie. Evidence: stores vtable 0x86DE78 (CTaskComplexDie) into this; callers (1): 0x637910 CTaskComplexDie::~CTaskComplexDie.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DE78` (CTaskComplexDie) (0x6300C0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x6300C6). No other member cleanup.

## 0x632B50 vt_86e1dc::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xEF` (0x632B50); `ret` -> returns 239 (TASK_SIMPLE_WAIT_FOR_PIZZA). Vtable: slot 4 (+0x10) of vtable 0x86E1DC.

## 0x632B60 vt_86e1dc::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x632B60; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86E1DC.

## 0x6344F0 vt_86e3cc::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x10D` (0x6344F0); `ret` -> returns 269 (TASK_SIMPLE_TRIGGER_LOOK_AT). Vtable: slot 4 (+0x10) of vtable 0x86E3CC.

## 0x634610 vt_86e3cc::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x634610; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86E3CC.

## 0x636850 sub_636850 (proposed: CTaskComplexUseEffectRunning::~CTaskComplexUseEffectRunning)  (catalogue: unknown/REA, port level 16)
- Name proposal: CTaskComplexUseEffectRunning::~CTaskComplexUseEffectRunning. Evidence: stores vtable 0x86E594 (CTaskComplexUseEffectRunning) into this; callers (1): 0x636830 CTaskComplexUseEffectRunning::~CTaskComplexUseEffectRunning.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E594` (CTaskComplexUseEffectRunning) (0x636850); tail-jump to base dtor CTaskComplexUseEffect::Destructor 0x632270 (0x636856). No other member cleanup.

## 0x636930 sub_636930 (proposed: CTaskComplexUseEffectSprinting::~CTaskComplexUseEffectSprinting)  (catalogue: unknown/REA, port level 16)
- Name proposal: CTaskComplexUseEffectSprinting::~CTaskComplexUseEffectSprinting. Evidence: stores vtable 0x86E5C0 (CTaskComplexUseEffectSprinting) into this; callers (1): 0x636910 CTaskComplexUseEffectSprinting::~CTaskComplexUseEffectSprinting.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E5C0` (CTaskComplexUseEffectSprinting) (0x636930); tail-jump to base dtor CTaskComplexUseEffect::Destructor 0x632270 (0x636936). No other member cleanup.

## 0x637320 sub_637320 (proposed: CTaskSimpleHitFromFront::~CTaskSimpleHitFromFront (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitFromFront::~CTaskSimpleHitFromFront (non-deleting dtor). Evidence: only caller 0x637300 is slot 0 (scalar deleting dtor) of vtable 0x86E5EC (CTaskSimpleHitFromFront); callers (1): 0x637300 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637320): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x637350 sub_637350 (proposed: CTaskSimpleHitFromBack::~CTaskSimpleHitFromBack (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitFromBack::~CTaskSimpleHitFromBack (non-deleting dtor). Evidence: only caller 0x637330 is slot 0 (scalar deleting dtor) of vtable 0x86DB70 (CTaskSimpleHitFromBack); callers (1): 0x637330 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637350): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x637380 sub_637380 (proposed: CTaskSimpleHitFromLeft::~CTaskSimpleHitFromLeft (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitFromLeft::~CTaskSimpleHitFromLeft (non-deleting dtor). Evidence: only caller 0x637360 is slot 0 (scalar deleting dtor) of vtable 0x86DBA4 (CTaskSimpleHitFromLeft); callers (1): 0x637360 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637380): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x6373B0 sub_6373b0 (proposed: CTaskSimpleHitFromRight::~CTaskSimpleHitFromRight (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitFromRight::~CTaskSimpleHitFromRight (non-deleting dtor). Evidence: only caller 0x637390 is slot 0 (scalar deleting dtor) of vtable 0x86DBD8 (CTaskSimpleHitFromRight); callers (1): 0x637390 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x6373B0): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x6373E0 sub_6373e0 (proposed: CTaskSimpleHitByGunFromFront::~CTaskSimpleHitByGunFromFront (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitByGunFromFront::~CTaskSimpleHitByGunFromFront (non-deleting dtor). Evidence: only caller 0x6373c0 is slot 0 (scalar deleting dtor) of vtable 0x86DC10 (CTaskSimpleHitByGunFromFront); callers (1): 0x6373c0 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x6373E0): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x637410 sub_637410 (proposed: CTaskSimpleHitByGunFromRear::~CTaskSimpleHitByGunFromRear (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitByGunFromRear::~CTaskSimpleHitByGunFromRear (non-deleting dtor). Evidence: only caller 0x6373f0 is slot 0 (scalar deleting dtor) of vtable 0x86DC4C (CTaskSimpleHitByGunFromRear); callers (1): 0x6373f0 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637410): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x637440 sub_637440 (proposed: CTaskSimpleHitByGunFromLeft::~CTaskSimpleHitByGunFromLeft (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitByGunFromLeft::~CTaskSimpleHitByGunFromLeft (non-deleting dtor). Evidence: only caller 0x637420 is slot 0 (scalar deleting dtor) of vtable 0x86DC88 (CTaskSimpleHitByGunFromLeft); callers (1): 0x637420 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637440): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x637470 sub_637470 (proposed: CTaskSimpleHitByGunFromRight::~CTaskSimpleHitByGunFromRight (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitByGunFromRight::~CTaskSimpleHitByGunFromRight (non-deleting dtor). Evidence: only caller 0x637450 is slot 0 (scalar deleting dtor) of vtable 0x86DCC4 (CTaskSimpleHitByGunFromRight); callers (1): 0x637450 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x637470): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x6389D0 vt_86e1dc::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00632b70 0x632B70 (0x6389D3); if `flags & 1` (0x6389D8) calls CTask::operator delete 0x61A5B0 (this) (0x6389E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E1DC.

## 0x6394D0 vt_86e3cc::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00634500 0x634500 (0x6394D3); if `flags & 1` (0x6394D8) calls CTask::operator delete 0x61A5B0 (this) (0x6394E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E3CC.

## 0x63A1D0 sub_63a1d0 (proposed: CEventLeaderQuitEnteringCarAsDriver::~CEventLeaderQuitEnteringCarAsDriver)  (catalogue: unknown/REA, port level 1)
- Name proposal: CEventLeaderQuitEnteringCarAsDriver::~CEventLeaderQuitEnteringCarAsDriver. Evidence: stores vtable 0x86E6B0 (CEventLeaderQuitEnteringCarAsDriver) into this; callers (1): 0x63a1b0 CEventLeaderQuitEnteringCarAsDriver::~CEventLeaderQuitEnteringCarAsDriver.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86E6B0` (CEventLeaderQuitEnteringCarAsDriver) (0x63A1D0); tail-jump to base dtor FUN_004ac010 0x4AC010 (0x63A1D6). No other member cleanup.

## 0x63BBD0 sub_63bbd0 (proposed: CTaskComplexLeaveBoat::~CTaskComplexLeaveBoat (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name evidence: stores vtable 0x86E854 (CTaskComplexLeaveBoat, same class whose ctor 0x63BB60 builds 0x14-byte tasks) into this; only caller 0x642250
  (CTaskComplexLeaveBoat scalar deleting dtor).
- `__thiscall void (this)`, SEH frame: vtbl = 0x86E854; if `+0xC` (target vehicle) != 0 `CleanUpOldReference(&+0xC)` 0x571A00; CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x647770 vt_86ee94::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x32F` (0x647770); `ret` -> returns 815 (TASK_SIMPLE_CAR_FORCE_PED_OUT). Vtable: slot 4 (+0x10) of vtable 0x86EE94.

## 0x647780 vt_86ee94::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x647780; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86EE94.

## 0x6477F0 vt_86ee94::ProcessPed (proposed: CTaskSimpleCarForcePedOut::ProcessPed)  (catalogue: specified/REA, port level 23)
- Name evidence: slot 7 of vtable 0x86EE94 whose GetTaskType 0x647770 returns 0x32F (TASK_SIMPLE_CAR_FORCE_PED_OUT); layout +0x8 vehicle, +0xC door
  (ctor 0x647710, see CTaskComplexLeaveCar.md CreateSubTask).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Returns AL = 1 (done) or 0.
- If `+0x8 == 0` return 1 (0x6477FA). If `veh->IsDriver(ped)` 0x6D1C40: `ped->PositionPedOutOfCollision(door, veh, true)` 0x5E0820 (args pushed
  1, veh, door), return 1.
- Else (0x647829): `f = CCarEnterExit::ComputeOppositeDoorFlag(veh, door, false)` 0x64E610; if `f & veh->m_nGettingOutFlags (+0x487)` (byte test):
  if `veh->m_pDriver (+0x460) != 0` return 0 (0x64784D -> 0x6478A5). Else `n = ComputePassengerIndexFromCarDoor(veh, door)` 0x64F1E0; scan
  `i = 0 .. n-1` for the first non-null `veh->m_apPassengers[i]` (+0x464 + 4*i); if one is found return 0 (0x64787F-0x647888). (Note: it tests the
  passenger slots BELOW the door's index, not the slot itself - original behaviour.)
- Otherwise `ped->PositionPedOutOfCollision(door, veh, true)` and return 1 (0x64788A-0x64789D).

## 0x64BF50 vt_86ee94::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00647790 0x647790 (0x64BF53); if `flags & 1` (0x64BF58) calls CTask::operator delete 0x61A5B0 (this) (0x64BF60, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EE94.

## 0x64E4B0 sub_64e4b0 (proposed: CVehicleAnimGroupData::ComputeCriticalBlendTime(int32 groupId, int32 animId))  (catalogue: unknown/REA, port level 1)
- Name evidence: indexes CVehicleAnimGroupData::m_vehicleAnimGroups 0xC1CDC0 (stride 0x94) and ends in CVehicleAnimGroup::ComputeCriticalBlendTime;
  only caller CTaskUtilityLineUpPedWithCar::ProcessPed 0x6513A0 (3 sites).
- `__cdecl float (int32 group [esp+4], int32 animId [esp+8])`, plain `ret`, result in ST0. Entry is obfuscated: `nop; jmp 0x40107D` performs
  `esi = group * 0x94` and jumps back to 0x64E4BB (no other effect).
- `g = &m_vehicleAnimGroups[group]`; `g->GetGroup(animId)` 0x6E3B00 (EAX result discarded, 0x64E4C9); returns `g->ComputeCriticalBlendTime(animId)`
  0x6E3C80 (0x64E4D1, ST0 passed through).

## 0x655DD0 sub_655dd0 (proposed: CTaskSimpleHitFromBehind::~CTaskSimpleHitFromBehind (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitFromBehind::~CTaskSimpleHitFromBehind (non-deleting dtor). Evidence: only caller 0x655db0 is slot 0 (scalar deleting dtor) of vtable 0x86F168 (CTaskSimpleHitFromBehind); callers (1): 0x655db0 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x655DD0): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x655E00 sub_655e00 (proposed: CTaskSimpleHitWall::~CTaskSimpleHitWall (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleHitWall::~CTaskSimpleHitWall (non-deleting dtor). Evidence: only caller 0x655de0 is slot 0 (scalar deleting dtor) of vtable 0x86F1A0 (CTaskSimpleHitWall); callers (1): 0x655de0 CTaskSimpleAnim::~CTaskSimpleAnim.
- Tail-jump thunk (0x655E00): `jmp CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0` with all registers and stack args unchanged (same convention as the target).

## 0x658740 vt_86f45c::vfunc_9  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x658740; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 9 (+0x24) of vtable 0x86F45C.

## 0x659230 vt_86f544::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x6A` (0x659230); `ret` -> returns 106 (TASK_COMPLEX_PRESENT_ID_TO_COP). Vtable: slot 4 (+0x10) of vtable 0x86F544.

## 0x6597B0 sub_6597b0 (proposed: CTaskComplexUseWaterCannon::~CTaskComplexUseWaterCannon)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexUseWaterCannon::~CTaskComplexUseWaterCannon. Evidence: stores vtable 0x86F5D0 (CTaskComplexUseWaterCannon) into this; callers (1): 0x65a620 CTaskComplexUseWaterCannon::~CTaskComplexUseWaterCannon.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86F5D0` (CTaskComplexUseWaterCannon) (0x6597B0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x6597B6). No other member cleanup.

## 0x6598A0 vt_86f5fc::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x6E` (0x6598A0); `ret` -> returns 110 (TASK_COMPLEX_EXTINGUISH_FIRE_ON_FOOT). Vtable: slot 4 (+0x10) of vtable 0x86F5FC.

## 0x659980 vt_86f5fc::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x659980); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86F5FC.

## 0x659DE0 vt_86f45c::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor thunk_FUN_0061a6f0 0x659E00 (0x659DE3); if `flags & 1` (0x659DE8) calls CTask::operator delete 0x61A5B0 (this) (0x659DF0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F45C.

## 0x65A0F0 vt_86f544::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00659240 0x659240 (0x65A0F3); if `flags & 1` (0x65A0F8) calls CTask::operator delete 0x61A5B0 (this) (0x65A100, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F544.

## 0x65A700 vt_86f5fc::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006598b0 0x6598B0 (0x65A703); if `flags & 1` (0x65A708) calls CTask::operator delete 0x61A5B0 (this) (0x65A710, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F5FC.

## 0x65EEF0 sub_65eef0 (proposed: CTaskSimpleGoToPointFine::~CTaskSimpleGoToPointFine)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleGoToPointFine::~CTaskSimpleGoToPointFine. Evidence: stores vtable 0x86F964 (CTaskSimpleGoToPointFine) into this; callers (1): 0x6634e0 CTaskSimpleGoToPointFine::~CTaskSimpleGoToPointFine.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86F964` (CTaskSimpleGoToPointFine) (0x65EEF0); tail-jump to base dtor CTaskSimpleGoTo::~CTaskSimpleGoTo 0x667A00 (0x65EEF6). No other member cleanup.

## 0x668630 sub_668630 (proposed: CTaskComplexGoToPointAndStandStill::~CTaskComplexGoToPointAndStandStill)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexGoToPointAndStandStill::~CTaskComplexGoToPointAndStandStill. Evidence: stores vtable 0x86FDA8 (CTaskComplexGoToPointAndStandStill) into this; callers (1): 0x66dc70 CTaskComplexGoToPointAndStandStillTimed::~CTaskComplexGoToPointAndStandStillTimed.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86FDA8` (CTaskComplexGoToPointAndStandStill) (0x668630); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x668636). No other member cleanup.

## 0x66AAD0 vt_86ff00::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3AB` (0x66AAD0); `ret` -> returns 939 (TASK_COMPLEX_AVOID_ENTITY). Vtable: slot 4 (+0x10) of vtable 0x86FF00.

## 0x66C4C0 sub_66c4c0 (proposed: CTaskSimpleSlideToCoord::~CTaskSimpleSlideToCoord)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimpleSlideToCoord::~CTaskSimpleSlideToCoord. Evidence: stores vtable 0x86FFEC (CTaskSimpleSlideToCoord) into this; callers (1): 0x6712c0 CTaskSimpleSlideToCoord::~CTaskSimpleSlideToCoord.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86FFEC` (CTaskSimpleSlideToCoord) (0x66C4C0); tail-jump to base dtor CTaskSimpleAnim::~CTaskSimpleAnim 0x61A6F0 (0x66C4C6). No other member cleanup.

## 0x66CB20 vt_87004c::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x38B` (0x66CB20); `ret` -> returns 907 (TASK_COMPLEX_SEEK_ENTITY). Vtable: slot 4 (+0x10) of vtable 0x87004C.

## 0x66CB90 vt_87004c::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0066cbb0 0x66CBB0 (0x66CB93); if `flags & 1` (0x66CB98) calls CTask::operator delete 0x61A5B0 (this) (0x66CBA0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x87004C.

## 0x66DFA0 sub_66dfa0 (proposed: CTaskComplexGoToPointShooting::~CTaskComplexGoToPointShooting (non-deleting dtor))  (catalogue: unknown/REA, port level 3)
- Name proposal: CTaskComplexGoToPointShooting::~CTaskComplexGoToPointShooting (non-deleting dtor). Evidence: only caller 0x66df80 is slot 0 (scalar deleting dtor) of vtable 0x86FE2C (CTaskComplexGoToPointShooting); callers (1): 0x66df80 CTaskComplexGoToPointShooting::~CTaskComplexGoToPointShooting.
- Tail-jump thunk (0x66DFA0): `jmp CTaskComplexGoToPointAiming::Destructor 0x668870` with all registers and stack args unchanged (same convention as the target).

## 0x66F6A0 vt_86ff00::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0066aae0 0x66AAE0 (0x66F6A3); if `flags & 1` (0x66F6A8) calls CTask::operator delete 0x61A5B0 (this) (0x66F6B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FF00.

## 0x670A60 vt_870114::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A3` (0x670A60); `ret` -> returns 931 (TASK_COMPLEX_FOLLOW_PATROL_ROUTE). Vtable: slot 4 (+0x10) of vtable 0x870114.

## 0x672BB0 vt_870114::Destructor  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006709f0 0x6709F0 (0x672BB3); if `flags & 1` (0x672BB8) calls CTask::operator delete 0x61A5B0 (this) (0x672BC0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870114.

## 0x674EE0 vt_8701a8::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x51` (0x674EE0); `ret` -> returns 81 (EVENT_INTERIOR_USE_INFO). Vtable: slot 1 (+0x4) of vtable 0x8701A8.

## 0x674EF0 vt_8701a8::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args: returns 0 / nullptr (`xor eax, eax` 0x674EF0; plain `ret`). Vtable: slot 3 (+0xC) of vtable 0x8701A8.

## 0x674F80 vt_8701a8::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x674F80; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 5 (+0x14) of vtable 0x8701A8.

## 0x674F90 vt_8701a8::Destructor  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor thunk_FUN_00401ac8 0x674ED0 (0x674F93); if `flags & 1` (0x674F98) calls CEvent::operator delete 0x4B5630 (this) (0x674FA0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x8701A8.

## 0x674FB0 vt_8701a8::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x9` (0x674FB0); `ret` -> returns 9. Vtable: slot 2 (+0x8) of vtable 0x8701A8.

## 0x675C80 vt_870314::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x57F` (0x675C80); `ret` -> returns 1407 (TASK_INTERIOR_SIT_ON_CHAIR). Vtable: slot 4 (+0x10) of vtable 0x870314.

## 0x678160 sub_678160 (proposed: CTaskComplexGetUpAndStandStill::~CTaskComplexGetUpAndStandStill)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexGetUpAndStandStill::~CTaskComplexGetUpAndStandStill. Evidence: stores vtable 0x870404 (CTaskComplexGetUpAndStandStill) into this; callers (1): 0x67ca00 CTaskComplexGetUpAndStandStill::~CTaskComplexGetUpAndStandStill.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870404` (CTaskComplexGetUpAndStandStill) (0x678160); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x678166). No other member cleanup.

## 0x678790 sub_678790 (proposed: CTaskComplexFallAndGetUp::~CTaskComplexFallAndGetUp)  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskComplexFallAndGetUp::~CTaskComplexFallAndGetUp. Evidence: stores vtable 0x870454 (CTaskComplexFallAndGetUp) into this; callers (1): 0x67cb50 CTaskComplexFallAndGetUp::~CTaskComplexFallAndGetUp.
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870454` (CTaskComplexFallAndGetUp) (0x678790); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x678796). No other member cleanup.

## 0x6935B0 sub_6935b0 (proposed: CTaskSimplePickUpEntity::~CTaskSimplePickUpEntity (non-deleting dtor))  (catalogue: unknown/REA, port level 2)
- Name proposal: CTaskSimplePickUpEntity::~CTaskSimplePickUpEntity (non-deleting dtor). Evidence: only caller 0x693590 is slot 0 (scalar deleting dtor) of vtable 0x870B50 (CTaskSimplePickUpEntity); callers (1): 0x693590 CTaskSimplePickUpEntity::~CTaskSimplePickUpEntity.
- Tail-jump thunk (0x6935B0): `jmp FUN_00691630 0x691630` with all registers and stack args unchanged (same convention as the target).
