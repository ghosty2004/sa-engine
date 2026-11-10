# CTaskComplexWanderProstitute (AI) - RE-SPEC-3 group I3

Vtable 0x870148 (gr TaskComplexWanderProstitute.h): CTaskComplexWanderStandard (0x38), +0x38 m_nStartTimeInMs (u32; used as the next-scan time).

## 0x672690 CTaskComplexWanderProstitute::CTaskComplexWanderProstitute  (catalogue: specified/REA, port level 3)
- `__thiscall this (eMoveState moveState [esp+4], uint8 dir [esp+8], bool bWanderSensibly [esp+0xC])`, `ret 0xC`, returns this.
- Behaviour: `CTaskComplexWanderStandard::CTaskComplexWanderStandard(this, moveState, dir, bWanderSensibly)` 0x48E4F0 (args forwarded in order);
  `vtbl = 0x870148`; `+0x38 = 0` (0x6726A9-0x6726AF). gr TaskComplexWanderProstitute.cpp:5 - matches.

## 0x672700 CTaskComplexWanderProstitute::ScanForStuff  (catalogue: specified/REA, port level 40)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`, SEH; one event local (CEventAcquaintancePed-sized) at [esp+0x14] after the pushes. Slot 12.
- Callees: CTaskComplexWanderStandard::ScanForStuff 0x672600; CPedIntelligence::GetPedEntities 0x4893E0 (thiscall, returns &m_pedScanner.m_apEntities);
  _rand 0x821B1E; _ftol 0x821B40; CTaskComplexProstituteSolicit::IsTaskValid 0x661BB0 (cdecl (ped, other)); CEventAcquaintancePed ctor 0x4AF820 /
  ~CEventAcquaintancePed 0x4AF890; CEventGroup::Add 0x4AB420.
- Behaviour:
  1. `CTaskComplexWanderStandard::ScanForStuff(this, ped)` (always, 0x672721).
  2. If `now [0xB7CB84] <= m_nStartTimeInMs (+0x38)` return (unsigned `jbe`, 0x67272B). Else `+0x38 = now + 2000` (0x672734-0x67273A).
  3. `intel = ped->m_pIntelligence`; `ents = intel->GetPedEntities()`. First loop i = 0..15: if `ents[i] != 0 && ents[i]->m_nPedType (+0x598) == 6`
     (a cop nearby) return immediately (0x672750-0x672769).
  4. Second loop j = 0..15 (0x672770-0x67287D), `p = ents[j]`; skip unless all of, in order: p != 0; `p->m_pPlayerData` (+0x480) != 0;
     `p->m_pVehicle` (+0x58C) != 0; `p->m_pPlayerData->m_pLastProstituteShagged (+0xA8) != ped`. Then
     `r = _ftol(((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 50000.0f [0x870184])` (0..49998, truncation; one rand per candidate that
     reached here); continue only if `p->m_pVehicle->m_pHandlingData (+0x384)->m_nMonetaryValue (+0xD8) > r` (unsigned, `jbe` skips, 0x6727D7).
     Speed test (0x6727E3-0x67281F), v = vehicle m_vecMoveSpeed (+0x44): `sz = v.z * 50.0f [0x858B40]; sy = v.y * 50.0f; sx = v.x * 50.0f;
     s = (sx*sx + sy*sy) + sz*sz`; continue only if `s < 4.0f [0x86F848]` (strict; NaN skips). Then `IsTaskValid(ped, p)` must be true.
     Event: `CEventAcquaintancePed::CEventAcquaintancePed(&ev, p)`, `ev.vtbl = 0x86CA38` (CEventAcquaintancePedRespect, type 39),
     `ev.m_TaskId (+0xE) = 0x51D` (1309 TASK_COMPLEX_PROSTITUTE_SOLICIT), `intel->m_eventGroup (+0x68).Add(&ev, false)`; then `ev.vtbl = 0x86CA38`
     and ~CEventAcquaintancePed 0x4AF890. The loop CONTINUES after an event (several clients may be added in one scan).
- gta-reversed: `plugin::CallMethod<0x672700>` - not reversed; spec from bytes.

## 0x674920 CTaskComplexWanderProstitute::CreateFirstSubTask  (catalogue: specified/REA, port level 8)
- `__thiscall CTask* (this, CPed* ped [esp+4])`: single `jmp 0x6740E0` (CTaskComplexWander::CreateFirstSubTask; its `ret 4` returns). Slot 9 of 0x870148.
  gr TaskComplexWanderProstitute.h:14 - matches.
