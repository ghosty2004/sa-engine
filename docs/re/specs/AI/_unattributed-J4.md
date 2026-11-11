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

## 0x499440 vt_85a428::Clone (proposed: UNCLEAR task class with vtable 0x85A428 ::Clone)  (catalogue: unknown/REA, port level 3)
- Name proposal: slot 1 (Clone) of the 15-slot task vtable 0x85A428 (slot 0 0x4994C0, slot 4 GetTaskType 0x499430); the class is not
  identified (no gr reference to the vtable).
- `__thiscall CTask* (this)`, plain `ret`, SEH frame. `sub = this->+0x28 ? this->+0x28->Clone() (vfunc +4) : 0`; `p = CTask::operator
  new(0x50)`; if p: return `ctor 0x4993D0(p, this->+0xC (dword), (u8)this->+0x10, sub)` (thiscall, 3 stack args; 0x4993D0 is a jump
  thunk into the .HOODLUM section 0x156E930). If the allocation fails return 0 (the cloned sub-object is leaked).

## 0x4AB9B0 sub_4ab9b0 (proposed: CEventGroup::~CEventGroup (non-deleting))  (catalogue: unknown/REA, port level 1)
- Name proposal: same body as the deleting dtor 0x4ABA20 without the delete; stores the CEventGroup vtable 0x85AAB0.
- `__thiscall void (this)`, plain `ret`: `this->vtbl = 0x85AAB0`; `CEventGroup::Flush(this, false)` 0x4AB370 (push 0).

## 0x4ABCA0 sub_4abca0 (proposed: CEventDamage::HasKilledPed)  (catalogue: unknown/REA, port level 0)
- Name proposal: gr EventDamage.h:68 `//! @addr 0x4ABCA0 bool HasKilledPed() const { return m_damageResponse.m_bHealthZero &&
  m_bAddToEventGroup; }`; called from CEventHandler::ComputeDamageResponse 0x4C02C4.
- `__thiscall bool (this)`, plain `ret`: returns EAX = 1 if `this->+0x40 != 0` (m_damageResponse.m_bHealthZero) and `this->+0xC != 0`
  (m_bAddToEventGroup), else EAX = 0 (full EAX written). Matches gr.

## 0x4ADBF0 sub_4adbf0 (proposed: CEventDamage::IsSameEventForAI)  (catalogue: unknown/REA, port level 0)
- Name proposal: gr EventDamage.h:65 `//! @addr 0x4ADBF0 IsSameEventForAI(const CEventDamage& o)`.
- `__thiscall bool (this, const CEventDamage* o [esp+4])`, `ret 4`: returns AL = 1 if `this->+0x14 == o->+0x14` (source entity) and
  `this->+0x1C == o->+0x1C` (weapon type), else AL = 0 (upper EAX = o pointer bits / stale). Matches gr.

## 0x4AE250 vt_85ae88::GetLocalSoundLevel (proposed: CEventPotentialGetRunOver::GetLocalSoundLevel)  (catalogue: unknown/REA, port level 0)
- Name proposal: slot 12 of event vtable 0x85AE88 whose GetEventType (0x4AE210) returns 12 = EVENT_POTENTIAL_GET_RUN_OVER.
- `__thiscall float (this)`: `fld [0x858628]` = 100.0f; returns ST0 = 100.0f; plain `ret`.

## 0x4AEB60 vt_85afe8::GetSourceEntity (proposed: CEventPedEnteredMyVehicle::GetSourceEntity)  (catalogue: unknown/REA, port level 0)
- Name proposal: slot 10 of event vtable 0x85AFE8 (GetEventType 0x4AEB40 returns 17 = EVENT_PED_ENTERED_MY_VEHICLE).
- `__thiscall CEntity* (this)`: returns `this->+0x14` (the ped that entered); plain `ret`.

## 0x4AEB70 vt_85afe8::GetLocalSoundLevel (proposed: CEventPedEnteredMyVehicle::GetLocalSoundLevel)  (catalogue: unknown/REA, port level 0)
- Name proposal: slot 12 of event vtable 0x85AFE8 (EVENT_PED_ENTERED_MY_VEHICLE).
- `__thiscall float (this)`: returns ST0 = `[0x858B34]=60.0f`; plain `ret`.

## 0x4AEC00 vt_85afe8::AffectsPed (proposed: CEventPedEnteredMyVehicle::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x85AFE8 (EVENT_PED_ENTERED_MY_VEHICLE).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = 1 iff `ped->IsAlive()` 0x5E0170 and `ped->+0x58C` (vehicle) != 0
  and `ped->+0x46C & 0x100` (bInVehicle) and `ped->+0x58C == this->+0x18` (event vehicle) and `this->+0x14 != 0` (setne); else 0.
  Checks in that order (short-circuit).

## 0x4AEC50 sub_4aec50 (proposed: CEventRevived::CEventRevived)  (catalogue: unknown/REA, port level 0)
- Name proposal: stores vtable 0x85B030 (GetEventType 0x4AEC70 returns 18 = EVENT_REVIVE) after the inlined CEvent base init.
- `__thiscall CEvent* (this)`, plain `ret`: `this->+4 = 0` (m_nTimeActive), `this->+8 = 0` (u8 m_bValid), `this->vtbl = 0x85B030`;
  returns this.

## 0x4AECB0 vt_85b030::AffectsPed (proposed: CEventRevived::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x85B030 (EVENT_REVIVE).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: if `ped->+0x484 == 2` (u8 m_nCreatedBy; 2 = mission ped) return
  AL = 0; else return `ped->IsAlive() ? 0 : 1` (0x5E0170; `neg al; sbb eax,eax; inc eax` -> full EAX = 1 when not alive, 0 when alive).

## 0x4AFC50 vt_85b298::AffectsPed (proposed: CEventVehicleThreat::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x85B298 (its GetEventType identifies CEventVehicleThreat).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = `ped->IsAlive()` 0x5E0170 != 0 (setne; upper EAX stale).

## 0x4B0E60 vt_86cba0::AffectsPed (proposed: CEventHealthLow::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x86CBA0 (its GetEventType identifies CEventHealthLow).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = `ped->IsAlive()` 0x5E0170 != 0 (setne; upper EAX stale).

## 0x4B0EA0 vt_86cc30::AffectsPed (proposed: CEventLowAngerAtPlayer::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x86CC30 (its GetEventType identifies CEventLowAngerAtPlayer).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = `ped->IsAlive()` 0x5E0170 != 0 (setne; upper EAX stale).

## 0x4B0EC0 vt_86cc78::AffectsPed (proposed: CEventHighAngerAtPlayer::AffectsPed)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 5 of event vtable 0x86CC78 (its GetEventType identifies CEventHighAngerAtPlayer).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: returns AL = `ped->IsAlive()` 0x5E0170 != 0 (setne; upper EAX stale).

## 0x4B1640 vt_8701a8::IsValid (proposed: CEventInteriorUseInfo::IsValid)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 14 of event vtable 0x8701A8 (GetEventType 0x674EE0 returns 81 = EVENT_INTERIOR_USE_INFO).
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: if ped != 0 return `ped->IsAlive()` 0x5E0170 (AL; EAX upper stale). Else return
  EAX = 1 if `this->+8` (m_bValid) != 0 or `this->+4 (m_nTimeActive) <= this->GetLifeTime()` (vfunc +0xC; signed compare), else 0.

## 0x4B5EA0 vt_85ae88::Destructor (proposed: CEventPotentialGetRunOver::~CEventPotentialGetRunOver)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventPotentialGetRunOver::~CEventPotentialGetRunOver (scalar deleting). Evidence: slot 0 (scalar deleting dtor) of event vtable 0x85AE88; calls the non-deleting dtor 0x4AE270.
- `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: `dtor 0x4AE270(this)`; if `flags & 1`: return the object to CEventPool `P = *(0xB744AC)` (ms_pEventPool): `idx = (this - P->+0) / 0x44` (signed division via the magic multiply 0x78787879, `sar 5`, rounding toward zero); `P->+4[idx] |= 0x80` (free flag); if `idx < P->+0xC` `P->+0xC = idx` (first-free hint). Returns this in EAX.

## 0x4B6030 vt_85afe8::Destructor (proposed: CEventPedEnteredMyVehicle::~CEventPedEnteredMyVehicle)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventPedEnteredMyVehicle::~CEventPedEnteredMyVehicle (scalar deleting). Evidence: slot 0 (scalar deleting dtor) of event vtable 0x85AFE8; calls the non-deleting dtor 0x4AEB90.
- `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: `dtor 0x4AEB90(this)`; if `flags & 1`: return the object to CEventPool `P = *(0xB744AC)` (ms_pEventPool): `idx = (this - P->+0) / 0x44` (signed division via the magic multiply 0x78787879, `sar 5`, rounding toward zero); `P->+4[idx] |= 0x80` (free flag); if `idx < P->+0xC` `P->+0xC = idx` (first-free hint). Returns this in EAX.

## 0x4B6080 vt_85b030::Destructor (proposed: CEventRevived::~CEventRevived)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventRevived::~CEventRevived (scalar deleting). Evidence: slot 0 (scalar deleting dtor) of event vtable 0x85B030; calls the non-deleting dtor 0x4AECA0. ECX is not saved across the call: the code relies on the callee returning with ECX = this.
- `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: `dtor 0x4AECA0(this)`; if `flags & 1`: return the object to CEventPool `P = *(0xB744AC)` (ms_pEventPool): `idx = (this - P->+0) / 0x44` (signed division via the magic multiply 0x78787879, `sar 5`, rounding toward zero); `P->+4[idx] |= 0x80` (free flag); if `idx < P->+0xC` `P->+0xC = idx` (first-free hint). Returns this in EAX.

## 0x4B6350 vt_85b298::Destructor (proposed: CEventVehicleThreat::~CEventVehicleThreat)  (catalogue: unknown/REA, port level 2)
- Name proposal: CEventVehicleThreat::~CEventVehicleThreat (scalar deleting). Evidence: slot 0 (scalar deleting dtor) of event vtable 0x85B298; calls the non-deleting dtor 0x4AFBF0.
- `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: `dtor 0x4AFBF0(this)`; if `flags & 1`: return the object to CEventPool `P = *(0xB744AC)` (ms_pEventPool): `idx = (this - P->+0) / 0x44` (signed division via the magic multiply 0x78787879, `sar 5`, rounding toward zero); `P->+4[idx] |= 0x80` (free flag); if `idx < P->+0xC` `P->+0xC = idx` (first-free hint). Returns this in EAX.

## 0x4B6F40 vt_85ae88::CloneEditable (proposed: CEventPotentialGetRunOver::CloneEditable)  (catalogue: unknown/REA, port level 3)
- Name proposal: CEventPotentialGetRunOver::CloneEditable. Evidence: slot 16 (CloneEditable) of event vtable 0x85AE88.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570; if p ==
  0 return 0; else return `ctor 0x4AE1A0(p, this->+0x14)` (thiscall).

## 0x4B7170 vt_85afe8::CloneEditable (proposed: CEventPedEnteredMyVehicle::CloneEditable)  (catalogue: unknown/REA, port level 3)
- Name proposal: CEventPedEnteredMyVehicle::CloneEditable. Evidence: slot 16 (CloneEditable) of event vtable 0x85AFE8.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570; if p ==
  0 return 0; else return `ctor 0x4AEAC0(p, this->+0x14, this->+0x18, this->+0x1C)` (thiscall).

## 0x4B71E0 vt_85b030::Clone (proposed: CEventRevived::Clone)  (catalogue: unknown/REA, port level 1)
- Name proposal: slot 4 (Clone) of event vtable 0x85B030 (EVENT_REVIVE).
- `__thiscall CEvent* (this)`, plain `ret`: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570; if p: `p->+4 = 0; p->+8 = 0 (u8);
  p->vtbl = 0x85B030`; return p (0 if the pool is full). `this` is not read.

## 0x4B7520 vt_85b298::CloneEditable (proposed: CEventVehicleThreat::CloneEditable)  (catalogue: unknown/REA, port level 3)
- Name proposal: CEventVehicleThreat::CloneEditable. Evidence: slot 16 (CloneEditable) of event vtable 0x85B298.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570; if p ==
  0 return 0; else return `ctor 0x4AFB50(p, this->+0x14)` (thiscall).

## 0x4B86C0 vt_85b8dc::Clone (proposed: CTaskSimpleOnEscalator::Clone)  (catalogue: unknown/REA, port level 2)
- Name proposal: slot 1 of task vtable 0x85B8DC whose GetTaskType (0x4B8720) returns 1308 = TASK_SIMPLE_ON_ESCALATOR.
- `__thiscall CTask* (this)`, plain `ret`, SEH frame: `p = CTask::operator new(0x20)` 0x61A5A0; if p: `CTaskSimpleStandStill::
  CTaskSimpleStandStill(p, 0, true, false, 8.0f)` 0x62F310 (pushes 8.0f, 0, 1, 0), `p->vtbl = 0x85B8DC`; return p (0 on failure).
  `this` is not read.

## 0x623B10 sub_623b10 (proposed: CTaskSimpleFight::StartAnim(CPed* ped, int32 move))  (catalogue: unknown/REA, port level 9)
- Name proposal: CTaskSimpleFight::StartAnim. Evidence: only caller CTaskSimpleFight::ProcessPed 0x629920; `this` is a CTaskSimpleFight
  (fields +0x1C m_pAnim, +0x20 m_pIdleAnim, +0x24..+0x27 used as gr's m_nComboSet / m_nCurrentMove / m_nNextCommand / m_nLastCommand);
  it is the function that blends every fight animation; gr declares `void StartAnim(CPed*, int32)` (2 stack args, matches `ret 8`).
- `__thiscall void (this, CPed* ped [esp+4], int32 move [esp+8])`, `ret 8`. Callees: BlendAnimation 0x4D4610 (cdecl (clump, group, id,
  f32 delta)), SetFinishCallback 0x4CEBE0 / SetDeleteCallback 0x4CEBC0 (callback 0x61DAE0 = the fight anim callback, data = this),
  SetCurrentTime 0x4CEA80, CStats::GetFatAndMuscleModifier 0x559AF0, CPed::SetMoveState 0x5DEC00, CPed::IsPlayer 0x5DF8F0,
  GetWeaponInfo 0x743C60, RemoveAnimBlockRef 0x4D3FD0, AddAnimBlockRef 0x4D3FB0, GetAnimBlockName 0x4D3A30, GetAnimationBlock 0x4D3940,
  CStreaming::RequestModel 0x4087E0, CPedIntelligence::GetTaskDuck 0x6010A0, sub_61C9B0 (thiscall (this, ped)).
- Combo data: `CD(k) = CTaskSimpleFight::m_aComboData[k - 4]` (0xC170D0, stride 0x88): +0 anim group, +0x44 ground-loop time
  (0xC17114), byte +0x85 = high byte of the u16 flags (0xC17155). CPed: +0x18 clump, +0x480 m_pPlayerData, +0x4D4 m_nAnimGroup,
  +0x534 m_nMoveState, +0x538 the following dword (gr m_nSwimmingMoveState), +0x5A0 weapons (stride 0x1C, +0 type), +0x718 active slot,
  +0x72D fighting style. CAnimBlendAssociation: +0x18 blend amount, +0x1C blend delta, +0x20 current time, +0x24 speed, +0x2E flags byte.
- END = `this->+0x27 = this->+0x26; this->+0x26 = 0; return`.
- 1. If `move < 0` (signed dword compare, 0x623B19): `this->+0x26 = 0`; return (no +0x27 update).
- 2. If m_pAnim: `m_pAnim->SetDeleteCallback(CDefaultAnimCallback 0x4D1390, 0)`; `m_pAnim = 0`.
- 3. `cmd = (i8)this->+0x26`; if `(u32)cmd > 0x12` END. Byte table 0x624098 -> jump table 0x624084: cmd 0 -> IDLE, 1 and 15..18 ->
  MOVE, 2 -> BLOCK, 3..10 -> PLAIN, 11..14 -> ATTACK.
- ATTACK (0x623B59): `+0x25 = (i8)move`; `+0x14 = 1`; `a = BlendAnimation(ped->+0x18, CD(+0x24).group, 0xD6 + (i8)move, 8.0f)`; `m_pAnim = a`;
  `a->SetFinishCallback(0x61DAE0, this)`. If `+0x25 == 3` and `a->+0x20 != 0.0f` (`test ah,0x44; jnp` skips only on equal; NaN sets
  the time): `a->SetCurrentTime(CD(+0x24).+0x44)`. If `ped->+0x480`: `a->+0x24 = GetFatAndMuscleModifier(3)` (cdecl, ST0); if
  `(i8)+0x25 < 4` `ped->SetMoveState(1)`. END.
- IDLE (0x623C1C): `+0x24 = 0; +0x25 = 0; +0x14 = 0`. If `ped->+0x534 >= 4` and ped->IsPlayer(): `+8 = 1` (finished); END.
  `k = GetWeaponInfo(ped weapon type of slot +0x718, 1)->+0x6E` (u8); if k == 4 `k = ped->+0x72D`.
  If `!(CD(k).+0x85 & 4)`: `c = 4`. Else `g = CD(k).group`: if g == 0x21 `c = k`; else if g != `+0xC`: { if `+0xA`: RemoveAnimBlockRef(
  (ms_aAnimAssocGroups[+0xC].+0 - 0xB5D4A0) >> 5) (groups at [0xB4EA34], stride 0x14); `+0xA = 0` }; `+0xC = g`; LOAD } else if `+0xA`
  `c = k` else LOAD. LOAD (0x623CEC): `b = ms_aAnimAssocGroups[+0xC].+0` or, if 0, `GetAnimationBlock(GetAnimBlockName(+0xC))`;
  `i = (b - 0xB5D4A0) >> 5`; if `b->+0x10` (loaded): AddAnimBlockRef(i); `+0xA = 1`; `c = k`; else `RequestModel(i + 0x63E7, 8)`; if
  `+0x26` in 11..14 `+0x26 = 11`; `c = 4`.
  `+0x24 = c` (0x623D33). If m_pIdleAnim == 0: `m_pIdleAnim = BlendAnimation(ped->+0x18, CD(c).group, 0xDF, 8.0f)` and
  SetDeleteCallback(0x61DAE0, this). Else if `idle->+0x18 < 1.0f` and `idle->+0x1C <= 0.0f` (each `jp` exits on NaN):
  `m_pIdleAnim = BlendAnimation(..., CD(c).group, 0xDF, 8.0f)` (no callback). If ped->IsPlayer(): `pd = ped->+0x480; pd->+0xC = 0;
  pd->+0x10 = 0; sub_61C9B0(this, ped)`. Then `+0x24 = 0` (0x623E12, overwrites c) and END.
- PLAIN (0x623E21, cmd 3..10): if ped->IsPlayer() END. `+0x14 = 0; +0x24 = 1`. cmd 7: `+0x25 = 0`, id 0x2D; cmd 9: `+0x25 = 2`, id 0x2E;
  cmd 8: `+0x25 = 1`; cmd 10: `+0x25 = 2`; other: `+0x25 = cmd - 3`; for 3..6, 8, 10 the id is `0x29 + (i8)+0x25`.
  `m_pAnim = BlendAnimation(ped->+0x18, 0, id, 8.0f)`. If `+0x26 == 3`: SetDeleteCallback(0x61DAE0, this); `a->+0x2E |= 4`. Else
  `a->+0x2E &= ~2; a->+0x2E &= ~0x20; a->+0x2E |= 8`; SetFinishCallback(0x61DAE0, this). END.
- BLOCK (0x623EF7, cmd 2): `+0x25 = 0; +0x14 = 0`; if `!(CD(+0x24).+0x85 & 2)` `+0x24 = 4`; `m_pAnim = BlendAnimation(ped->+0x18,
  CD(+0x24).group, 0xDE, 8.0f)`; SetFinishCallback(0x61DAE0, this). END.
- MOVE (0x623F68, cmd 1, 15..18): `pd = ped->+0x480`. If pd: `pd->+0xC = 0; pd->+0x10 = 0; sub_61C9B0(this, ped)`; cmd 0x11: `pd->+0x14 =
  2.0f`; cmd 0x10: `pd->+0x14 = 1.0f`. If !pd: `ped->SetMoveState(cmd == 0x11 ? 6 : cmd == 0x10 ? 4 : 1)`; `ped->+0x538 = ped->+0x534`.
  Anim (all `BlendAnimation(ped->+0x18, group, id, delta)`): 0x11 -> (m_nAnimGroup, 1, 8.0f); 0x10 -> (m_nAnimGroup, 0, 8.0f);
  0xF -> (m_nAnimGroup, 3, 4.0f); 0x12 and `ped->+0x46C & 0x4000000` and `GetTaskDuck(1)` != 0 -> (0, 0x37, 4.0f); otherwise
  (m_nAnimGroup, 3, 2.0f). Result not stored. Then if m_pIdleAnim `idle->+0x2E &= ~1` else `+8 = 1`. `+0x26 = 0x10`; END (so +0x27
  becomes 0x10 for every MOVE command).
- gta-reversed: absent (no hook). Open questions: the role of sub_61C9B0 (not specified here).

## 0x6240B0 sub_6240b0 (proposed: CTaskSimpleFight::FightStrike(CPed* ped, CVector& pos))  (catalogue: unknown/REA, port level 19)
- Name proposal: CTaskSimpleFight::FightStrike. Evidence: callers CTaskSimpleFight::ProcessPed 0x629920 and CTaskSimpleUseGun::ProcessPed
  0x62A380 (pistol-whip); `this` = CTaskSimpleFight (+0x24 combo set, +0x25 current move, +0x1C m_pAnim); args (CPed*, CVector&) match gr's
  `void FightStrike(CPed* ped, CVector& posn)`; it calls the three hit handlers 0x61CBA0 (ped), 0x61D0B0 (car), 0x61D400 (object)
  (= gr FightHitPed / FightHitCar / FightHitObj by argument shapes) and 0x61D5F0 (FightSetUpCol(float)).
- `__thiscall bool (this, CPed* ped [esp+4], CVector* pos [esp+8])`, `ret 8`, SEH frame, `sub esp,0x16C`. Returns AL = 0 always
  (0x6246F7). `pos` is modified in place (see step 4). CD = `m_aComboData[this->+0x24 - 4]` (0xC170D0, stride 0x88): +0x30 fRadius[5]
  (f32), byte `CD+0x50+move` = hit level of the move (move = (i8)this->+0x25), `fR = CD.fRadius[move]`.
- Locals: `missed` byte = 1; `hitPed` = 0; object list `objs[16]` and its count `nObj` (i16) = 0; CMatrix `M` (copy of ped->m_matrix);
  a second CMatrix local whose attach pointer/own flag (+0x40/+0x44) are zeroed at 0x62410F (inlined default ctor); both destroyed at
  exit with CMatrix dtor 0x59ACD0.
- 1. (0x6240D6) `GetWeaponInfo(active weapon type, 1)` 0x743C60 is called and the result is unused. If `ped == CWorld::FindPlayerPed(-1)`
  0x56E210 and the active slot weapon type (`ped->+0x5A0 + 0x1C*(i8)ped->+0x718`) != 0: `CGlass::BreakGlassPhysically(*pos (by value),
  fR)` 0x71CF50.
- 2. `I = ped->m_pIntelligence (+0x47C)`. If ped->IsPlayer(): `CWorld::FindObjectsInRange(*pos, 5.0f, true, &nObj, 16, objs, false, false,
  false, true, false)` 0x564A20. `FightSetUpCol(this, fR)` 0x61D5F0. `M = CMatrix(*ped->+0x14)` 0x59BCF0; `M.SetTranslateOnly(pos->x,
  pos->y, pos->z)` 0x59AF80.
- 3. Loop `i = 0; i < nObj + 32` (nObj re-read as i16 every iteration): `P = i < 16 ? I->+0x130[i] : 0` (ped), `V = 16 <= i < 32 ?
  I->+0xA0[i] (i.e. I->+0xE0[i-16]) : 0` (vehicle), `O = i >= 32 ? objs[i-32] : 0`; `e` = the non-null one; e == 0 -> next.
  - `r = ms_modelInfoPtrs[(i16)e->+0x22]->+0x14 (col model)->+0x24 (bounding sphere radius) + fR` (stored f32).
  - P: if hit level >= 4 (unsigned byte compare) `r = fR*0.5f [0x858B8C] + r`. Eligible if `P->+0x1C & 1` (uses collision) or
    `!P->IsAlive()` 0x5E0170 or (P in vehicle (+0x46C & 0x100) and `P->+0x58C` and `P->+0x58C->+0x590 == 9` (bike)); otherwise next.
  - V: eligible only if `V->+0x590 == 0` (automobile). O: eligible only if `O->+0x1C & 1`. Otherwise next.
  - `c = e->GetBoundCentre()` 0x534250. P: `dsq = (c.x-pos.x)^2 + (c.y-pos.y)^2` (2D; dx*dx + dy*dy). V/O: `dsq = dx*dx + dy*dy + dz*dz`.
    If `!(dsq < r*r)` (`fcompp`, `test ah,5; jp`: >= and NaN) next.
  - P (0x624393): `cd = CPedModelInfo::AnimatePedColModelSkinnedWorld(mi(P), P->+0x18)` 0x4C7170 `->+0x2C` (collision data); `tries = 0`.
    SPHERES: for `j = 0 .. (i16)cd->+0 - 1`, sphere `s = cd->+8 + 0x14*j` (+0 centre, +0xC radius): `d = s.centre - *pos` (stored as a
    local CVector D = (dx, dy, dz)); `dsq = dx*dx + dy*dy + dz*dz`; `t = s.r + fR`; if `dsq < t*t` (`test ah,5; jnp`): HIT.
    No sphere hit: if hit level < 4 next entity. Else `f = ped->+0x14 forward (+0x10..+0x18)`: `pos->x += f.x*1.5f*fR`, `pos->y +=
    f.y*1.5f*fR`, `pos->z += f.z*1.5f*fR` ([0x858CE8]=1.5f; products f.c*1.5 first, then *fR, stores in order x, y, z); `tries++`;
    if tries < 2 repeat SPHERES else next entity (so the caller's pos can be pushed forward twice per ped).
    HIT (0x6244CC): `h = sub_61CBA0(this, ped, P, pos, &D, 3)` (FightHitPed); if h != 0 `hitPed = h`; `missed = 0`; next entity.
  - V/O (0x6244FC): if `e->+0x14 == 0`: `AllocateMatrix` 0x54F560 and `e->+4 .UpdateMatrix(e->+0x14)` 0x54F1B0. `n =
    CCollision::ProcessColModels(&M, col1 [0xC17824], e->+0x14, e->GetColModel() 0x535300, CWorld::m_aTempColPts 0xB9ACD0, 0, 0, 0)`
    0x4185C0. If n > 0: V -> `sub_61D0B0(this, ped, V, &cp.pos (0xB9ACD0), &cp.normal (0xB9ACE0), (u16)(u8)[0xB9ACF4], (u8)[0xB9ACF3])`
    (FightHitCar; the u16 is pushed in a 32-bit slot whose upper half is stale); O -> `sub_61D400(this, ped, O, same args)` (FightHitObj).
    `missed` is not cleared.
- 4. After the loop (also reached directly when `nObj + 32 <= 0`): if missed and ped->IsPlayer(): local `CEventSoundQuiet(ped, 40.0f, -1,
  &(0,0,0))` 0x5E05B0; `RwHelper::GetEventGlobalGroup()` 0x4ABA50 `->Add(&ev, false)` 0x4AB420; dtor 0x5DEA00.
- 5. If `this->+0x24 == 7` and `this->+0x25 == 1` and m_pAnim: if `hitPed == 0` or `RpAnimBlendClumpGetAssociation(hitPed->+0x18, 0xDC)`
  0x4D68B0 == 0: `m_pAnim->+0x1C = -4.0f`; `m_pAnim->+0x2E &= ~1`; `m_pAnim->+0x2E |= 4`.
- 6. `ped->+0x720 = *pos` (3 dwords). Destroy both matrices, return 0.
- gta-reversed: absent. Open questions: none on control flow; the meaning of I->+0x130 / +0xE0 lists is inferred (ped / vehicle
  scanner entity arrays of CPedIntelligence).

## 0x624710 sub_624710 (proposed: CTaskSimpleFight::ChooseAttackPlayer(CPed* ped))  (catalogue: unknown/REA, port level 8)
- Name proposal: CTaskSimpleFight::ChooseAttackPlayer. Evidence: only caller CTaskSimpleFight::ProcessPed 0x629920 (4 sites); same
  combo-flag logic as its sibling 0x624A40 (which is the AI variant: rand(), target wanted level), but this one has no randomness, uses
  the command history (+0x26/+0x27) and turns the ped towards the nearest ped (player behaviour). gr declares `void ChooseAttackPlayer
  (CPed*)`; the bytes return a value (gr signature is wrong).
- `__thiscall int16 (this, CPed* ped [esp+4])`, `ret 4`; result in AX (on the normal path `mov ax, bp`, the upper half of EAX is stale;
  the two early returns set the whole EAX to -1 / 3). Callees: CPed::SetMoveState 0x5DEC00, CPed::IsAlive 0x5E0170, sub_61D6F0
  (thiscall (this, ped) -> bool; proposed CTaskSimpleFight::FindTargetOnGround: it calls CLocalisation::KickingWhenDown first).
- Globals: `[0x8D2E48]` i32 (initial value 2, max chain count), `[0x8D2E8C]` f32 (initial 2.0, facing search range), both .data
  (writable tunables; names unknown). `CDflags = byte at 0xC17154 + 0x88*(cs-4)` (low byte of CMeleeInfo flags), cs = (i8)this->+0x24.
- 1. `noTarget = (this->+0x18 == 0)` (the `= 1` store is in the stub 0x40377E). `r = -1`; unless (`+0x26` in 11..14 and `cs >= 4`)
  `r = 1`. `f = CDflags(cs)`; if `4 < cs <= 7` `f &= ped->+0x72E`. If r >= 0 (i.e. r == 1) go to FACE.
- 2. If `this->+0x1C` (m_pAnim) and `+0x25 != 3` and `+0x25 != 4`:
  - `ped->SetMoveState(1)`; `max = (f & 2) ? 2 : (f & 1) ? 1 : 0`. If `sub_61D6F0(ped)`: r = -1, FACE.
  - If `+0x26 != +0x27`: if `(i8)+0x15 > [0x8D2E48]` go to SAME; else `+0x15 += 1`; `t = (i8)+0x25 - (i8)+0x27 + (i8)+0x26` (low byte);
    `r = (t & 1) ? 0 : ((t & 2) ? 1 : 2)`; if `(i16)r <= (i16)max` FACE; else r = 0, FACE.
  - SAME (0x62481A, also when +0x26 == +0x27): `r = (i8)+0x25 + 1`; if `r <= max` FACE; else r = -1, FACE.
- 3. Else (no anim, or move 3/4): if `ped->+0x534 (m_nMoveState) > 4`: if `!(f & 8)` `+0x24 = 4`; r = 4; FACE.
  If `sub_61D6F0(ped)`: if m_pAnim and `+0x26 != +0x27` return EAX = -1; else { if `!(f & 4)` `+0x24 = 4` else if target (+0x18) is a
  ped ((+0x36 & 7) == 3) with `+0x46C & 0x4000000` (ducking) `+0x24 = 4` }; return EAX = 3 (no FACE).
  Else `+0x15 = 0`; r = 0; FACE.
- FACE (0x6248B4): if !noTarget return r. Else `best = -1000.0f`, `bestAbs = 1000.0f`; for i = 0..15: `p = ped->m_pIntelligence->+0x130[i]`;
  skip if 0 or !p->IsAlive(). `P` = ped position (matrix pos if ped->+0x14 else +4), `Q` = p position. `d = Q - P`; `dsq = dz*dz + dy*dy
  + dx*dx` (x87: dz^2, + dy^2, + dx^2); skip unless `dsq < R*R` with `R = [0x8D2E8C]` (fld R, fmul R, fcompp, `test ah,0x41; jne`).
  `a = atan2(-(Q.x - P.x), Q.y - P.y)` (fpatan with ST1 = -(dx) (fchs), ST0 = dy); `diff = a - ped->+0x558` (m_fCurrentRotation);
  if `diff > PI [0x858CB8]` `diff -= 2PI [0x858CBC]` else if `diff < -PI [0x858CC0]` `diff += 2PI`; `diff = fabs(diff)`; if `diff < bestAbs`
  { bestAbs = diff; best = a } (both stored f32).
  After the loop: if `best > -10.0f [0x859004]` (strict) `ped->+0x55C = best` (m_fAimingRotation). Return r.
- gta-reversed: absent (declared only). Open questions: none.

## 0x624A40 sub_624a40 (proposed: CTaskSimpleFight::ChooseAttackAI(CPed* ped))  (catalogue: unknown/REA, port level 8)
- Name proposal: CTaskSimpleFight::ChooseAttackAI. Evidence: only caller CTaskSimpleFight::ProcessPed 0x629920; random attack selection
  (rand) gated by the combo flags and the target player's wanted level; sibling of 0x624710 (player variant). gr declares `void
  ChooseAttackAI(CPed*)` but the bytes return a value in AX.
- `__thiscall int16 (this, CPed* ped [esp+4])`, `ret 4`; result in AX only (upper EAX stale).
- 1. `cs = (i8)this->+0x24`; `f = byte 0xC17154 + 0x88*(cs-4)`; if `4 < cs <= 7` `f &= ped->+0x72E`.
- 2. `r = (f32)rand() * [0x858C7C]=3.0518509e-05f` (rand 0x821B1E, full int, fild; stored f32 into the ped argument slot - ped is kept
  in EDI).
- 3. If `r > 0.8f [0x858C98]` (strict; NaN no) and `f & 2`: return 2 unless (`f & 0x40` and target `t = this->+0x18` is a ped
  ((t->+0x36 & 7) == 3) with `t->+0x480 != 0` and `CPlayerPed::GetWantedLevel(t)` 0x41BE60 > 0 and `t->+0x540 > 20.0f [0x858BA4]`), in
  which case go to step 5.
- 4. Else: if `sub_61D6F0(this, ped)` (FindTargetOnGround) and `f & 4` return 3.
- 5. If `r > 0.5f [0x858B8C]` and `f & 1` return 1; return 0.
- gta-reversed: absent. Open questions: none.

## 0x5E9850 sub_5e9850 (proposed: std::vector<CVector>::size)  (catalogue: unknown/REA, port level 0)
- Name proposal: MSVC 7.1 STL vector layout (+4 _Myfirst, +8 _Mylast, +0xC _Myend; element size 12 via the magic divide 0x2AAAAAAB,
  `sar 1`); only caller is the _Insert_n 0x5ED750 of the same vector, whose user is CPedShelterAttractor::GetDisplacement 0x5EF420.
- `__thiscall uint32 (this)`, plain `ret`: returns 0 if `this->+4 == 0`, else `(this->+8 - this->+4) / 12` (signed division rounded
  toward zero).

## 0x5ED750 sub_5ed750 (proposed: std::vector<CVector>::_Insert_n)  (catalogue: unknown/REA, port level 6)
- Name proposal: MSVC 7.1 `vector<CVector>::_Insert_n(iterator where, size_type count, const CVector& val)`; callers: insert 0x5EDE50.
  Element size 12; max_size 0x15555555; layout as in 0x5E9850.
- `__thiscall void (this, CVector* where [ebp+8], u32 count [ebp+0xC], const CVector* val [ebp+0x10])`, `ret 0xC`, EBP frame + SEH
  (rollback/free on exception). `v = *val` is copied to a local first (aliasing-safe). `cap = first ? (end - first)/12 : 0`,
  `size = first ? (last - first)/12 : 0`.
- If count == 0 return. If `0x15555555 - size < count`: `_Xlen()` 0x5ECC30 (throws length_error) and return.
- Reallocate if `cap < size + count`: `newcap = (0x15555555 - cap/2 < cap) ? 0 : cap + cap/2`; if `newcap < size + count` `newcap =
  size() 0x5E9850 + count`. `p = operator new(newcap*12)` 0x82119A. `q = uninitialized_copy(first, where, p)` 0x5EAEA0;
  `uninitialized_fill_n(q, count, v)` 0x5EB340; `uninitialized_copy(where, last, q + count)` 0x5EAEA0; `n = size + count` (old size
  recomputed); `free(first)` 0x8214BD if non-null; `end = p + newcap`, `last = p + n`, `first = p`.
- Else in place: `after = (last - where)/12`. If `after < count`: `_Ucopy(where, last, where + count)` 0x5EB9D0; `_Ufill(last, count -
  after, v)` 0x5EC140; `last += count`; `fill(where, oldLast, v)` 0x5EB1E0. Else: `last = _Ucopy(last - count, last, last)`;
  `copy_backward(where, oldLast - count, oldLast)` 0x5EB220; `fill(where, where + count, v)`.
- Standard semantics; port as a Vec<CVector> insert with the same capacity growth (1.5x) if allocation identity matters.

## 0x5EDE50 sub_5ede50 (proposed: std::vector<CVector>::insert(iterator, const CVector&))  (catalogue: unknown/REA, port level 7)
- Name proposal: MSVC 7.1 single-element insert; caller push_back 0x5EE370.
- `__thiscall CVector** (this, CVector** ret [esp+4], CVector* where [esp+8], const CVector* val [esp+0xC])`, `ret 0xC`.
  `off = (first != 0 && size != 0) ? (where - first)/12 : 0`; `_Insert_n(where, 1, val)` 0x5ED750; `*ret = this->+4 (first, re-read) +
  12*off`; returns ret in EAX.

## 0x5EE370 sub_5ee370 (proposed: std::vector<CVector>::push_back)  (catalogue: unknown/REA, port level 8)
- Name proposal: MSVC 7.1 push_back; only caller CPedShelterAttractor::GetDisplacement 0x5EF420.
- `__thiscall void (this, const CVector* v [esp+4])`, `ret 4`. If `first != 0` and `size < capacity` (unsigned): `uninitialized_fill_n(
  last, 1, v, ...)` 0x5EB340 (cdecl, 5 pushes: last, 1, v, this, v); `last += 12`. Else `insert(&tmp, last, v)` 0x5EDE50.

## 0x5F13A0 sub_5f13a0 (proposed: CPointRoute::RemovePoint(int32 index))  (catalogue: unknown/REA, port level 0)
- Name proposal: `this` = {int32 count at +0; CVector points[] at +4, stride 0xC} (gr CPointRoute layout); callers
  CTaskComplexFollowPedFootsteps::ControlSubTask 0x695090 and ::CreateSubTask 0x695E40 (footstep route).
- `__thiscall void (this, int32 i [esp+4])`, `ret 4`: for `k = i; k < count - 1; k++` (signed; count re-read each iteration):
  `points[k] = points[k+1]` (3 dword copies); then `count -= 1` (always, even when i >= count). No bounds check (negative i is not
  rejected).

## 0x5FF490 vt_86cba0::CloneEditable (proposed: CEventHealthLow::CloneEditable)  (catalogue: unknown/REA, port level 2)
- Name proposal: slot 16 (CloneEditable) of event vtable 0x86CBA0 whose GetEventType identifies CEventHealthLow.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEvent::operator new(0x14)` 0x4B5620; if p:
  `CEventEditableResponse::CEventEditableResponse(p)` 0x4AC450, `p->vtbl = 0x86CBA0`; return p (0 on failure). `this` is not read (no
  fields are copied, so the clone gets the default editable-response fields).

## 0x5FF550 vt_86cbe8::CloneEditable (proposed: CEventHealthReallyLow::CloneEditable)  (catalogue: unknown/REA, port level 2)
- Name proposal: slot 16 (CloneEditable) of event vtable 0x86CBE8 whose GetEventType identifies CEventHealthReallyLow.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEvent::operator new(0x14)` 0x4B5620; if p:
  `CEventEditableResponse::CEventEditableResponse(p)` 0x4AC450, `p->vtbl = 0x86CBE8`; return p (0 on failure). `this` is not read (no
  fields are copied, so the clone gets the default editable-response fields).

## 0x5FF610 vt_86cc30::CloneEditable (proposed: CEventLowAngerAtPlayer::CloneEditable)  (catalogue: unknown/REA, port level 2)
- Name proposal: slot 16 (CloneEditable) of event vtable 0x86CC30 whose GetEventType identifies CEventLowAngerAtPlayer.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEvent::operator new(0x14)` 0x4B5620; if p:
  `CEventEditableResponse::CEventEditableResponse(p)` 0x4AC450, `p->vtbl = 0x86CC30`; return p (0 on failure). `this` is not read (no
  fields are copied, so the clone gets the default editable-response fields).

## 0x5FF6D0 vt_86cc78::CloneEditable (proposed: CEventHighAngerAtPlayer::CloneEditable)  (catalogue: unknown/REA, port level 2)
- Name proposal: slot 16 (CloneEditable) of event vtable 0x86CC78 whose GetEventType identifies CEventHighAngerAtPlayer.
- `__thiscall CEventEditableResponse* (this)`, plain `ret`, SEH frame: `p = CEvent::operator new(0x14)` 0x4B5620; if p:
  `CEventEditableResponse::CEventEditableResponse(p)` 0x4AC450, `p->vtbl = 0x86CC78`; return p (0 on failure). `this` is not read (no
  fields are copied, so the clone gets the default editable-response fields).

## 0x609080 sub_609080 (proposed: CEventNewGangMember::~CEventNewGangMember (non-deleting))  (catalogue: unknown/REA, port level 1)
- Name proposal: only caller is the scalar deleting dtor CEventNewGangMember::~CEventNewGangMember 0x609060; stores the vtable 0x86D0A0.
- `__thiscall void (this)`, plain `ret`, SEH frame: `this->vtbl = 0x86D0A0`; if `this->+0xC` (member ped) != 0
  `CEntity::CleanUpOldReference(this->+0xC, &this->+0xC)` 0x571A00; base dtor 0x4AC010 (CEvent::~CEvent: only `this->vtbl = 0x85AB38`).
